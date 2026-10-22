using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Setting;

/// <summary>
/// The appearance settings stored in a theme pack.
/// </summary>
public sealed class ThemePack
{
    /// <summary>
    /// The name of the theme pack.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The theme of the launcher.
    /// </summary>
    public string? Theme { get; init; }

    /// <summary>
    /// The font of the interface.
    /// </summary>
    public string? FontFamily { get; init; }

    /// <summary>
    /// The size of the font of the interface.
    /// </summary>
    public double? FontSize { get; init; }

    /// <summary>
    /// The path of the background image extracted from the theme pack.
    /// </summary>
    public string? BackgroundImagePath { get; init; }

    /// <summary>
    /// Whether the launcher uses the transparency effects.
    /// </summary>
    public bool? EnableTransparencyEffects { get; init; }

    /// <summary>
    /// The seed color of the theme.
    /// </summary>
    public string? Color { get; init; }

    /// <summary>
    /// The color style of the theme.
    /// </summary>
    public string? ColorStyle { get; init; }

    /// <summary>
    /// The contrast level of the theme.
    /// </summary>
    public string? Contrast { get; init; }

    /// <summary>
    /// The identifier of the built-in wallpaper used by the theme.
    /// </summary>
    public string? BuiltinWallpaperId { get; init; }

    /// <summary>
    /// The opacity of the background of the theme.
    /// </summary>
    public double? BackgroundOpacity { get; init; }

    /// <summary>
    /// Whether the title bar of the launcher is transparent.
    /// </summary>
    public bool? TitleBarTransparent { get; init; }
}

/// <summary>
/// A theme pack installed in the launcher, like the installed theme packs of
/// Java HMCL.
/// </summary>
public sealed class InstalledThemePack
{
    /// <summary>
    /// The identifier of the theme pack.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The name of the theme pack shown to the user.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The version of the theme pack.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The directory holding the files of the theme pack.
    /// </summary>
    public string Directory { get; init; } = string.Empty;

    /// <summary>
    /// Whether the theme pack is shipped with the launcher.
    /// </summary>
    public bool Builtin { get; init; }

    /// <summary>
    /// The manifest of the theme pack.
    /// </summary>
    public ThemePackManifest Manifest { get; init; } = new();

    /// <summary>
    /// The number of the themes of the theme pack.
    /// </summary>
    public int ThemeCount => Manifest.Themes.Count;
}

/// <summary>
/// Exports and imports the appearance settings of the launcher as theme
/// packs, like the theme pack manager of Java HMCL.
/// </summary>
public static class ThemePackManager
{
    /// <summary>
    /// The name of the file holding the settings of a theme pack.
    /// </summary>
    public const string ManifestFileName = "manifest.json";

    /// <summary>
    /// The name of the file holding the settings of the theme packs written by
    /// the older versions of the launcher.
    /// </summary>
    public const string LegacyManifestFileName = "theme.json";

    /// <summary>
    /// The name of the file holding the background image of a theme pack.
    /// </summary>
    public const string BackgroundFileName = "background";

    /// <summary>
    /// Exports the appearance settings of the launcher into a theme pack.
    /// </summary>
    /// <param name="settings">The settings holding the appearance options.</param>
    /// <param name="destinationPath">The path of the theme pack to write.</param>
    /// <returns>The path of the written theme pack.</returns>
    public static string Export(LauncherSettings settings, string destinationPath)
    {
        ArgumentNullException.ThrowIfNull(settings);

        var directory = Path.GetDirectoryName(destinationPath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        var theme = new ThemePackTheme
        {
            Id = Path.GetFileNameWithoutExtension(destinationPath),
            Name = new LocalizedText(new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                [LocalizedText.DefaultKey] = Path.GetFileNameWithoutExtension(destinationPath)
            }),
            Color = string.IsNullOrEmpty(settings.ThemeColor) ? null : settings.ThemeColor,
            ColorStyle = settings.ThemeColorStyle.ToString().ToLowerInvariant(),
            Contrast = settings.ThemeContrast.ToString().ToLowerInvariant(),
            Background = new ThemePackBackground
            {
                Type = settings.BackgroundType switch
                {
                    BackgroundType.Image => "image",
                    BackgroundType.Network => "network",
                    BackgroundType.Builtin => "builtin",
                    BackgroundType.Color => "color",
                    _ => "none"
                },
                Id = settings.BackgroundType switch
                {
                    BackgroundType.Image => settings.BackgroundImagePath,
                    BackgroundType.Network => settings.NetworkBackgroundImageUrl,
                    BackgroundType.Builtin => settings.BuiltinWallpaperId,
                    _ => null
                },
                Opacity = settings.BackgroundOpacity
            },
            TitleBar = new ThemePackTitleBar { Transparent = settings.TitleBarTransparent },
            WindowTransparent = settings.EnableTransparencyEffects
        };

        var manifest = new ThemePackManifest
        {
            Id = Path.GetFileNameWithoutExtension(destinationPath),
            Version = "1.0.0",
            Name = new LocalizedText(new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                [LocalizedText.DefaultKey] = Path.GetFileNameWithoutExtension(destinationPath)
            }),
            Themes = new[] { theme }
        };

        using var archive = ZipFile.Open(destinationPath, ZipArchiveMode.Create);

        var entry = archive.CreateEntry(ManifestFileName);
        using (var stream = entry.Open())
        using (var writer = new StreamWriter(stream, new UTF8Encoding(false)))
        {
            writer.Write(manifest.ToJson().ToJsonString(new JsonSerializerOptions { WriteIndented = true }));
        }

        // The background image is copied into the theme pack.
        var background = settings.BackgroundImagePath;
        if (!string.IsNullOrEmpty(background) && File.Exists(background))
        {
            var backgroundEntry = archive.CreateEntry(BackgroundFileName +
                Path.GetExtension(background).ToLowerInvariant());

            using var backgroundStream = backgroundEntry.Open();
            using var fileStream = File.OpenRead(background);
            fileStream.CopyTo(backgroundStream);
        }

        return destinationPath;
    }

    /// <summary>
    /// Imports a theme pack and returns the appearance settings it holds.
    /// </summary>
    /// <param name="archivePath">The path of the theme pack to read.</param>
    /// <param name="targetDirectory">
    /// The directory receiving the files of the theme pack.
    /// </param>
    /// <returns>
    /// The imported theme, or null when the archive does not hold a theme pack.
    /// </returns>
    public static ThemePack? Import(string archivePath, string targetDirectory)
    {
        if (!File.Exists(archivePath))
            return null;

        try
        {
            using var archive = ZipFile.OpenRead(archivePath);
            var manifestEntry = archive.GetEntry(ManifestFileName) ??
                                archive.GetEntry(LegacyManifestFileName);
            if (manifestEntry is null)
                return null;

            JsonObject? manifest;
            using (var stream = manifestEntry.Open())
            using (var document = JsonDocument.Parse(stream))
            {
                manifest = JsonNode.Parse(document.RootElement.GetRawText()) as JsonObject;
            }

            if (manifest is null)
                return null;

            Directory.CreateDirectory(targetDirectory);

            string? backgroundPath = null;
            var backgroundEntry = archive.Entries.FirstOrDefault(entry =>
                entry.FullName.StartsWith(BackgroundFileName, StringComparison.OrdinalIgnoreCase));

            if (backgroundEntry != null)
            {
                var extension = Path.GetExtension(backgroundEntry.FullName);
                backgroundPath = Path.Combine(targetDirectory, $"background{extension}");

                using var backgroundStream = backgroundEntry.Open();
                using var fileStream = File.Create(backgroundPath);
                backgroundStream.CopyTo(fileStream);
            }

            // The theme packs written by the newer versions of the launcher hold
            // their appearance in the themes of their manifest.
            var packManifest = ThemePackManifest.FromJson(manifest);
            var theme = packManifest?.Themes.FirstOrDefault();

            if (theme is not null)
            {
                var backgroundType = theme.Background?.Type switch
                {
                    "image" => BackgroundType.Image,
                    "network" => BackgroundType.Network,
                    "builtin" => BackgroundType.Builtin,
                    "color" => BackgroundType.Color,
                    _ => BackgroundType.None
                };

                return new ThemePack
                {
                    Name = packManifest?.Name?.GetDisplayText(null) ??
                           packManifest?.Themes.FirstOrDefault()?.Name?.GetDisplayText(null) ??
                           Path.GetFileNameWithoutExtension(archivePath),
                    Theme = theme.Brightness,
                    Color = theme.Color,
                    ColorStyle = theme.ColorStyle,
                    Contrast = theme.Contrast,
                    BuiltinWallpaperId = backgroundType == BackgroundType.Builtin
                        ? theme.Background?.Id
                        : null,
                    BackgroundImagePath = backgroundType == BackgroundType.Image ? backgroundPath : null,
                    BackgroundOpacity = theme.Background?.Opacity,
                    TitleBarTransparent = theme.TitleBar?.Transparent,
                    EnableTransparencyEffects = theme.WindowTransparent
                };
            }

            return new ThemePack
            {
                Name = GetString(manifest, "name") ?? Path.GetFileNameWithoutExtension(archivePath),
                Theme = GetString(manifest, "theme"),
                FontFamily = GetString(manifest, "fontFamily"),
                FontSize = GetDouble(manifest, "fontSize"),
                EnableTransparencyEffects = GetBool(manifest, "enableTransparencyEffects"),
                BackgroundImagePath = backgroundPath
            };
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException
                                      or UnauthorizedAccessException or JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Reads the theme packs shipped with the launcher.
    /// </summary>
    /// <returns>The manifests of the built-in theme packs.</returns>
    public static IReadOnlyList<ThemePackManifest> GetBuiltinPacks()
    {
        var manifests = new List<ThemePackManifest>();

        foreach (var name in BuiltinPackNames)
        {
            var manifest = ReadBuiltinManifest(name);
            if (manifest is not null)
                manifests.Add(manifest);
        }

        return manifests;
    }

    /// <summary>
    /// The names of the built-in theme packs.
    /// </summary>
    private static readonly string[] BuiltinPackNames = { "hmcl.default", "hmcl.classic" };

    /// <summary>
    /// Lists the theme packs installed in the given directory, with the ones
    /// shipped with the launcher first.
    /// </summary>
    /// <param name="directory">The directory holding the installed theme packs.</param>
    /// <returns>The installed theme packs.</returns>
    public static IReadOnlyList<InstalledThemePack> ListInstalled(string directory)
    {
        var packs = new List<InstalledThemePack>();

        foreach (var name in BuiltinPackNames)
        {
            var manifest = ReadBuiltinManifest(name);
            if (manifest is null)
                continue;

            packs.Add(new InstalledThemePack
            {
                Id = manifest.Id,
                Name = manifest.Name?.GetDisplayText(null) ?? manifest.Id,
                Version = manifest.Version,
                Directory = string.Empty,
                Builtin = true,
                Manifest = manifest
            });
        }

        if (!string.IsNullOrEmpty(directory) && Directory.Exists(directory))
        {
            foreach (var child in Directory.GetDirectories(directory))
            {
                var manifestPath = Path.Combine(child, ThemePackManifest.ManifestFileName);
                if (!File.Exists(manifestPath))
                    continue;

                var manifest = ReadManifest(manifestPath);
                if (manifest is null || string.IsNullOrEmpty(manifest.Id))
                    continue;

                packs.Add(new InstalledThemePack
                {
                    Id = manifest.Id,
                    Name = manifest.Name?.GetDisplayText(null) ?? manifest.Id,
                    Version = manifest.Version,
                    Directory = child,
                    Builtin = false,
                    Manifest = manifest
                });
            }
        }

        return packs;
    }

    /// <summary>
    /// Installs a theme pack from an archive into the given directory.
    /// </summary>
    /// <param name="archivePath">The path of the archive holding the theme pack.</param>
    /// <param name="directory">The directory receiving the theme pack.</param>
    /// <returns>The installed theme pack, or null when it cannot be installed.</returns>
    public static InstalledThemePack? Install(string archivePath, string directory)
    {
        if (!File.Exists(archivePath) || string.IsNullOrEmpty(directory))
            return null;

        try
        {
            // The manifest of the archive is read first, so that a file which
            // is not a theme pack is not installed at all.
            ThemePackManifest? manifest;
            using (var archive = ZipFile.OpenRead(archivePath))
            {
                var entry = archive.GetEntry(ThemePackManifest.ManifestFileName);
                if (entry is null)
                    return null;

                using var stream = entry.Open();
                using var document = JsonDocument.Parse(stream);
                manifest = ThemePackManifest.FromJson(JsonNode.Parse(document.RootElement.GetRawText()));
            }

            if (manifest is null || string.IsNullOrEmpty(manifest.Id))
                return null;

            var target = Path.Combine(directory, manifest.Id);
            Directory.CreateDirectory(directory);

            if (Directory.Exists(target))
                Directory.Delete(target, true);

            ZipFile.ExtractToDirectory(archivePath, target);

            // The manifest is read again, so that the files written by the
            // archive are the ones used by the launcher.
            var installedManifest = ReadManifest(Path.Combine(target, ThemePackManifest.ManifestFileName))
                                   ?? manifest;

            return new InstalledThemePack
            {
                Id = installedManifest.Id,
                Name = installedManifest.Name?.GetDisplayText(null) ?? installedManifest.Id,
                Version = installedManifest.Version,
                Directory = target,
                Builtin = false,
                Manifest = installedManifest
            };
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException
                                      or UnauthorizedAccessException or JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Removes an installed theme pack from the launcher.
    /// </summary>
    /// <param name="pack">The theme pack removed.</param>
    /// <returns>Whether the theme pack was removed.</returns>
    public static bool Uninstall(InstalledThemePack pack)
    {
        if (pack is null || pack.Builtin || string.IsNullOrEmpty(pack.Directory))
            return false;

        try
        {
            if (Directory.Exists(pack.Directory))
                Directory.Delete(pack.Directory, true);

            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Reads the manifest of a theme pack.
    /// </summary>
    /// <param name="path">The path of the manifest.</param>
    /// <returns>The manifest, or null when it cannot be read.</returns>
    private static ThemePackManifest? ReadManifest(string path)
    {
        try
        {
            using var document = JsonDocument.Parse(File.ReadAllText(path));
            return ThemePackManifest.FromJson(JsonNode.Parse(document.RootElement.GetRawText()));
        }
        catch (Exception ex) when (ex is IOException or JsonException or InvalidOperationException)
        {
            return null;
        }
    }

    /// <summary>
    /// Reads the manifest of a built-in theme pack.
    /// </summary>
    /// <param name="name">The name of the theme pack.</param>
    /// <returns>The manifest, or null when it cannot be read.</returns>
    private static ThemePackManifest? ReadBuiltinManifest(string name)
    {
        try
        {
            var assembly = typeof(ThemePackManager).Assembly;
            using var stream = assembly.GetManifestResourceStream($"HMCLCore.NET.Resources.themes.{name}.manifest.json");
            if (stream is null)
                return null;

            using var document = JsonDocument.Parse(stream);
            var manifest = JsonNode.Parse(document.RootElement.GetRawText());
            return ThemePackManifest.FromJson(manifest);
        }
        catch (Exception ex) when (ex is IOException or JsonException or InvalidOperationException)
        {
            return null;
        }
    }

    /// <summary>
    /// Returns the string value of a member of the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static string? GetString(JsonObject root, string name)
        => root[name] is JsonValue value && value.TryGetValue<string>(out var text) ? text : null;

    /// <summary>
    /// Returns the numeric value of a member of the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static double? GetDouble(JsonObject root, string name)
        => root[name] is JsonValue value && value.TryGetValue<double>(out var number) ? number : null;

    /// <summary>
    /// Returns the boolean value of a member of the given object.
    /// </summary>
    /// <param name="root">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static bool? GetBool(JsonObject root, string name)
        => root[name] is JsonValue value && value.TryGetValue<bool>(out var flag) ? flag : null;
}
