using System;

namespace HMCL.NET.Util;

/// <summary>
/// Looks up the localized texts of the launcher, like the <c>i18n</c> function
/// of Java HMCL.
/// </summary>
public static class Lang
{
    /// <summary>
    /// Returns the text of the given key in the current language.
    /// </summary>
    /// <param name="key">The key of the text.</param>
    /// <param name="fallback">The text used when the key is unknown.</param>
    /// <returns>The text of the key.</returns>
    public static string Get(string key, string? fallback = null)
        => LanguageManager.Get(key, fallback);
}
