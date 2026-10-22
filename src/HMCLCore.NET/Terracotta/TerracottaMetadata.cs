using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Terracotta;

/// <summary>
/// The metadata of the Terracotta bundles, read from the file shipped with
/// the launcher.
/// </summary>
public sealed class TerracottaMetadata
{
    /// <summary>
    /// The name of the embedded file holding the metadata.
    /// </summary>
    public const string ResourceName = "HMCLCore.NET.Resources.terracotta.json";

    /// <summary>
    /// The metadata of the bundles, loaded from the embedded file.
    /// </summary>
    public static TerracottaMetadata? Current { get; } = Load();

    /// <summary>
    /// The newest version of the Terracotta bundles.
    /// </summary>
    public string LatestVersion { get; init; } = string.Empty;

    /// <summary>
    /// The packages of the bundles, keyed by the platform they are built for.
    /// </summary>
    public IReadOnlyDictionary<string, TerracottaPackage> Packages { get; init; } =
        new Dictionary<string, TerracottaPackage>(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// The addresses of the packages.
    /// </summary>
    public IReadOnlyList<string> Downloads { get; init; } = Array.Empty<string>();

    /// <summary>
    /// The addresses of the packages in the Chinese mirrors.
    /// </summary>
    public IReadOnlyList<string> DownloadsChina { get; init; } = Array.Empty<string>();

    /// <summary>
    /// The links shown to the user.
    /// </summary>
    public IReadOnlyList<TerracottaLink> Links { get; init; } = Array.Empty<TerracottaLink>();

    /// <summary>
    /// The platform the bundle of the machine is built for, for example
    /// <c>windows-x86_64</c>.
    /// </summary>
    public static string? CurrentClassifier
    {
        get
        {
            var os = Platform.CheckedOsName;
            var arch = GetArchName();
            return arch is null ? null : $"{os}-{arch}";
        }
    }

    /// <summary>
    /// Returns the name of the architecture used by the bundles.
    /// </summary>
    /// <returns>The name of the architecture, or null when it is not supported.</returns>
    private static string? GetArchName() => Platform.CurrentArch switch
    {
        Arch.X86_64 => "x86_64",
        Arch.ARM64 => "arm64",
        Arch.ARM32 => "arm64",
        _ => null
    };

    /// <summary>
    /// Loads the metadata from the embedded file.
    /// </summary>
    /// <returns>The metadata, or null when the file cannot be read.</returns>
    private static TerracottaMetadata? Load()
    {
        try
        {
            using var stream = typeof(TerracottaMetadata).GetTypeInfo().Assembly
                .GetManifestResourceStream(ResourceName);

            if (stream is null)
                return null;

            using var document = JsonDocument.Parse(stream);
            var root = document.RootElement;

            var packages = new Dictionary<string, TerracottaPackage>(StringComparer.OrdinalIgnoreCase);
            if (root.TryGetProperty("packages", out var packagesElement) &&
                packagesElement.ValueKind == JsonValueKind.Object)
            {
                foreach (var package in packagesElement.EnumerateObject())
                {
                    var files = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
                    if (package.Value.TryGetProperty("files", out var filesElement) &&
                        filesElement.ValueKind == JsonValueKind.Object)
                    {
                        foreach (var file in filesElement.EnumerateObject())
                            files[file.Name] = file.Value.GetString() ?? string.Empty;
                    }

                    packages[package.Name] = new TerracottaPackage
                    {
                        Hash = package.Value.TryGetProperty("hash", out var hash) ? hash.GetString() ?? string.Empty : string.Empty,
                        Files = files
                    };
                }
            }

            return new TerracottaMetadata
            {
                LatestVersion = root.TryGetProperty("version_latest", out var version)
                    ? version.GetString() ?? string.Empty
                    : string.Empty,
                Packages = packages,
                Downloads = ReadStringArray(root, "downloads"),
                DownloadsChina = ReadStringArray(root, "downloads_CN"),
                Links = ReadLinks(root)
            };
        }
        catch (Exception ex) when (ex is IOException or JsonException or InvalidOperationException)
        {
            return null;
        }
    }

    /// <summary>
    /// Reads an array of strings from the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The values of the member.</returns>
    private static IReadOnlyList<string> ReadStringArray(JsonElement root, string name)
    {
        if (!root.TryGetProperty(name, out var element) || element.ValueKind != JsonValueKind.Array)
            return Array.Empty<string>();

        return element.EnumerateArray()
            .Where(item => item.ValueKind == JsonValueKind.String)
            .Select(item => item.GetString() ?? string.Empty)
            .Where(item => !string.IsNullOrEmpty(item))
            .ToList();
    }

    /// <summary>
    /// Reads the links shown to the user from the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <returns>The links of the metadata.</returns>
    private static IReadOnlyList<TerracottaLink> ReadLinks(JsonElement root)
    {
        if (!root.TryGetProperty("links", out var element) || element.ValueKind != JsonValueKind.Array)
            return Array.Empty<TerracottaLink>();

        var links = new List<TerracottaLink>();
        foreach (var item in element.EnumerateArray())
        {
            if (!item.TryGetProperty("link", out var link) || link.ValueKind != JsonValueKind.String)
                continue;

            var description = string.Empty;
            if (item.TryGetProperty("desc", out var desc) && desc.ValueKind == JsonValueKind.Object)
            {
                foreach (var entry in desc.EnumerateObject())
                {
                    if (entry.Value.ValueKind == JsonValueKind.String)
                    {
                        description = entry.Value.GetString() ?? string.Empty;
                        break;
                    }
                }
            }

            links.Add(new TerracottaLink { Description = description, Link = link.GetString() ?? string.Empty });
        }

        return links;
    }

    /// <summary>
    /// Returns the addresses of the package of the current platform, the
    /// Chinese mirrors first when the machine is in the mainland of China.
    /// </summary>
    /// <returns>The addresses of the package.</returns>
    public IReadOnlyList<string> GetDownloadLinks()
    {
        var links = new List<string>();

        var primary = RegionUtils.IsChinaMainland ? DownloadsChina : Downloads;
        var secondary = RegionUtils.IsChinaMainland ? Downloads : DownloadsChina;

        foreach (var template in primary.Concat(secondary))
        {
            var link = template
                .Replace("${version}", LatestVersion, StringComparison.Ordinal)
                .Replace("${classifier}", CurrentClassifier ?? string.Empty, StringComparison.Ordinal);

            if (!string.IsNullOrEmpty(link))
                links.Add(link);
        }

        return links;
    }

    /// <summary>
    /// The name of the package of the current platform, like the name expected
    /// by the third-party download channels.
    /// </summary>
    public string PackageName =>
        $"terracotta-{LatestVersion}-{CurrentClassifier ?? "unknown"}-pkg.tar.gz";

    /// <summary>
    /// Returns the links shown to the user, with the placeholders replaced.
    /// </summary>
    /// <returns>The links of the bundles.</returns>
    public IReadOnlyList<TerracottaLink> GetLinks()
        => Links.Select(link => new TerracottaLink
        {
            Description = link.Description,
            Link = link.Link.Replace("${version}", LatestVersion, StringComparison.Ordinal)
        }).ToList();
}

/// <summary>
/// The package of a Terracotta bundle for one platform.
/// </summary>
public sealed class TerracottaPackage
{
    /// <summary>
    /// The SHA-512 hash of the package.
    /// </summary>
    public string Hash { get; init; } = string.Empty;

    /// <summary>
    /// The SHA-512 hashes of the files of the package, keyed by their name.
    /// </summary>
    public IReadOnlyDictionary<string, string> Files { get; init; } =
        new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
}

/// <summary>
/// A link shown to the user on the online page.
/// </summary>
public sealed class TerracottaLink
{
    /// <summary>
    /// The description of the link.
    /// </summary>
    public string Description { get; init; } = string.Empty;

    /// <summary>
    /// The address of the link.
    /// </summary>
    public string Link { get; init; } = string.Empty;
}
