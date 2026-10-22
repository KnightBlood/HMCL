using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Auth.AuthlibInjector;

/// <summary>
/// Manages the authlib-injector servers saved by the user.
/// </summary>
/// <remarks>
/// The manager works on the entries stored in the settings of the launcher, so
/// that the saved servers and their cached metadata survive a restart.
/// </remarks>
public sealed class AuthlibInjectorServerManager
{
    /// <summary>
    /// The entries of the saved servers.
    /// </summary>
    private readonly List<AuthlibInjectorServerEntry> _entries;

    /// <summary>
    /// Creates the manager.
    /// </summary>
    /// <param name="entries">The entries of the saved servers.</param>
    public AuthlibInjectorServerManager(List<AuthlibInjectorServerEntry> entries)
    {
        _entries = entries ?? throw new ArgumentNullException(nameof(entries));
    }

    /// <summary>
    /// The servers saved by the user.
    /// </summary>
    public IReadOnlyList<AuthlibInjectorServer> Servers =>
        _entries.Select(ToServer).ToList();

    /// <summary>
    /// Finds a saved server by its URL.
    /// </summary>
    /// <param name="url">The URL of the server.</param>
    /// <returns>The saved server, or <c>null</c> when it is not saved.</returns>
    public AuthlibInjectorServer? Find(string url)
    {
        var entry = _entries.FirstOrDefault(item => UrlEquals(item.Url, url));
        return entry is null ? null : ToServer(entry);
    }

    /// <summary>
    /// Saves a server, replacing the entry with the same URL.
    /// </summary>
    /// <param name="server">The server to save.</param>
    public void Add(AuthlibInjectorServer server)
    {
        if (server is null)
            throw new ArgumentNullException(nameof(server));

        var entry = new AuthlibInjectorServerEntry
        {
            Url = server.Url,
            Name = server.Name,
            MetadataResponse = server.MetadataResponse,
            MetadataTimestamp = server.MetadataTimestamp
        };

        var index = _entries.FindIndex(item => UrlEquals(item.Url, server.Url));
        if (index >= 0)
            _entries[index] = entry;
        else
            _entries.Add(entry);
    }

    /// <summary>
    /// Removes a saved server.
    /// </summary>
    /// <param name="url">The URL of the server.</param>
    /// <returns>Whether the server was saved and has been removed.</returns>
    public bool Remove(string url)
    {
        var index = _entries.FindIndex(item => UrlEquals(item.Url, url));
        if (index < 0)
            return false;

        _entries.RemoveAt(index);
        return true;
    }

    /// <summary>
    /// Updates the cached metadata of a saved server.
    /// </summary>
    /// <param name="server">The server whose metadata was refreshed.</param>
    public void UpdateMetadata(AuthlibInjectorServer server)
    {
        var entry = _entries.FirstOrDefault(item => UrlEquals(item.Url, server.Url));
        if (entry is null)
            return;

        entry.Name = server.Name;
        entry.MetadataResponse = server.MetadataResponse;
        entry.MetadataTimestamp = server.MetadataTimestamp;
    }

    /// <summary>
    /// Creates the server described by an entry.
    /// </summary>
    /// <param name="entry">The entry of the server.</param>
    /// <returns>The server, with its cached metadata when it is available.</returns>
    private static AuthlibInjectorServer ToServer(AuthlibInjectorServerEntry entry)
    {
        var server = new AuthlibInjectorServer(entry.Url);

        if (!string.IsNullOrEmpty(entry.MetadataResponse))
        {
            try
            {
                server.RestoreMetadataCache(entry.MetadataResponse, entry.MetadataTimestamp);
            }
            catch (Exception)
            {
                // The cached metadata is malformed; the server is shown without
                // a name until its metadata is fetched again.
            }
        }

        return server;
    }

    /// <summary>
    /// Compares two URLs of servers, ignoring their trailing slash.
    /// </summary>
    /// <param name="first">The first URL.</param>
    /// <param name="second">The second URL.</param>
    /// <returns>Whether the two URLs are equal.</returns>
    private static bool UrlEquals(string first, string second)
    {
        if (!first.EndsWith("/", StringComparison.Ordinal))
            first += "/";
        if (!second.EndsWith("/", StringComparison.Ordinal))
            second += "/";

        return string.Equals(first, second, StringComparison.OrdinalIgnoreCase);
    }
}
