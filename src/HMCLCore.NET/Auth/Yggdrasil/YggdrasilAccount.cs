using System;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth.Yggdrasil;

/// <summary>
/// Yggdrasil account for third-party authentication servers.
/// </summary>
public sealed class YggdrasilAccount : OAuthAccount
{
    /// <summary>
    /// The Yggdrasil server URL.
    /// </summary>
    public string ServerUrl { get; init; } = string.Empty;

    /// <summary>
    /// The access token from Yggdrasil authentication.
    /// </summary>
    public string? YggdrasilAccessToken { get; private set; }

    /// <summary>
    /// The client token from Yggdrasil authentication.
    /// </summary>
    public string? YggdrasilClientToken { get; private set; }

    /// <summary>
    /// The selected profile UUID.
    /// </summary>
    public string? ProfileUuid { get; private set; }

    /// <summary>
    /// The selected profile name.
    /// </summary>
    public string? ProfileName { get; private set; }

    /// <summary>
    /// The account type.
    /// </summary>
    public override string AccountType => "yggdrasil";

    public YggdrasilAccount(string username, string serverUrl) : base(username)
    {
        ServerUrl = serverUrl;
    }

    /// <summary>
    /// Sets the Yggdrasil authentication tokens.
    /// </summary>
    public void SetYggdrasilTokens(string accessToken, string clientToken)
    {
        YggdrasilAccessToken = accessToken;
        YggdrasilClientToken = clientToken;
        AccessToken = accessToken;
        ClientToken = clientToken;
    }

    /// <summary>
    /// Sets the selected profile.
    /// </summary>
    public void SetProfile(string uuid, string name)
    {
        ProfileUuid = uuid;
        ProfileName = name;
    }

    public override string GetDisplayName() => ProfileName ?? Username;

    /// <summary>
    /// The stable identifier of the account, built from its server and profile.
    /// </summary>
    public override string AccountId =>
        string.IsNullOrEmpty(ProfileUuid)
            ? $"{AccountType}:{ServerUrl}:{Username}"
            : $"{AccountType}:{ServerUrl}:{ProfileUuid}";
}

/// <summary>
/// Yggdrasil account factory.
/// </summary>
public sealed class YggdrasilAccountFactory : IAccountFactory
{
    public string AccountType => "yggdrasil";

    public Account Create(params object[] parameters)
    {
        if (parameters.Length < 2 ||
            parameters[0] is not string username ||
            parameters[1] is not string serverUrl)
            throw new ArgumentException("Username and server URL are required", nameof(parameters));
        return new YggdrasilAccount(username, serverUrl);
    }
}

/// <summary>
/// Offline account factory.
/// </summary>
public sealed class OfflineAccountFactory : IAccountFactory
{
    public string AccountType => "offline";

    public Account Create(params object[] parameters)
    {
        var username = parameters.Length > 0 && parameters[0] is string name ? name : "Player";
        return new OfflineAccount(username);
    }
}

/// <summary>
/// Classic account factory.
/// </summary>
public sealed class ClassicAccountFactory : IAccountFactory
{
    public string AccountType => "classic";

    public Account Create(params object[] parameters)
    {
        var username = parameters.Length > 0 && parameters[0] is string name ? name : "Player";
        return new ClassicAccount(username);
    }
}

/// <summary>
/// Account factory registry that creates accounts by type.
/// </summary>
public sealed class AccountFactory
{
    private readonly System.Collections.Generic.Dictionary<string, IAccountFactory> _factories = new();

    /// <summary>
    /// Registers an account factory for the given type.
    /// </summary>
    public void Register(string accountType, IAccountFactory factory)
    {
        _factories[accountType] = factory;
    }

    /// <summary>
    /// Returns whether the given account type is registered.
    /// </summary>
    public bool IsRegistered(string accountType) => _factories.ContainsKey(accountType);

    /// <summary>
    /// Creates an account of the given type.
    /// </summary>
    public Account Create(string accountType, params object[] parameters)
    {
        if (!_factories.TryGetValue(accountType, out var factory))
            throw new ArgumentException($"Unknown account type: {accountType}", nameof(accountType));
        return factory.Create(parameters);
    }

    /// <summary>
    /// Creates a default account factory with all built-in types registered.
    /// </summary>
    public static AccountFactory CreateDefault()
    {
        var factory = new AccountFactory();
        factory.Register("offline", new OfflineAccountFactory());
        factory.Register("classic", new ClassicAccountFactory());
        factory.Register("microsoft", new Microsoft.MicrosoftAccountFactory());
        factory.Register("yggdrasil", new YggdrasilAccountFactory());
        return factory;
    }
}

/// <summary>
/// Yggdrasil authentication service.
/// </summary>
public sealed class YggdrasilAuthService
{

    /// <summary>
    /// Authenticates with username and password against a Yggdrasil server.
    /// </summary>
    public async Task<YggdrasilResponse?> AuthenticateAsync(string serverUrl, string username, string password)
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

            using var response = await HttpClientProvider.Shared.PostAsync($"{serverUrl.TrimEnd('/')}/authenticate", content);
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
    public async Task<YggdrasilResponse?> RefreshAsync(string serverUrl, string accessToken, string clientToken)
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

            using var response = await HttpClientProvider.Shared.PostAsync($"{serverUrl.TrimEnd('/')}/refresh", content);
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
