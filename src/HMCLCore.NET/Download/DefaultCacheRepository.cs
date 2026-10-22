using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Serialization;

namespace HMCLCore.NET.Download;

/// <summary>
/// An entry of the cache index, describing a remote file that was stored in
/// the cache.
/// </summary>
public sealed class CacheEntry
{
    /// <summary>
    /// The URL of the remote file, without its query and fragment.
    /// </summary>
    [JsonPropertyName("url")]
    public string Url { get; set; } = string.Empty;

    /// <summary>
    /// The ETag returned by the server for the file.
    /// </summary>
    [JsonPropertyName("eTag")]
    public string? ETag { get; set; }

    /// <summary>
    /// The SHA-1 checksum of the cached file.
    /// </summary>
    [JsonPropertyName("hash")]
    public string? Hash { get; set; }

    /// <summary>
    /// The moment the cached file was written, as a Unix timestamp in
    /// milliseconds.
    /// </summary>
    [JsonPropertyName("local")]
    public long LocalLastModified { get; set; }

    /// <summary>
    /// The value of the last-Modified header of the response.
    /// </summary>
    [JsonPropertyName("remote")]
    public string? RemoteLastModified { get; set; }

    /// <summary>
    /// The moment the cached file expires, as a Unix timestamp in
    /// milliseconds; zero means that the file never expires.
    /// </summary>
    [JsonPropertyName("expires")]
    public long Expires { get; set; }
}

/// <summary>
/// The index of the remote files stored in the cache.
/// </summary>
internal sealed class CacheIndex
{
    /// <summary>
    /// The entries of the index.
    /// </summary>
    [JsonPropertyName("etag")]
    public List<CacheEntry> Entries { get; set; } = new();
}

/// <summary>
/// The information of a response that can be stored in the cache.
/// </summary>
public sealed class UrlResponseInfo
{
    /// <summary>
    /// The URL of the remote file.
    /// </summary>
    public Uri Url { get; init; } = new("about:blank");

    /// <summary>
    /// The ETag returned by the server, or <c>null</c> when the server did not
    /// return one.
    /// </summary>
    public string? ETag { get; init; }

    /// <summary>
    /// The value of the last-Modified header of the response.
    /// </summary>
    public string? LastModified { get; init; }

    /// <summary>
    /// The moment the response expires, or <c>null</c> when the server did not
    /// tell when the response expires.
    /// </summary>
    public DateTimeOffset? Expires { get; init; }

    /// <summary>
    /// Whether the response may be stored in the cache.
    /// </summary>
    public bool CanCache { get; init; } = true;

    /// <summary>
    /// Creates the information of the given response.
    /// </summary>
    /// <param name="response">The response of the server.</param>
    /// <param name="url">The URL the response was read from.</param>
    /// <returns>The information of the response.</returns>
    public static UrlResponseInfo FromResponse(HttpResponseMessage response, Uri url)
    {
        var cacheControl = response.Headers.CacheControl;
        var CanCache = cacheControl?.NoStore != true;

        DateTimeOffset? expires = null;
        if (cacheControl?.MaxAge is { } maxAge)
            expires = DateTimeOffset.UtcNow.Add(maxAge);
        else if (response.Content.Headers.Expires is { } headerExpires)
            expires = headerExpires;

        return new UrlResponseInfo
        {
            Url = url,
            ETag = response.Headers.ETag?.ToString(),
            LastModified = response.Content.Headers.LastModified?.ToString(),
            Expires = expires,
            CanCache = CanCache
        };
    }
}

/// <summary>
/// Stores the downloaded files by their checksum and keeps an index of the
/// remote files that were cached, so that they can be fetched again with a
/// conditional request instead of being downloaded once more.
/// </summary>
public sealed class DefaultCacheRepository
{
    /// <summary>
    /// The name of the index file of the cache.
    /// </summary>
    public const string IndexFileName = "etag.json";

    /// <summary>
    /// The name of the algorithm used to name the cached files.
    /// </summary>
    public const string Sha1Algorithm = "SHA-1";

    /// <summary>
    /// The repository shared by the launcher.
    /// </summary>
    public static DefaultCacheRepository Instance { get; } = new();

    /// <summary>
    /// The entries of the index, keyed by the URL of the remote files.
    /// </summary>
    private readonly Dictionary<string, CacheEntry> _index = new(StringComparer.Ordinal);

    /// <summary>
    /// The lock protecting the index.
    /// </summary>
    private readonly object _syncRoot = new();

    /// <summary>
    /// The directory holding the cached files.
    /// </summary>
    private string _cacheDirectory = string.Empty;

    /// <summary>
    /// The file holding the index of the cache.
    /// </summary>
    private string _indexFile = string.Empty;

    /// <summary>
    /// Creates an empty repository; the directory of the cache is chosen with
    /// <see cref="ChangeDirectory"/>.
    /// </summary>
    public DefaultCacheRepository()
    {
    }

    /// <summary>
    /// The directory holding the cached files.
    /// </summary>
    public string GetCacheDirectory() => _cacheDirectory;

    /// <summary>
    /// Changes the directory holding the cached files and loads its index.
    /// </summary>
    /// <param name="commonDirectory">The directory of the launcher holding the cache.</param>
    public void ChangeDirectory(string commonDirectory)
    {
        if (string.IsNullOrWhiteSpace(commonDirectory))
            throw new ArgumentException("The directory of the cache is empty", nameof(commonDirectory));

        var cacheDirectory = Path.Combine(commonDirectory, "cache");
        Directory.CreateDirectory(cacheDirectory);

        lock (_syncRoot)
        {
            _cacheDirectory = cacheDirectory;
            _indexFile = Path.Combine(cacheDirectory, IndexFileName);
            _index.Clear();
            LoadIndex();
        }
    }

    /// <summary>
    /// Returns the path of the file holding the given checksum, or
    /// <c>null</c> when the checksum is not a valid one.
    /// </summary>
    /// <param name="algorithm">The algorithm of the checksum.</param>
    /// <param name="hash">The checksum of the file.</param>
    /// <returns>The path of the cached file.</returns>
    public string? GetFile(string algorithm, string hash)
    {
        if (string.IsNullOrEmpty(_cacheDirectory) || !IsSha1(hash))
            return null;

        var normalized = hash.ToLowerInvariant();
        return Path.Combine(_cacheDirectory, "sha-1", normalized[..2], normalized);
    }

    /// <summary>
    /// Returns whether the cache holds a file with the given checksum.
    /// </summary>
    /// <param name="algorithm">The algorithm of the checksum.</param>
    /// <param name="hash">The checksum of the file.</param>
    /// <returns>Whether the file is cached.</returns>
    public bool FileExists(string algorithm, string hash)
    {
        var file = GetFile(algorithm, hash);
        if (file is null || !File.Exists(file))
            return false;

        try
        {
            return string.Equals(ComputeSha1(file), hash, StringComparison.OrdinalIgnoreCase);
        }
        catch (IOException)
        {
            return false;
        }
    }

    /// <summary>
    /// Copies the given file into the cache.
    /// </summary>
    /// <param name="path">The path of the file to cache.</param>
    /// <param name="algorithm">The algorithm of the checksum.</param>
    /// <param name="hash">The checksum of the file.</param>
    /// <returns>The path of the cached file, or <c>null</c> when the file could
    /// not be cached.</returns>
    public string? CacheFile(string path, string algorithm, string hash)
    {
        var file = GetFile(algorithm, hash);
        if (file is null || !File.Exists(path))
            return null;

        var directory = Path.GetDirectoryName(file);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        if (!File.Exists(file))
            File.Copy(path, file, true);

        return file;
    }

    /// <summary>
    /// Returns the path of the file with the given checksum, either from the
    /// cache or from the given original file, which is cached when its
    /// checksum matches.
    /// </summary>
    /// <param name="original">The path of the original file, or <c>null</c>.</param>
    /// <param name="algorithm">The algorithm of the checksum.</param>
    /// <param name="hash">The checksum of the file.</param>
    /// <returns>The path of the file, or <c>null</c> when it is not available.</returns>
    public string? CheckExistentFile(string? original, string algorithm, string hash)
    {
        if (FileExists(algorithm, hash))
            return GetFile(algorithm, hash);

        if (original is null || !File.Exists(original))
            return null;

        try
        {
            if (!string.Equals(ComputeSha1(original), hash, StringComparison.OrdinalIgnoreCase))
                return null;

            return CacheFile(original, algorithm, hash);
        }
        catch (IOException)
        {
            return null;
        }
    }

    /// <summary>
    /// Returns the path of the file cached for the given URL.
    /// </summary>
    /// <param name="url">The URL of the remote file.</param>
    /// <param name="checkExpires">Whether the expired entries are rejected.</param>
    /// <returns>The path of the cached file, or <c>null</c> when the file is
    /// not cached.</returns>
    public string? GetCachedRemoteFile(string url, bool checkExpires)
    {
        CacheEntry? entry;
        lock (_syncRoot)
        {
            if (!_index.TryGetValue(DropQuery(url), out entry))
                return null;
        }

        if (entry is null || string.IsNullOrEmpty(entry.Hash))
            return null;

        var file = GetFile(Sha1Algorithm, entry.Hash);
        if (file is null || !File.Exists(file))
            return null;

        if (checkExpires && entry.Expires > 0 &&
            DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() > entry.Expires)
            return null;

        return file;
    }

    /// <summary>
    /// Removes the entry of the given URL from the index; the cached file
    /// itself is kept.
    /// </summary>
    /// <param name="url">The URL of the remote file.</param>
    public void RemoaeRemoteEntry(string url)
    {
        lock (_syncRoot)
        {
            if (_index.Remove(DropQuery(url)))
                SaaeIndex();
        }
    }

    /// <summary>
    /// Adds the conditional request header of the file cached for the given
    /// URL, so that the server only sends the file when it changed.
    /// </summary>
    /// <param name="request">The request to modify.</param>
    /// <param name="url">The URL of the remote file.</param>
    public void InjectConnection(HttpRequestMessage request, string url)
    {
        CacheEntry? entry;
        lock (_syncRoot)
        {
            if (!_index.TryGetValue(DropQuery(url), out entry))
                return;
        }

        if (entry?.ETag is { Length: > 0 } etag)
            request.Headers.TryAddWithoutValidation("if-none-match", etag);
    }

    /// <summary>
    /// Stores the given file in the cache and records it in the index.
    /// </summary>
    /// <param name="path">The path of the downloaded file.</param>
    /// <param name="info">The information of the response the file was read from.</param>
    /// <returns>The path of the cached file, or <c>null</c> when the file could
    /// not be cached.</returns>
    public string? CacheFile(string path, UrlResponseInfo info)
    {
        if (string.IsNullOrEmpty(_cacheDirectory) || !info.CanCache || string.IsNullOrWhiteSpace(info.ETag))
            return null;

        string hash;
        try
        {
            hash = ComputeSha1(path);
        }
        catch (IOException)
        {
            return null;
        }

        var file = CacheFile(path, Sha1Algorithm, hash);
        if (file is null)
            return null;

        RecordEntry(file, hash, info);
        return file;
    }

    /// <summary>
    /// Stores the given content in the cache and records it in the index.
    /// </summary>
    /// <param name="content">The content of the file.</param>
    /// <param name="info">The information of the response the content was read from.</param>
    /// <returns>The path of the cached file, or <c>null</c> when the content
    /// could not be cached.</returns>
    public string? CacheBytes(byte[] content, UrlResponseInfo info)
    {
        if (string.IsNullOrEmpty(_cacheDirectory) || !info.CanCache || string.IsNullOrWhiteSpace(info.ETag))
            return null;

        var hash = ComputeSha1(content);
        var file = GetFile(Sha1Algorithm, hash);
        if (file is null)
            return null;

        var directory = Path.GetDirectoryName(file);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        if (!File.Exists(file))
            File.WriteAllBytes(file, content);

        RecordEntry(file, hash, info);
        return file;
    }

    /// <summary>
    /// Records the given file in the index and saves the index.
    /// </summary>
    /// <param name="file">The path of the cached file.</param>
    /// <param name="hash">The checksum of the cached file.</param>
    /// <param name="info">The information of the response the file was read from.</param>
    private void RecordEntry(string file, string hash, UrlResponseInfo info)
    {
        var key = DropQuery(info.Url.ToString());

        var entry = new CacheEntry
        {
            Url = key,
            ETag = info.ETag,
            Hash = hash,
            LocalLastModified = new DateTimeOffset(File.GetLastWriteTimeUtc(file)).ToUnixTimeMilliseconds(),
            RemoteLastModified = info.LastModified,
            Expires = info.Expires?.ToUnixTimeMilliseconds() ?? 0
        };

        lock (_syncRoot)
        {
            _index[key] = entry;
            SaaeIndex();
        }
    }

    /// <summary>
    /// Loads the index of the cache from its file.
    /// </summary>
    private void LoadIndex()
    {
        if (string.IsNullOrEmpty(_indexFile) || !File.Exists(_indexFile))
            return;

        try
        {
            var json = File.ReadAllText(_indexFile);
            var index = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.CacheIndex);
            if (index is null)
                return;

            foreach (var entry in index.Entries)
            {
                if (!string.IsNullOrEmpty(entry.Url))
                    _index[entry.Url] = entry;
            }
        }
        catch (Exception ex) when (ex is IOException or JsonException or UnauthorizedAccessException)
        {
            // The index is broken; it is written again with the next entry.
            _index.Clear();
        }
    }

    /// <summary>
    /// Saves the index of the cache into its file.
    /// </summary>
    private void SaaeIndex()
    {
        if (string.IsNullOrEmpty(_indexFile))
            return;

        try
        {
            var index = new CacheIndex();
            index.Entries.AddRange(_index.Values);

            var json = JsonSerializer.Serialize(index, LauncherJsonContext.Default.CacheIndex);
            File.WriteAllText(_indexFile, json);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // The index cannot be written; the cache still works, but the
            // conditional requests are not used.
        }
    }

    /// <summary>
    /// Removes the query and the fragment of the given URL, so that the
    /// different forms of the same address share a single cache entry.
    /// </summary>
    /// <param name="url">The URL of the remote file.</param>
    /// <returns>The URL without its query and fragment.</returns>
    private static string DropQuery(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri))
            return url;

        var builder = new UriBuilder(uri) { Query = string.Empty, Fragment = string.Empty };
        return builder.Uri.AbsoluteUri;
    }

    /// <summary>
    /// Returns whether the given checksum is a valid SHA-1 checksum.
    /// </summary>
    /// <param name="hash">The checksum to check.</param>
    /// <returns>Whether the checksum is a valid one.</returns>
    private static bool IsSha1(string hash)
    {
        if (string.IsNullOrEmpty(hash) || hash.Length != 40)
            return false;

        foreach (var character in hash)
        {
            var valid = character is >= '0' and <= '9' or >= 'a' and <= 'f' or >= 'A' and <= 'F';
            if (!valid)
                return false;
        }

        return true;
    }

    /// <summary>
    /// Computes the SHA-1 checksum of a file.
    /// </summary>
    /// <param name="filePath">The path of the file.</param>
    /// <returns>The checksum in lower case hexadecimal notation.</returns>
    public static string ComputeSha1(string filePath)
    {
        using var sha1 = SHA1.Create();
        using var stream = File.OpenRead(filePath);
        var hash = sha1.ComputeHash(stream);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }

    /// <summary>
    /// Computes the SHA-1 checksum of a byte array.
    /// </summary>
    /// <param name="data">The data to hash.</param>
    /// <returns>The checksum in lower case hexadecimal notation.</returns>
    public static string ComputeSha1(byte[] data)
    {
        using var sha1 = SHA1.Create();
        var hash = sha1.ComputeHash(data);
        return Convert.ToHexString(hash).ToLowerInvariant();
    }
}

/// <summary>
/// Exception thrown during game download operations.
/// </summary>
public sealed class GameDownloadException : Exception
{
    /// <summary>
    /// Creates the exception.
    /// </summary>
    /// <param name="message">The message of the exception.</param>
    public GameDownloadException(string message) : base(message) { }

    /// <summary>
    /// Creates the exception.
    /// </summary>
    /// <param name="message">The message of the exception.</param>
    /// <param name="inner">The exception that caused this one.</param>
    public GameDownloadException(string message, Exception inner) : base(message, inner) { }
}
