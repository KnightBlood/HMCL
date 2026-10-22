using System;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth.Microsoft;

/// <summary>
/// Microsoft account implementation using OAuth 2.0.
/// </summary>
public sealed class MicrosoftAccount : OAuthAccount
{
    /// <summary>
    /// The XBox Live token.
    /// </summary>
    public string? XBoxToken { get; private set; }

    /// <summary>
    /// The XSTS token.
    /// </summary>
    public string? XstsToken { get; private set; }

    /// <summary>
    /// The Minecraft access token.
    /// </summary>
    public string? MinecraftAccessToken { get; private set; }

    /// <summary>
    /// The Minecraft profile UUID.
    /// </summary>
    public string? MinecraftUuid { get; private set; }

    /// <summary>
    /// The Minecraft profile name.
    /// </summary>
    public string? MinecraftUsername { get; private set; }

    /// <summary>
    /// The account type.
    /// </summary>
    public override string AccountType => "microsoft";

    public MicrosoftAccount(string username) : base(username) { }

    /// <summary>
    /// Sets the Microsoft OAuth tokens.
    /// </summary>
    public void SetTokens(string accessToken, string? refreshToken, string? clientToken)
    {
        AccessToken = accessToken;
        RefreshToken = refreshToken;
        ClientToken = clientToken;
    }

    /// <summary>
    /// Sets the XBox Live and XSTS tokens.
    /// </summary>
    public void SetXboxTokens(string xboxToken, string xstsToken)
    {
        XBoxToken = xboxToken;
        XstsToken = xstsToken;
    }

    /// <summary>
    /// Sets the Minecraft session tokens.
    /// </summary>
    public void SetMinecraftSession(string accessToken, string uuid, string username)
    {
        MinecraftAccessToken = accessToken;
        MinecraftUuid = uuid;
        MinecraftUsername = username;
    }

    public override string GetDisplayName() => MinecraftUsername ?? Username;

    /// <summary>
    /// The stable identifier of the account, built from its Minecraft profile.
    /// </summary>
    public override string AccountId =>
        string.IsNullOrEmpty(MinecraftUuid)
            ? $"{AccountType}:{Username}"
            : $"{AccountType}:{MinecraftUuid}";
}

/// <summary>
/// Microsoft account factory.
/// </summary>
public sealed class MicrosoftAccountFactory : IAccountFactory
{
    public string AccountType => "microsoft";

    public Account Create(params object[] parameters)
    {
        if (parameters.Length == 0 || parameters[0] is not string username)
            throw new ArgumentException("Username is required", nameof(parameters));
        return new MicrosoftAccount(username);
    }
}

/// <summary>
/// Microsoft session information.
/// </summary>
public sealed class MicrosoftSession
{
    public string AccessToken { get; init; } = string.Empty;
    public string RefreshToken { get; init; } = string.Empty;
    public string Uuid { get; init; } = string.Empty;
    public string Username { get; init; } = string.Empty;
    public DateTimeOffset ExpiresAt { get; init; } = DateTimeOffset.UtcNow;
}

/// <summary>
/// Microsoft authentication service using OAuth 2.0.
/// </summary>
public sealed class MicrosoftService
{

    /// <summary>
    /// The Microsoft OAuth client ID for HMCL.
    /// </summary>
    public const string ClientId = "4c767b88-8f86-4c2a-9e8c-9e1a2e3b4f5c";

    /// <summary>
    /// The Microsoft OAuth redirect URI.
    /// </summary>
    public const string RedirectUri = "https://login.microsoftonline.com/common/oauth2/nativeclient";

    /// <summary>
    /// Returns the authorization URL for Microsoft login.
    /// </summary>
    public string GetAuthorizationUrl(string state)
    {
        return $"https://login.microsoftonline.com/common/oauth2/v2.0/authorize" +
               $"?client_id={ClientId}" +
               $"&response_type=code" +
               $"&redirect_uri={Uri.EscapeDataString(RedirectUri)}" +
               $"&scope=XboxLive.signin%20offline_access" +
               $"&state={state}";
    }

    /// <summary>
    /// Exchanges an authorization code for Microsoft tokens.
    /// </summary>
    public async Task<MicrosoftTokens?> ExchangeCodeAsync(string code)
    {
        try
        {
            var content = new FormUrlEncodedContent(new[]
            {
                new KeyValuePair<string, string>("client_id", ClientId),
                new KeyValuePair<string, string>("redirect_uri", RedirectUri),
                new KeyValuePair<string, string>("client_secret", ""),
                new KeyValuePair<string, string>("code", code),
                new KeyValuePair<string, string>("grant_type", "authorization_code")
            });

            using var response = await HttpClientProvider.Shared.PostAsync(
                "https://login.microsoftonline.com/common/oauth2/v2.0/token", content);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(json, LauncherJsonContext.Default.MicrosoftTokens);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Refreshes Microsoft access tokens using a refresh token.
    /// </summary>
    public async Task<MicrosoftTokens?> RefreshAsync(string refreshToken)
    {
        try
        {
            var content = new FormUrlEncodedContent(new[]
            {
                new KeyValuePair<string, string>("client_id", ClientId),
                new KeyValuePair<string, string>("redirect_uri", RedirectUri),
                new KeyValuePair<string, string>("client_secret", ""),
                new KeyValuePair<string, string>("refresh_token", refreshToken),
                new KeyValuePair<string, string>("grant_type", "refresh_token"),
                new KeyValuePair<string, string>("scope", "XboxLive.signin offline_access")
            });

            using var response = await HttpClientProvider.Shared.PostAsync(
                "https://login.microsoftonline.com/common/oauth2/v2.0/token", content);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(json, LauncherJsonContext.Default.MicrosoftTokens);
        }
        catch
        {
            return null;
        }
    }
}

/// <summary>
/// Microsoft OAuth token response.

/// <summary>
/// Microsoft account factory (alternative name for compatibility).
/// </summary>
public sealed class MicrosoftAccountFactory2 : IAccountFactory
{
    public string AccountType => "microsoft";

    public Account Create(params object[] parameters)
    {
        if (parameters.Length == 0 || parameters[0] is not string username)
            throw new ArgumentException("Username is required", nameof(parameters));
        return new MicrosoftAccount(username);
    }
}
