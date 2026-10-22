using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;

namespace HMCLCore.NET.Setting;

/// <summary>
/// The color style used to derive the colors of the launcher from the seed
/// color, like the MonetFX color styles of Java HMCL.
/// </summary>
public enum ThemeColorStyle
{
    /// <summary>
    /// The default color style of the operating system.
    /// </summary>
    Default,

    /// <summary>
    /// The colors keep the hue of the seed color.
    /// </summary>
    Fidelity,

    /// <summary>
    /// The colors are toned down, keeping the hue of the seed color.
    /// </summary>
    TonalSpot,

    /// <summary>
    /// The colors are vivid and bright.
    /// </summary>
    Vibrant,

    /// <summary>
    /// The colors are neutral, without a strong hue.
    /// </summary>
    Neutral,

    /// <summary>
    /// The colors are bright and playful.
    /// </summary>
    FruitSalad,

    /// <summary>
    /// The colors use the whole range of hues.
    /// </summary>
    Rainbow
}

/// <summary>
/// The contrast level of the colors of the launcher.
/// </summary>
public enum ThemeContrast
{
    /// <summary>
    /// The default contrast level of the operating system.
    /// </summary>
    Default,

    /// <summary>
    /// A low contrast, which is easier on the eyes.
    /// </summary>
    Low,

    /// <summary>
    /// A medium contrast.
    /// </summary>
    Medium,

    /// <summary>
    /// A high contrast, which is easier to read.
    /// </summary>
    High
}

/// <summary>
/// The built-in wallpapers of the launcher, like the built-in backgrounds of
/// Java HMCL.
/// </summary>
public enum BuiltinWallpaper
{
    /// <summary>
    /// The wallpaper of the launcher from 2021-08-26.
    /// </summary>
    Wallpaper20210826,

    /// <summary>
    /// The wallpaper of the launcher from 2016-02-25.
    /// </summary>
    Wallpaper20160225,

    /// <summary>
    /// The wallpaper of the launcher from 2015-06-22.
    /// </summary>
    Wallpaper20150622
}

/// <summary>
/// The colors of the launcher derived from a seed color, a color style and a
/// contrast level.
/// </summary>
/// <remarks>
/// The colors are computed like the ones of the design system of the operating
/// system: the seed color provides the hue, and the style and the contrast
/// change its saturation and its lightness.
/// </remarks>
public static class ThemeColorScheme
{
    /// <summary>
    /// The seed color used when the user does not choose one.
    /// </summary>
    public const string DefaultSeedColor = "#3F6AA2";

    /// <summary>
    /// Parses a color written in the "#RRGGBB" or "#AARRGGBB" format.
    /// </summary>
    /// <param name="value">The text of the color.</param>
    /// <param name="color">The parsed color.</param>
    /// <returns>Whether the color was parsed.</returns>
    public static bool TryParseColor(string? value, out uint color)
    {
        color = 0;
        if (string.IsNullOrWhiteSpace(value))
            return false;

        var text = value.Trim();
        if (text.StartsWith('#'))
            text = text[1..];

        if (text.Length == 6)
            text = "FF" + text;

        return text.Length == 8 &&
               uint.TryParse(text, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out color);
    }

    /// <summary>
    /// Returns the identifier of a built-in wallpaper.
    /// </summary>
    /// <param name="wallpaper">The wallpaper.</param>
    /// <returns>The identifier of the wallpaper.</returns>
    public static string GetWallpaperId(BuiltinWallpaper wallpaper) => wallpaper switch
    {
        BuiltinWallpaper.Wallpaper20160225 => "2016-02-25",
        BuiltinWallpaper.Wallpaper20150622 => "2015-06-22",
        _ => "2021-08-26"
    };

    /// <summary>
    /// Returns the built-in wallpaper with the given identifier.
    /// </summary>
    /// <param name="id">The identifier of the wallpaper.</param>
    /// <returns>The wallpaper, or null when the identifier is unknown.</returns>
    public static BuiltinWallpaper? ParseWallpaperId(string? id)
    {
        if (string.IsNullOrWhiteSpace(id))
            return null;

        return id.Trim() switch
        {
            "2016-02-25" => BuiltinWallpaper.Wallpaper20160225,
            "2015-06-22" => BuiltinWallpaper.Wallpaper20150622,
            "2021-08-26" => BuiltinWallpaper.Wallpaper20210826,
            _ => null
        };
    }

    /// <summary>
    /// Returns the seed color of a built-in wallpaper.
    /// </summary>
    /// <param name="wallpaper">The wallpaper.</param>
    /// <returns>The color of the wallpaper, in the "#RRGGBB" format.</returns>
    public static string GetWallpaperColor(BuiltinWallpaper wallpaper) => wallpaper switch
    {
        BuiltinWallpaper.Wallpaper20160225 => "#354264",
        BuiltinWallpaper.Wallpaper20150622 => "#FBC578",
        _ => "#3F6AA2"
    };

    /// <summary>
    /// Returns the name of the file holding a built-in wallpaper.
    /// </summary>
    /// <param name="wallpaper">The wallpaper.</param>
    /// <returns>The name of the file.</returns>
    public static string GetWallpaperFileName(BuiltinWallpaper wallpaper)
        => $"{GetWallpaperId(wallpaper)}.jpg";

    /// <summary>
    /// Returns the hue of a color, in degrees.
    /// </summary>
    /// <param name="color">The color, in the 0xAARRGGBB format.</param>
    /// <returns>The hue of the color.</returns>
    public static double GetHue(uint color)
    {
        var red = ((color >> 16) & 0xFF) / 255.0;
        var green = ((color >> 8) & 0xFF) / 255.0;
        var blue = (color & 0xFF) / 255.0;

        var max = Math.Max(red, Math.Max(green, blue));
        var min = Math.Min(red, Math.Min(green, blue));
        var delta = max - min;

        if (delta <= 0)
            return 0;

        double hue;
        if (max == red)
            hue = 60 * (((green - blue) / delta) % 6);
        else if (max == green)
            hue = 60 * (((blue - red) / delta) + 2);
        else
            hue = 60 * (((red - green) / delta) + 4);

        return (hue + 360) % 360;
    }

    /// <summary>
    /// Returns the saturation of a color, in the range from 0 to 1.
    /// </summary>
    /// <param name="color">The color, in the 0xAARRGGBB format.</param>
    /// <returns>The saturation of the color.</returns>
    public static double GetSaturation(uint color)
    {
        var red = ((color >> 16) & 0xFF) / 255.0;
        var green = ((color >> 8) & 0xFF) / 255.0;
        var blue = (color & 0xFF) / 255.0;

        var max = Math.Max(red, Math.Max(green, blue));
        var min = Math.Min(red, Math.Min(green, blue));

        return max <= 0 ? 0 : (max - min) / max;
    }

    /// <summary>
    /// Returns the lightness of a color, in the range from 0 to 1.
    /// </summary>
    /// <param name="color">The color, in the 0xAARRGGBB format.</param>
    /// <returns>The lightness of the color.</returns>
    public static double GetLightness(uint color)
    {
        var red = ((color >> 16) & 0xFF) / 255.0;
        var green = ((color >> 8) & 0xFF) / 255.0;
        var blue = (color & 0xFF) / 255.0;

        return (Math.Max(red, Math.Max(green, blue)) + Math.Min(red, Math.Min(green, blue))) / 2;
    }

    /// <summary>
    /// Builds a color from its hue, its saturation and its lightness.
    /// </summary>
    /// <param name="hue">The hue of the color, in degrees.</param>
    /// <param name="saturation">The saturation of the color, from 0 to 1.</param>
    /// <param name="lightness">The lightness of the color, from 0 to 1.</param>
    /// <returns>The color, in the 0xAARRGGBB format.</returns>
    public static uint FromHsl(double hue, double saturation, double lightness)
    {
        hue = (hue % 360 + 360) % 360;
        saturation = Math.Clamp(saturation, 0, 1);
        lightness = Math.Clamp(lightness, 0, 1);

        var chroma = (1 - Math.Abs(2 * lightness - 1)) * saturation;
        var secondary = chroma * (1 - Math.Abs((hue / 60) % 2 - 1));
        var match = lightness - chroma / 2;

        double red;
        double green;
        double blue;

        if (hue < 60)
        {
            red = chroma;
            green = secondary;
            blue = 0;
        }
        else if (hue < 120)
        {
            red = secondary;
            green = chroma;
            blue = 0;
        }
        else if (hue < 180)
        {
            red = 0;
            green = chroma;
            blue = secondary;
        }
        else if (hue < 240)
        {
            red = 0;
            green = secondary;
            blue = chroma;
        }
        else if (hue < 300)
        {
            red = secondary;
            green = 0;
            blue = chroma;
        }
        else
        {
            red = chroma;
            green = 0;
            blue = secondary;
        }

        return 0xFF000000 |
               (uint)Math.Round((red + match) * 255) << 16 |
               (uint)Math.Round((green + match) * 255) << 8 |
               (uint)Math.Round((blue + match) * 255);
    }

    /// <summary>
    /// Returns the saturation and the lightness used by a color style.
    /// </summary>
    /// <param name="style">The color style.</param>
    /// <param name="saturation">The saturation of the colors.</param>
    /// <param name="lightness">The lightness of the colors.</param>
    public static void GetStyleValues(ThemeColorStyle style, out double saturation, out double lightness)
    {
        switch (style)
        {
            case ThemeColorStyle.TonalSpot:
                saturation = 0.6;
                lightness = 0.45;
                break;

            case ThemeColorStyle.Vibrant:
                saturation = 0.9;
                lightness = 0.5;
                break;

            case ThemeColorStyle.Neutral:
                saturation = 0.15;
                lightness = 0.45;
                break;

            case ThemeColorStyle.FruitSalad:
                saturation = 0.85;
                lightness = 0.55;
                break;

            case ThemeColorStyle.Rainbow:
                saturation = 0.8;
                lightness = 0.5;
                break;

            case ThemeColorStyle.Fidelity:
            case ThemeColorStyle.Default:
            default:
                saturation = 0.7;
                lightness = 0.45;
                break;
        }
    }

    /// <summary>
    /// Computes the colors of the launcher from a seed color.
    /// </summary>
    /// <param name="seedColor">The seed color, in the "#RRGGBB" format.</param>
    /// <param name="style">The color style.</param>
    /// <param name="contrast">The contrast level.</param>
    /// <returns>The colors of the launcher.</returns>
    public static IReadOnlyDictionary<string, uint> Compute(
        string? seedColor, ThemeColorStyle style, ThemeContrast contrast)
    {
        if (!TryParseColor(seedColor, out var color))
            TryParseColor(DefaultSeedColor, out color);

        var hue = GetHue(color);
        var saturation = GetSaturation(color);
        var lightness = GetLightness(color);

        GetStyleValues(style, out var styleSaturation, out var styleLightness);

        // The style replaces the saturation and the lightness of the seed
        // color, but keeps its hue.
        saturation = styleSaturation * (0.6 + 0.4 * saturation);
        lightness = styleLightness;

        var contrastSaturation = contrast switch
        {
            ThemeContrast.Low => 0.85,
            ThemeContrast.Medium => 1.0,
            ThemeContrast.High => 1.15,
            _ => 1.0
        };

        var contrastLightness = contrast switch
        {
            ThemeContrast.Low => 0.08,
            ThemeContrast.Medium => 0.0,
            ThemeContrast.High => -0.08,
            _ => 0.0
        };

        saturation = Math.Clamp(saturation * contrastSaturation, 0, 1);
        lightness = Math.Clamp(lightness + contrastLightness, 0.05, 0.95);

        var colors = new Dictionary<string, uint>(StringComparer.Ordinal)
        {
            ["AccentBase"] = FromHsl(hue, saturation, lightness),
            ["AccentLight1"] = FromHsl(hue, saturation * 0.85, Math.Min(lightness + 0.12, 0.95)),
            ["AccentLight2"] = FromHsl(hue, saturation * 0.7, Math.Min(lightness + 0.24, 0.97)),
            ["AccentLight3"] = FromHsl(hue, saturation * 0.55, Math.Min(lightness + 0.36, 0.98)),
            ["AccentDark1"] = FromHsl(hue, saturation * 0.9, Math.Max(lightness - 0.12, 0.05)),
            ["AccentDark2"] = FromHsl(hue, saturation * 0.8, Math.Max(lightness - 0.24, 0.03)),
            ["AccentDark3"] = FromHsl(hue, saturation * 0.7, Math.Max(lightness - 0.36, 0.02)),
            ["Secondary"] = FromHsl(hue + 60, saturation * 0.6, lightness),
            ["Tertiary"] = FromHsl(hue + 120, saturation * 0.5, lightness),
            ["Neutral"] = FromHsl(hue, saturation * 0.1, 0.5)
        };

        return colors;
    }
}
