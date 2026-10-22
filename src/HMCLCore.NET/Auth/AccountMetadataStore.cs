using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Auth;

/// <summary>
/// Stores the metadata of the accounts in a detached JSON file.
/// </summary>
/// <remarks>
/// The metadata holds the public information of the accounts; the private
/// information, like the access tokens, is stored separately by
/// <see cref="AccountPrivateData"/>.
/// </remarks>
public sealed class AccountMetadataStore
{
    /// <summary>
    /// The JSON schema supported by this account metadata store.
    /// </summary>
    public const string CurrentSchema = "accounts/1.0.0";

    /// <summary>
    /// The JSON member holding the schema of the store.
    /// </summary>
    public const string PropertySchema = "schema";

    /// <summary>
    /// The JSON member holding the account records of the store.
    /// </summary>
    public const string PropertyAccounts = "accounts";

    /// <summary>
    /// The JSON member holding the username of the selected account.
    /// </summary>
    public const string PropertySelectedUsername = "selectedUsername";

    /// <summary>
    /// The schema used by this account metadata store file.
    /// </summary>
    public string Schema { get; set; } = CurrentSchema;

    /// <summary>
    /// The serialized account records.
    /// </summary>
    public List<JsonObject> Accounts { get; } = new();

    /// <summary>
    /// The username of the selected account, when it is known.
    /// </summary>
    public string? SelectedUsername { get; set; }

    /// <summary>
    /// Creates a store holding the given account records.
    /// </summary>
    /// <param name="records">The serialized account records.</param>
    /// <returns>The created store.</returns>
    public static AccountMetadataStore FromRecords(IEnumerable<JsonObject> records)
    {
        var store = new AccountMetadataStore();
        store.Accounts.AddRange(records);
        return store;
    }

    /// <summary>
    /// Loads the store from the given file.
    /// </summary>
    /// <param name="path">The path of the file to read.</param>
    /// <returns>
    /// The loaded store, or null when the file cannot be read or does not hold
    /// a valid store.
    /// </returns>
    public static AccountMetadataStore? Load(string path)
    {
        try
        {
            if (!File.Exists(path))
                return null;

            var node = JsonNode.Parse(File.ReadAllText(path)) as JsonObject;
            if (node is null)
                return null;

            var store = new AccountMetadataStore();

            if (node[PropertySchema] is JsonValue schemaValue && schemaValue.TryGetValue<string>(out var schema))
                store.Schema = schema;

            if (node[PropertySelectedUsername] is JsonValue selectedValue &&
                selectedValue.TryGetValue<string>(out var selected))
                store.SelectedUsername = selected;

            if (node[PropertyAccounts] is JsonArray accounts)
            {
                foreach (var account in accounts)
                {
                    if (account is JsonObject record)
                        store.Accounts.Add(record);
                }
            }

            return store;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Saves the store into the given file.
    /// </summary>
    /// <param name="path">The path of the file to write.</param>
    public void Save(string path)
    {
        var directory = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        var root = new JsonObject
        {
            [PropertySchema] = Schema,
            [PropertySelectedUsername] = SelectedUsername
        };

        var accounts = new JsonArray();
        foreach (var account in Accounts)
            accounts.Add(account.DeepClone());

        root[PropertyAccounts] = accounts;

        AccountPersistence.WriteAccountFile(path, root.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
    }
}
