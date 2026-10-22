using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;
using HMCLCore.NET.Game;

namespace HMCLCore.NET.Download;

/// <summary>
/// Supported meta-based mod loaders.
/// </summary>
public enum LoaderKind
{
    /// <summary>Fabric loader.</summary>
    Fabric,

    /// <summary>Quilt loader.</summary>
    Quilt,

    /// <summary>LegacyFabric loader, the continuation of Fabric for the versions before 1.14.</summary>
    LegacyFabric,

    /// <summary>Forge loader (installed through the official installer).</summary>
    Forge,

    /// <summary>NeoForge loader (installed through the official installer).</summary>
    NeoForge,

    /// <summary>OptiFine (installed through its own installer).</summary>
    OptiFine,

    /// <summary>LiteLoader (installed through its own installer).</summary>
    LiteLoader,

    /// <summary>Cleanroom (installed through its own installer).</summary>
    Cleanroom
}

/// <summary>
/// Installs Fabric/Quilt by fetching their meta profile JSON and merging it
/// into the vanilla version manifest.
/// </summary>
public static class MetaLoaderInstaller
{
    private static readonly HttpClient HttpClient = CreateHttpClient();

    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    /// <summary>
    /// Returns the available loader versions for a Minecraft version.
    /// </summary>
    /// <param name="kind">The loader kind.</param>
    /// <param name="gameVersion">The Minecraft version.</param>
    /// <returns>Loader versions with their stability flag, newest first.</returns>
    public static async Task<List<LoaderVersionInfo>> GetLoaderVersionsAsync(
        LoaderKind kind, string gameVersion)
    {
        // Forge/NeoForge are resolved through their own promotions/installer APIs.
        if (kind is LoaderKind.Forge or LoaderKind.NeoForge)
            return new List<LoaderVersionInfo>();

        var url = kind switch
        {
            LoaderKind.Fabric => $"https://meta.fabricmc.net/v2/versions/loader/{gameVersion}",
            LoaderKind.Quilt => $"https://meta.quiltmc.org/v3/versions/loader/{gameVersion}",
            LoaderKind.LegacyFabric => $"https://meta.legacyfabric.net/v2/versions/loader/{gameVersion}",
            _ => throw new ArgumentOutOfRangeException(nameof(kind))
        };

        var json = await HttpClientProvider.Shared.GetStringAsync(url);
        var entries = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.ListMetaLoaderEntry)
                      ?? new List<MetaLoaderEntry>();

        return entries
            .Where(entry => !string.IsNullOrEmpty(entry.Loader?.Version))
            .Select(entry => new LoaderVersionInfo
            {
                Version = entry.Loader!.Version!,
                Stable = entry.Loader.Stable
            })
            .ToList();
    }

    /// <summary>
    /// Fetches the loader profile and merges it into the vanilla manifest.
    /// </summary>
    /// <param name="kind">The loader kind.</param>
    /// <param name="vanilla">The vanilla instance manifest.</param>
    /// <param name="loaderVersion">
    /// The loader version; when null or empty the newest stable version is used.
    /// </param>
    /// <returns>The merged manifest, or null when the loader is unavailable.</returns>
    public static async Task<GameInstanceManifest?> CreateManifestAsync(
        LoaderKind kind, GameInstanceManifest vanilla, string? loaderVersion = null)
    {
        if (kind is LoaderKind.Forge or LoaderKind.NeoForge)
            return null;

        if (string.IsNullOrWhiteSpace(vanilla.Version))
            return null;

        var loader = loaderVersion;
        if (string.IsNullOrWhiteSpace(loader))
        {
            var versions = await GetLoaderVersionsAsync(kind, vanilla.Version);
            var stable = versions.FirstOrDefault(v => v.Stable);
            loader = stable?.Version ?? versions.FirstOrDefault()?.Version;
        }

        if (string.IsNullOrWhiteSpace(loader))
            return null;

        var profileUrl = kind switch
        {
            LoaderKind.Fabric => $"https://meta.fabricmc.net/v2/versions/loader/{vanilla.Version}/{loader}/profile/json",
            LoaderKind.Quilt => $"https://meta.quiltmc.org/v3/versions/loader/{vanilla.Version}/{loader}/profile/json",
            LoaderKind.LegacyFabric => $"https://meta.legacyfabric.net/v2/versions/loader/{vanilla.Version}/{loader}/profile/json",
            _ => throw new ArgumentOutOfRangeException(nameof(kind))
        };

        var json = await HttpClientProvider.Shared.GetStringAsync(profileUrl);
        var profile = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.MetaLoaderProfile);
        if (profile == null)
            return null;

        // Merge libraries, deduplicating by Maven coordinates.
        var libraries = new List<Library>(vanilla.Libraries);
        var seen = new HashSet<string>(libraries.Select(l => l.Name), StringComparer.OrdinalIgnoreCase);
        var patchLibraries = new List<Library>();
        foreach (var library in profile.Libraries ?? new List<MetaLoaderLibrary>())
        {
            if (string.IsNullOrEmpty(library.Name) || !seen.Add(library.Name))
                continue;

            patchLibraries.Add(new Library
            {
                Name = library.Name,
                Url = library.Url,
                Natives = SelectNativeClassifier(library.Natives),
                Extract = CreateExtractRules(library.Extract),
                Downloads = CreateLibraryDownloads(library)
            });
        }

        // Merge plain-string arguments (Fabric/Quilt profiles only use strings).
        var jvmArguments = new List<string>();
        jvmArguments.AddRange(ReadArguments(profile.Arguments, "jvm"));
        var gameArguments = new List<string>();
        gameArguments.AddRange(ReadArguments(profile.Arguments, "game"));

        var patch = new GameInstancePatch
        {
            Id = kind switch
            {
                LoaderKind.Fabric => "fabric",
                LoaderKind.Quilt => "quilt",
                LoaderKind.LegacyFabric => "legacyfabric",
                _ => null
            },
            Version = loader,
            Priority = GameInstancePatch.PriorityLoader,
            MainClass = profile.MainClass,
            Libraries = patchLibraries,
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = jvmArguments,
                Game = gameArguments
            }
        };

        return patch.ApplyTo(vanilla);
    }

    /// <summary>
    /// Converts the extract section of a loader profile into the extract rules
    /// of a library.
    /// </summary>
    /// <param name="extract">The extract section of the profile; may be null.</param>
    /// <returns>The extract rules, or null when the profile excludes
    /// nothing.</returns>
    private static List<ExtractRule>? CreateExtractRules(MetaLoaderExtract? extract)
    {
        if (extract?.Exclude == null || extract.Exclude.Count == 0)
            return null;

        var rules = new List<ExtractRule>();
        foreach (var pattern in extract.Exclude)
        {
            if (!string.IsNullOrEmpty(pattern))
                rules.Add(new ExtractRule { Pattern = pattern, Exclude = true });
        }

        return rules.Count > 0 ? rules : null;
    }

    /// <summary>
    /// Creates the download information of a library of a loader profile.
    /// </summary>
    /// <param name="library">The library of the profile.</param>
    /// <returns>
    /// The download information of the library, or null when the library
    /// cannot be downloaded.
    /// </returns>
    private static LibraryDownloads? CreateLibraryDownloads(MetaLoaderLibrary library)
    {
        if (library.Downloads?.Artifact != null)
        {
            return new LibraryDownloads
            {
                Artifact = new DownloadInfo
                {
                    Url = library.Downloads.Artifact.Url,
                    Sha1 = library.Downloads.Artifact.Sha1,
                    Size = library.Downloads.Artifact.Size
                }
            };
        }

        // The profiles of LegacyFabric describe their libraries with the
        // coordinates and the Maven repository only; the download URL is
        // derived from them.
        var url = BuildMavenUrl(library.Url, library.Name);
        if (string.IsNullOrEmpty(url))
            return null;

        return new LibraryDownloads
        {
            Artifact = new DownloadInfo
            {
                Url = url,
                Sha1 = library.Sha1 ?? string.Empty,
                Size = library.Size
            }
        };
    }

    /// <summary>
    /// Builds the download URL of a library from its Maven coordinates.
    /// </summary>
    /// <param name="baseUrl">The base URL of the Maven repository.</param>
    /// <param name="name">The Maven coordinates of the library.</param>
    /// <returns>The download URL, or an empty string when the coordinates are
    /// incomplete.</returns>
    private static string BuildMavenUrl(string? baseUrl, string? name)
    {
        if (string.IsNullOrEmpty(baseUrl) || string.IsNullOrEmpty(name))
            return string.Empty;

        var parts = name.Split(':');
        if (parts.Length < 3)
            return string.Empty;

        var groupPath = parts[0].Replace('.', '/');
        var artifact = parts[1];
        var version = parts[2];
        var classifier = parts.Length >= 4 && !string.IsNullOrEmpty(parts[3]) ? $"-{parts[3]}" : string.Empty;

        var root = baseUrl.EndsWith('/') ? baseUrl : baseUrl + "/";
        return $"{root}{groupPath}/{artifact}/{version}/{artifact}-{version}{classifier}.jar";
    }

    /// <summary>
    /// Reads the plain-string entries of one argument group from a loader profile.
    /// </summary>
    private static IEnumerable<string> ReadArguments(
        Dictionary<string, JsonElement>? arguments, string key)
    {
        if (arguments == null ||
            !arguments.TryGetValue(key, out var element) ||
            element.ValueKind != JsonValueKind.Array)
            yield break;

        foreach (var item in element.EnumerateArray())
        {
            if (item.ValueKind == JsonValueKind.String && item.GetString() is { } value)
                yield return value;
        }
    }

    /// <summary>
    /// Selects the native classifier of a library for the current operating
    /// system.
    /// </summary>
    /// <param name="natives">The native classifiers of the library, keyed by
    /// the name of the operating system.</param>
    /// <returns>The classifier of the current operating system, or null when
    /// the library has no natives.</returns>
    private static List<string>? SelectNativeClassifier(Dictionary<string, string>? natives)
    {
        if (natives == null || natives.Count == 0)
            return null;

        var osName = HMCLCore.NET.Util.Platform.MojangOsName;
        if (!natives.TryGetValue(osName, out var classifier) || string.IsNullOrEmpty(classifier))
            return null;

        classifier = classifier.Replace("${arch}", Environment.Is64BitProcess ? "64" : "32");
        return new List<string> { classifier };
    }

    /// <summary>
    /// Creates the shared HTTP client with a descriptive user agent.
    /// </summary>
    private static HttpClient CreateHttpClient()
    {
        var client = HttpClientProvider.CreateClient();
        client.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");
        return client;
    }

    internal sealed class MetaLoaderEntry
    {
        [System.Text.Json.Serialization.JsonPropertyName("loader")]
        public MetaLoaderInfo? Loader { get; init; }
    }

    internal sealed class MetaLoaderInfo
    {
        [System.Text.Json.Serialization.JsonPropertyName("version")]
        public string? Version { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("stable")]
        public bool Stable { get; init; }
    }

    internal sealed class MetaLoaderProfile
    {
        [System.Text.Json.Serialization.JsonPropertyName("mainClass")]
        public string? MainClass { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("arguments")]
        public Dictionary<string, JsonElement>? Arguments { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("libraries")]
        public List<MetaLoaderLibrary>? Libraries { get; init; }
    }

    /// <summary>
    /// The extract section of a library of a loader profile.
    /// </summary>
    internal sealed class MetaLoaderExtract
    {
        [System.Text.Json.Serialization.JsonPropertyName("exclude")]
        public List<string>? Exclude { get; init; }
    }

    internal sealed class MetaLoaderLibrary
    {
        [System.Text.Json.Serialization.JsonPropertyName("name")]
        public string? Name { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("url")]
        public string? Url { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("natives")]
        public Dictionary<string, string>? Natives { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("sha1")]
        public string? Sha1 { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("size")]
        public long Size { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("extract")]
        public MetaLoaderExtract? Extract { get; init; }

        [System.Text.Json.Serialization.JsonPropertyName("downloads")]
        public MetaLoaderDownloads? Downloads { get; init; }
    }

    internal sealed class MetaLoaderDownloads
    {
        [System.Text.Json.Serialization.JsonPropertyName("artifact")]
        public MetaLoaderArtifact? Artifact { get; init; }
    }

    internal sealed class MetaLoaderArtifact
    {
        [System.Text.Json.Serialization.JsonPropertyName("url")]
        public string Url { get; init; } = string.Empty;

        [System.Text.Json.Serialization.JsonPropertyName("sha1")]
        public string Sha1 { get; init; } = string.Empty;

        [System.Text.Json.Serialization.JsonPropertyName("size")]
        public long Size { get; init; }
    }
}

/// <summary>
/// A loader version returned by the loader metadata API.
/// </summary>
public sealed class LoaderVersionInfo
{
    /// <summary>
    /// The loader version string.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// Whether the loader version is marked stable.
    /// </summary>
    public bool Stable { get; init; }
}
