using System;
using System.Collections.Generic;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Auth.AuthlibInjector;

/// <summary>
/// An authlib-injector (Yggdrasil) server known by the launcher.
/// </summary>
/// <remarks>
/// The server publishes a metadata document at its root, which holds its
/// display name, the links shown to the user and the features it supports.
/// The metadata is cached so that the launcher does not have to contact the
/// server every time it is shown.
/// </remarks>
public sealed class AuthlibInjectorServer
{
    /// <summary>
    /// The header used by the authlib-injector servers to point at the real
    /// location of their API.
    /// </summary>
    private const string ApiLocationHeader = "x-authlib-injector-api-location";

    /// <summary>
    /// Creates the server.
    /// </summary>
    /// <param name="url">The URL of the server, ending with a slash.</param>
    public AuthlibInjectorServer(string url)
    {
        Url = url ?? throw new ArgumentNullException(nameof(url));
    }

    /// <summary>
    /// The URL of the server.
    /// </summary>
    public string Url { get; }

    /// <summary>
    /// The display name of the server, or <c>null</c> when the server did not
    /// publish one.
    /// </summary>
    public string? Name { get; private set; }

    /// <summary>
    /// The links published by the server.
    /// </summary>
    public IReadOnlyDictionary<string, string> Links { get; private set; } = new Dictionary<string, string>();

    /// <summary>
    /// Whether the server accepts logins with names that are not email
    /// addresses.
    /// </summary>
    public bool NonEmailLogin { get; private set; }

    /// <summary>
    /// The raw metadata document of the server, or <c>null</c> when it was
    /// never fetched.
    /// </summary>
    public string? MetadataResponse { get; private set; }

    /// <summary>
    /// The moment the metadata was fetched, as a Unix timestamp in
    /// milliseconds.
    /// </summary>
    public long MetadataTimestamp { get; private set; }

    /// <summary>
    /// The name shown to the user; the URL of the server is used when the
    /// server has no name.
    /// </summary>
    public string DisplayName => Name ?? Url;

    /// <summary>
    /// The URL of the server without the scheme and the default port, used to
    /// show the address of the server to the user.
    /// </summary>
    public string DisplayHostUrl
    {
        get
        {
            if (!Uri.TryCreate(Url, UriKind.Absolute, out var uri))
                return Url;

            var builder = new System.Text.StringBuilder(uri.Host);

            if (!uri.IsDefaultPort)
                builder.Append(':').Append(uri.Port);

            if (!string.Equals(uri.AbsolutePath, "/api/yggdrasil/", StringComparison.Ordinal))
                builder.Append(uri.AbsolutePath);

            return builder.ToString();
        }
    }

    /// <summary>
    /// Locates the server at the given URL and fetches its metadata.
    /// </summary>
    /// <param name="url">The URL entered by the user.</param>
    /// <param name="token">The token cancelling the request.</param>
    /// <returns>The server with its metadata.</returns>
    public static async Task<AuthlibInjectorServer> LocateAsync(string url, CancellationToken token = default)
    {
        var address = NormalizeUrl(url);

        using var client = HttpClientProvider.CreateClient();
        using var response = await client.GetAsync(address, token);
        response.EnsureSuccessStatusCode();

        var location = response.Headers.TryGetValues(ApiLocationHeader, out var values)
            ? values.FirstOrDefault()
            : null;

        string body;

        if (!string.IsNullOrWhiteSpace(location))
        {
            var absolute = new Uri(new Uri(address), location).ToString();
            if (!UrlEqualsIgnoreSlash(address, absolute))
            {
                address = absolute;
                using var apiResponse = await client.GetAsync(address, token);
                apiResponse.EnsureSuccessStatusCode();
                body = await apiResponse.Content.ReadAsStringAsync(token);
            }
            else
            {
                body = await response.Content.ReadAsStringAsync(token);
            }
        }
        else
        {
            body = await response.Content.ReadAsStringAsync(token);
        }

        if (!address.EndsWith("/", StringComparison.Ordinal))
            address += "/";

        var server = new AuthlibInjectorServer(address);
        server.SetMetadataResponse(body, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
        return server;
    }

    /// <summary>
    /// Refreshes the metadata of the server.
    /// </summary>
    /// <param name="token">The token cancelling the request.</param>
    /// <returns>A task that completes when the metadata was fetched.</returns>
    public async Task RefreshMetadataAsync(CancellationToken token = default)
    {
        using var client = HttpClientProvider.CreateClient();
        using var response = await client.GetAsync(Url, token);
        response.EnsureSuccessStatusCode();

        var body = await response.Content.ReadAsStringAsync(token);
        SetMetadataResponse(body, DateTimeOffset.UtcNow.ToUnixTimeMilliseconds());
    }

    /// <summary>
    /// Restores a cached metadata document without marking it as freshly
    /// fetched.
    /// </summary>
    /// <param name="metadataResponse">The cached metadata document.</param>
    /// <param name="metadataTimestamp">The moment the metadata was fetched.</param>
    public void RestoreMetadataCache(string metadataResponse, long metadataTimestamp)
    {
        SetMetadataResponse(metadataResponse, metadataTimestamp);
    }

    /// <summary>
    /// Parses a metadata document and remembers it.
    /// </summary>
    /// <param name="metadataResponse">The metadata document.</param>
    /// <param name="metadataTimestamp">The moment the metadata was fetched.</param>
    private void SetMetadataResponse(string metadataResponse, long metadataTimestamp)
    {
        using var document = JsonDocument.Parse(metadataResponse);
        var root = document.RootElement;

        MetadataResponse = metadataResponse;
        MetadataTimestamp = metadataTimestamp;

        if (root.ValueKind != JsonValueKind.Object ||
            !root.TryGetProperty("meta", out var meta) ||
            meta.ValueKind != JsonValueKind.Object)
        {
            Name = null;
            Links = new Dictionary<string, string>();
            NonEmailLogin = false;
            return;
        }

        Name = meta.TryGetProperty("serverName", out var serverName)
            ? serverName.GetString()
            : null;

        var links = new Dictionary<string, string>();
        if (meta.TryGetProperty("links", out var linksElement) && linksElement.ValueKind == JsonValueKind.Object)
        {
            foreach (var link in linksElement.EnumerateObject())
            {
                if (link.Value.ValueKind == JsonValueKind.String)
                    links[link.Name] = link.Value.GetString() ?? string.Empty;
            }
        }

        Links = links;

        NonEmailLogin = meta.TryGetProperty("feature.non_email_login", out var nonEmailLogin) &&
                        nonEmailLogin.ValueKind == JsonValueKind.True;
    }

    /// <summary>
    /// Normalizes the URL entered by the user, adding the scheme and the
    /// trailing slash.
    /// </summary>
    /// <param name="url">The URL entered by the user.</param>
    /// <returns>The normalized URL.</returns>
    private static string NormalizeUrl(string url)
    {
        var trimmed = (url ?? string.Empty).Trim();
        if (trimmed.Length == 0)
            throw new ArgumentException("The URL of the server is empty", nameof(url));

        if (!trimmed.Contains("://", StringComparison.Ordinal))
            trimmed = "https://" + trimmed;

        if (!Uri.TryCreate(trimmed, UriKind.Absolute, out var uri))
            throw new ArgumentException($"The URL of the server is invalid: {url}", nameof(url));

        var address = uri.ToString();
        if (!address.EndsWith("/", StringComparison.Ordinal))
            address += "/";

        return address;
    }

    /// <summary>
    /// Compares two URLs, ignoring their trailing slash.
    /// </summary>
    /// <param name="first">The first URL.</param>
    /// <param name="second">The second URL.</param>
    /// <returns>Whether the two URLs are equal.</returns>
    private static bool UrlEqualsIgnoreSlash(string first, string second)
    {
        if (!first.EndsWith("/", StringComparison.Ordinal))
            first += "/";
        if (!second.EndsWith("/", StringComparison.Ordinal))
            second += "/";

        return string.Equals(first, second, StringComparison.OrdinalIgnoreCase);
    }

    /// <inheritdoc/>
    public override string ToString() => Name is null ? Url : $"{Url} ({Name})";
}
