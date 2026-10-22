using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Setting;

/// <summary>
/// The result of the migration of a legacy configuration file.
/// </summary>
public sealed class LegacyConfigMigrationResult
{
    /// <summary>
    /// Creates the result of the migration.
    /// </summary>
    /// <param name="path">The path of the migrated file.</param>
    /// <param name="settings">The settings read from the legacy file.</param>
    /// <param name="accounts">The account records read from the legacy file.</param>
    public LegacyConfigMigrationResult(string path, LauncherSettings settings, IReadOnlyList<JsonObject> accounts)
    {
        Path = path;
        Settings = settings;
        Accounts = accounts;
    }

    /// <summary>
    /// The path of the migrated file.
    /// </summary>
    public string Path { get; }

    /// <summary>
    /// The settings read from the legacy file.
    /// </summary>
    public LauncherSettings Settings { get; }

    /// <summary>
    /// The account records read from the legacy file.
    /// </summary>
    public IReadOnlyList<JsonObject> Accounts { get; }
}

/// <summary>
/// Migrates the configuration files written by the old versions of the
/// launcher into the current settings file.
/// </summary>
/// <remarks>
/// The old versions of the launcher used <c>hmcl.json</c> (and
/// <c>.hmcl.json</c> on Linux) as their configuration file; those files are
/// only read, never written back.
/// </remarks>
public static class LegacyConfigMigrator
{
    /// <summary>
    /// The names of the legacy configuration files.
    /// </summary>
    private static readonly string[] LegacyFileNames = { "hmcl.json", ".hmcl.json" };

    /// <summary>
    /// The name of the file recording that the legacy configuration was
    /// migrated.
    /// </summary>
    public const string ReceiptFileName = "legacy-config-migration.json";

    /// <summary>
    /// Returns the path of the legacy configuration file of the given
    /// directory, when there is one.
    /// </summary>
    /// <param name="directory">The directory holding the files of the launcher.</param>
    /// <returns>The path of the legacy file, or null when there is none.</returns>
    public static string? LocateLegacyConfig(string directory)
    {
        foreach (var name in LegacyFileNames)
        {
            var path = Path.Combine(directory, name);
            if (File.Exists(path))
                return path;
        }

        return null;
    }

    /// <summary>
    /// Returns whether the legacy configuration of the given directory was
    /// already migrated.
    /// </summary>
    /// <param name="directory">The directory holding the files of the launcher.</param>
    /// <returns>Whether the legacy configuration was already migrated.</returns>
    public static bool IsAlreadyMigrated(string directory)
    {
        var receipt = Path.Combine(directory, ReceiptFileName);
        if (!File.Exists(receipt))
            return false;

        try
        {
            var node = JsonNode.Parse(File.ReadAllText(receipt)) as JsonObject;
            return node?["migrated"]?.GetValue<bool>() == true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>
    /// Records that the legacy configuration was migrated.
    /// </summary>
    /// <param name="directory">The directory holding the files of the launcher.</param>
    /// <param name="path">The path of the migrated file.</param>
    public static void RecordMigration(string directory, string path)
    {
        try
        {
            var receipt = new JsonObject
            {
                ["migrated"] = true,
                ["path"] = path,
                ["migratedAt"] = DateTimeOffset.UtcNow.ToString("O")
            };

            File.WriteAllText(Path.Combine(directory, ReceiptFileName),
                receipt.ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        }
        catch (Exception)
        {
            // The receipt is optional; the migration is applied again when it
            // cannot be written.
        }
    }

    /// <summary>
    /// Reads the legacy configuration file of the given directory and converts
    /// it into the current settings.
    /// </summary>
    /// <param name="directory">The directory holding the files of the launcher.</param>
    /// <returns>
    /// The migrated settings, or null when there is no legacy file or when it
    /// cannot be read.
    /// </returns>
    public static LegacyConfigMigrationResult? Migrate(string directory)
    {
        var path = LocateLegacyConfig(directory);
        if (path is null)
            return null;

        return MigrateFile(path);
    }

    /// <summary>
    /// Reads the given legacy configuration file and converts it into the
    /// current settings.
    /// </summary>
    /// <param name="path">The path of the legacy file.</param>
    /// <returns>
    /// The migrated settings, or null when the file cannot be read or does not
    /// hold a legacy configuration.
    /// </returns>
    public static LegacyConfigMigrationResult? MigrateFile(string path)
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

        // The version of the legacy file; the files of a newer version are
        // not migrated.
        var version = root["_version"]?.GetValue<int>() ?? 0;
        if (version > 2)
            return null;

        var settings = new LauncherSettings();
        var accounts = new List<JsonObject>();

        // The memory of the game and the Java runtime.
        if (root["memory"] is JsonValue memory && memory.TryGetValue<int>(out var memoryValue))
            settings.Memory = memoryValue;

        if (root["java"] is JsonValue java && java.TryGetValue<string>(out var javaPath))
            settings.JavaPath = javaPath;

        if (root["javaArgs"] is JsonValue javaArgs && javaArgs.TryGetValue<string>(out var javaArgsValue))
        {
            settings.UseCustomJvmArgs = !string.IsNullOrWhiteSpace(javaArgsValue);
            settings.CustomJvmArgs = javaArgsValue;
        }

        // The size of the window.
        if (root["width"] is JsonValue width && width.TryGetValue<int>(out var widthValue))
            settings.WindowWidth = widthValue;

        if (root["height"] is JsonValue height && height.TryGetValue<int>(out var heightValue))
            settings.WindowHeight = heightValue;

        // The language of the launcher.
        if (root["localization"] is JsonValue localization && localization.TryGetValue<string>(out var language))
            settings.Language = ConvertLanguage(language);

        // The appearance of the launcher.
        if (root["fontFamily"] is JsonValue fontFamily && fontFamily.TryGetValue<string>(out var fontFamilyValue))
            settings.FontFamily = fontFamilyValue;

        if (root["fontSize"] is JsonValue fontSize && fontSize.TryGetValue<double>(out var fontSizeValue))
            settings.FontSize = fontSizeValue;

        var background = GetString(root, "bgpath") ?? GetString(root, "backgroundImage");
        if (!string.IsNullOrEmpty(background))
            settings.BackgroundImagePath = background;

        // The sources of the downloads.
        var source = ConvertDownloadSource(root);
        settings.VersionListSource = source;
        settings.FileDownloadSource = source;

        // The proxy of the launcher.
        var proxy = ConvertProxy(root);
        if (proxy != null)
            settings.ProxyConfig = proxy;

        // The accounts of the launcher.
        if (root["accounts"] is JsonArray legacyAccounts)
        {
            foreach (var account in legacyAccounts)
            {
                if (account is JsonObject record)
                    accounts.Add(record);
            }
        }

        if (root["selectedAccount"] is JsonValue selected && selected.TryGetValue<string>(out var selectedName))
            settings.LastUsername = selectedName;

        return new LegacyConfigMigrationResult(path, settings, accounts);
    }

    /// <summary>
    /// Converts the language of the old versions of the launcher into the
    /// language codes used today.
    /// </summary>
    /// <param name="language">The language of the legacy file.</param>
    /// <returns>The language code used by the launcher.</returns>
    private static string ConvertLanguage(string language) => language switch
    {
        "zh_CN" => "zh-Hans",
        "zh" => "zh-Hant",
        _ => language
    };

    /// <summary>
    /// Reads the source of the downloads of the legacy file.
    /// </summary>
    /// <param name="root">The legacy file.</param>
    /// <returns>The source of the downloads.</returns>
    private static DownloadSource ConvertDownloadSource(JsonObject root)
    {
        var automatic = true;
        if (root["autoChooseDownloadType"] is JsonValue autoValue && autoValue.TryGetValue<bool>(out var auto))
            automatic = auto;

        var legacy = GetString(root, "downloadType") ?? GetString(root, "versionListSource");
        var source = legacy?.ToLowerInvariant() switch
        {
            "default" or "balanced" => DownloadSource.Default,
            "official" or "mojang" => DownloadSource.Official,
            "mirror" or "bmclapi" => DownloadSource.Mirror,
            _ => automatic ? DownloadSource.Default : DownloadSource.Official
        };

        return source;
    }

    /// <summary>
    /// Reads the proxy of the legacy file.
    /// </summary>
    /// <param name="root">The legacy file.</param>
    /// <returns>The proxy of the launcher, or null when it has none.</returns>
    private static ProxyConfig? ConvertProxy(JsonObject root)
    {
        var hasProxy = false;
        if (root["hasProxy"] is JsonValue hasProxyValue && hasProxyValue.TryGetValue<bool>(out var enabled))
            hasProxy = enabled;

        if (!hasProxy)
            return null;

        var type = "http";
        var legacy = GetString(root, "proxyType");
        if (legacy != null)
        {
            type = legacy.ToLowerInvariant() switch
            {
                "direct" => "direct",
                "socks" => "socks5",
                _ => "http"
            };
        }

        return new ProxyConfig
        {
            Enabled = true,
            Type = type,
            Host = GetString(root, "proxyAddress") ?? GetString(root, "proxyHost") ?? string.Empty,
            Port = GetInt(root, "proxyPort") ?? 8080,
            Username = GetString(root, "proxyUserName") ?? GetString(root, "proxyUser"),
            Password = GetString(root, "proxyPassword")
        };
    }

    /// <summary>
    /// Returns the string value of a member of the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static string? GetString(JsonObject root, string name)
    {
        foreach (var member in root)
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
    /// Returns the integer value of a member of the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static int? GetInt(JsonObject root, string name)
    {
        foreach (var member in root)
        {
            if (!member.Key.Equals(name, StringComparison.OrdinalIgnoreCase))
                continue;

            return member.Value is JsonValue value && value.TryGetValue<int>(out var number)
                ? number
                : null;
        }

        return null;
    }
}
