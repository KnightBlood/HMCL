using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Game;

/// <summary>
/// A missing or corrupted file discovered during an integrity check.
/// </summary>
public sealed class IntegrityIssue
{
    /// <summary>
    /// The issue category (库 / 客户端 / 资源索引 / 资源).
    /// </summary>
    public string Kind { get; init; } = string.Empty;

    /// <summary>
    /// The local file path.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The download URL that can repair the file, when known.
    /// </summary>
    public string? Url { get; init; }

    /// <summary>
    /// The expected SHA1 hash, when known.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// Whether the file is missing (true) or present with a wrong hash (false).
    /// </summary>
    public bool Missing { get; init; }
}

/// <summary>
/// Result of checking an instance's files against their recorded hashes.
/// </summary>
public sealed class IntegrityReport
{
    /// <summary>
    /// The number of files inspected.
    /// </summary>
    public int CheckedFiles { get; set; }

    /// <summary>
    /// The number of missing files.
    /// </summary>
    public int MissingFiles { get; set; }

    /// <summary>
    /// The number of files whose SHA1 does not match.
    /// </summary>
    public int CorruptFiles { get; set; }

    /// <summary>
    /// All discovered issues.
    /// </summary>
    public List<IntegrityIssue> Issues { get; } = new();

    /// <summary>
    /// Whether every inspected file is present and valid.
    /// </summary>
    public bool IsHealthy => MissingFiles == 0 && CorruptFiles == 0;

    /// <summary>
    /// A human-readable summary of the report.
    /// </summary>
    public string Summary => IsHealthy
        ? $"校验完成：{CheckedFiles} 个文件全部正常"
        : $"校验完成：{CheckedFiles} 个文件，缺失 {MissingFiles}，损坏 {CorruptFiles}";
}

/// <summary>
/// Verifies and repairs an instance's libraries, client JAR, asset index and
/// asset objects using the hashes recorded in the version manifest.
/// </summary>
public static class GameIntegrityChecker
{

    /// <summary>
    /// Checks every applicable file referenced by the manifest.
    /// </summary>
    /// <param name="manifest">The instance manifest.</param>
    /// <param name="baseDirectory">The repository base directory.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The integrity report.</returns>
    public static async Task<IntegrityReport> VerifyAsync(
        GameInstanceManifest manifest, string baseDirectory, CancellationToken token = default)
    {
        var report = new IntegrityReport();
        var librariesDirectory = Path.Combine(baseDirectory, "libraries");

        // Libraries (including loader-provided artifacts).
        foreach (var library in manifest.Libraries)
        {
            if (!library.IsApplicableToCurrentOs())
                continue;

            var path = library.GetLocalPath(librariesDirectory);
            var url = DefaultDependencyManager.ResolveLibraryDownloadUrl(library);
            await CheckFileAsync(
                path, url, library.Downloads?.Artifact?.Sha1 ?? string.Empty,
                "库", report, token);
        }

        // Client JAR.
        var clientPath = Path.Combine(
            baseDirectory, "versions", manifest.Version, $"{manifest.Version}.jar");
        await CheckFileAsync(
            clientPath, manifest.Downloads?.Client?.Url,
            manifest.Downloads?.Client?.Sha1 ?? string.Empty,
            "客户端", report, token);

        // Asset index and objects.
        if (manifest.AssetIndex is { } assetIndex && !string.IsNullOrEmpty(assetIndex.Id))
        {
            var indexPath = Path.Combine(baseDirectory, "assets", "indexes", $"{assetIndex.Id}.json");
            await CheckFileAsync(indexPath, assetIndex.Url, assetIndex.Sha1, "资源索引", report, token);

            if (File.Exists(indexPath))
            {
                foreach (var hash in ParseAssetHashes(indexPath))
                {
                    var objectPath = Path.Combine(
                        baseDirectory, "assets", "objects", hash[..2], hash);
                    await CheckFileAsync(
                        objectPath,
                        $"https://resources.download.minecraft.net/{hash[..2]}/{hash}",
                        hash, "资源", report, token);
                }
            }
        }

        return report;
    }

    /// <summary>
    /// Re-downloads every issue that has a known URL.
    /// </summary>
    /// <param name="report">The report produced by <see cref="VerifyAsync"/>.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The number of repaired files.</returns>
    public static async Task<int> RepairAsync(
        IntegrityReport report, CancellationToken token = default)
    {
        var repaired = 0;

        foreach (var issue in report.Issues)
        {
            token.ThrowIfCancellationRequested();

            if (string.IsNullOrEmpty(issue.Url))
                continue;

            foreach (var url in MirrorSource.GetFileUris(issue.Url))
            {
                try
                {
                    using var response = await HttpClientProvider.Shared.GetAsync(
                        url, HttpCompletionOption.ResponseHeadersRead, token);
                    if (!response.IsSuccessStatusCode)
                        continue;

                    Directory.CreateDirectory(Path.GetDirectoryName(issue.Path)!);
                    await using var source = await response.Content.ReadAsStreamAsync(token);
                    await using var destination = File.Create(issue.Path);
                    await source.CopyToAsync(destination, token);
                    repaired++;
                    break;
                }
                catch
                {
                    // The address did not work; the next one is tried.
                }
            }
        }

        return repaired;
    }

    /// <summary>
    /// Checks a single file's presence and SHA1 hash, recording issues.
    /// </summary>
    private static async Task CheckFileAsync(
        string path,
        string? url,
        string expectedSha1,
        string kind,
        IntegrityReport report,
        CancellationToken token)
    {
        report.CheckedFiles++;

        if (!File.Exists(path))
        {
            report.MissingFiles++;
            report.Issues.Add(new IntegrityIssue
            {
                Kind = kind,
                Path = path,
                Url = url,
                Sha1 = expectedSha1,
                Missing = true
            });
            return;
        }

        if (string.IsNullOrEmpty(expectedSha1))
            return;

        try
        {
            await using var stream = File.OpenRead(path);
            var hash = await SHA1.HashDataAsync(stream, token);
            var actual = Convert.ToHexString(hash);
            if (!actual.Equals(expectedSha1, StringComparison.OrdinalIgnoreCase))
            {
                report.CorruptFiles++;
                report.Issues.Add(new IntegrityIssue
                {
                    Kind = kind,
                    Path = path,
                    Url = url,
                    Sha1 = expectedSha1,
                    Missing = false
                });
            }
        }
        catch (IOException)
        {
            // Unreadable files are reported as corrupt so they can be re-downloaded.
            report.CorruptFiles++;
            report.Issues.Add(new IntegrityIssue
            {
                Kind = kind,
                Path = path,
                Url = url,
                Sha1 = expectedSha1,
                Missing = false
            });
        }
    }

    /// <summary>
    /// Reads the content hashes referenced by an asset index.
    /// </summary>
    private static IEnumerable<string> ParseAssetHashes(string indexPath)
    {
        AssetObjectsDownloadTask.AssetIndexFile? index;
        try
        {
            index = JsonSerializer.Deserialize(File.ReadAllText(indexPath), LauncherJsonContext.Default.AssetIndexFile);
        }
        catch (JsonException)
        {
            yield break;
        }

        if (index?.Objects == null)
            yield break;

        foreach (var entry in index.Objects.Values)
        {
            if (!string.IsNullOrEmpty(entry.Hash) && entry.Hash.Length >= 2)
                yield return entry.Hash;
        }
    }

    /// <summary>
}
