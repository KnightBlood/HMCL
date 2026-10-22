using System;
using System.Collections.Generic;
using System.Globalization;

namespace HMCLCore.NET.Util;

/// <summary>
/// April Fools' Day utilities, mirroring the Java HMCL implementation.
/// </summary>
/// <remarks>
/// The mode is enabled on the first of April in the regions that support it.
/// The user can disable it in the settings, and the mode can be forced on or
/// off with the <c>hmcl.april_fools</c> system property or the
/// <c>HMCL_APRIL_FOOLS</c> environment variable.
/// </remarks>
public static class AprilFools
{
    /// <summary>
    /// The regions whose users are offered the April Fools' mode.
    /// </summary>
    private static readonly HashSet<string> SupportedRegions = new(StringComparer.OrdinalIgnoreCase)
    {
        "CN", "TW", "HK", "MO", "JP", "KR", "VN", "SG", "MY",
        "ES", "DE", "FR", "GB", "RU", "UA", "US"
    };

    /// <summary>
    /// Whether the April Fools' mode is enabled.
    /// </summary>
    public static bool IsEnabled { get; private set; }

    /// <summary>
    /// Determines whether the April Fools' mode should be used and remembers
    /// the result.
    /// </summary>
    /// <param name="disabled">Whether the user disabled the mode in the settings.</param>
    /// <returns>Whether the mode is enabled.</returns>
    public static bool Initialize(bool disabled)
    {
        IsEnabled = ComputeEnabled(disabled);
        return IsEnabled;
    }

    /// <summary>
    /// Determines whether the April Fools' mode should be used.
    /// </summary>
    /// <param name="disabled">Whether the user disabled the mode in the settings.</param>
    /// <returns>Whether the mode is enabled.</returns>
    public static bool ComputeEnabled(bool disabled)
    {
        var value = Environment.GetEnvironmentVariable("HMCL_APRIL_FOOLS");

        if (string.Equals(value, "true", StringComparison.OrdinalIgnoreCase))
            return true;

        if (string.Equals(value, "false", StringComparison.OrdinalIgnoreCase))
            return false;

        if (disabled)
            return false;

        if (!SupportedRegions.Contains(GetCurrentRegion()))
            return false;

        var now = DateTime.Now;
        return now.Month == 4 && now.Day == 1;
    }

    /// <summary>
    /// Returns the country code of the current culture.
    /// </summary>
    /// <returns>The country code, or an empty string when it is unknown.</returns>
    private static string GetCurrentRegion()
    {
        var name = CultureInfo.CurrentCulture.Name;
        var index = name.IndexOf('-');
        return index >= 0 && index < name.Length - 1 ? name.Substring(index + 1) : string.Empty;
    }
}
