using System;
using System.Globalization;

namespace HMCLCore.NET.Util;

/// <summary>
/// Helpers detecting the region of the machine the launcher runs on.
/// </summary>
public static class RegionUtils
{
    /// <summary>
    /// Whether the machine is located in the mainland of China.
    /// </summary>
    /// <remarks>
    /// The machine is considered to be in the mainland of China when its time
    /// zone is the time zone of China, or when it is in the UTC+8 time zone
    /// and its language is set to Chinese.
    /// </remarks>
    public static bool IsChinaMainland { get; } = CheckChinaMainland();

    /// <summary>
    /// Checks whether the machine is located in the mainland of China.
    /// </summary>
    /// <returns>Whether the machine is located in the mainland of China.</returns>
    private static bool CheckChinaMainland()
    {
        var timeZone = TimeZoneInfo.Local;

        if (timeZone.Id.Equals("China Standard Time", StringComparison.OrdinalIgnoreCase) ||
            timeZone.Id.Equals("Asia/Shanghai", StringComparison.OrdinalIgnoreCase))
            return true;

        if (timeZone.BaseUtcOffset != TimeSpan.FromHours(8))
            return false;

        try
        {
            var region = new RegionInfo(CultureInfo.CurrentCulture.Name);
            return region.TwoLetterISORegionName.Equals("CN", StringComparison.OrdinalIgnoreCase);
        }
        catch (ArgumentException)
        {
            // The culture does not know its region; the machine is not
            // considered to be in the mainland of China.
            return false;
        }
    }
}
