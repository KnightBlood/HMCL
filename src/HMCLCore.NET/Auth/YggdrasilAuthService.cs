using System;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth;

/// <summary>
/// Yggdrasil authentication service for third-party authentication servers.
/// </summary>
public sealed class YggdrasilAuthService
{

    /// <summary>
    /// The Yggdrasil server URL.
    /// </summary>
    public string ServerUrl { get; init; } = string.Empty;

    /// <summary>
    /// Authenticates with username and password.
    /// </summary>
    public async Task<YggdrasilResponse?> AuthenticateAsync(string username, string password)
    {
        try
        {
            var request = new YggdrasilAuthRequest
            {
                Username = username,
                Password = password,
                ClientToken = Guid.NewGuid().ToString("N"),
                RequestUser = true
            };

            var json = JsonSerializer.Serialize(request, LauncherJsonContext.Default.YggdrasilAuthRequest);
            using var content = new StringContent(json, System.Text.Encoding.UTF8, "application/json");

            using var response = await HttpClientProvider.Shared.PostAsync($"{ServerUrl}/authenticate", content);
            response.EnsureSuccessStatusCode();

            var responseJson = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(responseJson, LauncherJsonContext.Default.YggdrasilResponse);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Refreshes Yggdrasil tokens.
    /// </summary>
    public async Task<YggdrasilResponse?> RefreshAsync(string accessToken, string clientToken)
    {
        try
        {
            var request = new YggdrasilRefreshRequest
            {
                AccessToken = accessToken,
                ClientToken = clientToken,
                RequestUser = true
            };

            var json = JsonSerializer.Serialize(request, LauncherJsonContext.Default.YggdrasilAuthRequest);
            using var content = new StringContent(json, System.Text.Encoding.UTF8, "application/json");

            using var response = await HttpClientProvider.Shared.PostAsync($"{ServerUrl}/refresh", content);
            response.EnsureSuccessStatusCode();

            var responseJson = await response.Content.ReadAsStringAsync();
            return JsonSerializer.Deserialize(responseJson, LauncherJsonContext.Default.YggdrasilResponse);
        }
        catch
        {
            return null;
        }
    }
}

/// <summary>
/// Yggdrasil authentication request.
/// </summary>
public sealed class YggdrasilAuthRequest
{
    public string Username { get; init; } = string.Empty;
    public string Password { get; init; } = string.Empty;
    public string ClientToken { get; init; } = string.Empty;
    public bool RequestUser { get; init; }
}

/// <summary>
/// Yggdrasil refresh request.
/// </summary>
public sealed class YggdrasilRefreshRequest
{
    public string AccessToken { get; init; } = string.Empty;
    public string ClientToken { get; init; } = string.Empty;
    public bool RequestUser { get; init; }
}

/// <summary>
/// Yggdrasil authentication response.
/// </summary>
public sealed class YggdrasilResponse
{
    public string AccessToken { get; init; } = string.Empty;
    public string ClientToken { get; init; } = string.Empty;
    public YggdrasilUser? User { get; init; }
    public YggdrasilProfile[] AvailableProfiles { get; init; } = Array.Empty<YggdrasilProfile>();
    public YggdrasilProfile SelectedProfile { get; init; } = new();
}

/// <summary>
/// Yggdrasil user info.
/// </summary>
public sealed class YggdrasilUser
{
    public string Id { get; init; } = string.Empty;
    public string Username { get; init; } = string.Empty;
}

/// <summary>
/// Yggdrasil game profile.
/// </summary>
public sealed class YggdrasilProfile
{
    public string Id { get; init; } = string.Empty;
    public string Name { get; init; } = string.Empty;
    public YggdrasilProperty[] properties { get; init; } = Array.Empty<YggdrasilProperty>();
}

/// <summary>
/// Yggdrasil profile property (e.g., textures).
/// </summary>
public sealed class YggdrasilProperty
{
    public string Name { get; init; } = string.Empty;
    public string Value { get; init; } = string.Empty;
    public string? Signature { get; init; }
}