using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Setting;

/// <summary>
/// A text written in several languages, like the localized texts of the theme
/// packs of Java HMCL.
/// </summary>
public sealed class LocalizedText
{
    /// <summary>
    /// The key of the text used when the language of the user is unknown.
    /// </summary>
    public const string DefaultKey = "default";

    /// <summary>
    /// Creates an empty text.
    /// </summary>
    public LocalizedText()
    {
        Texts = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Creates a text with the given translations.
    /// </summary>
    /// <param name="texts">The translations, keyed by the language.</param>
    public LocalizedText(IDictionary<string, string> texts)
    {
        Texts = new Dictionary<string, string>(texts, StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>
    /// The translations of the text, keyed by the language.
    /// </summary>
    public IReadOnlyDictionary<string, string> Texts { get; }

    /// <summary>
    /// Whether the text holds no translation at all.
    /// </summary>
    public bool IsEmpty => Texts.Count == 0;

    /// <summary>
    /// Returns the text written in the given language.
    /// </summary>
    /// <param name="language">The identifier of the language, like <c>zh-Hans</c>.</param>
    /// <returns>The text, or null when it is unknown.</returns>
    public string? Get(string language)
    {
        if (string.IsNullOrWhiteSpace(language))
            return null;

        if (Texts.TryGetValue(language, out var text))
            return text;

        // The language of the user may be more specific than the translation,
        // so the language without its region is tried as well.
        var separator = language.IndexOfAny(new[] { '-', '_' });
        if (separator > 0 && Texts.TryGetValue(language[..separator], out var shortText))
            return shortText;

        return null;
    }

    /// <summary>
    /// Returns the text shown to the user.
    /// </summary>
    /// <param name="language">The identifier of the language of the user.</param>
    /// <returns>The text in the language of the user, or the default text.</returns>
    public string GetDisplayText(string? language)
    {
        var text = Get(language ?? string.Empty);
        if (!string.IsNullOrEmpty(text))
            return text;

        return Texts.TryGetValue(DefaultKey, out var fallback) ? fallback : string.Empty;
    }

    /// <summary>
    /// Reads a text from a JSON value.
    /// </summary>
    /// <param name="value">The JSON value.</param>
    /// <returns>The text, or null when the value is not a text.</returns>
    public static LocalizedText? FromJson(JsonNode? value)
    {
        if (value is JsonObject objectValue)
        {
            var texts = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
            foreach (var (key, child) in objectValue)
            {
                if (child is JsonValue text && text.TryGetValue<string>(out var content))
                    texts[key] = content;
            }

            return new LocalizedText(texts);
        }

        if (value is JsonValue scalar && scalar.TryGetValue<string>(out var single))
            return new LocalizedText(new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase)
            {
                [DefaultKey] = single
            });

        return null;
    }

    /// <summary>
    /// Writes the text into a JSON value.
    /// </summary>
    /// <returns>The JSON value holding the text.</returns>
    public JsonNode ToJson()
    {
        if (Texts.Count == 1 && Texts.TryGetValue(DefaultKey, out var single))
            return JsonValue.Create(single);

        var result = new JsonObject();
        foreach (var (key, text) in Texts)
            result[key] = text;

        return result;
    }
}

/// <summary>
/// The author of a theme pack or of one of its themes.
/// </summary>
public sealed class ThemePackAuthor
{
    /// <summary>
    /// The name of the author.
    /// </summary>
    public LocalizedText? Name { get; init; }

    /// <summary>
    /// The address of the home page of the author.
    /// </summary>
    public string? Url { get; init; }
}

/// <summary>
/// The background of a theme.
/// </summary>
public sealed class ThemePackBackground
{
    /// <summary>
    /// The kind of the background: <c>builtin</c>, <c>image</c>, <c>network</c>,
    /// <c>color</c> or <c>none</c>.
    /// </summary>
    public string Type { get; init; } = "none";

    /// <summary>
    /// The identifier of the background: the identifier of a built-in wallpaper
    /// or the address of an image.
    /// </summary>
    public string? Id { get; init; }

    /// <summary>
    /// The opacity of the background, between 0 and 1.
    /// </summary>
    public double? Opacity { get; init; }
}

/// <summary>
/// The title bar of a theme.
/// </summary>
public sealed class ThemePackTitleBar
{
    /// <summary>
    /// Whether the title bar of the launcher is transparent.
    /// </summary>
    public bool? Transparent { get; init; }
}

/// <summary>
/// One theme of a theme pack, holding the appearance of the launcher.
/// </summary>
public sealed class ThemePackTheme
{
    /// <summary>
    /// The identifier of the theme inside its pack.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The name of the theme shown to the user.
    /// </summary>
    public LocalizedText? Name { get; init; }

    /// <summary>
    /// The description of the theme.
    /// </summary>
    public LocalizedText? Description { get; init; }

    /// <summary>
    /// The authors of the theme.
    /// </summary>
    public IReadOnlyList<ThemePackAuthor> Authors { get; init; } = Array.Empty<ThemePackAuthor>();

    /// <summary>
    /// The path of the icon of the theme, relative to the pack.
    /// </summary>
    public string? Icon { get; init; }

    /// <summary>
    /// The seed color of the theme.
    /// </summary>
    public string? Color { get; init; }

    /// <summary>
    /// The brightness of the theme: <c>light</c> or <c>dark</c>.
    /// </summary>
    public string? Brightness { get; init; }

    /// <summary>
    /// The color style of the theme.
    /// </summary>
    public string? ColorStyle { get; init; }

    /// <summary>
    /// The contrast level of the theme.
    /// </summary>
    public string? Contrast { get; init; }

    /// <summary>
    /// The background of the theme.
    /// </summary>
    public ThemePackBackground? Background { get; init; }

    /// <summary>
    /// The title bar of the theme.
    /// </summary>
    public ThemePackTitleBar? TitleBar { get; init; }

    /// <summary>
    /// Whether the window of the launcher is transparent.
    /// </summary>
    public bool? WindowTransparent { get; init; }

    /// <summary>
    /// The conditional patches applied to the theme.
    /// </summary>
    public IReadOnlyList<ThemePackOverride> Overrides { get; init; } = Array.Empty<ThemePackOverride>();
}

/// <summary>
/// A conditional patch applied to a theme of a theme pack.
/// </summary>
public sealed class ThemePackOverride
{
    /// <summary>
    /// The condition telling when the patch is applied.
    /// </summary>
    public ThemePackCondition? Condition { get; init; }

    /// <summary>
    /// The values of the patch.
    /// </summary>
    public ThemePackTheme? Appearance { get; init; }
}

/// <summary>
/// The condition telling when a patch of a theme is applied.
/// </summary>
public sealed class ThemePackCondition
{
    /// <summary>
    /// The name of the property the condition applies to.
    /// </summary>
    public string Property { get; init; } = string.Empty;

    /// <summary>
    /// The value the property must have.
    /// </summary>
    public string? Value { get; init; }

    /// <summary>
    /// Whether the condition is met.
    /// </summary>
    /// <param name="context">The values of the properties of the launcher.</param>
    /// <returns>Whether the patch must be applied.</returns>
    public bool IsMet(IReadOnlyDictionary<string, string> context)
    {
        if (context is null)
            return false;

        if (!context.TryGetValue(Property, out var actual))
            return false;

        if (Value is null)
            return string.IsNullOrEmpty(actual);

        return string.Equals(actual, Value, StringComparison.OrdinalIgnoreCase);
    }
}

/// <summary>
/// The manifest of a theme pack, like the manifest of the theme packs of Java
/// HMCL.
/// </summary>
public sealed class ThemePackManifest
{
    /// <summary>
    /// The address of the schema of the manifests.
    /// </summary>
    public const string SchemaUrl = "https://schemas.glavo.site/hmcl/theme-pack/1.0.0";

    /// <summary>
    /// The name of the file holding the manifest of a theme pack.
    /// </summary>
    public const string ManifestFileName = "manifest.json";

    /// <summary>
    /// The identifier of the theme pack.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The version of the theme pack.
    /// </summary>
    public string Version { get; init; } = "1.0.0";

    /// <summary>
    /// The name of the theme pack shown to the user.
    /// </summary>
    public LocalizedText? Name { get; init; }

    /// <summary>
    /// The description of the theme pack.
    /// </summary>
    public LocalizedText? Description { get; init; }

    /// <summary>
    /// The authors of the theme pack.
    /// </summary>
    public IReadOnlyList<ThemePackAuthor> Authors { get; init; } = Array.Empty<ThemePackAuthor>();

    /// <summary>
    /// The themes of the theme pack.
    /// </summary>
    public IReadOnlyList<ThemePackTheme> Themes { get; init; } = Array.Empty<ThemePackTheme>();

    /// <summary>
    /// Reads a manifest from a JSON value.
    /// </summary>
    /// <param name="value">The JSON value holding the manifest.</param>
    /// <returns>The manifest, or null when the value does not hold one.</returns>
    public static ThemePackManifest? FromJson(JsonNode? value)
    {
        if (value is not JsonObject root)
            return null;

        var themes = new List<ThemePackTheme>();
        if (root["themes"] is JsonArray themeArray)
        {
            foreach (var item in themeArray)
            {
                var theme = ReadTheme(item);
                if (theme is not null)
                    themes.Add(theme);
            }
        }

        return new ThemePackManifest
        {
            Id = root["id"]?.GetValue<string>() ?? string.Empty,
            Version = root["version"]?.GetValue<string>() ?? "1.0.0",
            Name = LocalizedText.FromJson(root["name"]),
            Description = LocalizedText.FromJson(root["description"]),
            Authors = ReadAuthors(root["authors"]),
            Themes = themes
        };
    }

    /// <summary>
    /// Writes the manifest into a JSON value.
    /// </summary>
    /// <returns>The JSON value holding the manifest.</returns>
    public JsonObject ToJson()
    {
        var root = new JsonObject
        {
            ["$schema"] = SchemaUrl,
            ["id"] = Id,
            ["version"] = Version
        };

        if (Name is not null)
            root["name"] = Name.ToJson();

        if (Description is not null)
            root["description"] = Description.ToJson();

        if (Authors.Count > 0)
        {
            var authors = new JsonArray();
            foreach (var author in Authors)
            {
                var entry = new JsonObject();
                if (author.Name is not null)
                    entry["name"] = author.Name.ToJson();
                if (!string.IsNullOrEmpty(author.Url))
                    entry["url"] = author.Url;

                authors.Add((System.Text.Json.Nodes.JsonNode)entry);
            }

            root["authors"] = authors;
        }

        if (Themes.Count > 0)
        {
            var themes = new JsonArray();
            foreach (var theme in Themes)
                themes.Add((System.Text.Json.Nodes.JsonNode)WriteTheme(theme));

            root["themes"] = themes;
        }

        return root;
    }

    /// <summary>
    /// Reads the authors of a theme pack.
    /// </summary>
    /// <param name="value">The JSON value holding the authors.</param>
    /// <returns>The authors.</returns>
    private static IReadOnlyList<ThemePackAuthor> ReadAuthors(JsonNode? value)
    {
        if (value is not JsonArray array)
            return Array.Empty<ThemePackAuthor>();

        var authors = new List<ThemePackAuthor>();
        foreach (var item in array)
        {
            if (item is not JsonObject entry)
                continue;

            authors.Add(new ThemePackAuthor
            {
                Name = LocalizedText.FromJson(entry["name"]),
                Url = entry["url"]?.GetValue<string>()
            });
        }

        return authors;
    }

    /// <summary>
    /// Reads a theme from a JSON value.
    /// </summary>
    /// <param name="value">The JSON value holding the theme.</param>
    /// <returns>The theme, or null when the value does not hold one.</returns>
    private static ThemePackTheme? ReadTheme(JsonNode? value)
    {
        if (value is not JsonObject root)
            return null;

        var background = root["background"] is JsonObject backgroundObject
            ? new ThemePackBackground
            {
                Type = backgroundObject["type"]?.GetValue<string>() ?? "none",
                Id = backgroundObject["id"]?.GetValue<string>(),
                Opacity = backgroundObject["opacity"]?.GetValue<double>()
            }
            : null;

        var titleBar = root["titleBar"] is JsonObject titleBarObject
            ? new ThemePackTitleBar
            {
                Transparent = titleBarObject["transparent"]?.GetValue<bool>()
            }
            : null;

        var overrides = new List<ThemePackOverride>();
        if (root["overrides"] is JsonArray overrideArray)
        {
            foreach (var item in overrideArray)
            {
                if (item is not JsonObject entry)
                    continue;

                var condition = entry["condition"] is JsonObject conditionObject
                    ? new ThemePackCondition
                    {
                        Property = conditionObject["property"]?.GetValue<string>() ?? string.Empty,
                        Value = conditionObject["value"]?.GetValue<string>()
                    }
                    : null;

                overrides.Add(new ThemePackOverride
                {
                    Condition = condition,
                    Appearance = ReadTheme(entry["appearance"] ?? entry["theme"])
                });
            }
        }

        return new ThemePackTheme
        {
            Id = root["id"]?.GetValue<string>() ?? string.Empty,
            Name = LocalizedText.FromJson(root["name"]),
            Description = LocalizedText.FromJson(root["description"]),
            Authors = ReadAuthors(root["authors"]),
            Icon = root["icon"]?.GetValue<string>(),
            Color = root["color"]?.GetValue<string>(),
            Brightness = root["brightness"]?.GetValue<string>(),
            ColorStyle = root["colorStyle"]?.GetValue<string>(),
            Contrast = root["contrast"]?.GetValue<string>(),
            Background = background,
            TitleBar = titleBar,
            WindowTransparent = root["windowTransparent"]?.GetValue<bool>(),
            Overrides = overrides
        };
    }

    /// <summary>
    /// Writes a theme into a JSON value.
    /// </summary>
    /// <param name="theme">The theme written.</param>
    /// <returns>The JSON value holding the theme.</returns>
    private static JsonObject WriteTheme(ThemePackTheme theme)
    {
        var root = new JsonObject
        {
            ["id"] = theme.Id
        };

        if (theme.Name is not null)
            root["name"] = theme.Name.ToJson();

        if (theme.Description is not null)
            root["description"] = theme.Description.ToJson();

        if (!string.IsNullOrEmpty(theme.Icon))
            root["icon"] = theme.Icon;

        if (!string.IsNullOrEmpty(theme.Color))
            root["color"] = theme.Color;

        if (!string.IsNullOrEmpty(theme.Brightness))
            root["brightness"] = theme.Brightness;

        if (!string.IsNullOrEmpty(theme.ColorStyle))
            root["colorStyle"] = theme.ColorStyle;

        if (!string.IsNullOrEmpty(theme.Contrast))
            root["contrast"] = theme.Contrast;

        if (theme.Background is not null)
        {
            var background = new JsonObject
            {
                ["type"] = theme.Background.Type
            };

            if (!string.IsNullOrEmpty(theme.Background.Id))
                background["id"] = theme.Background.Id;

            if (theme.Background.Opacity is { } opacity)
                background["opacity"] = opacity;

            root["background"] = background;
        }

        if (theme.TitleBar is not null)
        {
            var titleBar = new JsonObject();
            if (theme.TitleBar.Transparent is { } transparent)
                titleBar["transparent"] = transparent;

            root["titleBar"] = titleBar;
        }

        if (theme.WindowTransparent is { } windowTransparent)
            root["windowTransparent"] = windowTransparent;

        return root;
    }
}
