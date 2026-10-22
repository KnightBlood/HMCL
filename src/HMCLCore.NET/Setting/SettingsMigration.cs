using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Setting;

/// <summary>
/// A migration of a settings file from one schema version to another one.
/// </summary>
public sealed class SettingsMigration
{
    /// <summary>
    /// Creates a migration.
    /// </summary>
    /// <param name="from">The version the migration starts from.</param>
    /// <param name="to">The version the migration leads to.</param>
    /// <param name="apply">The migration of the contents of the file.</param>
    public SettingsMigration(
        JsonSchema.SchemaVersion from,
        JsonSchema.SchemaVersion to,
        Func<JsonObject, JsonObject> apply)
    {
        From = from ?? throw new ArgumentNullException(nameof(from));
        To = to ?? throw new ArgumentNullException(nameof(to));
        Apply = apply ?? throw new ArgumentNullException(nameof(apply));
    }

    /// <summary>
    /// The version the migration starts from.
    /// </summary>
    public JsonSchema.SchemaVersion From { get; }

    /// <summary>
    /// The version the migration leads to.
    /// </summary>
    public JsonSchema.SchemaVersion To { get; }

    /// <summary>
    /// The migration of the contents of the file.
    /// </summary>
    public Func<JsonObject, JsonObject> Apply { get; }
}

/// <summary>
/// The schema of the settings files of the launcher and the migrations that
/// upgrade the files written by the older versions of the launcher.
/// </summary>
public static class SettingsMigrations
{
    /// <summary>
    /// The schema of the settings files written by the launcher.
    /// </summary>
    public static JsonSchema CurrentSchema { get; } = JsonSchema.Of("launcher-settings", 1, 2, 0);

    /// <summary>
    /// The version of the settings files written by the launcher before the
    /// game directories became a list.
    /// </summary>
    public static JsonSchema.SchemaVersion LegacyVersion { get; } = new(1, 0, 0);

    /// <summary>
    /// The migrations of the settings files, ordered by the version they start
    /// from.
    /// </summary>
    public static IReadOnlyList<SettingsMigration> All { get; } = new[]
    {
        new SettingsMigration(
            new JsonSchema.SchemaVersion(1, 0, 0),
            new JsonSchema.SchemaVersion(1, 1, 0),
            MigrateGameDirectories),
        new SettingsMigration(
            new JsonSchema.SchemaVersion(1, 1, 0),
            new JsonSchema.SchemaVersion(1, 2, 0),
            MigrateBackground)
    };

    /// <summary>
    /// Migrates the contents of a settings file to the current schema.
    /// </summary>
    /// <param name="root">The contents of the settings file.</param>
    /// <param name="version">
    /// The version of the settings file; the legacy version is assumed when the
    /// file does not have a schema marker.
    /// </param>
    /// <returns>The migrated contents of the settings file.</returns>
    public static JsonObject Migrate(JsonObject root, JsonSchema.SchemaVersion? version = null)
    {
        ArgumentNullException.ThrowIfNull(root);

        var current = version ?? LegacyVersion;
        var result = root;

        while (true)
        {
            var migration = FindNext(current);
            if (migration is null)
                break;

            result = migration.Apply(result);
            current = migration.To;
        }

        return result;
    }

    /// <summary>
    /// Returns the migration that leads from the given version to the next
    /// one.
    /// </summary>
    /// <param name="version">The version of the settings file.</param>
    /// <returns>The migration, or null when the file is up to date.</returns>
    private static SettingsMigration? FindNext(JsonSchema.SchemaVersion version)
    {
        foreach (var migration in All)
        {
            if (migration.From.CompareTo(version) == 0)
                return migration;
        }

        return null;
    }

    /// <summary>
    /// Migrates the settings files that hold no background image to the ones
    /// that use the wallpaper of the launcher, and turns the transparency of
    /// the window off, so that the window is not see-through by default.
    /// </summary>
    /// <param name="root">The contents of the settings file.</param>
    /// <returns>The migrated contents of the settings file.</returns>
    private static JsonObject MigrateBackground(JsonObject root)
    {
        var backgroundType = GetProperty(root, "BackgroundType") as JsonValue;
        if (backgroundType is not null &&
            string.Equals(backgroundType.GetValue<string>(), "None", StringComparison.Ordinal))
        {
            root["BackgroundType"] = "Default";
        }

        var wallpaperId = GetProperty(root, "BuiltinWallpaperId") as JsonValue;
        if (wallpaperId is not null && string.IsNullOrEmpty(wallpaperId.GetValue<string>()))
        {
            root["BuiltinWallpaperId"] = "2016-02-25";
        }

        var opacity = GetProperty(root, "BackgroundOpacity") as JsonValue;
        if (opacity is not null && Math.Abs(opacity.GetValue<double>() - 0.5) < 0.001)
        {
            root["BackgroundOpacity"] = 1.0;
        }

        var transparency = GetProperty(root, "EnableTransparencyEffects") as JsonValue;
        if (transparency is not null && transparency.GetValue<bool>())
        {
            root["EnableTransparencyEffects"] = false;
        }

        return root;
    }

    /// <summary>
    /// Migrates the settings files that hold a single game directory to the
    /// settings files that hold a list of game directories.
    /// </summary>
    /// <param name="root">The contents of the settings file.</param>
    /// <returns>The migrated contents of the settings file.</returns>
    private static JsonObject MigrateGameDirectories(JsonObject root)
    {
        var directories = GetProperty(root, "GameDirectories") as JsonArray;
        var legacyPath = (GetProperty(root, "GameDirectory") as JsonValue)?.GetValue<string>();

        if ((directories is null || directories.Count == 0) && !string.IsNullOrWhiteSpace(legacyPath))
        {
            var directory = new JsonObject
            {
                ["Id"] = "default",
                ["Path"] = legacyPath
            };

            root["GameDirectories"] = new JsonArray(directory);
            root["SelectedGameDirectoryId"] = "default";
        }

        return root;
    }

    /// <summary>
    /// Reads a property of a JSON object, ignoring the case of its name.
    /// </summary>
    /// <param name="root">The JSON object to read.</param>
    /// <param name="name">The name of the property.</param>
    /// <returns>The value of the property, or null when it is not present.</returns>
    private static JsonNode? GetProperty(JsonObject root, string name)
    {
        if (root.TryGetPropertyValue(name, out var value))
            return value;

        foreach (var property in root)
        {
            if (string.Equals(property.Key, name, StringComparison.OrdinalIgnoreCase))
                return property.Value;
        }

        return null;
    }
}
