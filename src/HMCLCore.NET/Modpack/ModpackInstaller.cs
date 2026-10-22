using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Serialization;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Modpack;

/// <summary>
/// Reads modpack archives without extracting them: detects the pack format
/// (CurseForge manifest.json vs Modrinth modrinth.index.json) and returns its
/// metadata for preview and for driving the installation task.
/// </summary>
public static class ModpackReader
{
    /// <summary>
    /// Loads modpack metadata from a local zip/mrpack archive.
    /// </summary>
    /// <param name="archivePath">The archive to read.</param>
    /// <returns>The parsed modpack, or null when the archive has no recognizable manifest.</returns>
    public static Modpack? Load(string archivePath)
    {
        try
        {
            if (!File.Exists(archivePath))
                return null;

            using var archive = ZipFile.OpenRead(archivePath);

            // Modrinth packs ship modrinth.index.json; CurseForge packs manifest.json.
            var modrinthEntry = archive.GetEntry("modrinth.index.json");
            var curseForgeEntry = archive.GetEntry("manifest.json");
            var mcbbsEntry = archive.GetEntry("mcbbs.pack-manifest.json");
            var multiMcEntry = archive.GetEntry("mmc-pack.json");

            if (modrinthEntry != null)
                return ReadModrinth(modrinthEntry, archivePath);

            if (curseForgeEntry != null)
                return ReadCurseForge(curseForgeEntry, archivePath);

            if (mcbbsEntry != null)
                return ReadMcbbs(mcbbsEntry, archivePath);

            if (multiMcEntry != null)
                return ReadMultiMc(multiMcEntry, archivePath);

            // A server pack holds the files of an instance without a manifest.
            if (archive.Entries.Any(entry => entry.FullName.StartsWith("mods/", StringComparison.OrdinalIgnoreCase)))
                return ReadServerPack(archivePath);

            return null;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Parses a Modrinth-style index entry into modpack metadata.
    /// </summary>
    private static Modpack? ReadModrinth(ZipArchiveEntry indexEntry, string archivePath)
    {
        using var stream = indexEntry.Open();
        var index = JsonSerializer.Deserialize(stream, LauncherJsonContext.Default.ModrinthPackIndex);
        if (index == null)
            return null;

        var modpack = new Modpack
        {
            Name = index.Name ?? Path.GetFileNameWithoutExtension(archivePath),
            Version = index.VersionId,
            Description = index.Summary,
            FilePath = archivePath,
            GameVersion = index.Dependencies?.GetValueOrDefault("minecraft"),
            Manifest = new ModpackManifest
            {
                MinecraftVersion = index.Dependencies?.GetValueOrDefault("minecraft") ?? string.Empty,
                ModLoader = index.Dependencies?.FirstOrDefault(d =>
                    !d.Key.Equals("minecraft", StringComparison.OrdinalIgnoreCase)).Value
            }
        };

        foreach (var file in index.Files ?? new List<ModrinthPackFile>())
        {
            if (string.IsNullOrEmpty(file.Path))
                continue;

            // The files which are only used by the server are optional, so
            // that the user may skip them.
            var serverOnly = file.Env is { Count: > 0 } &&
                             file.Env.All(environment =>
                                 environment.Equals("server", StringComparison.OrdinalIgnoreCase));

            modpack.Manifest.Files.Add(new ModpackFileTransfer
            {
                Path = file.Path,
                Url = file.Downloads?.FirstOrDefault() ?? string.Empty,
                Sha1 = file.Hashes?.GetValueOrDefault("sha1") ?? string.Empty,
                Optional = serverOnly
            });
        }

        return modpack;
    }

    /// <summary>
    /// Parses a MCBBS manifest entry into modpack metadata.
    /// </summary>
    /// <param name="manifestEntry">The manifest of the pack.</param>
    /// <param name="archivePath">The path of the archive.</param>
    /// <returns>The parsed modpack.</returns>
    private static Modpack ReadMcbbs(ZipArchiveEntry manifestEntry, string archivePath)
    {
        using var stream = manifestEntry.Open();
        var manifest = JsonSerializer.Deserialize(stream, LauncherJsonContext.Default.McbbsPackManifest);

        var gameVersion = manifest?.Addons?
            .FirstOrDefault(addon => string.Equals(addon.Id, "game", StringComparison.OrdinalIgnoreCase))
            ?.Version ?? string.Empty;

        return new Modpack
        {
            Name = manifest?.Name ?? Path.GetFileNameWithoutExtension(archivePath),
            Version = manifest?.Version,
            Author = manifest?.Author,
            Description = manifest?.Description,
            FilePath = archivePath,
            GameVersion = gameVersion,
            Layout = ModpackLayout.Root,
            Manifest = new ModpackManifest
            {
                ManifestVersion = manifest?.ManifestVersion ?? 1,
                MinecraftVersion = gameVersion
            }
        };
    }

    /// <summary>
    /// Parses a MultiMC manifest entry into modpack metadata.
    /// </summary>
    /// <param name="manifestEntry">The manifest of the pack.</param>
    /// <param name="archivePath">The path of the archive.</param>
    /// <returns>The parsed modpack.</returns>
    private static Modpack ReadMultiMc(ZipArchiveEntry manifestEntry, string archivePath)
    {
        using var stream = manifestEntry.Open();
        var manifest = JsonSerializer.Deserialize(stream, LauncherJsonContext.Default.MultiMcPackManifest);

        var gameVersion = manifest?.Components?
            .FirstOrDefault(component => string.Equals(component.Uid, "net.minecraft", StringComparison.OrdinalIgnoreCase))
            ?.Version ?? string.Empty;

        var loader = manifest?.Components?
            .FirstOrDefault(component => component.Uid != null && component.Uid.Contains("loader", StringComparison.OrdinalIgnoreCase));

        return new Modpack
        {
            Name = manifest?.Name ?? Path.GetFileNameWithoutExtension(archivePath),
            Version = manifest?.Version,
            Author = manifest?.Author,
            Description = manifest?.Description,
            FilePath = archivePath,
            GameVersion = gameVersion,
            Layout = ModpackLayout.Root,
            Manifest = new ModpackManifest
            {
                ManifestVersion = manifest?.FormatVersion ?? 1,
                MinecraftVersion = gameVersion,
                ModLoader = loader?.Uid
            },
            MultiMcSettings = manifest is null
                ? null
                : new MultiMcInstanceSettings
                {
                    JavaPath = manifest.JavaPath,
                    JvmArgs = manifest.JvmArgs,
                    WrapperCommand = manifest.WrapperCommand,
                    PreLaunchCommand = manifest.PreLaunchCommand,
                    PostExitCommand = manifest.PostExitCommand,
                    MaxMemory = manifest.MaxMemory,
                    MinMemory = manifest.MinMemory,
                    PermGen = manifest.PermGen,
                    Width = manifest.Width,
                    Height = manifest.Height,
                    Fullscreen = manifest.Fullscreen,
                    ShowConsole = manifest.ShowConsole,
                    OverrideMemory = manifest.OverrideMemory,
                    OverrideJavaLocation = manifest.OverrideJavaLocation,
                    OverrideJavaArgs = manifest.OverrideJavaArgs,
                    OverrideConsole = manifest.OverrideConsole,
                    OverrideCommands = manifest.OverrideCommands,
                    OverrideWindow = manifest.OverrideWindow
                }
        };
    }

    /// <summary>
    /// Reads a server pack, which holds the files of an instance without any
    /// manifest.
    /// </summary>
    /// <param name="archivePath">The path of the archive.</param>
    /// <returns>The parsed modpack.</returns>
    private static Modpack ReadServerPack(string archivePath) => new()
    {
        Name = Path.GetFileNameWithoutExtension(archivePath),
        FilePath = archivePath,
        Layout = ModpackLayout.Root,
        Manifest = new ModpackManifest { ManifestVersion = 1 }
    };

    /// <summary>
    /// Parses a CurseForge manifest entry into modpack metadata.
    /// </summary>
    private static Modpack? ReadCurseForge(ZipArchiveEntry manifestEntry, string archivePath)
    {
        using var stream = manifestEntry.Open();
        var manifest = JsonSerializer.Deserialize(stream, LauncherJsonContext.Default.CurseForgePackManifest);
        if (manifest?.Minecraft == null)
            return null;

        var primaryLoader = manifest.Minecraft.ModLoaders?.FirstOrDefault(l => l.Primary == true) ??
                            manifest.Minecraft.ModLoaders?.FirstOrDefault();

        var modpack = new Modpack
        {
            Name = manifest.Name ?? Path.GetFileNameWithoutExtension(archivePath),
            Version = manifest.Version,
            Author = manifest.Author,
            FilePath = archivePath,
            GameVersion = manifest.Minecraft.Version,
            Manifest = new ModpackManifest
            {
                ManifestVersion = manifest.ManifestVersion,
                MinecraftVersion = manifest.Minecraft.Version ?? string.Empty,
                ModLoader = primaryLoader?.Id
            }
        };

        foreach (var file in manifest.Files ?? new List<CurseForgePackFile>())
        {
            if (file.ProjectId == null || file.FileId == null)
                continue;

            modpack.Manifest.Mods.Add(new ModpackMod
            {
                ProjectId = file.ProjectId.Value.ToString(),
                FileId = file.FileId.Value,
                Name = file.Name,
                Required = file.Required != false
            });
        }

        return modpack;
    }
}

/// <summary>
/// Extracts a modpack archive into an instance directory: overrides are unpacked
/// in place and Modrinth-style remote files are downloaded when missing.
/// </summary>
public sealed class ModpackInstallTask : TaskBase
{

    // Archive prefixes that map into the instance directory, per pack format.
    private static readonly string[] OverridePrefixes =
    {
        "overrides/", "clientOverrides/", "client-overrides/"
    };

    private readonly string _archivePath;
    private readonly string _instanceDirectory;
    private readonly Modpack _modpack;

    /// <summary>
    /// Optional resolver turning CurseForge project/file ids into download URLs;
    /// absent when no API key is configured.
    /// </summary>
    private readonly Func<int, int, Task<string?>>? _curseForgeFileResolver;

    private int _extractedFiles;
    private int _downloadedFiles;
    private int _resolvedFiles;
    private int _unresolvedFiles;

    /// <summary>
    /// The number of override files extracted from the archive.
    /// </summary>
    public int ExtractedFiles => _extractedFiles;

    /// <summary>
    /// The number of remote files downloaded (Modrinth packs only).
    /// </summary>
    public int DownloadedFiles => _downloadedFiles;

    /// <summary>
    /// The number of CurseForge-referenced mods downloaded through the resolver.
    /// </summary>
    public int ResolvedFiles => _resolvedFiles;

    /// <summary>
    /// The number of CurseForge-referenced mods that could not be resolved.
    /// </summary>
    public int UnresolvedFiles => _unresolvedFiles;

    /// <summary>
    /// The CurseForge-referenced mods that could not be resolved, reported to
    /// the user after the installation.
    /// </summary>
    public IReadOnlyList<UnresolvedMod> UnresolvedMods => _unresolvedMods;

    private readonly List<UnresolvedMod> _unresolvedMods = new();

    /// <summary>
    /// Creates a new ModpackInstallTask.
    /// </summary>
    /// <param name="archivePath">The modpack archive.</param>
    /// <param name="instanceDirectory">The instance root receiving the pack contents.</param>
    /// <param name="modpack">The metadata produced by <see cref="ModpackReader.Load"/>.</param>
    /// <param name="curseForgeFileResolver">
    /// Optional resolver mapping CurseForge (projectID, fileID) pairs to download URLs;
    /// when omitted, referenced mods are counted as unresolved instead of downloaded.
    /// </param>
    /// <param name="excludedFiles">
    /// The keys of the optional files the user chose not to install.
    /// </param>
    public ModpackInstallTask(string archivePath, string instanceDirectory, Modpack modpack,
        Func<int, int, Task<string?>>? curseForgeFileResolver = null,
        IReadOnlySet<string>? excludedFiles = null)
    {
        _archivePath = archivePath ?? throw new ArgumentNullException(nameof(archivePath));
        _instanceDirectory = instanceDirectory ?? throw new ArgumentNullException(nameof(instanceDirectory));
        _modpack = modpack ?? throw new ArgumentNullException(nameof(modpack));
        _curseForgeFileResolver = curseForgeFileResolver;
        _excludedFiles = excludedFiles is null
            ? new HashSet<string>(StringComparer.OrdinalIgnoreCase)
            : new HashSet<string>(excludedFiles, StringComparer.OrdinalIgnoreCase);
        Name = $"安装整合包 {_modpack.Name}";
    }

    /// <summary>
    /// The keys of the optional files the user chose not to install.
    /// </summary>
    private readonly HashSet<string> _excludedFiles;

    /// <summary>
    /// Installs the pack: extracts overrides, then downloads the pack's remote files.
    /// Progress is reported as completed entries against the total entry count.
    /// </summary>
    /// <remarks>
    /// When the installation fails, the files written into the instance
    /// directory are removed, so that a failed installation does not leave a
    /// half-installed instance behind.
    /// </remarks>
    public override async Task Run(CancellationToken token = default)
    {
        var writtenFiles = new List<string>();
        var createdDirectories = new List<string>();

        try
        {
            await InstallAsync(token, writtenFiles, createdDirectories);
        }
        catch
        {
            Rollback(writtenFiles, createdDirectories);
            throw;
        }
    }

    /// <summary>
    /// Installs the content of the pack into the instance directory.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <param name="writtenFiles">The list collecting the written files.</param>
    /// <param name="createdDirectories">The list collecting the created directories.</param>
    /// <returns>A task that completes when the pack is installed.</returns>
    private async Task InstallAsync(
        CancellationToken token, List<string> writtenFiles, List<string> createdDirectories)
    {
        Directory.CreateDirectory(_instanceDirectory);

        using var archive = ZipFile.OpenRead(_archivePath);

        // 1) Extract every override entry into the instance directory.
        var overrideEntries = archive.Entries.Where(IsOverrideEntry).ToList();
        var transfers = _modpack.Manifest?.Files ?? new List<ModpackFileTransfer>();
        var curseForgeMods = _modpack.Manifest?.Mods ?? new List<ModpackMod>();
        var total = overrideEntries.Count + transfers.Count + curseForgeMods.Count;
        var completed = 0L;

        foreach (var entry in overrideEntries)
        {
            token.ThrowIfCancellationRequested();

            // The override prefix maps onto the instance root (skip the pack root itself).
            var relativePath = GetRelativePath(entry);
            if (string.IsNullOrEmpty(relativePath))
            {
                completed++;
                OnProgressChanged(completed, total);
                continue;
            }

            var destination = Path.Combine(_instanceDirectory, relativePath);
            if (string.IsNullOrEmpty(entry.Name))
            {
                // Zero-name entries represent directories in zip archives.
                EnsureDirectory(destination, createdDirectories);
                completed++;
                OnProgressChanged(completed, total);
                continue;
            }

            EnsureDirectory(Path.GetDirectoryName(destination)!, createdDirectories);
            using (var entryStream = entry.Open())
            using (var fileStream = File.Create(destination))
            {
                await entryStream.CopyToAsync(fileStream, token);
            }

            writtenFiles.Add(destination);
            _extractedFiles++;
            completed++;
            OnProgressChanged(completed, total);
        }

        // 2) Download Modrinth-style remote files, reusing matching local files.
        foreach (var transfer in transfers)
        {
            token.ThrowIfCancellationRequested();

            // The optional files the user chose not to install are skipped.
            if (_excludedFiles.Contains(transfer.Path))
            {
                completed++;
                OnProgressChanged(completed, total);
                continue;
            }

            if (string.IsNullOrEmpty(transfer.Path) || string.IsNullOrEmpty(transfer.Url))
            {
                completed++;
                OnProgressChanged(completed, total);
                continue;
            }

            var destination = Path.Combine(_instanceDirectory, transfer.Path);
            EnsureDirectory(Path.GetDirectoryName(destination)!, createdDirectories);

            if (!await IsExistingContentValid(destination, transfer.Sha1))
            {
                using var response = await HttpClientProvider.Shared.GetAsync(
                    transfer.Url, HttpCompletionOption.ResponseHeadersRead, token);
                response.EnsureSuccessStatusCode();
                using var contentStream = await response.Content.ReadAsStreamAsync(token);
                using var fileStream = File.Create(destination);
                await contentStream.CopyToAsync(fileStream, token);

                writtenFiles.Add(destination);
                _downloadedFiles++;
            }

            completed++;
            OnProgressChanged(completed, total);
            Name = $"安装整合包 {_modpack.Name} ({completed}/{total})";
        }

        // 3) Resolve and download CurseForge-referenced mods when a resolver is
        // configured; otherwise each reference counts as unresolved.
        foreach (var mod in curseForgeMods)
        {
            token.ThrowIfCancellationRequested();

            var url = await ResolveCurseForgeFileAsync(mod, token);
            if (string.IsNullOrEmpty(url))
            {
                _unresolvedFiles++;
                _unresolvedMods.Add(new UnresolvedMod
                {
                    ProjectId = mod.ProjectId,
                    FileId = mod.FileId,
                    Name = mod.Name
                });
            }
            else
            {
                var destination = Path.Combine(_instanceDirectory, "mods", GetFileNameFromUrl(url!));
                EnsureDirectory(Path.GetDirectoryName(destination)!, createdDirectories);

                using var response = await HttpClientProvider.Shared.GetAsync(
                    url!, HttpCompletionOption.ResponseHeadersRead, token);
                response.EnsureSuccessStatusCode();
                using var contentStream = await response.Content.ReadAsStreamAsync(token);
                using var fileStream = File.Create(destination);
                await contentStream.CopyToAsync(fileStream, token);

                writtenFiles.Add(destination);
                _resolvedFiles++;
            }

            completed++;
            OnProgressChanged(completed, total);
            Name = $"安装整合包 {_modpack.Name} ({completed}/{total})";
        }

        // Record the pack contents in the instance so a later update can
        // compute the differences against this version.
        WriteConfiguration();
    }

    /// <summary>
    /// Creates the given directory and remembers it, so that it can be
    /// removed when the installation fails.
    /// </summary>
    /// <param name="directory">The directory to create.</param>
    /// <param name="createdDirectories">The list collecting the created directories.</param>
    private static void EnsureDirectory(string directory, List<string> createdDirectories)
    {
        if (string.IsNullOrEmpty(directory) || Directory.Exists(directory))
            return;

        var missing = new List<string>();
        var current = directory;
        while (!string.IsNullOrEmpty(current) && !Directory.Exists(current))
        {
            missing.Add(current);
            var parent = Path.GetDirectoryName(current);
            if (string.IsNullOrEmpty(parent) || parent == current)
                break;

            current = parent;
        }

        Directory.CreateDirectory(directory);

        // The directories are removed in the reverse order, so that the
        // children are removed before their parents.
        missing.Reverse();
        createdDirectories.AddRange(missing);
    }

    /// <summary>
    /// Removes the files and the directories written by a failed installation.
    /// </summary>
    /// <param name="writtenFiles">The files written by the installation.</param>
    /// <param name="createdDirectories">The directories created by the installation.</param>
    private static void Rollback(List<string> writtenFiles, List<string> createdDirectories)
    {
        foreach (var file in writtenFiles)
        {
            try
            {
                if (File.Exists(file))
                    File.Delete(file);
            }
            catch (IOException)
            {
                // A locked file cannot be removed; the rollback continues.
            }
            catch (UnauthorizedAccessException)
            {
                // Same as above.
            }
        }

        foreach (var directory in createdDirectories)
        {
            try
            {
                if (Directory.Exists(directory) && !Directory.EnumerateFileSystemEntries(directory).Any())
                    Directory.Delete(directory);
            }
            catch (IOException)
            {
                // A non-empty directory is kept.
            }
            catch (UnauthorizedAccessException)
            {
                // Same as above.
            }
        }
    }

    /// <summary>
    /// Writes the description of the installed pack into the instance directory.
    /// </summary>
    private void WriteConfiguration()
    {
        try
        {
            var configuration = ModpackUpdatePlanner.Describe(_archivePath, _modpack);
            configuration.Write(_instanceDirectory);
        }
        catch (IOException)
        {
            // The configuration is optional; a failure must not break the install.
        }
    }

    /// <summary>
    /// Queries the configured resolver for a CurseForge mod's download URL,
    /// returning null when no resolver is set or the ids cannot be parsed.
    /// </summary>
    private async Task<string?> ResolveCurseForgeFileAsync(ModpackMod mod, CancellationToken token)
    {
        if (_curseForgeFileResolver == null)
            return null;

        if (!int.TryParse(mod.ProjectId, out var projectId))
            return null;

        token.ThrowIfCancellationRequested();
        return await _curseForgeFileResolver(projectId, mod.FileId);
    }

    /// <summary>
    /// Extracts the original file name from a CDN download URL, decoding
    /// percent-encoding and stripping any query string.
    /// </summary>
    private static string GetFileNameFromUrl(string url)
    {
        var trimmed = url.TrimEnd('/');
        var lastSlash = trimmed.LastIndexOf('/');
        var rawName = lastSlash >= 0 ? trimmed[(lastSlash + 1)..] : trimmed;
        var decoded = Uri.UnescapeDataString(rawName);
        var query = decoded.IndexOf('?');
        return query > 0 ? decoded[..query] : decoded;
    }
    /// <summary>
    /// Returns whether a local file can be reused: either no hash is known, or
    /// the existing file's SHA1 matches the transfer's recorded hash.
    /// </summary>
    private static async Task<bool> IsExistingContentValid(string destination, string sha1)
    {
        if (!File.Exists(destination))
            return false;

        if (string.IsNullOrEmpty(sha1))
            return false;

        await using var stream = File.OpenRead(destination);
        using var sha1Provider = System.Security.Cryptography.SHA1.Create();
        var hash = await sha1Provider.ComputeHashAsync(stream);
        var hex = Convert.ToHexString(hash);

        return hex.Equals(sha1, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Returns whether the archive entry belongs to the files of the pack.
    /// </summary>
    private bool IsOverrideEntry(ZipArchiveEntry entry)
    {
        // The packs storing their files at the root of the archive use every
        // entry but the manifest itself.
        if (_modpack.Layout == ModpackLayout.Root)
        {
            var name = entry.FullName;
            return !name.Equals("mcbbs.pack-manifest.json", StringComparison.OrdinalIgnoreCase) &&
                   !name.Equals("mmc-pack.json", StringComparison.OrdinalIgnoreCase) &&
                   !name.Equals("pack.json", StringComparison.OrdinalIgnoreCase) &&
                   !name.Equals("manifest.json", StringComparison.OrdinalIgnoreCase) &&
                   !name.Equals("modrinth.index.json", StringComparison.OrdinalIgnoreCase) &&
                   !name.EndsWith('/');
        }

        foreach (var prefix in OverridePrefixes)
        {
            if (entry.FullName.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
                return true;
        }

        return false;
    }

    /// <summary>
    /// Returns the path of the file of the pack, relative to the directory of
    /// the instance.
    /// </summary>
    /// <param name="entry">The entry of the archive.</param>
    /// <returns>The relative path of the file.</returns>
    private string GetRelativePath(ZipArchiveEntry entry)
    {
        var name = entry.FullName;

        if (_modpack.Layout == ModpackLayout.Root)
            return name;

        // The override prefix maps onto the instance root (skip the pack root itself).
        var slash = name.IndexOf('/');
        return slash >= 0 ? name[(slash + 1)..] : string.Empty;
    }
}

/// <summary>
/// Modrinth modpack index (modrinth.index.json).
/// </summary>
internal sealed class ModrinthPackIndex
{
    /// <summary>
    /// The pack name.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The pack version id.
    /// </summary>
    [JsonPropertyName("versionId")]
    public string? VersionId { get; init; }

    /// <summary>
    /// The pack summary.
    /// </summary>
    [JsonPropertyName("summary")]
    public string? Summary { get; init; }

    /// <summary>
    /// Dependencies including the Minecraft version and loader id.
    /// </summary>
    [JsonPropertyName("dependencies")]
    public Dictionary<string, string>? Dependencies { get; init; }

    /// <summary>
    /// The remote files of the pack.
    /// </summary>
    [JsonPropertyName("files")]
    public List<ModrinthPackFile>? Files { get; init; }
}

/// <summary>
/// A single remote file in a Modrinth pack.
/// </summary>
internal sealed class ModrinthPackFile
{
    /// <summary>
    /// The destination path relative to the game directory.
    /// </summary>
    [JsonPropertyName("path")]
    public string? Path { get; init; }

    /// <summary>
    /// Content hashes keyed by algorithm.
    /// </summary>
    [JsonPropertyName("hashes")]
    public Dictionary<string, string>? Hashes { get; init; }

    /// <summary>
    /// The download URLs.
    /// </summary>
    [JsonPropertyName("downloads")]
    public List<string>? Downloads { get; init; }

    /// <summary>
    /// The environments the file is used in: <c>client</c> and <c>server</c>.
    /// </summary>
    [JsonPropertyName("env")]
    public List<string>? Env { get; init; }
}

/// <summary>
/// CurseForge modpack manifest (manifest.json).
/// </summary>
internal sealed class CurseForgePackManifest
{
    /// <summary>
    /// The manifest type/version.
    /// </summary>
    [JsonPropertyName("manifestVersion")]
    public int ManifestVersion { get; init; }

    /// <summary>
    /// The pack name.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The pack version.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }

    /// <summary>
    /// The pack author.
    /// </summary>
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    /// <summary>
    /// The targeted Minecraft version and loaders.
    /// </summary>
    [JsonPropertyName("minecraft")]
    public CurseForgeMinecraftSection? Minecraft { get; init; }

    /// <summary>
    /// The remote mods referenced by project/file ids.
    /// </summary>
    [JsonPropertyName("files")]
    public List<CurseForgePackFile>? Files { get; init; }
}

/// <summary>
/// The Minecraft section of a CurseForge manifest.
/// </summary>
internal sealed class CurseForgeMinecraftSection
{
    /// <summary>
    /// The Minecraft version.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }

    /// <summary>
    /// The mod loaders requested by the pack.
    /// </summary>
    [JsonPropertyName("modLoaders")]
    public List<CurseForgeModLoader>? ModLoaders { get; init; }
}

/// <summary>
/// A mod loader entry in a CurseForge manifest.
/// </summary>
internal sealed class CurseForgeModLoader
{
    /// <summary>
    /// The loader id (e.g. "forge-36.0.15").
    /// </summary>
    [JsonPropertyName("id")]
    public string? Id { get; init; }

    /// <summary>
    /// Whether this is the primary loader.
    /// </summary>
    [JsonPropertyName("primary")]
    public bool Primary { get; init; }
}

/// <summary>
/// A referenced remote mod in a CurseForge manifest.
/// </summary>
internal sealed class CurseForgePackFile
{
    /// <summary>
    /// The CurseForge project id.
    /// </summary>
    [JsonPropertyName("projectID")]
    public int? ProjectId { get; init; }

    /// <summary>
    /// The CurseForge file id.
    /// </summary>
    [JsonPropertyName("fileID")]
    public int? FileId { get; init; }

    /// <summary>
    /// Whether the dependency is required.
    /// </summary>
    [JsonPropertyName("required")]
    public bool? Required { get; init; }

    /// <summary>
    /// The name of the mod, when the manifest holds it.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }
}

/// <summary>
/// The manifest of a MCBBS modpack.
/// </summary>
internal sealed class McbbsPackManifest
{
    /// <summary>
    /// The version of the manifest.
    /// </summary>
    [JsonPropertyName("manifestVersion")]
    public int ManifestVersion { get; init; } = 1;

    /// <summary>
    /// The name of the pack.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The version of the pack.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }

    /// <summary>
    /// The author of the pack.
    /// </summary>
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    /// <summary>
    /// The description of the pack.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; init; }

    /// <summary>
    /// The addons of the pack, describing the game and the mod loaders.
    /// </summary>
    [JsonPropertyName("addons")]
    public List<McbbsPackAddon>? Addons { get; init; }
}

/// <summary>
/// An addon of a MCBBS modpack.
/// </summary>
internal sealed class McbbsPackAddon
{
    /// <summary>
    /// The identifier of the addon, for example "game".
    /// </summary>
    [JsonPropertyName("id")]
    public string? Id { get; init; }

    /// <summary>
    /// The version of the addon.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }
}

/// <summary>
/// The manifest of a MultiMC modpack.
/// </summary>
internal sealed class MultiMcPackManifest
{
    /// <summary>
    /// The version of the manifest.
    /// </summary>
    [JsonPropertyName("formatVersion")]
    public int FormatVersion { get; init; } = 1;

    /// <summary>
    /// The name of the pack.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The version of the pack.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }

    /// <summary>
    /// The author of the pack.
    /// </summary>
    [JsonPropertyName("author")]
    public string? Author { get; init; }

    /// <summary>
    /// The description of the pack.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; init; }

    /// <summary>
    /// The components of the pack, describing the game and the mod loaders.
    /// </summary>
    [JsonPropertyName("components")]
    public List<MultiMcPackComponent>? Components { get; init; }

    /// <summary>
    /// The path of the Java runtime used by the instance.
    /// </summary>
    [JsonPropertyName("JavaPath")]
    public string? JavaPath { get; init; }

    /// <summary>
    /// The arguments of the Java virtual machine used by the instance.
    /// </summary>
    [JsonPropertyName("JvmArgs")]
    public string? JvmArgs { get; init; }

    /// <summary>
    /// The command wrapping the launch of the game.
    /// </summary>
    [JsonPropertyName("WrapperCommand")]
    public string? WrapperCommand { get; init; }

    /// <summary>
    /// The command run before the game starts.
    /// </summary>
    [JsonPropertyName("PreLaunchCommand")]
    public string? PreLaunchCommand { get; init; }

    /// <summary>
    /// The command run after the game exits.
    /// </summary>
    [JsonPropertyName("PostExitCommand")]
    public string? PostExitCommand { get; init; }

    /// <summary>
    /// The maximum memory of the game, in MiB.
    /// </summary>
    [JsonPropertyName("MaxMemAlloc")]
    public int? MaxMemory { get; init; }

    /// <summary>
    /// The minimum memory of the game, in MiB.
    /// </summary>
    [JsonPropertyName("MinMemAlloc")]
    public int? MinMemory { get; init; }

    /// <summary>
    /// The size of the permanent generation area, in MiB.
    /// </summary>
    [JsonPropertyName("PermGen")]
    public int? PermGen { get; init; }

    /// <summary>
    /// The width of the window of the game.
    /// </summary>
    [JsonPropertyName("MinecraftWinWidth")]
    public int? Width { get; init; }

    /// <summary>
    /// The height of the window of the game.
    /// </summary>
    [JsonPropertyName("MinecraftWinHeight")]
    public int? Height { get; init; }

    /// <summary>
    /// Whether the game starts in the fullscreen mode.
    /// </summary>
    [JsonPropertyName("LaunchMaximized")]
    public bool Fullscreen { get; init; }

    /// <summary>
    /// Whether the console of the game is shown.
    /// </summary>
    [JsonPropertyName("ShowConsole")]
    public bool ShowConsole { get; init; }

    /// <summary>
    /// Whether the memory of the instance overrides the one of the launcher.
    /// </summary>
    [JsonPropertyName("OverrideMemory")]
    public bool OverrideMemory { get; init; }

    /// <summary>
    /// Whether the Java runtime of the instance overrides the one of the
    /// launcher.
    /// </summary>
    [JsonPropertyName("OverrideJavaLocation")]
    public bool OverrideJavaLocation { get; init; }

    /// <summary>
    /// Whether the arguments of the virtual machine of the instance override
    /// the ones of the launcher.
    /// </summary>
    [JsonPropertyName("OverrideJavaArgs")]
    public bool OverrideJavaArgs { get; init; }

    /// <summary>
    /// Whether the console of the instance overrides the one of the launcher.
    /// </summary>
    [JsonPropertyName("OverrideConsole")]
    public bool OverrideConsole { get; init; }

    /// <summary>
    /// Whether the commands of the instance override the ones of the launcher.
    /// </summary>
    [JsonPropertyName("OverrideCommands")]
    public bool OverrideCommands { get; init; }

    /// <summary>
    /// Whether the window of the instance overrides the one of the launcher.
    /// </summary>
    [JsonPropertyName("OverrideWindow")]
    public bool OverrideWindow { get; init; }
}

/// <summary>
/// A component of a MultiMC modpack.
/// </summary>
internal sealed class MultiMcPackComponent
{
    /// <summary>
    /// The identifier of the component, for example "net.minecraft".
    /// </summary>
    [JsonPropertyName("uid")]
    public string? Uid { get; init; }

    /// <summary>
    /// The version of the component.
    /// </summary>
    [JsonPropertyName("version")]
    public string? Version { get; init; }
}
