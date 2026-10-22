using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Events;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Parses a downloaded asset index and downloads every referenced asset object
/// into the local assets directory using the content-addressed object layout
/// (objects/{hash prefix}/{hash}).
/// </summary>
public sealed class AssetObjectsDownloadTask : TaskBase
{

    private readonly string _assetIndexPath;
    private readonly string _assetsDirectory;

    /// <summary>
    /// Creates a new AssetObjectsDownloadTask.
    /// </summary>
    /// <param name="assetIndexPath">Local path of the asset index JSON to parse.</param>
    /// <param name="assetsDirectory">Root of the assets directory containing indexes/ and objects/.</param>
    public AssetObjectsDownloadTask(string assetIndexPath, string assetsDirectory)
    {
        _assetIndexPath = assetIndexPath ?? throw new ArgumentNullException(nameof(assetIndexPath));
        _assetsDirectory = assetsDirectory ?? throw new ArgumentNullException(nameof(assetsDirectory));
        Name = "下载资源文件";
    }

    /// <summary>
    /// Parses the asset index, then downloads all missing objects sequentially,
    /// reporting progress as completed object count against the total count.
    /// </summary>
    public override async Task Run(CancellationToken token = default)
    {
        if (!File.Exists(_assetIndexPath))
            throw new FileNotFoundException("Asset index not found; download it first", _assetIndexPath);

        var index = JsonSerializer.Deserialize(File.ReadAllText(_assetIndexPath), LauncherJsonContext.Default.AssetIndexFile);
        if (index?.Objects == null)
            return;

        // Collect the object/hash pairs that still need downloading.
        var missing = new List<(string Hash, string Path)>();
        foreach (var (key, entry) in index.Objects)
        {
            token.ThrowIfCancellationRequested();

            var hash = entry.Hash;
            if (string.IsNullOrEmpty(hash) || hash.Length < 2)
                continue;

            var objectPath = Path.Combine(_assetsDirectory, "objects", hash.Substring(0, 2), hash);
            if (File.Exists(objectPath))
                continue;

            missing.Add((hash, objectPath));
        }

        var completed = 0L;
        foreach (var (hash, objectPath) in missing)
        {
            token.ThrowIfCancellationRequested();

            // Content-addressed resource URL, rewritten for the active mirror source.
            var url = $"https://resources.download.minecraft.net/{hash.Substring(0, 2)}/{hash}";

            Directory.CreateDirectory(Path.GetDirectoryName(objectPath)!);

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(completed, missing.Count));

            var succeeded = await FileDownloader.DownloadAsync(
                MirrorSource.GetFileUris(url),
                objectPath,
                new FileDownloadOptions { Sha1 = hash },
                progress,
                token);

            if (!succeeded)
                throw new IOException($"下载失败: {url}");

            completed++;
            OnProgressChanged(completed, missing.Count);
            Name = $"下载资源文件 {completed}/{missing.Count}";
        }
    }

    /// <summary>
    /// Asset index JSON structure (objects map asset keys to their content hashes).
    /// </summary>
    internal sealed class AssetIndexFile
    {
        /// <summary>
        /// Maps the virtual asset path onto its content entry.
        /// </summary>
        public Dictionary<string, AssetEntry>? Objects { get; init; }
    }

    /// <summary>
    /// Single asset object entry in the index.
    /// </summary>
    internal sealed class AssetEntry
    {
        /// <summary>
        /// The SHA1 hash of the object content.
        /// </summary>
        public string Hash { get; init; } = string.Empty;

        /// <summary>
        /// The object size in bytes.
        /// </summary>
        public long Size { get; init; }
    }
}
