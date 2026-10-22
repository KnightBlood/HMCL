using System;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth;

/// <summary>
/// Microsoft account authentication service using OAuth 2.0.
/// </summary>
public sealed class MicrosoftAuthService
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
/// </summary>
public sealed class MicrosoftTokens
{
    public string AccessToken { get; init; } = string.Empty;
    public string RefreshToken { get; init; } = string.Empty;
    public string TokenType { get; init; } = "Bearer";
    public int ExpiresIn { get; init; }
    public string Scope { get; init; } = string.Empty;
    public DateTimeOffset ExpiresAt { get; init; } = DateTimeOffset.UtcNow;
}