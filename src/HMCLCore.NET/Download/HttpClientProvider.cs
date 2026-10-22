using System;
using System.Net;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Creates the HTTP clients used by the launcher and applies the proxy
/// configured by the user to them.
/// </summary>
/// <remarks>
/// The proxy of the settings is applied to the process wide default proxy of
/// <see cref="HttpClient"/>; the clients created by
/// <see cref="CreateClient"/> use it explicitly, so that the downloads of the
/// launcher are routed through the proxy as well.
/// </remarks>
public static class HttpClientProvider
{
    private static IWebProxy? _proxy;
    private static HttpClient? _shared;

    /// <summary>
    /// The shared HTTP client used by the downloads of the launcher.
    /// </summary>
    public static HttpClient Shared => _shared ??= CreateClient();

    /// <summary>
    /// Applies the given proxy to the clients of the launcher.
    /// </summary>
    /// <param name="type">The type of the proxy, for example <c>http</c>.</param>
    /// <param name="host">The host of the proxy.</param>
    /// <param name="port">The port of the proxy.</param>
    /// <param name="username">The user name used to authenticate, or null.</param>
    /// <param name="password">The password used to authenticate, or null.</param>
    public static void ApplyProxy(string? type, string? host, int port, string? username, string? password)
    {
        if (string.IsNullOrWhiteSpace(host) || port <= 0 || port > 65535)
        {
            ClearProxy();
            return;
        }

        var scheme = string.IsNullOrWhiteSpace(type) ? "http" : type.Trim().ToLowerInvariant();
        if (scheme is not ("http" or "https" or "socks4" or "socks4a" or "socks5"))
        {
            ClearProxy();
            return;
        }

        var address = $"{scheme}://{host}:{port}";
        var proxy = new WebProxy(address) { UseDefaultCredentials = false };

        if (!string.IsNullOrEmpty(username))
            proxy.Credentials = new NetworkCredential(username, password ?? string.Empty);

        _proxy = proxy;
        ResetSharedClient();
    }

    /// <summary>
    /// Removes the proxy from the clients of the launcher.
    /// </summary>
    public static void ClearProxy()
    {
        _proxy = null;
        ResetSharedClient();
    }

    /// <summary>
    /// Creates an HTTP client that uses the configured proxy.
    /// </summary>
    /// <param name="timeout">The timeout of the requests, or null for the
    /// default timeout of 100 seconds.</param>
    /// <returns>The created HTTP client.</returns>
    public static HttpClient CreateClient(TimeSpan? timeout = null)
    {
        HttpMessageHandler handler;

        if (_proxy is not null && IsSocksProxy(_proxy))
        {
            // The SOCKS protocols are only supported by the socket based
            // handler of .NET.
            handler = new SocketsHttpHandler
            {
                UseProxy = true,
                Proxy = _proxy
            };
        }
        else
        {
            handler = new HttpClientHandler
            {
                UseProxy = _proxy != null,
                Proxy = _proxy
            };
        }

        var client = new HttpClient(handler, disposeHandler: true);
        client.Timeout = timeout ?? TimeSpan.FromSeconds(100);
        client.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");
        return client;
    }

    /// <summary>
    /// Returns whether the given proxy uses one of the SOCKS protocols.
    /// </summary>
    /// <param name="proxy">The proxy to check.</param>
    /// <returns>Whether the proxy is a SOCKS proxy.</returns>
    private static bool IsSocksProxy(IWebProxy proxy)
    {
        var address = proxy.GetProxy(new Uri("https://example.com"));
        return address is not null &&
               address.Scheme is "socks4" or "socks4a" or "socks5";
    }

    /// <summary>
    /// Discards the shared client, so that the next request uses the current
    /// proxy configuration.
    /// </summary>
    private static void ResetSharedClient()
    {
        var client = _shared;
        _shared = null;
        client?.Dispose();
    }
}
