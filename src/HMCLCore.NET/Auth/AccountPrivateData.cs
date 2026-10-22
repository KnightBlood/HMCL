using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Auth;

/// <summary>
/// Stores the private data of the accounts in a protected payload.
/// </summary>
/// <remarks>
/// The private data holds the credentials of the accounts, like the access
/// tokens; it is stored in a file of its own so that it can be protected and
/// removed without touching the metadata of the accounts.
/// </remarks>
public sealed class AccountPrivateData
{
    /// <summary>
    /// The JSON schema supported by this account private data store.
    /// </summary>
    public const string CurrentSchema = "account-private-data/1.0.0";

    /// <summary>
    /// The JSON member holding the schema of the store.
    /// </summary>
    public const string PropertySchema = "schema";

    /// <summary>
    /// The JSON member holding the private data of the accounts.
    /// </summary>
    public const string PropertyPrivateData = "privateData";

    /// <summary>
    /// The protection mode used when the private data is written.
    /// </summary>
    public ProtectionMode Mode { get; set; } = ProtectionMode.ObfuscatedV1;

    /// <summary>
    /// The schema used by this account private data store file.
    /// </summary>
    public string Schema { get; set; } = CurrentSchema;

    /// <summary>
    /// The private data of the accounts, keyed by the identifier of the
    /// account.
    /// </summary>
    public Dictionary<string, JsonObject> PrivateData { get; } = new(StringComparer.Ordinal);

    /// <summary>
    /// Returns whether this store contains private data for the given account.
    /// </summary>
    /// <param name="accountId">The identifier of the account.</param>
    /// <returns>Whether the store contains private data for the account.</returns>
    public bool Contains(string accountId) => PrivateData.ContainsKey(accountId);

    /// <summary>
    /// Removes the private data of the given account.
    /// </summary>
    /// <param name="accountId">The identifier of the account.</param>
    /// <returns>Whether private data was removed.</returns>
    public bool Remove(string accountId) => PrivateData.Remove(accountId);

    /// <summary>
    /// Stores the private data of the given account, or removes the existing
    /// private data when the given data is empty.
    /// </summary>
    /// <param name="accountId">The identifier of the account.</param>
    /// <param name="data">The private data of the account.</param>
    public void Put(string accountId, JsonObject data)
    {
        if (data.Count == 0)
            PrivateData.Remove(accountId);
        else
            PrivateData[accountId] = data;
    }

    /// <summary>
    /// Returns the private data of the given account.
    /// </summary>
    /// <param name="accountId">The identifier of the account.</param>
    /// <returns>The private data of the account, or null when it is unknown.</returns>
    public JsonObject? Get(string accountId)
        => PrivateData.TryGetValue(accountId, out var data) ? data : null;

    /// <summary>
    /// Replaces the private data of this store with the private data of
    /// another store.
    /// </summary>
    /// <param name="other">The store to copy the private data from.</param>
    public void ReplaceWith(AccountPrivateData other)
    {
        PrivateData.Clear();
        foreach (var entry in other.PrivateData)
            Put(entry.Key, entry.Value);
    }

    /// <summary>
    /// Loads the store from the given file.
    /// </summary>
    /// <param name="path">The path of the file to read.</param>
    /// <returns>
    /// The loaded store, or null when the file cannot be read or does not hold
    /// a valid store.
    /// </returns>
    public static AccountPrivateData? Load(string path)
    {
        try
        {
            if (!File.Exists(path))
                return null;

            var node = JsonNode.Parse(File.ReadAllText(path)) as JsonObject;
            if (node is null)
                return null;

            var store = new AccountPrivateData();

            if (node[PropertySchema] is JsonValue schemaValue && schemaValue.TryGetValue<string>(out var schema))
                store.Schema = schema;

            var payload = ProtectedPayload.Reveal(node);
            if (payload is not JsonObject payloadObject)
                return store;

            if (payloadObject[PropertyPrivateData] is not JsonArray privateData)
                return store;

            foreach (var entry in privateData)
            {
                if (entry is not JsonObject item)
                    continue;

                var accountId = item["accountId"]?.GetValue<string>();
                if (string.IsNullOrEmpty(accountId))
                    continue;

                if (item[PropertyPrivateData] is JsonObject data)
                    store.PrivateData[accountId] = data;
            }

            return store;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or JsonException
                                      or InvalidOperationException or FormatException)
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

        var payload = new JsonObject();
        var privateData = new JsonArray();

        foreach (var entry in PrivateData)
        {
            var item = new JsonObject
            {
                ["accountId"] = entry.Key,
                [PropertyPrivateData] = entry.Value.DeepClone()
            };

            privateData.Add((System.Text.Json.Nodes.JsonNode)item);
        }

        payload[PropertyPrivateData] = privateData;

        var envelope = ProtectedPayload.Protect(payload, Mode);
        envelope[PropertySchema] = Schema;

        AccountPersistence.WriteAccountFile(path, envelope.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
    }
}
