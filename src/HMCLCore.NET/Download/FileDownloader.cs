using System;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Events;

namespace HMCLCore.NET.Download;

/// <summary>
/// Controls how a file is downloaded.
/// </summary>
public sealed class FileDownloadOptions
{
    /// <summary>
    /// The expected SHA-1 checksum of the file, or null when the file is not
    /// verified.
    /// </summary>
    public string? Sha1 { get; init; }

    /// <summary>
    /// The number of attempts made before the download is given up.
    /// </summary>
    public int MaxAttempts { get; init; } = 3;

    /// <summary>
    /// Whether an interrupted download is resumed instead of started again.
    /// </summary>
    public bool Resume { get; init; } = true;

    /// <summary>
    /// The delay between two attempts.
    /// </summary>
    public TimeSpan RetryDelay { get; init; } = TimeSpan.FromSeconds(2);
}

/// <summary>
/// The result of a single download attempt.
/// </summary>
public enum DownloadOutcome
{
    /// <summary>
    /// The file was downloaded.
    /// </summary>
    Downloaded,

    /// <summary>
    /// The remote file was not modified; the cached copy was used.
    /// </summary>
    NotModified,

    /// <summary>
    /// The download failed.
    /// </summary>
    Failed
}

/// <summary>
/// Downloads files, resuming interrupted downloads, retrying failed requests
/// and verifying the checksum of the downloaded files.
/// </summary>
public static class FileDownloader
{
    /// <summary>
    /// The suffix of the file holding the data of an unfinished download.
    /// </summary>
    public const string PartialSuffix = ".part";

    /// <summary>
    /// Downloads a file to the given destination.
    /// </summary>
    /// <param name="url">The URL of the file.</param>
    /// <param name="destination">The path of the downloaded file.</param>
    /// <param name="options">The options of the download; may be null.</param>
    /// <param name="progress">Receives the progress of the download.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>Whether the file was downloaded.</returns>
    public static async Task<bool> DownloadAsync(
        Uri url,
        string destination,
        FileDownloadOptions? options = null,
        IProgress<DownloadProgressEventArgs>? progress = null,
        CancellationToken token = default)
        => await DownloadAsync(new[] { url }, destination, options, progress, token);

    /// <summary>
    /// Downloads a file to the given destination, trying the given addresses
    /// in order until one of them works.
    /// </summary>
    /// <param name="urls">
    /// The addresses of the file; the first one is the preferred one, the
    /// other ones are the fallbacks used when it cannot be downloaded.
    /// </param>
    /// <param name="destination">The path of the downloaded file.</param>
    /// <param name="options">The options of the download; may be null.</param>
    /// <param name="progress">Receives the progress of the download.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>Whether the file was downloaded.</returns>
    public static async Task<bool> DownloadAsync(
        IReadOnlyList<Uri> urls,
        string destination,
        FileDownloadOptions? options = null,
        IProgress<DownloadProgressEventArgs>? progress = null,
        CancellationToken token = default)
    {
        options ??= new FileDownloadOptions();

        if (string.IsNullOrWhiteSpace(destination))
            return false;

        if (urls.Count == 0)
            return false;

        // A file that is already present and matches its checksum does not have
        // to be downloaded again.
        if (File.Exists(destination))
        {
            if (string.IsNullOrEmpty(options.Sha1))
                return true;

            if (string.Equals(ComputeSha1(destination), options.Sha1, StringComparison.OrdinalIgnoreCase))
                return true;
        }

        var directory = Path.GetDirectoryName(destination);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        var partialPath = destination + PartialSuffix;
        var attempts = Math.Max(1, options.MaxAttempts);

        for (var attempt = 1; attempt <= attempts; attempt++)
        {
            token.ThrowIfCancellationRequested();

            foreach (var url in urls)
            {
                try
                {
                    var outcome = await DownloadOnceAsync(url, partialPath, options, progress, token);
                    if (outcome == DownloadOutcome.NotModified)
                    {
                        // The remote file did not change since it was cached; the
                        // cached copy is used.
                        File.Move(partialPath, destination, true);
                        return true;
                    }

                    if (outcome == DownloadOutcome.Downloaded)
                    {
                        if (!string.IsNullOrEmpty(options.Sha1) &&
                            !string.Equals(ComputeSha1(partialPath), options.Sha1, StringComparison.OrdinalIgnoreCase))
                        {
                            // The downloaded file is broken; it is downloaded again
                            // with the next attempt.
                            TryDelete(partialPath);
                            continue;
                        }

                        File.Move(partialPath, destination, true);
                        return true;
                    }
                }
                catch (OperationCanceledException)
                {
                    throw;
                }
                catch (Exception)
                {
                    // The address did not work; the next one is tried.
                }
            }

            if (attempt < attempts)
                await Task.Delay(options.RetryDelay, token);
        }

        return false;
    }

    /// <summary>
    /// Downloads a file into the given partial file.
    /// </summary>
    /// <param name="url">The URL of the file.</param>
    /// <param name="partialPath">The path of the partial file.</param>
    /// <param name="options">The options of the download.</param>
    /// <param name="progress">Receives the progress of the download.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The result of the attempt.</returns>
    private static async Task<DownloadOutcome> DownloadOnceAsync(
        Uri url,
        string partialPath,
        FileDownloadOptions options,
        IProgress<DownloadProgressEventArgs>? progress,
        CancellationToken token)
    {
        var offset = 0L;
        if (options.Resume && File.Exists(partialPath))
            offset = new FileInfo(partialPath).Length;

        using var request = new HttpRequestMessage(HttpMethod.Get, url);
        if (offset > 0)
            request.Headers.Range = new System.Net.Http.Headers.RangeHeaderValue(offset, null);

        // The file cached for the URL is validated by the server, so that only
        // the changed files are downloaded again.
        DefaultCacheRepository.Instance.InjectConnection(request, url.ToString());

        using var response = await HttpClientProvider.Shared.SendAsync(
            request, HttpCompletionOption.ResponseHeadersRead, token);

        if (response.StatusCode == HttpStatusCode.NotModified)
        {
            // The remote file did not change; the cached copy is used.
            var cached = DefaultCacheRepository.Instance.GetCachedRemoteFile(url.ToString(), false);
            if (cached is null)
                return DownloadOutcome.Failed;

            TryDelete(partialPath);
            File.Copy(cached, partialPath, true);
            return DownloadOutcome.NotModified;
        }

        if (response.StatusCode == HttpStatusCode.RequestedRangeNotSatisfiable)
        {
            // The partial file is longer than the remote file; it is discarded.
            TryDelete(partialPath);
            return DownloadOutcome.Failed;
        }

        response.EnsureSuccessStatusCode();

        var append = offset > 0 && response.StatusCode == HttpStatusCode.PartialContent;
        if (!append)
            offset = 0;

        var totalBytes = response.Content.Headers.ContentLength.HasValue
            ? response.Content.Headers.ContentLength.Value + offset
            : (long?)null;

        await using (var source = await response.Content.ReadAsStreamAsync(token))
        await using (var destination = new FileStream(
            partialPath, append ? FileMode.Append : FileMode.Create, FileAccess.Write, FileShare.None))
        {
            var buffer = new byte[81920];
            var downloaded = offset;
            int read;

            while ((read = await source.ReadAsync(buffer, token)) > 0)
            {
                await destination.WriteAsync(buffer.AsMemory(0, read), token);
                downloaded += read;
                progress?.Report(new DownloadProgressEventArgs(downloaded, totalBytes));
            }
        }

        progress?.Report(new DownloadProgressEventArgs(
            new FileInfo(partialPath).Length, totalBytes ?? new FileInfo(partialPath).Length));

        // The downloaded file is stored in the cache, so that the next request
        // for it can be validated by the server instead of downloading it
        // again.
        var info = UrlResponseInfo.FromResponse(response, url);
        if (info.CanCache && !string.IsNullOrWhiteSpace(info.ETag))
            DefaultCacheRepository.Instance.CacheFile(partialPath, info);

        return DownloadOutcome.Downloaded;
    }

    /// <summary>
    /// Computes the SHA-1 checksum of a file.
    /// </summary>
    /// <param name="path">The path of the file.</param>
    /// <returns>The checksum in lower case hexadecimal notation.</returns>
    public static string ComputeSha1(string path)
    {
        using var stream = File.OpenRead(path);
        var hash = SHA1.HashData(stream);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    /// <summary>
    /// Deletes a file, ignoring the errors of the deletion.
    /// </summary>
    /// <param name="path">The path of the file to delete.</param>
    private static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path))
                File.Delete(path);
        }
        catch (IOException)
        {
            // The file is locked; it is deleted with the next attempt.
        }
        catch (UnauthorizedAccessException)
        {
            // The file cannot be deleted; it is overwritten with the next attempt.
        }
    }
}
