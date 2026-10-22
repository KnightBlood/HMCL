using System;
using Avalonia;
using Avalonia.Media;
using Avalonia.Styling;

namespace HMCL.NET.Util;

/// <summary>
/// Applies the appearance settings (theme, font) to the running Avalonia
/// application. Mirrors the parts of Java HMCL's <c>PersonalizationPage</c>
/// that can be honoured by the .NET UI layer.
/// </summary>
public static class AppearanceManager
{
    /// <summary>
    /// The resource key holding the interface font family.
    /// </summary>
    public const string FontFamilyResourceKey = "HMCL.FontFamily";

    /// <summary>
    /// The resource key holding the interface font size.
    /// </summary>
    public const string FontSizeResourceKey = "HMCL.FontSize";

    /// <summary>
    /// The theme name that follows the operating system setting.
    /// </summary>
    public static readonly string SystemTheme = Lang.Get("settings.launcher.brightness.auto", "跟随系统");

    /// <summary>
    /// The name of the light theme.
    /// </summary>
    public static readonly string LightTheme = Lang.Get("settings.launcher.brightness.light", "浅色");

    /// <summary>
    /// The name of the dark theme.
    /// </summary>
    public static readonly string DarkTheme = Lang.Get("settings.launcher.brightness.dark", "深色");

    /// <summary>
    /// The theme names offered in the settings UI.
    /// </summary>
    public static readonly string[] Themes = { SystemTheme, LightTheme, DarkTheme };

    /// <summary>
    /// The identifier of the theme that follows the operating system setting.
    /// </summary>
    public const string SystemThemeId = "system";

    /// <summary>
    /// The identifier of the light theme.
    /// </summary>
    public const string LightThemeId = "light";

    /// <summary>
    /// The identifier of the dark theme.
    /// </summary>
    public const string DarkThemeId = "dark";

    /// <summary>
    /// Returns the identifier of the theme shown with the given name.
    /// </summary>
    /// <param name="name">The name of the theme shown to the user.</param>
    /// <returns>
    /// The identifier of the theme, which is stored in the settings so that the
    /// language of the user does not change it.
    /// </returns>
    public static string GetThemeId(string? name)
    {
        if (string.IsNullOrWhiteSpace(name))
            return SystemThemeId;

        var trimmed = name.Trim();

        if (string.Equals(trimmed, LightThemeId, StringComparison.OrdinalIgnoreCase) ||
            string.Equals(trimmed, LightTheme, StringComparison.OrdinalIgnoreCase))
        {
            return LightThemeId;
        }

        if (string.Equals(trimmed, DarkThemeId, StringComparison.OrdinalIgnoreCase) ||
            string.Equals(trimmed, DarkTheme, StringComparison.OrdinalIgnoreCase))
        {
            return DarkThemeId;
        }

        return SystemThemeId;
    }

    /// <summary>
    /// Returns the name of the theme with the given identifier, shown to the
    /// user.
    /// </summary>
    /// <param name="id">The identifier of the theme.</param>
    /// <returns>The name of the theme.</returns>
    public static string GetThemeName(string? id)
    {
        if (string.IsNullOrWhiteSpace(id))
            return SystemTheme;

        return id.Trim().ToLowerInvariant() switch
        {
            LightThemeId => LightTheme,
            DarkThemeId => DarkTheme,
            _ => SystemTheme
        };
    }

    /// <summary>
    /// The resource key holding the accent color of the launcher.
    /// </summary>
    public const string AccentColorResourceKey = "SystemAccentColor";

    /// <summary>
    /// Whether the animations of the user interface are disabled.
    /// </summary>
    public static bool AnimationsDisabled { get; private set; }

    /// <summary>
    /// Applies the theme, the font and the accent color to the application.
    /// </summary>
    /// <param name="theme">The theme name; unknown names follow the system theme.</param>
    /// <param name="fontFamily">The font family, or null to use the system font.</param>
    /// <param name="fontSize">The base font size in points.</param>
    /// <param name="accentColor">
    /// The custom accent color of the launcher, in the "#RRGGBB" format; it is
    /// empty when the color of the system is used.
    /// </param>
    /// <param name="colorStyle">The color style used to derive the colors.</param>
    /// <param name="contrast">The contrast level of the colors.</param>
    /// <param name="disableAnimations">Whether the animations are disabled.</param>
    /// <param name="textRenderingMode">The way the text is rendered.</param>
    public static void Apply(
        string? theme,
        string? fontFamily,
        double fontSize,
        string? accentColor = null,
        HMCLCore.NET.Setting.ThemeColorStyle colorStyle = HMCLCore.NET.Setting.ThemeColorStyle.Default,
        HMCLCore.NET.Setting.ThemeContrast contrast = HMCLCore.NET.Setting.ThemeContrast.Default,
        bool disableAnimations = false,
        string? textRenderingMode = null)
    {
        var app = Application.Current;
        if (app == null)
            return;

        try
        {
            var themeId = GetThemeId(theme);

            app.RequestedThemeVariant = themeId == LightThemeId
                ? ThemeVariant.Light
                : themeId == DarkThemeId
                    ? ThemeVariant.Dark
                    : ThemeVariant.Default;

            app.Resources[FontFamilyResourceKey] = string.IsNullOrWhiteSpace(fontFamily)
                ? FontFamily.Default
                : new FontFamily(fontFamily);

            app.Resources[FontSizeResourceKey] = fontSize > 0 ? fontSize : 13d;

            AnimationsDisabled = disableAnimations;

            ApplyTextRenderingMode(textRenderingMode);
            ApplyAccentColors(accentColor, colorStyle, contrast);
        }
        catch (Exception)
        {
            // Theming is best effort; a failure must not break the launcher.
        }
    }

    /// <summary>
    /// Applies the colors derived from the seed color to the application.
    /// </summary>
    /// <param name="accentColor">
    /// The custom accent color of the launcher, in the "#RRGGBB" format; it is
    /// empty when the color of the system is used.
    /// </param>
    /// <param name="colorStyle">The color style used to derive the colors.</param>
    /// <param name="contrast">The contrast level of the colors.</param>
    private static void ApplyAccentColors(
        string? accentColor,
        HMCLCore.NET.Setting.ThemeColorStyle colorStyle,
        HMCLCore.NET.Setting.ThemeContrast contrast)
    {
        var app = Application.Current;
        if (app is null)
            return;

        if (string.IsNullOrWhiteSpace(accentColor) ||
            !HMCLCore.NET.Setting.ThemeColorScheme.TryParseColor(accentColor, out _))
        {
            // The color of the operating system is used.
            app.Resources.Remove(AccentColorResourceKey);
            foreach (var key in AccentColorKeys)
                app.Resources.Remove(key);

            return;
        }

        var colors = HMCLCore.NET.Setting.ThemeColorScheme.Compute(accentColor, colorStyle, contrast);

        foreach (var (key, value) in colors)
        {
            var color = Color.FromArgb(
                (byte)(value >> 24), (byte)(value >> 16), (byte)(value >> 8), (byte)value);

            if (string.Equals(key, "AccentBase", StringComparison.Ordinal))
                app.Resources[AccentColorResourceKey] = color;
            else
                app.Resources[key] = color;
        }
    }

    /// <summary>
    /// The resource keys holding the shades of the accent color.
    /// </summary>
    private static readonly string[] AccentColorKeys =
    {
        "AccentLight1", "AccentLight2", "AccentLight3",
        "AccentDark1", "AccentDark2", "AccentDark3",
        "Secondary", "Tertiary", "Neutral"
    };

    /// <summary>
    /// Applies the way the text of the user interface is rendered.
    /// </summary>
    /// <param name="textRenderingMode">
    /// The name of the mode: <c>default</c>, <c>antialias</c>,
    /// <c>subpixel</c> or <c>alias</c>.
    /// </param>
    private static void ApplyTextRenderingMode(string? textRenderingMode)
    {
        CurrentTextRenderingMode = textRenderingMode?.Trim().ToLowerInvariant() switch
        {
            "antialias" => TextRenderingMode.Antialias,
            "subpixel" => TextRenderingMode.SubpixelAntialias,
            "alias" => TextRenderingMode.Alias,
            _ => (TextRenderingMode?)null
        };
    }

    /// <summary>
    /// The way the text of the user interface is rendered, or null when the
    /// default of the platform is used.
    /// </summary>
    public static TextRenderingMode? CurrentTextRenderingMode { get; private set; }

    /// <summary>
    /// Applies the way the text is rendered to a visual and to everything it
    /// contains.
    /// </summary>
    /// <param name="root">The root of the visual tree of the launcher.</param>
    public static void ApplyTextRenderingMode(Avalonia.Visual root)
    {
        if (CurrentTextRenderingMode is { } mode)
            TextOptions.SetTextOptions(root, new TextOptions { TextRenderingMode = mode });
    }

    /// <summary>
    /// Applies the theme variant of the launcher to the application, so that
    /// the controls created afterwards use the colors of the theme.
    /// </summary>
    /// <param name="theme">The identifier or the name of the theme.</param>
    public static void ApplyThemeVariant(string? theme)
    {
        var app = Application.Current;
        if (app is null)
            return;

        try
        {
            var themeId = GetThemeId(theme);

            app.RequestedThemeVariant = themeId == LightThemeId
                ? ThemeVariant.Light
                : themeId == DarkThemeId
                    ? ThemeVariant.Dark
                    : ThemeVariant.Default;
        }
        catch (Exception)
        {
            // Theming is best effort; a failure must not break the launcher.
        }
    }

    /// <summary>
    /// Converts a theme name into the matching Avalonia theme variant.
    /// </summary>
    /// <param name="theme">The theme name.</param>
    /// <returns>The theme variant, or null to follow the system theme.</returns>
    public static ThemeVariant? GetThemeVariant(string? theme)
    {
        return GetThemeId(theme) switch
        {
            LightThemeId => ThemeVariant.Light,
            DarkThemeId => ThemeVariant.Dark,
            _ => null
        };
    }
}
