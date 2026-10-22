using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace HMCLCore.NET.Addon;

/// <summary>
/// Metadata read from a mod file (jar/zip) describing the mod it contains.
/// </summary>
public sealed class AddonMetadata
{
    /// <summary>
    /// The mod identifier, e.g. the Fabric mod id or the Forge modId.
    /// This usually matches the project slug on remote repositories.
    /// </summary>
    public string ModId { get; init; } = string.Empty;

    /// <summary>
    /// The human-readable mod name.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The mod version.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The mod description.
    /// </summary>
    public string? Description { get; init; }

    /// <summary>
    /// The mod loader the mod was built for.
    /// </summary>
    public ModLoaderType Loader { get; init; } = ModLoaderType.None;

    /// <summary>
    /// The mod authors, joined by ", ".
    /// </summary>
    public string? Authors { get; init; }
}

/// <summary>
/// Reads mod metadata from the descriptor files inside mod archives.
/// Supports Fabric, Quilt, Forge, NeoForge and LiteLoader descriptors.
/// </summary>
public static class AddonMetadataReader
{
    /// <summary>
    /// The file names that are recognized as mod descriptors.
    /// </summary>
    private static readonly string[] DescriptorNames =
    {
        "fabric.mod.json",
        "quilt.mod.json",
        "META-INF/mods.toml",
        "META-INF/neoforge.mods.toml",
        "mcmod.info",
        "litemod.json"
    };

    /// <summary>
    /// Reads the metadata of a mod archive.
    /// </summary>
    /// <param name="filePath">The path of the jar/zip/litemod file.</param>
    /// <returns>The metadata, or null when the file is not a mod or cannot be read.</returns>
    public static AddonMetadata? Read(string filePath)
    {
        if (string.IsNullOrEmpty(filePath) || !File.Exists(filePath))
            return null;

        var extension = Path.GetExtension(filePath).ToLowerInvariant();
        if (extension is not (".jar" or ".zip" or ".litemod"))
            return null;

        try
        {
            using var archive = ZipFile.OpenRead(filePath);
            foreach (var descriptor in DescriptorNames)
            {
                var entry = archive.GetEntry(descriptor);
                if (entry == null)
                    continue;

                using var stream = entry.Open();
                var metadata = ParseDescriptor(descriptor, stream);
                if (metadata != null)
                    return metadata;
            }
        }
        catch
        {
            // Ignore unreadable or malformed archives.
        }

        return null;
    }

    /// <summary>
    /// Parses a mod descriptor from the given stream.
    /// </summary>
    /// <param name="descriptorName">The descriptor file name inside the archive.</param>
    /// <param name="stream">The descriptor content.</param>
    /// <returns>The parsed metadata, or null when the descriptor is unsupported.</returns>
    public static AddonMetadata? ParseDescriptor(string descriptorName, Stream stream)
    {
        try
        {
            return descriptorName switch
            {
                "fabric.mod.json" => ParseFabric(stream),
                "quilt.mod.json" => ParseQuilt(stream),
                "META-INF/mods.toml" => ParseForgeToml(stream, ModLoaderType.Forge),
                "META-INF/neoforge.mods.toml" => ParseForgeToml(stream, ModLoaderType.NeoForge),
                "mcmod.info" => ParseMcmodInfo(stream),
                "litemod.json" => ParseLiteMod(stream),
                _ => null
            };
        }
        catch
        {
            return null;
        }
    }

    private static AddonMetadata? ParseFabric(Stream stream)
    {
        using var document = JsonDocument.Parse(stream);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object)
            return null;

        var id = GetString(root, "id");
        if (string.IsNullOrEmpty(id))
            return null;

        return new AddonMetadata
        {
            ModId = id,
            Name = GetString(root, "name") ?? id,
            Version = GetString(root, "version") ?? string.Empty,
            Description = GetString(root, "description"),
            Authors = ParseFabricAuthors(root),
            Loader = ModLoaderType.Fabric
        };
    }

    private static string? ParseFabricAuthors(JsonElement root)
    {
        if (!root.TryGetProperty("authors", out var authors) || authors.ValueKind != JsonValueKind.Array)
            return null;

        var names = new List<string>();
        foreach (var author in authors.EnumerateArray())
        {
            if (author.ValueKind == JsonValueKind.String)
            {
                names.Add(author.GetString() ?? string.Empty);
            }
            else if (author.ValueKind == JsonValueKind.Object)
            {
                var name = GetString(author, "name");
                if (!string.IsNullOrEmpty(name))
                    names.Add(name);
            }
        }

        return names.Count > 0 ? string.Join(", ", names) : null;
    }

    private static AddonMetadata? ParseQuilt(Stream stream)
    {
        using var document = JsonDocument.Parse(stream);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object)
            return null;

        // Quilt stores the identity under "quilt_loader" and repeats the
        // display fields under "metadata".
        var loader = root.TryGetProperty("quilt_loader", out var loaderElement) && loaderElement.ValueKind == JsonValueKind.Object
            ? loaderElement
            : root;

        var id = GetString(loader, "id");
        if (string.IsNullOrEmpty(id))
            return null;

        var name = GetString(loader, "name") ?? id;
        var version = GetString(loader, "version") ?? string.Empty;
        string? description = GetString(loader, "description");

        if (root.TryGetProperty("metadata", out var metadata) && metadata.ValueKind == JsonValueKind.Object)
        {
            name = GetString(metadata, "name") ?? name;
            version = GetString(metadata, "version") ?? version;
            description = GetString(metadata, "description") ?? description;
        }

        return new AddonMetadata
        {
            ModId = id,
            Name = name,
            Version = version,
            Description = description,
            Loader = ModLoaderType.Quilt
        };
    }

    private static AddonMetadata? ParseForgeToml(Stream stream, ModLoaderType loader)
    {
        using var reader = new StreamReader(stream);
        var content = reader.ReadToEnd();

        // The descriptor uses one [[mods]] table per mod; the first one is
        // the primary mod of the archive.
        var section = content;
        var sectionIndex = content.IndexOf("[[mods]]", StringComparison.Ordinal);
        if (sectionIndex >= 0)
        {
            var next = content.IndexOf("[[mods]]", sectionIndex + 8, StringComparison.Ordinal);
            section = next >= 0 ? content[sectionIndex..next] : content[sectionIndex..];
        }

        var modId = ReadTomlString(section, "modId");
        if (string.IsNullOrEmpty(modId))
            return null;

        return new AddonMetadata
        {
            ModId = modId,
            Name = ReadTomlString(section, "displayName") ?? modId,
            Version = ReadTomlString(section, "version") ?? string.Empty,
            Description = ReadTomlString(section, "description"),
            Authors = ReadTomlString(section, "authors"),
            Loader = loader
        };
    }

    private static string? ReadTomlString(string content, string key)
    {
        var pattern = $"\"{Regex.Escape(key)}\"\\s*=\\s*\"([^\"]*)\"";
        var match = Regex.Match(content, pattern, RegexOptions.IgnoreCase);
        return match.Success ? match.Groups[1].Value : null;
    }

    private static AddonMetadata? ParseMcmodInfo(Stream stream)
    {
        using var document = JsonDocument.Parse(stream);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Array)
            return null;

        // The legacy format is a list of "modList" entries.
        foreach (var element in root.EnumerateArray())
        {
            if (element.ValueKind != JsonValueKind.Object)
                continue;

            if (!element.TryGetProperty("modList", out var list) || list.ValueKind != JsonValueKind.Array)
                continue;

            foreach (var entry in list.EnumerateArray())
            {
                if (entry.ValueKind != JsonValueKind.Object)
                    continue;

                var modId = GetString(entry, "modid");
                if (string.IsNullOrEmpty(modId))
                    continue;

                return new AddonMetadata
                {
                    ModId = modId,
                    Name = GetString(entry, "name") ?? modId,
                    Version = GetString(entry, "version") ?? string.Empty,
                    Description = GetString(entry, "description"),
                    Authors = GetString(entry, "author"),
                    Loader = ModLoaderType.Forge
                };
            }
        }

        return null;
    }

    private static AddonMetadata? ParseLiteMod(Stream stream)
    {
        using var document = JsonDocument.Parse(stream);
        var root = document.RootElement;
        if (root.ValueKind != JsonValueKind.Object)
            return null;

        var name = GetString(root, "name");
        if (string.IsNullOrEmpty(name))
            return null;

        return new AddonMetadata
        {
            ModId = name,
            Name = name,
            Version = GetString(root, "version") ?? string.Empty,
            Description = GetString(root, "description"),
            Authors = GetString(root, "author"),
            Loader = ModLoaderType.LiteLoader
        };
    }

    private static string? GetString(JsonElement element, string propertyName)
    {
        if (!element.TryGetProperty(propertyName, out var value))
            return null;

        return value.ValueKind switch
        {
            JsonValueKind.String => value.GetString(),
            JsonValueKind.Number => value.GetRawText(),
            _ => null
        };
    }
}
