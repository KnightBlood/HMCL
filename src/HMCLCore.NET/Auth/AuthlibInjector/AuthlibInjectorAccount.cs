using System;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;

namespace HMCLCore.NET.Auth.AuthlibInjector;

/// <summary>
/// Represents an AuthlibInjector account for third-party authentication.
/// </summary>
public sealed class AuthlibInjectorAccount
{
    /// <summary>
    /// The username.
    /// </summary>
    public string Username { get; init; } = string.Empty;

    /// <summary>
    /// The password (may be empty for offline mode).
    /// </summary>
    public string Password { get; init; } = string.Empty;

    /// <summary>
    /// The AuthlibInjector server URL.
    /// </summary>
    public string ServerUrl { get; init; } = string.Empty;

    /// <summary>
    /// The client token.
    /// </summary>
    public string ClientToken { get; init; } = Guid.NewGuid().ToString("N");

    /// <summary>
    /// Returns the account type identifier.
    /// </summary>
    public string AccountType => "authlib-injector";

    /// <summary>
    /// Returns whether this account is offline mode.
    /// </summary>
    public bool IsOffline => string.IsNullOrEmpty(ServerUrl);
}

/// <summary>
/// Factory for creating AuthlibInjector accounts.
/// </summary>
public sealed class AuthlibInjectorAccountFactory
{
    /// <summary>
    /// Creates a new AuthlibInjector account.
    /// </summary>
    public AuthlibInjectorAccount Create(string username, string password, string serverUrl)
    {
        return new AuthlibInjectorAccount
        {
            Username = username,
            Password = password,
            ServerUrl = serverUrl
        };
    }

    /// <summary>
    /// Creates an offline AuthlibInjector account.
    /// </summary>
    public AuthlibInjectorAccount CreateOffline(string username)
    {
        return new AuthlibInjectorAccount
        {
            Username = username,
            Password = string.Empty,
            ServerUrl = string.Empty
        };
    }
}

/// <summary>
/// Provides AuthlibInjector artifact information and download.
/// </summary>
public sealed class AuthlibInjectorArtifactInfo
{
    /// <summary>
    /// The artifact URL.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The SHA1 hash of the artifact.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// The file size in bytes.
    /// </summary>
    public long Size { get; init; }
}

/// <summary>
/// Provides AuthlibInjector artifacts from remote sources.
/// </summary>
public abstract class AuthlibInjectorArtifactProvider
{
    /// <summary>
    /// Returns whether this provider can supply artifacts for the given server URL.
    /// </summary>
    public abstract bool CanHandle(string serverUrl);

    /// <summary>
    /// Returns artifact information for the given server URL.
    /// </summary>
    public abstract Task<AuthlibInjectorArtifactInfo?> GetArtifactInfoAsync(string serverUrl);
}

/// <summary>
/// Simple AuthlibInjector artifact provider with hardcoded artifact info.
/// </summary>
public sealed class SimpleAuthlibInjectorArtifactProvider : AuthlibInjectorArtifactProvider
{
    /// <summary>
    /// The default AuthlibInjector artifact URL.
    /// </summary>
    public const string DefaultUrl = "https://github.com/yushijinhun/authlib-injector/releases/latest/download/authlib-injector.jar";

    /// <summary>
    /// Returns whether this provider can handle the given server URL.
    /// </summary>
    public override bool CanHandle(string serverUrl) => true;

    /// <summary>
    /// Returns artifact information for the given server URL.
    /// </summary>
    public override Task<AuthlibInjectorArtifactInfo?> GetArtifactInfoAsync(string serverUrl)
    {
        return Task.FromResult<AuthlibInjectorArtifactInfo?>(new AuthlibInjectorArtifactInfo
        {
            Url = DefaultUrl,
            Sha1 = string.Empty,
            Size = 0
        });
    }
}

/// <summary>
/// Exception thrown during AuthlibInjector download or extraction.
/// </summary>
public sealed class AuthlibInjectorDownloadException : Exception
{
    public AuthlibInjectorDownloadException(string message) : base(message) { }
    public AuthlibInjectorDownloadException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>
/// Extracts AuthlibInjector JAR to the game directory.
/// </summary>
public sealed class AuthlibInjectorExtractor
{
    private readonly string _gameDirectory;

    public AuthlibInjectorExtractor(string gameDirectory)
    {
        _gameDirectory = gameDirectory;
    }

    /// <summary>
    /// Extracts the AuthlibInjector JAR to the game directory.
    /// </summary>
    public void Extract(string sourceJarPath)
    {
        if (!File.Exists(sourceJarPath))
            throw new FileNotFoundException("AuthlibInjector JAR not found", sourceJarPath);

        // Copy authlib-injector.jar to game directory/libraries or appropriate location
        var destDir = Path.Combine(_gameDirectory, "libraries");
        Directory.CreateDirectory(destDir);
        var destPath = Path.Combine(destDir, "authlib-injector.jar");
        File.Copy(sourceJarPath, destPath, overwrite: true);
    }
}

/// <summary>
/// Provides AuthlibInjector server configuration.
/// </summary>
public sealed class AuthlibInjectorProvider
{
    /// <summary>
    /// The default server URL.
    /// </summary>
    public string DefaultServerUrl { get; init; } = "https://authlib.yushi.moe/";

    /// <summary>
    /// Returns whether the given server URL is valid.
    /// </summary>
    public bool IsValid(string serverUrl)
    {
        if (string.IsNullOrEmpty(serverUrl))
            return false;

        try
        {
            var uri = new Uri(serverUrl);
            return uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps;
        }
        catch
        {
            return false;
        }
    }
}

/// <summary>
/// Account factory bound to a specific AuthlibInjector server.
/// </summary>
public sealed class BoundAuthlibInjectorAccountFactory
{
    private readonly string _serverUrl;

    public BoundAuthlibInjectorAccountFactory(string serverUrl)
    {
        _serverUrl = serverUrl;
    }

    /// <summary>
    /// Creates an account bound to this server.
    /// </summary>
    public AuthlibInjectorAccount Create(string username, string password)
    {
        return new AuthlibInjectorAccount
        {
            Username = username,
            Password = password,
            ServerUrl = _serverUrl
        };
    }
}

/// <summary>
/// Drag-and-drop support for AuthlibInjector JAR files.
/// </summary>
public static class AuthlibInjectorDnD
{
    /// <summary>
    /// Returns whether the given file is a valid AuthlibInjector JAR.
    /// </summary>
    public static bool IsAuthlibInjectorJar(string filePath)
    {
        return Path.GetExtension(filePath).Equals(".jar", StringComparison.OrdinalIgnoreCase);
    }
}
