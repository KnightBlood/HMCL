using System.Net;
using HMCLCore.NET.Download;
using HMCLCore.NET.Setting;

namespace HMCL.NET.Util;

/// <summary>
/// Applies the configured proxy as the process-wide default web proxy and to
/// the HTTP clients of the launcher, so every request of the launcher is
/// routed through it. The http, https and socks protocols are supported; the
/// socks protocols are only applied to the clients of the launcher, because
/// the legacy web request stack of .NET does not support them.
/// </summary>
public static class ProxyApplier
{
    /// <summary>
    /// Applies or clears the default web proxy for the current process.
    /// </summary>
    /// <param name="config">The proxy configuration to apply, or null to clear.</param>
    public static void Apply(HMCLCore.NET.Setting.ProxyConfig? config)
    {
        if (config == null || !config.Enabled || string.IsNullOrWhiteSpace(config.Host))
        {
            WebRequest.DefaultWebProxy = null;
            HttpClientProvider.ClearProxy();
            return;
        }

        var type = (config.Type ?? "http").Trim().ToLowerInvariant();
        var supported = type is "http" or "https" or "socks4" or "socks4a" or "socks5";

        if (!supported)
        {
            WebRequest.DefaultWebProxy = null;
            HttpClientProvider.ClearProxy();
            return;
        }

        // The socks protocols are not supported by the legacy web request
        // stack, so the system default proxy is left untouched for them.
        if (type is "http" or "https")
        {
            var address = $"{type}://{config.Host}:{config.Port}";
            var proxy = new WebProxy(address) { UseDefaultCredentials = false };

            if (!string.IsNullOrEmpty(config.Username))
                proxy.Credentials = new NetworkCredential(config.Username, config.Password ?? string.Empty);

            WebRequest.DefaultWebProxy = proxy;
        }
        else
        {
            WebRequest.DefaultWebProxy = null;
        }

        HttpClientProvider.ApplyProxy(
            config.Type, config.Host, config.Port, config.Username, config.Password);
    }
}
