using System;
using HMCLCore.NET.Events;
using System.Collections.Generic;
using System.IO;
using System.Net;
using System.Net.Http;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Represents a download task that can report progress.
/// </summary>
public abstract class DownloadTask : TaskBase
{
    /// <summary>
    /// The URL to download from.
    /// </summary>
    public Uri Url { get; init; }

    /// <summary>
    /// The destination file path.
    /// </summary>
    public string Destination { get; init; }

    /// <summary>
    /// Whether the addresses of the file are chosen with the source of the
    /// version lists instead of the source of the files of the game.
    /// </summary>
    public bool IsVersionList { get; init; }

    /// <summary>
    /// Returns the addresses of the file, ordered by the preference of the
    /// user; the first one is the preferred one and the other ones are used
    /// when it cannot be downloaded.
    /// </summary>
    /// <returns>The addresses of the file.</returns>
    public IReadOnlyList<Uri> GetUrls()
    {
        var candidates = IsVersionList
            ? MirrorSource.GetVersionListCandidates(Url.ToString())
            : MirrorSource.GetFileCandidates(Url.ToString());

        var urls = new List<Uri>(candidates.Count);
        foreach (var candidate in candidates)
        {
            if (Uri.TryCreate(candidate, UriKind.Absolute, out var uri))
                urls.Add(uri);
        }

        return urls.Count > 0 ? urls : new[] { Url };
    }

    /// <summary>
    /// The name shown for this task.
    /// </summary>
    public override string Name { get; set; } = "下载文件";

    /// <summary>
    /// The total size of the file in bytes, if known.
    /// </summary>
    public long? TotalBytes { get; protected set; }

    /// <summary>
    /// The number of bytes downloaded so far.
    /// </summary>
    public long DownloadedBytes { get; protected set; }

    /// <summary>
    /// Raised when download progress changes.
    /// </summary>
    public new event EventHandler<DownloadProgressEventArgs>? ProgressChanged;

    protected DownloadTask(Uri url, string destination)
    {
        Url = url;
        Destination = destination;
    }

    /// <summary>
    /// Raises the ProgressChanged event.
    /// </summary>
    protected new void OnProgressChanged(long downloadedBytes, long? totalBytes)
    {
        DownloadedBytes = downloadedBytes;
        TotalBytes = totalBytes;
        base.OnProgressChanged(downloadedBytes, totalBytes);
        ProgressChanged?.Invoke(this, new DownloadProgressEventArgs(downloadedBytes, totalBytes));
    }
}

/// <summary>
/// Provides downloads for the launcher.
/// </summary>
public class DownloadProvider
{
    /// <summary>
    /// The name of the download provider (shown to the user).
    /// </summary>
    public virtual string Name => "Default";

    /// <summary>
    /// The user agent sent with the requests of the provider.
    /// </summary>
    protected virtual string? UserAgent => null;

    /// <summary>
    /// Downloads a file from the specified URL to the destination.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <param name="destination">The path of the downloaded file.</param>
    /// <returns>Whether the file was downloaded.</returns>
    public virtual async Task<bool> DownloadAsync(Uri url, string destination)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, url);
            if (!string.IsNullOrEmpty(UserAgent))
                request.Headers.UserAgent.ParseAdd(UserAgent);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var directory = Path.GetDirectoryName(destination);
            if (!string.IsNullOrEmpty(directory))
                Directory.CreateDirectory(directory);

            using var fileStream = File.Create(destination);
            await using var contentStream = await response.Content.ReadAsStreamAsync();
            await contentStream.CopyToAsync(fileStream);

            return true;
        }
        catch
        {
            // The file could not be downloaded; the caller decides what to do.
            return false;
        }
    }

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public virtual bool CanHandle(Uri url) => true;
}

/// <summary>
/// Manages download providers.
/// </summary>
public sealed class DownloadProviderManager
{
    private readonly List<DownloadProvider> _providers = new();

    /// <summary>
    /// All registered download providers.
    /// </summary>
    public IReadOnlyList<DownloadProvider> Providers => _providers.AsReadOnly();

    /// <summary>
    /// Registers a download provider.
    /// </summary>
    public void RegisterProvider(DownloadProvider provider)
    {
        _providers.Add(provider);
    }

    /// <summary>
    /// Returns the first provider that can handle the specified URL.
    /// </summary>
    public DownloadProvider? FindProvider(Uri url)
    {
        foreach (var provider in _providers)
        {
            if (provider.CanHandle(url))
                return provider;
        }
        return null;
    }

    /// <summary>
    /// Downloads a file using the best matching provider.
    /// </summary>
    public async Task<bool> DownloadAsync(Uri url, string destination)
    {
        var provider = FindProvider(url);
        if (provider == null)
            throw new InvalidOperationException($"No download provider found for URL: {url}");
        return await provider.DownloadAsync(url, destination);
    }
}

/// <summary>
/// Represents a proxy configuration.
/// </summary>
public sealed class ProxyConfig
{
    /// <summary>
    /// Whether the proxy is enabled.
    /// </summary>
    public bool IsEnabled { get; init; }

    /// <summary>
    /// The proxy host.
    /// </summary>
    public string? Host { get; init; }

    /// <summary>
    /// The proxy port.
    /// </summary>
    public int Port { get; init; }

    /// <summary>
    /// The username for proxy authentication.
    /// </summary>
    public string? Username { get; init; }

    /// <summary>
    /// The password for proxy authentication.
    /// </summary>
    public string? Password { get; init; }

    /// <summary>
    /// Returns whether this proxy configuration is valid.
    /// </summary>
    public bool IsValid => IsEnabled && !string.IsNullOrEmpty(Host) && Port > 0 && Port < 65536;
}

/// <summary>
/// Manages proxy configuration for downloads.
/// </summary>
public sealed class ProxyManager
{
    private ProxyConfig _config = new();

    /// <summary>
    /// The current proxy configuration.
    /// </summary>
    public ProxyConfig Config
    {
        get => _config;
        set => _config = value ?? throw new ArgumentNullException(nameof(value));
    }

    /// <summary>
    /// Returns whether a proxy is currently configured and enabled.
    /// </summary>
    public bool IsProxyEnabled => Config.IsEnabled;

    /// <summary>
    /// Initializes the proxy manager with default settings.
    /// </summary>
    public void Init()
    {
        _config = new ProxyConfig { IsEnabled = false };
    }
}