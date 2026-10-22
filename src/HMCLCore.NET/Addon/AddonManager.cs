using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Addon;

/// <summary>
/// Represents a local mod installation.
/// </summary>
public sealed class LocalMod
{
    /// <summary>
    /// The mod file path.
    /// </summary>
    public string FilePath { get; set; } = string.Empty;

    /// <summary>
    /// The mod name.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The mod version.
    /// </summary>
    public string? Version { get; init; }

    /// <summary>
    /// The mod identifier declared by the mod's metadata (e.g. the Fabric mod id).
    /// This usually matches the project slug on remote repositories.
    /// </summary>
    public string? ModId { get; init; }

    /// <summary>
    /// The mod description.
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// The mod authors.
    /// </summary>
    public string? Authors { get; init; }

    /// <summary>
    /// The mod loader type (Fabric, Forge, Quilt, LiteLoader).
    /// </summary>
    public string? ModLoaderType { get; set; }

    /// <summary>
    /// Whether the mod is enabled.
    /// </summary>
    public bool Enabled { get; set; } = true;

    /// <summary>
    /// Returns whether this is a Fabric mod.
    /// </summary>
    public bool IsFabric => ModLoaderType?.Equals("fabric", StringComparison.OrdinalIgnoreCase) == true;

    /// <summary>
    /// Returns whether this is a Forge mod.
    /// </summary>
    public bool IsForge => ModLoaderType?.Equals("forge", StringComparison.OrdinalIgnoreCase) == true;

    /// <summary>
    /// Returns whether this is a Quilt mod.
    /// </summary>
    public bool IsQuilt => ModLoaderType?.Equals("quilt", StringComparison.OrdinalIgnoreCase) == true;

    /// <summary>
    /// The name of the mod shown to the user; the name of the project of MCMod
    /// is used when the mod is known, so that the user recognizes the mods
    /// installed by the other launchers.
    /// </summary>
    public string DisplayName
    {
        get
        {
            var info = AddonProjectInfoRegistry.Mod.GetByModId(ModId);
            if (info is not null)
            {
                if (!string.IsNullOrEmpty(info.MainName))
                    return info.MainName!;

                if (!string.IsNullOrEmpty(info.SubName))
                    return info.SubName!;
            }

            return Name;
        }
    }
}

/// <summary>
/// Local mod manager that scans and tracks installed mods.
/// </summary>
public sealed class LocalAddonManager
{
    private readonly string _modsDirectory;
    private readonly List<LocalMod> _mods = new();

    /// <summary>
    /// All locally installed mods.
    /// </summary>
    public IReadOnlyList<LocalMod> Mods => _mods.AsReadOnly();

    /// <summary>
    /// Creates a new LocalAddonManager.
    /// </summary>
    public LocalAddonManager(string modsDirectory)
    {
        _modsDirectory = modsDirectory ?? throw new ArgumentNullException(nameof(modsDirectory));
    }

    /// <summary>
    /// Scans the mods directory for installed mods.
    /// </summary>
    public void Scan()
    {
        _mods.Clear();

        if (!Directory.Exists(_modsDirectory))
            return;

        foreach (var file in Directory.GetFiles(_modsDirectory, "*.jar", SearchOption.AllDirectories))
        {
            _mods.Add(CreateMod(file, enabled: true));
        }

        // Disabled mods are jar files renamed with the ".disabled" suffix.
        foreach (var file in Directory.GetFiles(_modsDirectory, "*.jar.disabled", SearchOption.AllDirectories))
        {
            _mods.Add(CreateMod(file, enabled: false));
        }
    }

    /// <summary>
    /// Builds a mod entry for the given file, reading its embedded metadata when possible.
    /// </summary>
    /// <param name="file">The mod file path.</param>
    /// <param name="enabled">Whether the mod is enabled.</param>
    /// <returns>The mod entry.</returns>
    private LocalMod CreateMod(string file, bool enabled)
    {
        var metadata = AddonMetadataReader.Read(file);
        var fileName = Path.GetFileNameWithoutExtension(file);
        if (!enabled)
            fileName = Path.GetFileNameWithoutExtension(fileName);

        var mod = new LocalMod
        {
            FilePath = file,
            Name = metadata?.Name is { Length: > 0 } name ? name : fileName,
            Version = metadata?.Version is { Length: > 0 } version ? version : null,
            ModId = metadata?.ModId is { Length: > 0 } modId ? modId : null,
            Description = metadata?.Description,
            Authors = metadata?.Authors,
            Enabled = enabled
        };

        if (metadata != null)
        {
            mod.ModLoaderType = metadata.Loader switch
            {
                ModLoaderType.Fabric => "fabric",
                ModLoaderType.Forge => "forge",
                ModLoaderType.NeoForge => "neoforge",
                ModLoaderType.Quilt => "quilt",
                ModLoaderType.LiteLoader => "liteloader",
                _ => null
            };
        }
        else
        {
            // Fall back to inferring the loader from the relative path.
            var relativePath = Path.GetRelativePath(_modsDirectory, file);
            if (relativePath.Contains("fabric", StringComparison.OrdinalIgnoreCase))
                mod.ModLoaderType = "fabric";
            else if (relativePath.Contains("forge", StringComparison.OrdinalIgnoreCase))
                mod.ModLoaderType = "forge";
            else if (relativePath.Contains("quilt", StringComparison.OrdinalIgnoreCase))
                mod.ModLoaderType = "quilt";
        }

        return mod;
    }

    /// <summary>
    /// Returns enabled mods.
    /// </summary>
    public IEnumerable<LocalMod> GetEnabledMods() => _mods.Where(m => m.Enabled);

    /// <summary>
    /// Returns disabled mods.
    /// </summary>
    public IEnumerable<LocalMod> GetDisabledMods() => _mods.Where(m => !m.Enabled);

    /// <summary>
    /// Toggles a mod between enabled and disabled by renaming its file with
    /// or without the ".disabled" suffix, then rescans the directory.
    /// </summary>
    /// <param name="mod">The mod to toggle.</param>
    /// <returns>The updated mod, or null when the rename failed.</returns>
    public LocalMod? ToggleEnabled(LocalMod mod)
    {
        var newPath = mod.Enabled
            ? mod.FilePath + ".disabled"
            : mod.FilePath.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase)
                ? mod.FilePath[..^".disabled".Length]
                : mod.FilePath;

        try
        {
            if (File.Exists(newPath))
                return null;

            File.Move(mod.FilePath, newPath);
            mod.FilePath = newPath;
            mod.Enabled = !mod.Enabled;
            return mod;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Deletes a mod file from disk and rescans the directory.
    /// </summary>
    /// <param name="mod">The mod to remove.</param>
    /// <returns>Whether the mod file was deleted.</returns>
    public bool DeleteMod(LocalMod mod)
    {
        try
        {
            File.Delete(mod.FilePath);
            _mods.Remove(mod);
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Copies a mod file into the mods directory and rescans it. Used when the
    /// user drops a mod file onto the launcher.
    /// </summary>
    /// <param name="sourcePath">The mod file to install.</param>
    /// <returns>The installed mod, or null when the file could not be installed.</returns>
    public LocalMod? Install(string sourcePath)
    {
        if (string.IsNullOrWhiteSpace(sourcePath) || !File.Exists(sourcePath))
            return null;

        if (!sourcePath.EndsWith(".jar", StringComparison.OrdinalIgnoreCase))
            return null;

        try
        {
            Directory.CreateDirectory(_modsDirectory);

            var target = Path.Combine(_modsDirectory, Path.GetFileName(sourcePath));
            if (!string.Equals(
                    Path.GetFullPath(target), Path.GetFullPath(sourcePath),
                    StringComparison.OrdinalIgnoreCase))
            {
                File.Copy(sourcePath, target, overwrite: true);
            }

            Scan();

            var fileName = Path.GetFileName(target);
            return _mods.FirstOrDefault(m =>
                string.Equals(Path.GetFileName(m.FilePath), fileName, StringComparison.OrdinalIgnoreCase));
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }
    }
}

/// <summary>
/// Represents a resource pack installed in a game instance.
/// </summary>
public sealed class ResourcePack
{
    /// <summary>
    /// The full path of the pack archive or directory.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The file name of the pack, including the extension for archives.
    /// </summary>
    public string FileName { get; init; } = string.Empty;

    /// <summary>
    /// The display name of the pack.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The pack description read from pack.mcmeta.
    /// </summary>
    public string? Description { get; set; }

    /// <summary>
    /// Whether the pack is enabled in the instance options.
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// The pack format number declared in pack.mcmeta.
    /// </summary>
    public int Format { get; set; }

    /// <summary>
    /// Whether the pack is a directory rather than an archive.
    /// </summary>
    public bool IsDirectory { get; init; }

    /// <summary>
    /// Whether the pack provides a readable pack.mcmeta file.
    /// </summary>
    public bool HasPackMeta { get; init; }

    /// <summary>
    /// The compatibility of the pack with the Minecraft version of the instance.
    /// </summary>
    public ResourcePackCompatibility Compatibility { get; set; } = ResourcePackCompatibility.Compatible;

    /// <summary>
    /// The warning shown next to the pack when it is not compatible.
    /// </summary>
    public string? Warning => Compatibility switch
    {
        ResourcePackCompatibility.TooNew => "资源包版本过新",
        ResourcePackCompatibility.TooOld => "资源包版本过旧",
        ResourcePackCompatibility.Invalid => "资源包元数据无效",
        ResourcePackCompatibility.MissingPackMeta => "缺少 pack.mcmeta",
        ResourcePackCompatibility.MissingGameMeta => "无法确定游戏版本",
        _ => null
    };
}

/// <summary>
/// Manages the resource packs of a game instance: scanning the resourcepacks
/// directory, enabling and disabling packs through options.txt and reporting
/// compatibility problems.
/// </summary>
public sealed class ResourcePackManager
{
    /// <summary>
    /// The options key holding the list of enabled resource packs.
    /// </summary>
    private const string EnabledPacksKey = "resourcePacks";

    /// <summary>
    /// The options key holding the list of enabled but incompatible packs.
    /// </summary>
    private const string IncompatiblePacksKey = "incompatibleResourcePacks";

    private readonly string _resourcePacksDirectory;
    private readonly string? _optionsFile;
    private readonly string? _minecraftVersion;
    private readonly List<ResourcePack> _packs = new();

    /// <summary>
    /// Creates a new resource pack manager.
    /// </summary>
    /// <param name="resourcePacksDirectory">The resourcepacks directory of the instance.</param>
    /// <param name="optionsFile">The options.txt of the instance, or null when it is unknown.</param>
    /// <param name="minecraftVersion">The Minecraft version of the instance, or null when unknown.</param>
    public ResourcePackManager(string resourcePacksDirectory, string? optionsFile = null, string? minecraftVersion = null)
    {
        _resourcePacksDirectory = resourcePacksDirectory ?? throw new ArgumentNullException(nameof(resourcePacksDirectory));
        _optionsFile = optionsFile;
        _minecraftVersion = minecraftVersion;
    }

    /// <summary>
    /// All resource packs found in the resourcepacks directory.
    /// </summary>
    public IReadOnlyList<ResourcePack> Packs => _packs;

    /// <summary>
    /// The resourcepacks directory managed by this instance.
    /// </summary>
    public string DirectoryPath => _resourcePacksDirectory;

    /// <summary>
    /// The Minecraft version the compatibility is checked against.
    /// </summary>
    public string? MinecraftVersion => _minecraftVersion;

    /// <summary>
    /// Scans the resource packs directory and refreshes the pack list.
    /// </summary>
    public void Scan()
    {
        _packs.Clear();

        if (!Directory.Exists(_resourcePacksDirectory))
            return;

        var options = LoadOptions();
        var enabledPacks = options.GetStringList(EnabledPacksKey);
        var incompatiblePacks = options.GetStringList(IncompatiblePacksKey);

        foreach (var directory in Directory.GetDirectories(_resourcePacksDirectory))
        {
            var pack = CreatePack(directory, true);
            if (pack != null)
                _packs.Add(pack);
        }

        foreach (var file in Directory.GetFiles(_resourcePacksDirectory, "*.zip"))
        {
            var pack = CreatePack(file, false);
            if (pack != null)
                _packs.Add(pack);
        }

        foreach (var pack in _packs)
        {
            var id = GetPackId(pack.FileName);
            var isIncompatible = pack.Compatibility != ResourcePackCompatibility.Compatible;
            pack.Enabled = Contains(enabledPacks, id);
            if (pack.Enabled && isIncompatible)
                pack.Enabled = Contains(incompatiblePacks, id);
        }
    }

    /// <summary>
    /// Enables or disables a resource pack by updating the instance options.
    /// </summary>
    /// <param name="pack">The pack to change.</param>
    /// <param name="enabled">Whether the pack should be enabled.</param>
    /// <returns>Whether the options were changed.</returns>
    public bool SetEnabled(ResourcePack pack, bool enabled)
    {
        if (_optionsFile == null)
            return false;

        var options = LoadOptions();
        var enabledPacks = new List<string>(options.GetStringList(EnabledPacksKey));
        var incompatiblePacks = new List<string>(options.GetStringList(IncompatiblePacksKey));
        var id = GetPackId(pack.FileName);
        var isIncompatible = pack.Compatibility != ResourcePackCompatibility.Compatible;
        var modified = false;

        if (enabled)
        {
            if (!Contains(enabledPacks, id))
            {
                enabledPacks.Add(id);
                modified = true;
            }

            if (isIncompatible && !Contains(incompatiblePacks, id))
            {
                incompatiblePacks.Add(id);
                modified = true;
            }
            else if (!isIncompatible && Contains(incompatiblePacks, id))
            {
                RemoveAll(incompatiblePacks, id);
                modified = true;
            }
        }
        else
        {
            if (Contains(enabledPacks, id))
            {
                RemoveAll(enabledPacks, id);
                modified = true;
            }

            if (Contains(incompatiblePacks, id))
            {
                RemoveAll(incompatiblePacks, id);
                modified = true;
            }
        }

        if (!modified)
            return false;

        options.SetStringList(EnabledPacksKey, enabledPacks);
        options.SetStringList(IncompatiblePacksKey, incompatiblePacks);
        options.Save();

        pack.Enabled = enabled;
        return true;
    }

    /// <summary>
    /// Deletes a resource pack from the resourcepacks directory.
    /// </summary>
    /// <param name="pack">The pack to delete.</param>
    /// <returns>Whether the pack was deleted.</returns>
    public bool Remove(ResourcePack pack)
    {
        try
        {
            if (Directory.Exists(pack.Path))
                Directory.Delete(pack.Path, recursive: true);
            else if (File.Exists(pack.Path))
                File.Delete(pack.Path);
            else
                return false;
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }

        _packs.Remove(pack);
        return true;
    }

    /// <summary>
    /// Copies a resource pack archive into the resourcepacks directory.
    /// </summary>
    /// <param name="sourcePath">The archive to import.</param>
    /// <returns>The imported pack, or null when the import failed.</returns>
    public ResourcePack? Import(string sourcePath)
    {
        if (!File.Exists(sourcePath))
            return null;

        try
        {
            Directory.CreateDirectory(_resourcePacksDirectory);
            var target = System.IO.Path.Combine(_resourcePacksDirectory, System.IO.Path.GetFileName(sourcePath));
            File.Copy(sourcePath, target, overwrite: true);

            var pack = CreatePack(target, false);
            if (pack != null)
            {
                _packs.Add(pack);
                return pack;
            }
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }

        return null;
    }

    /// <summary>
    /// Creates a pack description for the given file or directory.
    /// </summary>
    /// <param name="path">The path of the pack.</param>
    /// <param name="isDirectory">Whether the path is a directory.</param>
    /// <returns>The pack, or null when the path is not a usable pack.</returns>
    private ResourcePack? CreatePack(string path, bool isDirectory)
    {
        var fileName = System.IO.Path.GetFileName(path);
        var meta = ReadPackMeta(path, isDirectory);

        var pack = new ResourcePack
        {
            Path = path,
            FileName = fileName,
            Name = isDirectory ? fileName : System.IO.Path.GetFileNameWithoutExtension(fileName),
            Description = meta?.Pack?.Description,
            Format = meta?.Pack?.PackFormat ?? 0,
            IsDirectory = isDirectory,
            HasPackMeta = meta != null
        };

        pack.Compatibility = GetCompatibility(pack);
        return pack;
    }

    /// <summary>
    /// Reads the pack.mcmeta of a pack archive or directory.
    /// </summary>
    /// <param name="path">The path of the pack.</param>
    /// <param name="isDirectory">Whether the path is a directory.</param>
    /// <returns>The parsed metadata, or null when it is missing or invalid.</returns>
    private static PackMcMeta? ReadPackMeta(string path, bool isDirectory)
    {
        try
        {
            if (isDirectory)
            {
                var metaPath = System.IO.Path.Combine(path, "pack.mcmeta");
                if (!File.Exists(metaPath))
                    return null;

                return JsonSerializer.Deserialize(File.ReadAllText(metaPath), LauncherJsonContext.Default.PackMcMeta);
            }

            using var archive = ZipFile.OpenRead(path);
            var entry = archive.GetEntry("pack.mcmeta");
            if (entry == null)
                return null;

            using var stream = entry.Open();
            using var reader = new StreamReader(stream);
            return JsonSerializer.Deserialize(reader.ReadToEnd(), LauncherJsonContext.Default.PackMcMeta);
        }
        catch (Exception ex) when (ex is IOException or JsonException or InvalidDataException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <summary>
    /// Determines how well a pack matches the Minecraft version of the instance.
    /// </summary>
    /// <param name="pack">The pack to check.</param>
    /// <returns>The compatibility of the pack.</returns>
    public ResourcePackCompatibility GetCompatibility(ResourcePack pack)
    {
        if (!pack.HasPackMeta)
            return ResourcePackCompatibility.MissingPackMeta;

        var required = ResourcePackFormats.GetRequiredFormat(_minecraftVersion);
        if (required == null)
            return ResourcePackCompatibility.MissingGameMeta;

        if (pack.Format <= 0)
            return ResourcePackCompatibility.Invalid;

        if (pack.Format < required)
            return ResourcePackCompatibility.TooOld;

        if (pack.Format > required)
            return ResourcePackCompatibility.TooNew;

        return ResourcePackCompatibility.Compatible;
    }

    /// <summary>
    /// Returns the identifier of a pack as stored in options.txt.
    /// </summary>
    /// <param name="fileName">The file name of the pack.</param>
    /// <returns>The pack identifier.</returns>
    private string GetPackId(string fileName) =>
        ResourcePackFormats.SupportsNewOptionsFormat(_minecraftVersion) ? "file/" + fileName : fileName;

    /// <summary>
    /// Loads the instance options.
    /// </summary>
    /// <returns>The options file, empty when the instance has none.</returns>
    private GameOptionsFile LoadOptions() =>
        _optionsFile == null ? new GameOptionsFile() : GameOptionsFile.Load(_optionsFile);

    /// <summary>
    /// Returns whether the list contains the given identifier, accepting both
    /// the plain and the "file/" prefixed form.
    /// </summary>
    /// <param name="values">The list to search.</param>
    /// <param name="id">The identifier to look for.</param>
    /// <returns>Whether the identifier is present.</returns>
    private static bool Contains(IReadOnlyList<string> values, string id)
    {
        foreach (var value in values)
        {
            if (string.Equals(value, id, StringComparison.Ordinal))
                return true;

            if (string.Equals(value, "file/" + id, StringComparison.Ordinal))
                return true;
        }

        return false;
    }

    /// <summary>
    /// Removes the identifier from the list, accepting both the plain and the
    /// "file/" prefixed form.
    /// </summary>
    /// <param name="values">The list to modify.</param>
    /// <param name="id">The identifier to remove.</param>
    private static void RemoveAll(List<string> values, string id)
    {
        values.RemoveAll(value =>
            string.Equals(value, id, StringComparison.Ordinal) ||
            string.Equals(value, "file/" + id, StringComparison.Ordinal));
    }
}

/// <summary>
/// Represents a data pack installed in a world.
/// </summary>
public sealed class DataPack
{
    /// <summary>
    /// The full path of the pack archive or directory.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The file name of the pack, including the extension for archives.
    /// </summary>
    public string FileName { get; init; } = string.Empty;

    /// <summary>
    /// The identifier of the pack, without any file extension.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The pack description read from pack.mcmeta.
    /// </summary>
    public string? Description { get; set; }

    /// <summary>
    /// Whether the pack is active.
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// Whether the pack is a directory rather than an archive.
    /// </summary>
    public bool IsDirectory { get; init; }
}

/// <summary>
/// Manages the data packs of a world: scanning the datapacks directory,
/// enabling and disabling packs and installing new ones.
/// </summary>
public sealed class DataPackManager
{
    /// <summary>
    /// The extension appended to a pack to mark it as disabled.
    /// </summary>
    private const string DisabledExtension = ".disabled";

    private readonly string _dataPacksDirectory;
    private readonly List<DataPack> _packs = new();

    /// <summary>
    /// Creates a new data pack manager.
    /// </summary>
    /// <param name="dataPacksDirectory">The datapacks directory of the world.</param>
    public DataPackManager(string dataPacksDirectory)
    {
        _dataPacksDirectory = dataPacksDirectory ?? throw new ArgumentNullException(nameof(dataPacksDirectory));
    }

    /// <summary>
    /// All data packs found in the datapacks directory.
    /// </summary>
    public IReadOnlyList<DataPack> Packs => _packs;

    /// <summary>
    /// The datapacks directory managed by this instance.
    /// </summary>
    public string DirectoryPath => _dataPacksDirectory;

    /// <summary>
    /// Scans the data packs directory and refreshes the pack list.
    /// </summary>
    public void Scan()
    {
        _packs.Clear();

        if (!Directory.Exists(_dataPacksDirectory))
            return;

        foreach (var directory in Directory.GetDirectories(_dataPacksDirectory))
        {
            var pack = CreatePack(directory, true);
            if (pack != null)
                _packs.Add(pack);
        }

        foreach (var file in Directory.GetFiles(_dataPacksDirectory))
        {
            var extension = System.IO.Path.GetExtension(file);
            if (!extension.Equals(".zip", StringComparison.OrdinalIgnoreCase) &&
                !extension.Equals(".zip" + DisabledExtension, StringComparison.OrdinalIgnoreCase))
            {
                continue;
            }

            var pack = CreatePack(file, false);
            if (pack != null)
                _packs.Add(pack);
        }
    }

    /// <summary>
    /// Enables or disables a data pack by renaming its status file.
    /// </summary>
    /// <param name="pack">The pack to change.</param>
    /// <param name="enabled">Whether the pack should be active.</param>
    /// <returns>Whether the pack was changed.</returns>
    public bool SetEnabled(DataPack pack, bool enabled)
    {
        if (pack.Enabled == enabled)
            return false;

        try
        {
            var statusFile = GetStatusFile(pack);
            if (!File.Exists(statusFile))
                return false;

            var newStatusFile = enabled
                ? statusFile[..^DisabledExtension.Length]
                : statusFile + DisabledExtension;

            File.Move(statusFile, newStatusFile);
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }

        pack.Enabled = enabled;
        return true;
    }

    /// <summary>
    /// Deletes a data pack.
    /// </summary>
    /// <param name="pack">The pack to delete.</param>
    /// <returns>Whether the pack was deleted.</returns>
    public bool Remove(DataPack pack)
    {
        try
        {
            if (Directory.Exists(pack.Path))
                Directory.Delete(pack.Path, recursive: true);
            else if (File.Exists(pack.Path))
                File.Delete(pack.Path);
            else
                return false;
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }

        _packs.Remove(pack);
        return true;
    }

    /// <summary>
    /// Installs a data pack archive into the datapacks directory.
    /// </summary>
    /// <param name="archivePath">The archive to install.</param>
    /// <returns>The installed pack, or null when the archive is not a data pack.</returns>
    public DataPack? Install(string archivePath)
    {
        if (!File.Exists(archivePath))
            return null;

        try
        {
            Directory.CreateDirectory(_dataPacksDirectory);

            using (var archive = ZipFile.OpenRead(archivePath))
            {
                var hasNested = archive.GetEntry("datapacks/") != null ||
                                archive.Entries.Any(e => e.FullName.StartsWith("datapacks/", StringComparison.OrdinalIgnoreCase));
                var hasMeta = archive.GetEntry("pack.mcmeta") != null;

                if (!hasNested && !hasMeta)
                    return null;

                if (hasNested)
                {
                    // The archive contains one or more packs in a "datapacks" folder.
                    foreach (var entry in archive.Entries)
                    {
                        if (!entry.FullName.StartsWith("datapacks/", StringComparison.OrdinalIgnoreCase))
                            continue;

                        var relative = entry.FullName["datapacks/".Length..];
                        if (relative.Length == 0)
                            continue;

                        var target = System.IO.Path.Combine(_dataPacksDirectory, relative.Replace('/', System.IO.Path.DirectorySeparatorChar));
                        if (entry.FullName.EndsWith('/'))
                        {
                            Directory.CreateDirectory(target);
                            continue;
                        }

                        Directory.CreateDirectory(System.IO.Path.GetDirectoryName(target)!);
                        entry.ExtractToFile(target, overwrite: true);
                    }
                }
                else
                {
                    // The archive holds a single pack: keep the archive as it is.
                    var target = System.IO.Path.Combine(_dataPacksDirectory, System.IO.Path.GetFileName(archivePath));
                    File.Copy(archivePath, target, overwrite: true);
                }
            }
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException or UnauthorizedAccessException)
        {
            return null;
        }

        Scan();

        var fileName = System.IO.Path.GetFileName(archivePath);
        return _packs.FirstOrDefault(p => string.Equals(p.FileName, fileName, StringComparison.OrdinalIgnoreCase));
    }

    /// <summary>
    /// Creates a pack description for the given file or directory.
    /// </summary>
    /// <param name="path">The path of the pack.</param>
    /// <param name="isDirectory">Whether the path is a directory.</param>
    /// <returns>The pack, or null when the path is not a usable pack.</returns>
    private static DataPack? CreatePack(string path, bool isDirectory)
    {
        var fileName = System.IO.Path.GetFileName(path);
        var disabled = fileName.EndsWith(DisabledExtension, StringComparison.OrdinalIgnoreCase);
        var baseName = disabled ? fileName[..^DisabledExtension.Length] : fileName;
        var name = System.IO.Path.GetFileNameWithoutExtension(baseName);

        string? description = null;
        if (isDirectory)
        {
            var metaPath = System.IO.Path.Combine(path, "pack.mcmeta");
            var disabledMetaPath = metaPath + DisabledExtension;
            var target = File.Exists(metaPath) ? metaPath : disabledMetaPath;
            if (File.Exists(target))
                description = ReadMetaDescription(target);
        }
        else
        {
            description = ReadZipMetaDescription(path);
        }

        if (description == null && !isDirectory && !disabled)
        {
            // A zip without pack.mcmeta is not a valid data pack.
            return null;
        }

        return new DataPack
        {
            Path = path,
            FileName = fileName,
            Name = name,
            Description = description,
            IsDirectory = isDirectory,
            Enabled = !disabled
        };
    }

    /// <summary>
    /// Returns the file that marks the enabled state of a pack.
    /// </summary>
    /// <param name="pack">The pack.</param>
    /// <returns>The path of the status file.</returns>
    private static string GetStatusFile(DataPack pack)
    {
        if (pack.IsDirectory)
        {
            var metaPath = System.IO.Path.Combine(pack.Path, "pack.mcmeta");
            return File.Exists(metaPath) ? metaPath : metaPath + DisabledExtension;
        }

        return pack.Enabled ? pack.Path : pack.Path[..^DisabledExtension.Length];
    }

    /// <summary>
    /// Reads the description of a pack.mcmeta file.
    /// </summary>
    /// <param name="metaPath">The path of pack.mcmeta.</param>
    /// <returns>The description, or null when it cannot be read.</returns>
    private static string? ReadMetaDescription(string metaPath)
    {
        try
        {
            var meta = JsonSerializer.Deserialize(File.ReadAllText(metaPath), LauncherJsonContext.Default.PackMcMeta);
            return meta?.Pack?.Description;
        }
        catch (Exception ex) when (ex is IOException or JsonException or UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <summary>
    /// Reads the description of the pack.mcmeta inside a pack archive.
    /// </summary>
    /// <param name="archivePath">The path of the archive.</param>
    /// <returns>The description, or null when it cannot be read.</returns>
    private static string? ReadZipMetaDescription(string archivePath)
    {
        try
        {
            using var archive = ZipFile.OpenRead(archivePath);
            var entry = archive.GetEntry("pack.mcmeta");
            if (entry == null)
                return null;

            using var stream = entry.Open();
            using var reader = new StreamReader(stream);
            var meta = JsonSerializer.Deserialize(reader.ReadToEnd(), LauncherJsonContext.Default.PackMcMeta);
            return meta?.Pack?.Description;
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException or JsonException or UnauthorizedAccessException)
        {
            return null;
        }
    }
}

/// <summary>
/// Mod loader types supported by HMCL.
/// </summary>
public enum ModLoaderType
{
    None,
    Fabric,
    Forge,
    Quilt,
    LiteLoader,
    NeoForge
}

/// <summary>
/// Remote addon repository for downloading mods from CurseForge/Modrinth.
/// </summary>
public abstract class RemoteAddonRepository
{
    /// <summary>
    /// The name of the repository.
    /// </summary>
    public abstract string Name { get; }

    /// <summary>
    /// Searches for addons matching the query.
    /// </summary>
    public abstract Task<List<RemoteAddon>> SearchAsync(string query);

    /// <summary>
    /// Returns addon details by ID.
    /// </summary>
    public abstract Task<RemoteAddon?> GetAddonAsync(string id);

    /// <summary>
    /// Downloads an addon file.
    /// </summary>
    public abstract Task<bool> DownloadAsync(RemoteAddon addon, string destination);
}

/// <summary>
/// Represents a remote addon available for download.
/// </summary>
public sealed class RemoteAddon
{
    /// <summary>
    /// The addon ID.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The addon name.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The addon description.
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// The addon version.
    /// </summary>
    public string? Version { get; init; }

    /// <summary>
    /// The supported Minecraft versions.
    /// </summary>
    public List<string> SupportedVersions { get; init; } = new();

    /// <summary>
    /// The download URL of the concrete file.
    /// </summary>
    public string? DownloadUrl { get; set; }

    /// <summary>
    /// The mod loader type.
    /// </summary>
    public ModLoaderType ModLoader { get; init; } = ModLoaderType.None;
}
