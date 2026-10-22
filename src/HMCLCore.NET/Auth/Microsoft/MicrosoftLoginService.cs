using System;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth.Microsoft;

/// <summary>
/// Performs the full Microsoft login chain:
/// OAuth tokens → Xbox Live authentication → XSTS authorization →
/// Minecraft access token → Minecraft profile.
/// </summary>
public sealed class MicrosoftLoginService
{

    private readonly MicrosoftService _oauthService;

    /// <summary>
    /// Creates a new MicrosoftLoginService.
    /// </summary>
    public MicrosoftLoginService()
    {
        _oauthService = new MicrosoftService();
    }

    /// <summary>
    /// Returns the Microsoft authorization URL that the user must open in a browser.
    /// </summary>
    /// <param name="state">OAuth state parameter for CSRF verification.</param>
    public string GetAuthorizationUrl(string state) => _oauthService.GetAuthorizationUrl(state);

    /// <summary>
    /// Runs the complete login chain from an authorization code to a Minecraft session.
    /// </summary>
    /// <param name="code">The OAuth authorization code obtained from the redirect.</param>
    /// <returns>The login session, or null when any step of the chain fails.</returns>
    public async Task<MicrosoftLoginSession?> LoginWithCodeAsync(string code)
    {
        var tokens = await _oauthService.ExchangeCodeAsync(code);
        if (tokens == null || string.IsNullOrEmpty(tokens.AccessToken))
            return null;

        var session = await LoginWithTokensAsync(tokens.AccessToken, tokens.RefreshToken);
        return session;
    }

    /// <summary>
    /// Runs the login chain from an existing (possibly refreshed) OAuth token pair.
    /// </summary>
    /// <param name="msAccessToken">The Microsoft OAuth access token.</param>
    /// <param name="msRefreshToken">The Microsoft OAuth refresh token, may be empty.</param>
    /// <returns>The login session, or null when any step of the chain fails.</returns>
    public async Task<MicrosoftLoginSession?> LoginWithTokensAsync(string msAccessToken, string msRefreshToken)
    {
        // Step 1: exchange the Microsoft token for an Xbox Live user token.
        var xbl = await AuthenticateXblAsync(msAccessToken);
        if (xbl?.Token == null || xbl.GetUserHash() == null)
            return null;

        // Step 2: authorize the user token against XSTS for the Minecraft relying party.
        var xsts = await AuthorizeXstsAsync(xbl.Token);
        if (xsts?.Token == null || xsts.GetUserHash() == null)
            return null;

        // Step 3: trade the XBL3.0 identity token for a Minecraft access token.
        var mcResponse = await LoginMinecraftAsync(xsts.GetUserHash()!, xsts.Token);
        if (mcResponse?.AccessToken == null)
            return null;

        // Step 4: fetch the player's Minecraft profile (UUID + name).
        var profile = await GetProfileAsync(mcResponse.AccessToken);
        if (profile == null || string.IsNullOrEmpty(profile.Id) || string.IsNullOrEmpty(profile.Name))
            return null;

        return new MicrosoftLoginSession
        {
            MicrosoftAccessToken = msAccessToken,
            MicrosoftRefreshToken = msRefreshToken,
            XblToken = xbl.Token,
            XstsToken = xsts.Token,
            UserHash = xsts.GetUserHash()!,
            MinecraftAccessToken = mcResponse.AccessToken,
            ProfileUuid = profile.Id,
            ProfileName = profile.Name
        };
    }

    /// <summary>
    /// Authenticates the Microsoft access token against the Xbox Live user service.
    /// </summary>
    private static async Task<XboxTokenResponse?> AuthenticateXblAsync(string msAccessToken)
    {
        try
        {
            var request = new XboxTokenRequest
            {
                Properties = new XboxTokenProperties
                {
                    AuthMethod = "RSI",
                    SiteName = "user.auth.xboxlive.com",
                    RpsTicket = $"d={msAccessToken}"
                },
                RelyingParty = "http://auth.xboxlive.com",
                TokenType = "JWT"
            };

            return await PostXboxTokenRequestAsync("https://user.auth.xboxlive.com/user/authenticate", request);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Authorizes the Xbox Live user token against the XSTS service for Minecraft.
    /// </summary>
    private static async Task<XboxTokenResponse?> AuthorizeXstsAsync(string XblToken)
    {
        try
        {
            var request = new XboxTokenRequest
            {
                Properties = new XboxTokenProperties
                {
                    SandboxId = "RETAIL",
                    UserTokens = new[] { XblToken }
                },
                RelyingParty = "rp://api.minecraftservices.com/",
                TokenType = "JWT"
            };

            return await PostXboxTokenRequestAsync("https://xsts.auth.xboxlive.com/xsts/authorize", request);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Serializes and posts one Xbox token request.
    /// </summary>
    private static async Task<XboxTokenResponse?> PostXboxTokenRequestAsync(string endpoint, XboxTokenRequest request)
    {
        var json = JsonSerializer.Serialize(request, LauncherJsonContext.Default.XboxTokenRequest);
        using var content = new StringContent(json, Encoding.UTF8, "application/json");
        using var response = await HttpClientProvider.Shared.PostAsync(endpoint, content);
        response.EnsureSuccessStatusCode();

        var responseJson = await response.Content.ReadAsStringAsync();
        return JsonSerializer.Deserialize(responseJson, LauncherJsonContext.Default.XboxTokenResponse);
    }

    /// <summary>
    /// Exchanges the XBL3.0 identity token for a Minecraft access token.
    /// </summary>
    private static async Task<MinecraftLoginResponse?> LoginMinecraftAsync(string userHash, string xstsToken)
    {
        try
        {
            var payload = new MinecraftIdentityRequest
            {
                IdentityToken = $"XBL3.0 x={userHash};{xstsToken}"
            };

            var json = JsonSerializer.Serialize(payload, LauncherJsonContext.Default.MinecraftIdentityRequest);
            using var content = new StringContent(json, Encoding.UTF8, "application/json");
            using var response = await HttpClientProvider.Shared.PostAsync(
                "https://api.minecraftservices.com/authentication/login_with_xbox", content);
            response.EnsureSuccessStatusCode();

            var responseJson = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(responseJson, LauncherJsonContext.Default.MinecraftLoginResponse);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Fetches the Minecraft profile attached to the access token.
    /// </summary>
    /// <param name="MinecraftAccessToken">The Minecraft access token.</param>
    public static async Task<MinecraftProfile?> GetProfileAsync(string MinecraftAccessToken)
    {
        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get,
                "https://api.minecraftservices.com/minecraft/profile");
            request.Headers.Authorization = new System.Net.Http.Headers.AuthenticationHeaderValue("Bearer", MinecraftAccessToken);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(json, LauncherJsonContext.Default.MinecraftProfile);
        }
        catch
        {
            return null;
        }
    }
}

/// <summary>
/// The outcome of a successful Microsoft login chain.
/// </summary>
public sealed class MicrosoftLoginSession
{
    /// <summary>
    /// The Microsoft OAuth access token.
    /// </summary>
    public string MicrosoftAccessToken { get; init; } = string.Empty;

    /// <summary>
    /// The Microsoft OAuth refresh token.
    /// </summary>
    public string MicrosoftRefreshToken { get; init; } = string.Empty;

    /// <summary>
    /// The Xbox Live user token.
    /// </summary>
    public string XblToken { get; init; } = string.Empty;

    /// <summary>
    /// The XSTS authorization token.
    /// </summary>
    public string XstsToken { get; init; } = string.Empty;

    /// <summary>
    /// The Xbox user hash receiaed with the XSTS token.
    /// </summary>
    public string UserHash { get; init; } = string.Empty;

    /// <summary>
    /// The Minecraft access token used for game launches.
    /// </summary>
    public string MinecraftAccessToken { get; init; } = string.Empty;

    /// <summary>
    /// The Minecraft profile UUID.
    /// </summary>
    public string ProfileUuid { get; init; } = string.Empty;

    /// <summary>
    /// The Minecraft profile name.
    /// </summary>
    public string ProfileName { get; init; } = string.Empty;
}

/// <summary>
/// Xbox Live token request body.
/// </summary>
public sealed class XboxTokenRequest
{
    /// <summary>
    /// The request properties.
    /// </summary>
    [JsonPropertyName("Properties")]
    public XboxTokenProperties Properties { get; init; } = new();

    /// <summary>
    /// The relying party consuming the token.
    /// </summary>
    [JsonPropertyName("RelyingParty")]
    public string RelyingParty { get; init; } = string.Empty;

    /// <summary>
    /// The requested token type (always sWT).
    /// </summary>
    [JsonPropertyName("TokenType")]
    public string TokenType { get; init; } = "JWT";
}

/// <summary>
/// Xbox Live token request properties.
/// </summary>
public sealed class XboxTokenProperties
{
    /// <summary>
    /// The auth method (RSI for user tokens; null for XSTS requests).
    /// </summary>
    [JsonPropertyName("AuthMethod")]
    public string? AuthMethod { get; init; }

    /// <summary>
    /// The site name (null for XSTS requests).
    /// </summary>
    [JsonPropertyName("SiteName")]
    public string? SiteName { get; init; }

    /// <summary>
    /// The relying party source ticket (null for XSTS requests).
    /// </summary>
    [JsonPropertyName("RpsTicket")]
    public string? RpsTicket { get; init; }

    /// <summary>
    /// The sandbox id (XSTS requests only).
    /// </summary>
    [JsonPropertyName("SandboxId")]
    public string? SandboxId { get; init; }

    /// <summary>
    /// The user tokens (XSTS requests only).
    /// </summary>
    [JsonPropertyName("UserTokens")]
    public string[]? UserTokens { get; init; }
}

/// <summary>
/// Xbox Live token response body.
/// </summary>
public sealed class XboxTokenResponse
{
    /// <summary>
    /// The issued token.
    /// </summary>
    [JsonPropertyName("Token")]
    public string? Token { get; init; }

    /// <summary>
    /// The display claims including the per-user user hash.
    /// </summary>
    [JsonPropertyName("DisplayClaims")]
    public XboxDisplayClaims? DisplayClaims { get; init; }

    /// <summary>
    /// Returns the user hash of the first claim.
    /// </summary>
    public string? GetUserHash() => DisplayClaims?.Users is { Length: > 0 } users ? users[0].UserHash : null;
}

/// <summary>
/// Xbox Live display claims wrapper.
/// </summary>
public sealed class XboxDisplayClaims
{
    /// <summary>
    /// The per-user claims.
    /// </summary>
    [JsonPropertyName("xui")]
    public XboxUserClaim[]? Users { get; init; }
}

/// <summary>
/// A single Xbox user claim.
/// </summary>
public sealed class XboxUserClaim
{
    /// <summary>
    /// The Xbox user hash.
    /// </summary>
    [JsonPropertyName("uhs")]
    public string? UserHash { get; init; }
}

/// <summary>
/// Minecraft login-with-Xbox request body.
/// </summary>
public sealed class MinecraftIdentityRequest
{
    /// <summary>
    /// The XBL3.0 identity token.
    /// </summary>
    [JsonPropertyName("identityToken")]
    public string IdentityToken { get; init; } = string.Empty;
}

/// <summary>
/// Minecraft login-with-Xbox response body.
/// </summary>
public sealed class MinecraftLoginResponse
{
    /// <summary>
    /// The Minecraft access token.
    /// </summary>
    [JsonPropertyName("access_token")]
    public string? AccessToken { get; init; }
}

/// <summary>
/// The Minecraft player profile.
/// </summary>
public sealed class MinecraftProfile
{
    /// <summary>
    /// The player UUID.
    /// </summary>
    [JsonPropertyName("id")]
    public string? Id { get; init; }

    /// <summary>
    /// The player name.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }
}
