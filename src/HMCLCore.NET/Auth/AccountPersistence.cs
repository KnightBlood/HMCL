using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;
using HMCLCore.NET.Auth.Microsoft;

namespace HMCLCore.NET.Auth;

/// <summary>
/// Serializes the registered accounts to JSON so credentials survive launcher restarts.
/// </summary>
/// <remarks>
/// The public information of the accounts is stored in a metadata file, while
/// their credentials are stored in a separate file holding a protected payload.
/// </remarks>
public static class AccountPersistence
{
    /// <summary>
    /// The name of the file holding the private data of the accounts.
    /// </summary>
    public const string PrivateDataFileName = "account-private-data.json";

    private static readonly JsonSerializerOptions JsonOptions = new() { WriteIndented = true };

    /// <summary>
    /// Returns the path of the private data file belonging to the given
    /// metadata file.
    /// </summary>
    /// <param name="metadataPath">The path of the metadata file.</param>
    /// <returns>The path of the private data file.</returns>
    public static string GetPrivateDataPath(string metadataPath)
    {
        var directory = Path.GetDirectoryName(metadataPath);
        return string.IsNullOrEmpty(directory)
            ? PrivateDataFileName
            : Path.Combine(directory, PrivateDataFileName);
    }

    /// <summary>
    /// Saves the given accounts and the selected account's username to the file.
    /// </summary>
    /// <param name="path">Target JSON file path.</param>
    /// <param name="accounts">The accounts to serialize.</param>
    /// <param name="selectedUsername">Username of the currently selected account, if any.</param>
    public static void Save(string path, IEnumerable<Account> accounts, string? selectedUsername)
        => Save(path, GetPrivateDataPath(path), accounts, selectedUsername);

    /// <summary>
    /// Saves the given accounts and the selected account's username to the
    /// given files.
    /// </summary>
    /// <param name="metadataPath">Target path of the metadata file.</param>
    /// <param name="privateDataPath">Target path of the private data file.</param>
    /// <param name="accounts">The accounts to serialize.</param>
    /// <param name="selectedUsername">Username of the currently selected account, if any.</param>
    public static void Save(string metadataPath, string privateDataPath, IEnumerable<Account> accounts, string? selectedUsername)
    {
        try
        {
            var store = new AccountMetadataStore
            {
                SelectedUsername = selectedUsername
            };

            var privateData = new AccountPrivateData();

            foreach (var account in accounts)
            {
                store.Accounts.Add(ToMetadata(account));
                privateData.Put(account.AccountId, ToPrivateData(account));
            }

            store.Save(metadataPath);
            privateData.Save(privateDataPath);
        }
        catch (Exception)
        {
            // Persistence is best-effort and never blocks the account manager.
        }
    }

    /// <summary>
    /// Writes a file of the accounts, backing up the file and removing its
    /// read-only attribute when it cannot be written.
    /// </summary>
    /// <param name="path">The path of the file to write.</param>
    /// <param name="content">The content of the file.</param>
    public static void WriteAccountFile(string path, string content)
    {
        var directory = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        try
        {
            File.WriteAllText(path, content);
            return;
        }
        catch (UnauthorizedAccessException)
        {
            // The file cannot be written; it is backed up and written again.
        }
        catch (IOException)
        {
            // The file is locked by another process; nothing can be done.
            return;
        }

        if (!BackupReadOnlyFile(path))
            return;

        try
        {
            File.WriteAllText(path, content);
        }
        catch (Exception)
        {
            // The file still cannot be written; the settings are kept in memory.
        }
    }

    /// <summary>
    /// Backs up a read-only file and removes its read-only attribute.
    /// </summary>
    /// <param name="path">The path of the file to back up.</param>
    /// <returns>Whether the file was backed up.</returns>
    private static bool BackupReadOnlyFile(string path)
    {
        try
        {
            if (!File.Exists(path))
                return false;

            var attributes = File.GetAttributes(path);
            if ((attributes & FileAttributes.ReadOnly) == 0)
                return false;

            var backup = $"{path}.{DateTime.Now:yyyyMMdd-HHmmss}.bak";
            File.Copy(path, backup);
            File.SetAttributes(path, attributes & ~FileAttributes.ReadOnly);
            return true;
        }
        catch (Exception)
        {
            // The file cannot be backed up; it stays read-only.
            return false;
        }
    }

    /// <summary>
    /// Loads accounts from the given file.
    /// </summary>
    /// <param name="path">Source JSON file path.</param>
    /// <returns>The restored accounts and the selected account's username.</returns>
    public static (List<Account> Accounts, string? SelectedUsername) Load(string path)
        => Load(path, GetPrivateDataPath(path));

    /// <summary>
    /// Loads accounts from the given files.
    /// </summary>
    /// <param name="metadataPath">Source path of the metadata file.</param>
    /// <param name="privateDataPath">Source path of the private data file.</param>
    /// <returns>The restored accounts and the selected account's username.</returns>
    public static (List<Account> Accounts, string? SelectedUsername) Load(string metadataPath, string privateDataPath)
    {
        var accounts = new List<Account>();

        try
        {
            var privateData = AccountPrivateData.Load(privateDataPath) ?? new AccountPrivateData();

            AccountMetadataStore store;
            if (IsLegacyFile(metadataPath))
            {
                // The file may still hold the accounts of the old format, which
                // kept the credentials of the accounts in the same file.
                var migrated = MigrateLegacyFile(metadataPath, privateData);
                if (migrated is null)
                    return (accounts, null);

                store = migrated.Store;
                privateData = migrated.PrivateData;

                // The migrated accounts are written back in the new format, so
                // that the old file is converted only once.
                try
                {
                    store.Save(metadataPath);
                    privateData.Save(privateDataPath);
                }
                catch (Exception)
                {
                    // The migration stays in memory when it cannot be written.
                }
            }
            else
            {
                var loaded = AccountMetadataStore.Load(metadataPath);
                if (loaded is null)
                    return (accounts, null);

                store = loaded;
            }

            foreach (var record in store.Accounts)
            {
                var account = FromMetadata(record, privateData);
                if (account != null)
                    accounts.Add(account);
            }

            return (accounts, store.SelectedUsername);
        }
        catch (Exception)
        {
            return (accounts, null);
        }
    }

    /// <summary>
    /// Returns whether the given file holds the accounts of the old format,
    /// which kept the credentials of the accounts in the same file as their
    /// metadata.
    /// </summary>
    /// <param name="path">The path of the file to check.</param>
    /// <returns>Whether the file uses the old format.</returns>
    private static bool IsLegacyFile(string path)
    {
        try
        {
            if (!File.Exists(path))
                return false;

            if (JsonNode.Parse(File.ReadAllText(path)) is not JsonObject root)
                return false;

            return root.ContainsKey("Accounts");
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>
    /// The result of the migration of an account file of the old format.
    /// </summary>
    /// <param name="Store">The metadata store built from the old file.</param>
    /// <param name="PrivateData">The private data extracted from the old file.</param>
    private sealed record MigrationResult(AccountMetadataStore Store, AccountPrivateData PrivateData);

    /// <summary>
    /// Migrates an account file of the old format, which kept the credentials
    /// of the accounts in the same file as their metadata.
    /// </summary>
    /// <param name="path">The path of the file to migrate.</param>
    /// <param name="privateData">The private data extracted from the file.</param>
    /// <returns>The migrated store, or null when the file is not of the old format.</returns>
    private static MigrationResult? MigrateLegacyFile(string path, AccountPrivateData privateData)
    {
        JsonObject? root;
        try
        {
            if (!File.Exists(path))
                return null;

            root = JsonNode.Parse(File.ReadAllText(path)) as JsonObject;
        }
        catch (Exception)
        {
            return null;
        }

        if (root is null)
            return null;

        // The old format kept the accounts in the "Accounts" member and the
        // name of the selected account in the "SelectedUsername" member.
        if (root["Accounts"] is not JsonArray legacyAccounts)
            return null;

        var store = new AccountMetadataStore();

        if (root["SelectedUsername"] is JsonValue selectedValue &&
            selectedValue.TryGetValue<string>(out var selected))
            store.SelectedUsername = selected;

        foreach (var entry in legacyAccounts)
        {
            if (entry is not JsonObject record)
                continue;

            var type = GetString(record, "Type") ?? GetString(record, "type");
            if (string.IsNullOrEmpty(type))
                continue;

            var username = GetString(record, "Username") ?? GetString(record, "username") ?? string.Empty;
            var serverUrl = GetString(record, "ServerUrl") ?? GetString(record, "serverUrl");
            var uuid = GetString(record, "Uuid") ?? GetString(record, "uuid");
            var profileName = GetString(record, "ProfileName") ?? GetString(record, "profileName");

            var metadata = new JsonObject
            {
                ["type"] = type,
                ["username"] = username
            };

            if (!string.IsNullOrEmpty(serverUrl))
                metadata["serverUrl"] = serverUrl;
            if (!string.IsNullOrEmpty(uuid))
                metadata["uuid"] = uuid;
            if (!string.IsNullOrEmpty(profileName))
                metadata["profileName"] = profileName;

            store.Accounts.Add(metadata);

            // The credentials of the old format are moved into the private data.
            var credentials = new JsonObject();
            CopyString(record, credentials, "AccessToken", "accessToken");
            CopyString(record, credentials, "RefreshToken", "refreshToken");
            CopyString(record, credentials, "ClientToken", "clientToken");

            if (credentials.Count > 0)
                privateData.Put(AccountIdOf(type, serverUrl, uuid, username), credentials);
        }

        return new MigrationResult(store, privateData);
    }

    /// <summary>
    /// Returns the value of a member of the given object, ignoring the case
    /// of its name.
    /// </summary>
    /// <param name="record">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static string? GetString(JsonObject record, string name)
    {
        foreach (var member in record)
        {
            if (!member.Key.Equals(name, StringComparison.OrdinalIgnoreCase))
                continue;

            return member.Value is JsonValue value && value.TryGetValue<string>(out var text)
                ? text
                : null;
        }

        return null;
    }

    /// <summary>
    /// Copies a member of an object into another one when it holds a value.
    /// </summary>
    /// <param name="source">The object to read.</param>
    /// <param name="target">The object to write.</param>
    /// <param name="sourceName">The name of the member to read.</param>
    /// <param name="targetName">The name of the member to write.</param>
    private static void CopyString(JsonObject source, JsonObject target, string sourceName, string targetName)
    {
        var value = GetString(source, sourceName);
        if (!string.IsNullOrEmpty(value))
            target[targetName] = value;
    }

    /// <summary>
    /// Builds the identifier of an account from its metadata.
    /// </summary>
    /// <param name="type">The type of the account.</param>
    /// <param name="serverUrl">The URL of the authentication server.</param>
    /// <param name="uuid">The UUID of the selected profile.</param>
    /// <param name="username">The name of the account.</param>
    /// <returns>The identifier of the account.</returns>
    private static string AccountIdOf(string type, string? serverUrl, string? uuid, string username)
    {
        if (string.Equals(type, "microsoft", StringComparison.OrdinalIgnoreCase))
            return string.IsNullOrEmpty(uuid) ? $"microsoft:{username}" : $"microsoft:{uuid}";

        if (string.Equals(type, "yggdrasil", StringComparison.OrdinalIgnoreCase))
            return string.IsNullOrEmpty(uuid)
                ? $"yggdrasil:{serverUrl}:{username}"
                : $"yggdrasil:{serverUrl}:{uuid}";

        return $"{type}:{username}";
    }

    /// <summary>
    /// Converts an account onto its metadata record.
    /// </summary>
    /// <param name="account">The account to convert.</param>
    /// <returns>The metadata of the account.</returns>
    private static JsonObject ToMetadata(Account account)
    {
        var record = new JsonObject
        {
            ["type"] = account.AccountType,
            ["username"] = account.Username
        };

        switch (account)
        {
            case Yggdrasil.YggdrasilAccount ygg:
                if (!string.IsNullOrEmpty(ygg.ServerUrl))
                    record["serverUrl"] = ygg.ServerUrl;
                if (!string.IsNullOrEmpty(ygg.ProfileUuid))
                    record["uuid"] = ygg.ProfileUuid;
                if (!string.IsNullOrEmpty(ygg.ProfileName))
                    record["profileName"] = ygg.ProfileName;
                break;

            case MicrosoftAccount ms:
                if (!string.IsNullOrEmpty(ms.MinecraftUuid))
                    record["uuid"] = ms.MinecraftUuid;
                if (!string.IsNullOrEmpty(ms.MinecraftUsername))
                    record["profileName"] = ms.MinecraftUsername;
                break;

            case OfflineAccount offline:
                if (!string.IsNullOrEmpty(offline.SkinPath))
                    record["skinPath"] = offline.SkinPath;
                break;
        }

        return record;
    }

    /// <summary>
    /// Converts an account onto its private data record.
    /// </summary>
    /// <param name="account">The account to convert.</param>
    /// <returns>The private data of the account.</returns>
    private static JsonObject ToPrivateData(Account account)
    {
        var data = new JsonObject();

        switch (account)
        {
            case Yggdrasil.YggdrasilAccount ygg:
                AddIfPresent(data, "accessToken", ygg.YggdrasilAccessToken);
                AddIfPresent(data, "clientToken", ygg.YggdrasilClientToken);
                break;

            case MicrosoftAccount ms:
                AddIfPresent(data, "accessToken", ms.AccessToken);
                AddIfPresent(data, "refreshToken", ms.RefreshToken);
                AddIfPresent(data, "clientToken", ms.ClientToken);
                break;
        }

        return data;
    }

    /// <summary>
    /// Adds a member to the given object when the value is not empty.
    /// </summary>
    /// <param name="data">The object to write.</param>
    /// <param name="name">The name of the member.</param>
    /// <param name="value">The value of the member.</param>
    private static void AddIfPresent(JsonObject data, string name, string? value)
    {
        if (!string.IsNullOrEmpty(value))
            data[name] = value;
    }

    /// <summary>
    /// Rebuilds an account from its metadata and its private data.
    /// </summary>
    /// <param name="record">The metadata of the account.</param>
    /// <param name="privateData">The private data of the accounts.</param>
    /// <returns>The restored account, or null when it cannot be restored.</returns>
    private static Account? FromMetadata(JsonObject record, AccountPrivateData privateData)
    {
        var type = GetString(record, "type");
        if (string.IsNullOrEmpty(type))
            return null;

        var username = GetString(record, "username") ?? string.Empty;
        var serverUrl = GetString(record, "serverUrl");
        var uuid = GetString(record, "uuid");
        var profileName = GetString(record, "profileName");
        var skinPath = GetString(record, "skinPath");

        var credentials = privateData.Get(AccountIdOf(type, serverUrl, uuid, username));

        switch (type)
        {
            case "offline":
            case "classic":
            {
                var account = new OfflineAccount(username);
                if (!string.IsNullOrEmpty(skinPath))
                    account.SkinPath = skinPath;
                return account;
            }

            case "yggdrasil":
            {
                var account = new Yggdrasil.YggdrasilAccount(username, serverUrl ?? string.Empty);
                if (credentials != null)
                {
                    var accessToken = GetString(credentials, "accessToken");
                    var clientToken = GetString(credentials, "clientToken");
                    if (!string.IsNullOrEmpty(accessToken))
                        account.SetYggdrasilTokens(accessToken, clientToken ?? string.Empty);
                }

                if (!string.IsNullOrEmpty(uuid))
                    account.SetProfile(uuid, profileName ?? username);

                return account;
            }

            case "microsoft":
            {
                var account = new MicrosoftAccount(username);
                if (credentials != null)
                {
                    var accessToken = GetString(credentials, "accessToken");
                    var refreshToken = GetString(credentials, "refreshToken");
                    var clientToken = GetString(credentials, "clientToken");
                    if (!string.IsNullOrEmpty(accessToken))
                        account.SetTokens(accessToken, refreshToken, clientToken);
                }

                if (!string.IsNullOrEmpty(uuid))
                    account.SetMinecraftSession(credentials?["accessToken"]?.GetValue<string>() ?? string.Empty,
                        uuid, profileName ?? username);

                return account;
            }

            default:
                return null;
        }
    }
}
