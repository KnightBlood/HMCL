using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;
using System.Text.Json;
using HMCLCore.NET.Game;

namespace HMCLCore.NET.Modpack;

/// <summary>
/// The options of the export of a modpack.
/// </summary>
public sealed class ModpackExportOptions
{
    /// <summary>
    /// The information written into the manifest of the exported pack.
    /// </summary>
    public ModpackExportInfo Info { get; set; } = new();

    /// <summary>
    /// The type of the exported pack.
    /// </summary>
    public ModpackExportType Type { get; set; } = ModpackExportType.Modrinth;

    /// <summary>
    /// The names of the entries of the directory of the instance that are
    /// copied into the exported pack; an empty list means that the suggested
    /// entries are used.
    /// </summary>
    public IReadOnlyList<string> IncludedEntries { get; set; } = Array.Empty<string>();

    /// <summary>
    /// Whether the optional (disabled) mods are copied into the exported pack.
    /// </summary>
    public bool IncludeOptional { get; set; }
}

/// <summary>
/// Exports the files of a game instance into a modpack archive.
/// </summary>
public sealed class ModpackExporter
{
    /// <summary>
    /// The entries of the directory of an instance that are copied into an
    /// exported pack by default.
    /// </summary>
    public static readonly IReadOnlyList<string> SuggestedEntries = new List<string>
    {
        "mods", "config", "defaultconfigs", "shaderpacks", "resourcepacks",
        "kubejs", "scripts", "datapacks", "advancements", "stats",
        "options.txt", "servers.dat", "usercache.json"
    };

    /// <summary>
    /// The entries of the directory of an instance that are never copied into
    /// an exported pack.
    /// </summary>
    public static readonly IReadOnlyList<string> ExcludedEntries = new List<string>
    {
        "logs", "crash-reports", "cache", "screenshots", "versions", "libraries",
        "assets", "natives", "backups", "launcher_profiles.json", "launcher_mod.xml",
        "crash-reports.txt", "hmcl.json", "options.txt.tmp"
    };

    /// <summary>
    /// Exports the files of the specified instance into a modpack archive.
    /// </summary>
    /// <param name="instance">The game instance to export.</param>
    /// <param name="destinationPath">The path of the archive to write.</param>
    /// <param name="options">The options of the export.</param>
    /// <returns>The path of the written archive.</returns>
    public string ExportInstance(GameInstance instance, string destinationPath, ModpackExportOptions options)
    {
        ArgumentNullException.ThrowIfNull(instance);
        ArgumentNullException.ThrowIfNull(options);

        if (string.IsNullOrWhiteSpace(destinationPath))
            throw new ArgumentException("The path of the exported pack must not be empty.", nameof(destinationPath));

        var instanceDir = instance.GameDirectory;
        if (!Directory.Exists(instanceDir))
            throw new DirectoryNotFoundException($"Instance directory not found: {instanceDir}");

        var directory = Path.GetDirectoryName(destinationPath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        using var zip = ZipFile.Open(destinationPath, ZipArchiveMode.Create);

        switch (options.Type)
        {
            case ModpackExportType.Modrinth:
                WriteModrinthManifest(zip, instance, options.Info);
                WriteEntries(zip, instanceDir, "overrides", options);
                break;

            case ModpackExportType.CurseForge:
                WriteCurseForgeManifest(zip, instance, options.Info);
                WriteEntries(zip, instanceDir, "overrides", options);
                break;

            case ModpackExportType.Mcbbs:
                WriteMcbbsManifest(zip, instance, options.Info);
                WriteEntries(zip, instanceDir, string.Empty, options);
                break;

            case ModpackExportType.Server:
                WriteServerManifest(zip, instance, options.Info);
                WriteEntries(zip, instanceDir, string.Empty, options, includeWorld: true);
                break;
        }

        return destinationPath;
    }

    /// <summary>
    /// Exports the mods and the configuration of the specified instance into a
    /// Modrinth modpack archive.
    /// </summary>
    /// <param name="instance">The game instance to export.</param>
    /// <param name="destinationZipPath">The path of the archive to write.</param>
    /// <param name="includeOptional">Whether the optional (disabled) mods are copied.</param>
    /// <returns>The path of the written archive.</returns>
    public string ExportInstance(GameInstance instance, string destinationZipPath, bool includeOptional = false)
        => ExportInstance(instance, destinationZipPath, new ModpackExportOptions
        {
            Info = new ModpackExportInfo
            {
                Name = instance.GetDisplayName(),
                Version = "1.0",
                Description = "Exported by HMCL.NET"
            },
            IncludeOptional = includeOptional
        });

    /// <summary>
    /// Writes the index of a Modrinth modpack.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="instance">The exported instance.</param>
    /// <param name="info">The information of the exported pack.</param>
    private static void WriteModrinthManifest(ZipArchive zip, GameInstance instance, ModpackExportInfo info)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            writer.WriteStartObject();
            writer.WriteNumber("formatVersion", 1);
            writer.WriteString("game", "minecraft");
            writer.WriteString("versionId", info.Version);
            writer.WriteString("name", info.Name);
            writer.WriteString("summary", info.Description);
            writer.WriteStartArray("files");
            writer.WriteEndArray();
            writer.WriteStartObject("dependencies");
            writer.WriteString("minecraft", instance.Version);
            if (!string.IsNullOrEmpty(instance.Loader))
                writer.WriteString(instance.Loader.ToLowerInvariant(), instance.LoaderVersion);
            writer.WriteEndObject();
            writer.WriteEndObject();
        }

        WriteEntry(zip, "modrinth.index.json", stream.ToArray());
    }

    /// <summary>
    /// Writes the manifest of a CurseForge modpack.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="instance">The exported instance.</param>
    /// <param name="info">The information of the exported pack.</param>
    private static void WriteCurseForgeManifest(ZipArchive zip, GameInstance instance, ModpackExportInfo info)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            writer.WriteStartObject();
            writer.WriteString("manifestType", "minecraftModpack");
            writer.WriteNumber("manifestVersion", 1);
            writer.WriteString("name", info.Name);
            writer.WriteString("version", info.Version);
            writer.WriteString("author", info.Author);
            writer.WriteString("overrides", "overrides");
            writer.WriteStartObject("minecraft");
            writer.WriteString("version", instance.Version);
            writer.WriteStartArray("modLoaders");
            if (!string.IsNullOrEmpty(instance.Loader))
            {
                writer.WriteStartObject();
                writer.WriteString("id", instance.Loader.ToLowerInvariant());
                writer.WriteBoolean("primary", true);
                writer.WriteEndObject();
            }
            writer.WriteEndArray();
            writer.WriteEndObject();
            writer.WriteStartArray("files");
            writer.WriteEndArray();
            writer.WriteEndObject();
        }

        WriteEntry(zip, "manifest.json", stream.ToArray());
    }

    /// <summary>
    /// Writes the manifest of a MCBBS modpack.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="instance">The exported instance.</param>
    /// <param name="info">The information of the exported pack.</param>
    private static void WriteMcbbsManifest(ZipArchive zip, GameInstance instance, ModpackExportInfo info)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            writer.WriteStartObject();
            writer.WriteString("manifestType", "minecraftModpack");
            writer.WriteNumber("manifestVersion", 1);
            writer.WriteString("name", info.Name);
            writer.WriteString("version", info.Version);
            writer.WriteString("author", info.Author);
            writer.WriteString("description", info.Description);
            writer.WriteStartArray("addons");
            writer.WriteStartObject();
            writer.WriteString("id", "game");
            writer.WriteString("version", instance.Version);
            writer.WriteEndObject();
            writer.WriteEndArray();
            writer.WriteEndObject();
        }

        WriteEntry(zip, "mcbbs.pack-manifest.json", stream.ToArray());
    }

    /// <summary>
    /// Writes the manifest of a server pack.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="instance">The exported instance.</param>
    /// <param name="info">The information of the exported pack.</param>
    private static void WriteServerManifest(ZipArchive zip, GameInstance instance, ModpackExportInfo info)
    {
        using var stream = new MemoryStream();
        using (var writer = new Utf8JsonWriter(stream, new JsonWriterOptions { Indented = true }))
        {
            writer.WriteStartObject();
            writer.WriteString("name", info.Name);
            writer.WriteString("version", info.Version);
            writer.WriteString("author", info.Author);
            writer.WriteString("description", info.Description);
            writer.WriteString("minecraftVersion", instance.Version);
            writer.WriteEndObject();
        }

        WriteEntry(zip, "pack.json", stream.ToArray());
    }

    /// <summary>
    /// Copies the selected entries of the directory of the instance into the
    /// archive.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="instanceDir">The directory of the instance.</param>
    /// <param name="prefix">The directory of the archive receiving the files.</param>
    /// <param name="options">The options of the export.</param>
    /// <param name="includeWorld">Whether the worlds of the instance are copied.</param>
    private static void WriteEntries(
        ZipArchive zip,
        string instanceDir,
        string prefix,
        ModpackExportOptions options,
        bool includeWorld = false)
    {
        var entries = options.IncludedEntries.Count > 0
            ? options.IncludedEntries
            : SuggestedEntries;

        foreach (var entry in entries)
        {
            if (ExcludedEntries.Contains(entry, StringComparer.OrdinalIgnoreCase))
                continue;

            if (!includeWorld && entry.Equals("world", StringComparison.OrdinalIgnoreCase))
                continue;

            var source = Path.Combine(instanceDir, entry);
            if (!File.Exists(source) && !Directory.Exists(source))
                continue;

            var destination = string.IsNullOrEmpty(prefix)
                ? entry
                : Path.Combine(prefix, entry);

            if (Directory.Exists(source))
            {
                foreach (var file in Directory.GetFiles(source, "*", SearchOption.AllDirectories))
                {
                    var relative = Path.GetRelativePath(source, file);
                    if (IsExcludedFile(relative, options))
                        continue;

                    AddZipEntry(zip, file, Path.Combine(destination, relative));
                }
            }
            else
            {
                AddZipEntry(zip, source, destination);
            }
        }
    }

    /// <summary>
    /// Returns whether the given file is left out of the exported pack.
    /// </summary>
    /// <param name="relativePath">The path of the file, relative to the copied directory.</param>
    /// <param name="options">The options of the export.</param>
    /// <returns>Whether the file is left out.</returns>
    private static bool IsExcludedFile(string relativePath, ModpackExportOptions options)
    {
        var name = Path.GetFileName(relativePath);

        if (ExcludedEntries.Contains(name, StringComparer.OrdinalIgnoreCase))
            return true;

        if (!options.IncludeOptional && name.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase))
            return true;

        return false;
    }

    /// <summary>
    /// Writes the given bytes into a new entry of the archive.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="entryPath">The path of the entry in the archive.</param>
    /// <param name="content">The content of the entry.</param>
    private static void WriteEntry(ZipArchive zip, string entryPath, byte[] content)
    {
        var entry = zip.CreateEntry(entryPath.Replace('\\', '/'));
        using var stream = entry.Open();
        stream.Write(content, 0, content.Length);
    }

    /// <summary>
    /// Copies a file into a new entry of the archive.
    /// </summary>
    /// <param name="zip">The archive being written.</param>
    /// <param name="filePath">The path of the copied file.</param>
    /// <param name="entryPath">The path of the entry in the archive.</param>
    private static void AddZipEntry(ZipArchive zip, string filePath, string entryPath)
    {
        var normalizedEntryPath = entryPath.Replace('\\', '/');
        var entry = zip.CreateEntry(normalizedEntryPath);
        using var entryStream = entry.Open();
        using var fileStream = File.OpenRead(filePath);
        fileStream.CopyTo(entryStream);
    }
}
