using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;
using HMCLCore.NET.Events;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Downloads the remote version manifest from Mojang.
/// </summary>
public sealed class GameRemoteVersionsTask : DownloadTask
{

    /// <summary>
    /// The URL to download the version manifest from.
    /// </summary>
    public string ManifestUrl { get; init; } = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

    /// <summary>
    /// Creates a new GameRemoteVersionsTask.
    /// </summary>
    public GameRemoteVersionsTask(string destination) : base(new Uri("about:blank"), destination)
    {
        Name = "获取远程版本列表";
    }

    /// <summary>
    /// Downloads the remote version manifest.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                GetUrls(), Destination, null, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {ManifestUrl}");
        }
        catch (OperationCanceledException)
        {
            throw new OperationCanceledException("Task was cancelled");
        }
    }
}

/// <summary>
/// Downloads assets from the Minecraft asset server.
/// </summary>
public sealed class AssetDownloadTask : DownloadTask
{
    /// <summary>
    /// The asset hash (SHA1).
    /// </summary>
    public string Hash { get; init; } = string.Empty;

    /// <summary>
    /// The asset size in bytes.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// The assets directory.
    /// </summary>
    public string AssetsDirectory { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new AssetDownloadTask.
    /// </summary>
    public AssetDownloadTask(string hash, long size, string assetsDirectory) : base(new Uri("about:blank"), string.Empty)
    {
        Hash = hash;
        Size = size;
        AssetsDirectory = assetsDirectory;
        Name = "下载资源";
    }

    /// <summary>
    /// Downloads the asset file.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            var assetDir = Path.Combine(AssetsDirectory, "objects", Hash.Substring(0, 2));
            Directory.CreateDirectory(assetDir);
            var assetPath = Path.Combine(assetDir, Hash);

            if (File.Exists(assetPath))
            {
                OnProgressChanged(100, 100);
                return;
            }

            // Assets are stored at https://resources.download.minecraft.net/<hash[:2]>/<hash>
            var assetUrl = $"https://resources.download.minecraft.net/{Hash.Substring(0, 2)}/{Hash}";

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                MirrorSource.GetFileUris(assetUrl), assetPath, new FileDownloadOptions { Sha1 = Hash }, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {assetUrl}");
        }
        catch (OperationCanceledException)
        {
            throw new OperationCanceledException("Task was cancelled");
        }
    }
}

/// <summary>
/// Downloads the asset index file.
/// </summary>
public sealed class AssetIndexDownloadTask : DownloadTask
{
    /// <summary>
    /// The asset index URL.
    /// </summary>
    public Uri IndexUrl { get; init; } = new("about:blank");

    /// <summary>
    /// The destination path for the asset index.
    /// </summary>
    public string DestinationPath { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new AssetIndexDownloadTask.
    /// </summary>
    public AssetIndexDownloadTask(Uri indexUrl, string destinationPath) : base(indexUrl, destinationPath)
    {
        IndexUrl = indexUrl;
        DestinationPath = destinationPath;
        Name = "下载资源索引";
    }

    /// <summary>
    /// Downloads the asset index file.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                IndexUrl, DestinationPath, null, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {IndexUrl}");
        }
        catch (OperationCanceledException)
        {
            throw new OperationCanceledException("Task was cancelled");
        }
    }
}

/// <summary>
/// Downloads native libraries and extracts them.
/// </summary>
public sealed class NativeLibraryDownloadTask : DownloadTask
{
    /// <summary>
    /// The library to download.
    /// </summary>
    public HMCLCore.NET.Game.Library Library { get; init; } = new();

    /// <summary>
    /// The natives directory.
    /// </summary>
    public string NativesDirectory { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new NativeLibraryDownloadTask.
    /// </summary>
    public NativeLibraryDownloadTask(HMCLCore.NET.Game.Library library, string nativesDirectory) : base(new Uri("about:blank"), string.Empty)
    {
        Library = library;
        NativesDirectory = nativesDirectory;
        Name = "下载原生库";
    }

    /// <summary>
    /// Downloads the native library.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            var localPath = Library.GetLocalPath(NativesDirectory);
            if (File.Exists(localPath))
            {
                OnProgressChanged(100, 100);
                return;
            }

            var downloadUrl = Library.Downloads?.Artifact?.Url;
            if (downloadUrl == null)
                return;

            using var httpClient = HttpClientProvider.CreateClient();
            using var response = await HttpClientProvider.Shared.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead, token);
            response.EnsureSuccessStatusCode();

            Directory.CreateDirectory(Path.GetDirectoryName(localPath)!);
            using var fileStream = new FileStream(localPath, FileMode.Create, FileAccess.Write, FileShare.None);
            using var contentStream = await response.Content.ReadAsStreamAsync(token);

            var buffer = new byte[8192];
            int bytesRead;
            var totalRead = 0L;

            while ((bytesRead = await contentStream.ReadAsync(buffer, 0, buffer.Length, token)) > 0)
            {
                token.ThrowIfCancellationRequested();
                await fileStream.WriteAsync(buffer, 0, bytesRead, token);
                totalRead += bytesRead;
                OnProgressChanged(totalRead, response.Content.Headers.ContentLength);
            }

            fileStream.Close();
        }
        catch (OperationCanceledException)
        {
            throw new OperationCanceledException("Task was cancelled");
        }
    }
}
