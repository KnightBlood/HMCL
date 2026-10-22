using System;

namespace HMCLCore.NET.Auth;

/// <summary>
/// Base class for all Minecraft account types.
/// </summary>
public abstract class Account
{
    /// <summary>
    /// The account username.
    /// </summary>
    public string Username { get; init; } = string.Empty;

    /// <summary>
    /// The account type (offline, microsoft, yggdrasil, authlib-injector).
    /// </summary>
    public abstract string AccountType { get; }

    /// <summary>
    /// Returns whether this account is logged in.
    /// </summary>
    public abstract bool IsLoggedIn { get; }

    /// <summary>
    /// Returns whether this account is offline mode.
    /// </summary>
    public abstract bool IsOffline { get; }

    /// <summary>
    /// Returns a display-friendly name for the account.
    /// </summary>
    public abstract string GetDisplayName();

    /// <summary>
    /// The stable identifier of the account, used to store its private data.
    /// </summary>
    public virtual string AccountId => $"{AccountType}:{Username}";

    /// <summary>
    /// The path of the local skin of the account, when it has one.
    /// </summary>
    public virtual string? SkinPath { get; set; }

    public override string ToString() => $"{Username} ({AccountType})";
}

/// <summary>
/// Exception thrown when authentication fails.
/// </summary>
public sealed class AuthenticationException : Exception
{
    public AuthenticationException(string message) : base(message) { }
    public AuthenticationException(string message, Exception inner) : base(message, inner) { }
}

/// <summary>
/// Exception thrown when a character is deleted.
/// </summary>
public sealed class CharacterDeletedException : Exception
{
    public CharacterDeletedException(string message) : base(message) { }
}

/// <summary>
/// Exception thrown when credentials expire.
/// </summary>
public sealed class CredentialExpiredException : Exception
{
    public CredentialExpiredException(string message) : base(message) { }
}

/// <summary>
/// Exception thrown when no character is selected.
/// </summary>
public sealed class NoSelectedCharacterException : Exception
{
    public NoSelectedCharacterException(string message) : base(message) { }
}

/// <summary>
/// Exception thrown when not logged in.
/// </summary>
public sealed class NotLoggedInException : Exception
{
    public NotLoggedInException(string message) : base(message) { }
}

/// <summary>
/// Authentication information for an account session.
/// </summary>
public sealed class AuthInfo
{
    /// <summary>
    /// The access token.
    /// </summary>
    public string AccessToken { get; init; } = string.Empty;

    /// <summary>
    /// The client token.
    /// </summary>
    public string ClientToken { get; init; } = string.Empty;

    /// <summary>
    /// The UUID (without hyphens).
    /// </summary>
    public string Uuid { get; init; } = string.Empty;

    /// <summary>
    /// The username.
    /// </summary>
    public string Username { get; init; } = string.Empty;

    /// <summary>
    /// The account type.
    /// </summary>
    public string AccountType { get; init; } = string.Empty;

    /// <summary>
    /// The expires at timestamp.
    /// </summary>
    public DateTimeOffset ExpiresAt { get; init; } = DateTimeOffset.UtcNow;

    /// <summary>
    /// Returns whether the auth info is expired.
    /// </summary>
    public bool IsExpired => DateTimeOffset.UtcNow >= ExpiresAt;
}

/// <summary>
/// OAuth 2.0 authentication helper.
/// </summary>
public static class OAuth
{
    /// <summary>
    /// Generates a random state string for OAuth flow.
    /// </summary>
    public static string GenerateState() => Guid.NewGuid().ToString("N");

    /// <summary>
    /// Returns whether the given URL is a valid OAuth redirect.
    /// </summary>
    public static bool IsValidRedirectUri(string uri)
    {
        if (string.IsNullOrEmpty(uri)) return false;
        try { var u = new Uri(uri); return u.IsAbsoluteUri; } catch { return false; }
    }
}

/// <summary>
/// Base class for OAuth-based accounts.
/// </summary>
public abstract class OAuthAccount : Account
{
    /// <summary>
    /// The access token.
    /// </summary>
    public string? AccessToken { get; protected set; }

    /// <summary>
    /// The refresh token.
    /// </summary>
    public string? RefreshToken { get; protected set; }

    /// <summary>
    /// The client token.
    /// </summary>
    public string? ClientToken { get; protected set; }

    /// <summary>
    /// Returns whether this account is logged in.
    /// </summary>
    public override bool IsLoggedIn => !string.IsNullOrEmpty(AccessToken);

    /// <summary>
    /// Returns whether this account is offline mode.
    /// </summary>
    public override bool IsOffline => false;

    protected OAuthAccount(string username)
    {
        Username = username;
    }

    public override string GetDisplayName() => Username;
}

/// <summary>
/// Interface for account factories.
/// </summary>
public interface IAccountFactory
{
    /// <summary>
    /// The account type this factory creates.
    /// </summary>
    string AccountType { get; }

    /// <summary>
    /// Creates an account from the given parameters.
    /// </summary>
    Account Create(params object[] parameters);
}

    /// <summary>
    /// Manages account login, logout, and storage.
    /// </summary>
    public sealed class AccountManager
    {
        private readonly System.Collections.Generic.List<Account> _accounts = new();

        /// <summary>
        /// Raised whenever the account list or selection changes, allowing
        /// listeners to persist the new state.
        /// </summary>
        public event Action? AccountsChanged;

        /// <summary>
        /// All registered accounts.
        /// </summary>
        public System.Collections.Generic.IReadOnlyList<Account> Accounts => _accounts.AsReadOnly();

        /// <summary>
        /// The currently selected account, if any.
        /// </summary>
        public Account? SelectedAccount { get; private set; }

        /// <summary>
        /// Adds an account to the manager.
        /// </summary>
        public void AddAccount(Account account)
        {
            _accounts.Add(account);
            AccountsChanged?.Invoke();
        }

        /// <summary>
        /// Removes an account from the manager.
        /// </summary>
        public bool RemoveAccount(Account account)
        {
            var removed = _accounts.Remove(account);
            if (removed)
                AccountsChanged?.Invoke();
            return removed;
        }

        /// <summary>
        /// Selects an account as the active account.
        /// </summary>
        public void SelectAccount(Account account)
        {
            if (!_accounts.Contains(account))
                throw new ArgumentException("Account not registered", nameof(account));
            SelectedAccount = account;
            AccountsChanged?.Invoke();
        }

        /// <summary>
        /// Deselects the current account.
        /// </summary>
        public void DeselectAccount()
        {
            SelectedAccount = null;
            AccountsChanged?.Invoke();
        }

    /// <summary>
    /// Returns whether there is a selected account.
    /// </summary>
    public bool HasSelectedAccount => SelectedAccount != null;
}

/// <summary>
/// Classic Minecraft account (legacy).
/// </summary>
public sealed class ClassicAccount : Account
{
    public override string AccountType => "classic";
    public override bool IsLoggedIn => true;
    public override bool IsOffline => true;

    public ClassicAccount(string username)
    {
        Username = username;
    }

    public override string GetDisplayName() => Username;
}

/// <summary>
/// Offline mode account (no authentication).
/// </summary>
public sealed class OfflineAccount : Account
{
    public override string AccountType => "offline";
    public override bool IsLoggedIn => true;
    public override bool IsOffline => true;

    public OfflineAccount(string username)
    {
        Username = username;
    }

    public override string GetDisplayName() => Username;
}
