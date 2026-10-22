using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using HMCLCore.NET.Events;
using HMCLCore.NET.Tasks;
using HMCLCore.NET.Util.Versioning;

namespace HMCLCore.NET.Download;

/// <summary>
/// Metadata for a Minecraft version.
/// </summary>
public sealed class VersionMetadata
{
    /// <summary>
    /// The version ID (e.g., "1.20.1").
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The version type (release, snapshot, etc.).
    /// </summary>
    public string Type { get; init; } = "release";

    /// <summary>
    /// The release time.
    /// </summary>
    public DateTime ReleaseTime { get; init; }

    /// <summary>
    /// The URL to download the version JSON.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The SHA1 hash of the version JSON.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// The size of the version JSON.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// Whether this is a release version.
    /// </summary>
    public bool IsRelease => Type == "release";

    /// <summary>
    /// Whether this is a snapshot version.
    /// </summary>
    public bool IsSnapshot => Type == "snapshot";

    /// <summary>
    /// Whether this is an old beta version.
    /// </summary>
    public bool IsOldBeta => Type.Equals("old_beta", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Whether this is an old alpha version.
    /// </summary>
    public bool IsOldAlpha => Type.Equals("old_alpha", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Whether this is an April Fools' Day version, mirroring the Java HMCL
    /// behaviour that only marks the snapshots of that day.
    /// </summary>
    public bool IsAprilFools =>
        IsSnapshot && GameVersionNumber.IsAprilFools(GameVersionNumber.AsGameVersion(Id));

    /// <summary>
    /// The release time formatted for UI display, or empty when unknown.
    /// </summary>
    public string ReleaseTimeText =>
        ReleaseTime == default ? string.Empty : ReleaseTime.ToString("yyyy-MM-dd");
}

/// <summary>
/// Manages version index/manifest downloading and caching.
/// </summary>
public sealed class VersionIndexManager
{
    private readonly string _cacheDirectory;
    private readonly DownloadProviderManager _providerManager;

    public VersionIndexManager(string cacheDirectory, DownloadProviderManager providerManager)
    {
        _cacheDirectory = cacheDirectory;
        _providerManager = providerManager;
    }

    /// <summary>
    /// Downloads the version index from Mojang.
    /// </summary>
    public async Task<string> DownloadVersionIndexAsync(CancellationToken token = default)
    {
        var uri = new Uri("https://launchermeta.mojang.com/mc/game/version_manifest.json");
        var dest = Path.Combine(_cacheDirectory, "version_manifest.json");

        using var httpClient = HttpClientProvider.CreateClient();
        using var response = await HttpClientProvider.Shared.GetAsync(uri, token);
        response.EnsureSuccessStatusCode();

        Directory.CreateDirectory(_cacheDirectory);
        using var fileStream = File.Create(dest);
        using var contentStream = await response.Content.ReadAsStreamAsync(token);
        await contentStream.CopyToAsync(fileStream);

        return dest;
    }
}

/// <summary>
/// Helper for version index operations.
/// </summary>
public static class VersionIndexHelper
{
    /// <summary>
    /// Parses a version manifest JSON into version metadata entries.
    /// </summary>
    public static List<VersionMetadata> ParseVersionManifest(string json)
    {
        using var doc = JsonDocument.Parse(json);
        var versions = new List<VersionMetadata>();

        if (doc.RootElement.TryGetProperty("versions", out var versionsElement))
        {
            foreach (var version in versionsElement.EnumerateArray())
            {
                versions.Add(new VersionMetadata
                {
                    Id = version.GetProperty("id").GetString() ?? string.Empty,
                    Type = version.GetProperty("type").GetString() ?? "release",
                    ReleaseTime = version.GetProperty("releaseTime").GetDateTime(),
                    Url = version.GetProperty("url").GetString() ?? string.Empty
                });
            }
        }

        return versions;
    }
}

/// <summary>
/// Compares the versions of Minecraft by their game version number, so that
/// the releases, the snapshots and the versions of the older eras are ordered
/// the way the official launcher orders them.
/// </summary>
public sealed class VersionMetadataComparer : IComparer<VersionMetadata>
{
    /// <summary>
    /// The shared comparer.
    /// </summary>
    public static VersionMetadataComparer Instance { get; } = new();

    /// <inheritdoc/>
    public int Compare(VersionMetadata? x, VersionMetadata? y)
    {
        if (ReferenceEquals(x, y))
            return 0;
        if (x is null)
            return -1;
        if (y is null)
            return 1;

        return GameVersionNumber.Compare(x.Id, y.Id);
    }
}

/// <summary>
/// Download provider for Minecraft libraries.
/// </summary>
public sealed class LibraryDownloadProvider : DownloadProvider
{
    /// <summary>
    /// The base URL for library downloads.
    /// </summary>
    public string BaseUrl { get; init; } = "https://libraries.minecraft.net/";

    /// <summary>
    /// The name of this download provider.
    /// </summary>
    public override string Name => "Minecraft Libraries";

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public override bool CanHandle(Uri url)
    {
        var host = url.Host.ToLowerInvariant();
        return host.Contains("libraries.minecraft.net") ||
               host.Contains("repo.maven.apache.org") ||
               host.Contains("forgecdn.net");
    }
}

/// <summary>
/// Downloads game version files.
/// </summary>
public sealed class GameDownloadTask : DownloadTask
{
    /// <summary>
    /// The expected SHA-1 checksum of the file, or null when the file is not
    /// verified.
    /// </summary>
    public string? Sha1 { get; init; }

    /// <summary>
    /// Creates a new GameDownloadTask.
    /// </summary>
    public GameDownloadTask(Uri url, string destination) : base(url, destination)
    {
        Name = "下载游戏版本";
    }

    /// <summary>
    /// Downloads the game version JAR file.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                GetUrls(), Destination, new FileDownloadOptions { Sha1 = Sha1 }, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {Url}");
        }
        catch (OperationCanceledException)
        {
            throw new OperationCanceledException("Task was cancelled");
        }
    }
}

/// <summary>
/// Downloads game libraries.
/// </summary>
public sealed class GameLibrariesTask : DownloadTask
{
    /// <summary>
    /// The libraries to download.
    /// </summary>
    public List<HMCLCore.NET.Game.Library> Libraries { get; init; } = new();

    /// <summary>
    /// The libraries directory.
    /// </summary>
    public string LibrariesDirectory { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new GameLibrariesTask.
    /// </summary>
    public GameLibrariesTask(List<HMCLCore.NET.Game.Library> libraries, string librariesDirectory) : base(new Uri("about:blank"), string.Empty)
    {
        Libraries = libraries;
        LibrariesDirectory = librariesDirectory;
        Name = "下载游戏库";
    }

    /// <summary>
    /// Downloads all libraries.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        var index = 0;
        foreach (var library in Libraries)
        {
            token.ThrowIfCancellationRequested();

            if (!library.IsApplicableToCurrentOs())
                continue;

            var localPath = library.GetLocalPath(LibrariesDirectory);
            if (File.Exists(localPath))
            {
                index++;
                OnProgressChanged(index, Libraries.Count);
                continue;
            }

            var downloadUrl = library.Downloads?.Artifact?.Url;
            if (downloadUrl != null)
            {
                try
                {
                    var progress = new InlineProgress<DownloadProgressEventArgs>(
                        p => OnProgressChanged(index, Libraries.Count));

                    await FileDownloader.DownloadAsync(
                        MirrorSource.GetFileUris(downloadUrl),
                        localPath,
                        new FileDownloadOptions { Sha1 = library.Downloads?.Artifact?.Sha1 },
                        progress,
                        token);
                }
                catch
                {
                    // Library download failed, continue
                }
            }

            index++;
            OnProgressChanged(index, Libraries.Count);
        }
    }
}

/// <summary>
/// Represents version metadata for remote fetching.
/// </summary>
public sealed class VersionRemoteMetadata
{
    /// <summary>
    /// The version ID.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The version type.
    /// </summary>
    public string Type { get; init; } = "release";

    /// <summary>
    /// The release time.
    /// </summary>
    public DateTime ReleaseTime { get; init; }

    /// <summary>
    /// The URL to download the version JSON.
    /// </summary>
    public string Url { get; init; } = string.Empty;
}
