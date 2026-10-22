using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Text;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Styling;

namespace HMCL.NET.Util;

/// <summary>
/// Describes a language offered by the launcher.
/// </summary>
public sealed class LanguageInfo
{
    /// <summary>
    /// Creates the description of a language.
    /// </summary>
    /// <param name="id">The identifier of the language, for example <c>zh_CN</c>.</param>
    /// <param name="displayName">The name of the language shown in the user interface.</param>
    /// <param name="resourceName">The name of the embedded resource holding the texts.</param>
    /// <param name="localResourceName">
    /// The name of the embedded resource holding the texts of the launcher that
    /// Java HMCL does not know; when it is empty the English texts are used.
    /// </param>
    public LanguageInfo(string id, string displayName, string resourceName, string? localResourceName = null)
    {
        Id = id ?? throw new ArgumentNullException(nameof(id));
        DisplayName = displayName ?? throw new ArgumentNullException(nameof(displayName));
        ResourceName = resourceName ?? throw new ArgumentNullException(nameof(resourceName));
        LocalResourceName = localResourceName;
    }

    /// <summary>
    /// The identifier of the language, used to load its resources.
    /// </summary>
    public string Id { get; }

    /// <summary>
    /// The name of the language shown in the user interface.
    /// </summary>
    public string DisplayName { get; }

    /// <summary>
    /// The name of the embedded resource holding the texts of the language.
    /// </summary>
    public string ResourceName { get; }

    /// <summary>
    /// The name of the embedded resource holding the texts of the launcher that
    /// Java HMCL does not know.
    /// </summary>
    public string? LocalResourceName { get; }

    /// <inheritdoc/>
    public override string ToString() => DisplayName;
}

/// <summary>
/// Loads the localized resources of the launcher and applies them to the
/// resources of the running application.
/// </summary>
/// <remarks>
/// The texts of the languages are stored in the property files of Java HMCL,
/// so that both launchers show the same texts. The properties are loaded on
/// demand, so the language can be changed without restarting the launcher. The
/// keys managed by this class are replaced when the language changes, so the
/// resources of the other parts of the launcher are left untouched.
/// </remarks>
public static class LanguageManager
{
    /// <summary>
    /// The identifier of the Chinese (Simplified) language.
    /// </summary>
    public const string ChineseLanguage = "zh_CN";

    /// <summary>
    /// The identifier of the Chinese (Traditional) language.
    /// </summary>
    public const string ChineseTraditionalLanguage = "zh_TW";

    /// <summary>
    /// The identifier of the English language.
    /// </summary>
    public const string EnglishLanguage = "en";

    /// <summary>
    /// The identifier of the language used when no other language fits.
    /// </summary>
    public const string FallbackLanguage = EnglishLanguage;

    /// <summary>
    /// The keys of the application resources that are owned by the language
    /// resources.
    /// </summary>
    private static readonly HashSet<string> ManagedKeys = new();

    /// <summary>
    /// The languages offered by the launcher.
    /// </summary>
    public static IReadOnlyList<LanguageInfo> AvailableLanguages { get; } = new List<LanguageInfo>
    {
        new(ChineseLanguage, "简体中文", "HMCL.NET.Strings.lang.I18N_zh_Hans.properties",
            "HMCL.NET.Strings.lang.HMCL_zh_Hans.properties"),
        new(ChineseTraditionalLanguage, "繁體中文", "HMCL.NET.Strings.lang.I18N_zh_Hant.properties",
            "HMCL.NET.Strings.lang.HMCL_zh_Hant.properties"),
        new(EnglishLanguage, "English", "HMCL.NET.Strings.lang.I18N.properties",
            "HMCL.NET.Strings.lang.HMCL.properties"),
        new("ja", "日本語", "HMCL.NET.Strings.lang.I18N_ja.properties",
            "HMCL.NET.Strings.lang.HMCL_ja.properties"),
        new("ru", "Русский", "HMCL.NET.Strings.lang.I18N_ru.properties",
            "HMCL.NET.Strings.lang.HMCL_ru.properties"),
        new("uk", "Українська", "HMCL.NET.Strings.lang.I18N_uk.properties",
            "HMCL.NET.Strings.lang.HMCL_uk.properties"),
        new("de", "Deutsch", "HMCL.NET.Strings.lang.I18N_de.properties",
            "HMCL.NET.Strings.lang.HMCL_de.properties"),
        new("es", "Español", "HMCL.NET.Strings.lang.I18N_es.properties",
            "HMCL.NET.Strings.lang.HMCL_es.properties"),
        new("ar", "العربية", "HMCL.NET.Strings.lang.I18N_ar.properties",
            "HMCL.NET.Strings.lang.HMCL_ar.properties"),
        new("lzh", "文言", "HMCL.NET.Strings.lang.I18N_lzh.properties",
            "HMCL.NET.Strings.lang.HMCL_lzh.properties")
    };

    /// <summary>
    /// The name of the resource holding the English texts of the launcher.
    /// </summary>
    private const string EnglishLocalResourceName = "HMCL.NET.Strings.lang.HMCL.properties";

    /// <summary>
    /// The identifier of the language currently in use.
    /// </summary>
    public static string CurrentLanguage { get; private set; } = FallbackLanguage;

    /// <summary>
    /// Raised after the localized resources were replaced.
    /// </summary>
    public static event EventHandler<string>? LanguageChanged;

    /// <summary>
    /// Resolves the identifier of the language to use.
    /// </summary>
    /// <param name="languageId">
    /// The identifier stored in the settings; an empty value means that the
    /// language of the operating system is used.
    /// </param>
    /// <returns>The identifier of one of the <see cref="AvailableLanguages"/>.</returns>
    public static string Resolve(string? languageId)
    {
        if (!string.IsNullOrWhiteSpace(languageId))
        {
            var trimmed = languageId.Trim();
            foreach (var language in AvailableLanguages)
            {
                if (string.Equals(language.Id, trimmed, StringComparison.OrdinalIgnoreCase))
                    return language.Id;
            }
        }

        return ResolveSystemLanguage();
    }

    /// <summary>
    /// Returns the identifier of the language of the operating system.
    /// </summary>
    /// <returns>The identifier of one of the <see cref="AvailableLanguages"/>.</returns>
    public static string ResolveSystemLanguage()
    {
        for (var culture = CultureInfo.CurrentUICulture;
             culture is not null && !string.IsNullOrEmpty(culture.Name);
             culture = culture.Parent)
        {
            var name = culture.Name.Replace('-', '_');
            foreach (var language in AvailableLanguages)
            {
                if (string.Equals(language.Id, name, StringComparison.OrdinalIgnoreCase))
                    return language.Id;
            }

            // Every Chinese locale uses the simplified Chinese resources, except
            // the ones written in the traditional script.
            if (culture.Name.StartsWith("zh", StringComparison.OrdinalIgnoreCase))
            {
                return culture.Name.EndsWith("Hant", StringComparison.OrdinalIgnoreCase) ||
                       culture.Name.EndsWith("TW", StringComparison.OrdinalIgnoreCase) ||
                       culture.Name.EndsWith("HK", StringComparison.OrdinalIgnoreCase) ||
                       culture.Name.EndsWith("MO", StringComparison.OrdinalIgnoreCase)
                    ? ChineseTraditionalLanguage
                    : ChineseLanguage;
            }
        }

        return FallbackLanguage;
    }

    /// <summary>
    /// Loads the resources of the given language and applies them to the
    /// running application.
    /// </summary>
    /// <param name="languageId">The identifier of the language.</param>
    /// <returns>Whether the resources were loaded and applied.</returns>
    public static bool Apply(string? languageId)
    {
        var application = Application.Current;
        if (application is null)
            return false;

        var id = Resolve(languageId);
        Dictionary<string, string>? loaded = null;

        if (TryLoad(id, out var texts))
        {
            loaded = texts;
        }
        else
        {
            // The resources of the requested language could not be loaded; the
            // launcher falls back to another language instead of showing an
            // empty user interface.
            foreach (var fallback in AvailableLanguages)
            {
                if (string.Equals(fallback.Id, id, StringComparison.Ordinal))
                    continue;

                if (TryLoad(fallback.Id, out texts))
                {
                    loaded = texts;
                    id = fallback.Id;
                    break;
                }
            }
        }

        if (loaded is null)
            return false;

        var resources = application.Resources;
        foreach (var key in ManagedKeys)
            resources.Remove(key);
        ManagedKeys.Clear();

        foreach (var (key, value) in loaded)
        {
            resources[key] = value;
            ManagedKeys.Add(key);
        }

        CurrentLanguage = id;
        LanguageChanged?.Invoke(null, id);
        return true;
    }

    /// <summary>
    /// Returns the text of the given key in the current language.
    /// </summary>
    /// <param name="key">The key of the text.</param>
    /// <param name="fallback">The text used when the key is unknown.</param>
    /// <returns>The text of the key.</returns>
    public static string Get(string key, string? fallback = null)
    {
        if (Application.Current?.TryFindResource(key, ActualThemeVariantVariant, out var value) == true &&
            value is string text)
        {
            return text;
        }

        return fallback ?? key;
    }

    /// <summary>
    /// The theme used to look the resources up.
    /// </summary>
    private static ThemeVariant ActualThemeVariantVariant =>
        Application.Current?.ActualThemeVariant ?? ThemeVariant.Default;

    /// <summary>
    /// Loads the texts of a language.
    /// </summary>
    /// <param name="languageId">The identifier of the language.</param>
    /// <param name="texts">The loaded texts.</param>
    /// <returns>Whether the texts were loaded.</returns>
    private static bool TryLoad(string languageId, out Dictionary<string, string> texts)
    {
        texts = new Dictionary<string, string>(StringComparer.Ordinal);

        var language = AvailableLanguages.FirstOrDefault(item =>
            string.Equals(item.Id, languageId, StringComparison.OrdinalIgnoreCase));

        if (language is null)
            return false;

        try
        {
            using var stream = typeof(LanguageManager).GetTypeInfo().Assembly
                .GetManifestResourceStream(language.ResourceName);

            if (stream is null)
                return false;

            using (var reader = new StreamReader(stream, Encoding.UTF8))
                ParseProperties(reader, texts);

            // The texts of the launcher that Java HMCL does not know are loaded
            // from their own file; the English texts are used when the language
            // has none of its own.
            if (language.LocalResourceName != null)
            {
                using var localStream = typeof(LanguageManager).GetTypeInfo().Assembly
                    .GetManifestResourceStream(language.LocalResourceName);

                if (localStream != null)
                {
                    using var localReader = new StreamReader(localStream, Encoding.UTF8);
                    ParseProperties(localReader, texts);
                }
            }

            // The English texts fill the keys the language does not translate.
            if (!string.Equals(language.LocalResourceName, EnglishLocalResourceName, StringComparison.Ordinal))
            {
                using var englishStream = typeof(LanguageManager).GetTypeInfo().Assembly
                    .GetManifestResourceStream(EnglishLocalResourceName);

                if (englishStream != null)
                {
                    using var englishReader = new StreamReader(englishStream, Encoding.UTF8);
                    var english = new Dictionary<string, string>(StringComparer.Ordinal);
                    ParseProperties(englishReader, english);

                    foreach (var (key, value) in english)
                    {
                        if (!texts.ContainsKey(key))
                            texts[key] = value;
                    }
                }
            }

            return texts.Count > 0;
        }
        catch (Exception)
        {
            // The resources of the language are missing or malformed; the
            // launcher keeps the resources it already has.
            texts = new Dictionary<string, string>(StringComparer.Ordinal);
            return false;
        }
    }

    /// <summary>
    /// Parses a property file into the given texts.
    /// </summary>
    /// <param name="reader">The reader of the property file.</param>
    /// <param name="texts">The parsed texts.</param>
    private static void ParseProperties(TextReader reader, Dictionary<string, string> texts)
    {
        var current = new StringBuilder();

        while (reader.ReadLine() is { } line)
        {
            var trimmed = line.TrimStart();

            if (current.Length > 0)
            {
                // A line ending with a backslash continues the entry on the
                // next line, like the property files of Java HMCL.
                if (EndsWithContinuation(current))
                {
                    current.Length -= 1;
                    current.Append('\n').Append(trimmed);
                    continue;
                }

                // An entry may also be continued by a line starting with a
                // backslash.
                if (trimmed.StartsWith('\\'))
                {
                    current.Append('\n').Append(trimmed.TrimStart('\\'));
                    continue;
                }

                AddEntry(texts, current.ToString());
                current.Clear();
            }

            if (trimmed.Length == 0 || trimmed.StartsWith('#') || trimmed.StartsWith('!'))
                continue;

            current.Append(trimmed);
        }

        if (current.Length > 0)
            AddEntry(texts, current.ToString());
    }

    /// <summary>
    /// Returns whether the entry being parsed is continued on the next line.
    /// </summary>
    /// <param name="builder">The entry being parsed.</param>
    /// <returns>Whether the entry ends with an odd number of backslashes.</returns>
    private static bool EndsWithContinuation(StringBuilder builder)
    {
        var count = 0;
        for (var index = builder.Length - 1; index >= 0 && builder[index] == '\\'; index--)
            count++;

        return count % 2 == 1;
    }

    /// <summary>
    /// Adds one entry to the parsed texts.
    /// </summary>
    /// <param name="texts">The parsed texts.</param>
    /// <param name="entry">The entry to parse.</param>
    private static void AddEntry(Dictionary<string, string> texts, string entry)
    {
        var separator = entry.IndexOf('=');
        if (separator < 0)
            separator = entry.IndexOf(':');

        if (separator < 0)
            return;

        var key = entry[..separator].Trim();
        var value = entry[(separator + 1)..].Trim();

        if (key.Length == 0)
            return;

        texts[key] = Unescape(value);
    }

    /// <summary>
    /// Resolves the escape sequences of a property value.
    /// </summary>
    /// <param name="value">The value of the property.</param>
    /// <returns>The resolved value.</returns>
    private static string Unescape(string value)
    {
        var builder = new StringBuilder(value.Length);
        for (var i = 0; i < value.Length; i++)
        {
            if (value[i] != '\\' || i + 1 >= value.Length)
            {
                builder.Append(value[i]);
                continue;
            }

            i++;
            switch (value[i])
            {
                case 'n': builder.Append('\n'); break;
                case 't': builder.Append('\t'); break;
                case 'r': builder.Append('\r'); break;
                case 'u':
                    if (i + 4 < value.Length &&
                        int.TryParse(value.Substring(i + 1, 4), NumberStyles.HexNumber, null, out var code))
                    {
                        builder.Append((char)code);
                        i += 4;
                    }
                    else
                    {
                        builder.Append('u');
                    }
                    break;
                default: builder.Append(value[i]); break;
            }
        }

        return builder.ToString();
    }
}
