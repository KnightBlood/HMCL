using System;

namespace HMCLCore.NET.Auth.AuthlibInjector;

/// <summary>
/// An authlib-injector server saved in the settings of the launcher.
/// </summary>
/// <remarks>
/// The entry keeps the cached metadata of the server, so that the launcher can
/// show the name and the links of the server without contacting it.
/// </remarks>
public sealed class AuthlibInjectorServerEntry
{
    /// <summary>
    /// The URL of the server.
    /// </summary>
    public string Url { get; set; } = string.Empty;

    /// <summary>
    /// The display name of the server, or <c>null</c> when it is unknown.
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// The cached metadata document of the server.
    /// </summary>
    public string? MetadataResponse { get; set; }

    /// <summary>
    /// The moment the cached metadata was fetched, as a Unix timestamp in
    /// milliseconds.
    /// </summary>
    public long MetadataTimestamp { get; set; }
}
