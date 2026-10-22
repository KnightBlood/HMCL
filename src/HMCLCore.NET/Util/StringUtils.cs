using System;
using System.Collections.Generic;
using System.Text;

namespace HMCLCore.NET.Util;

/// <summary>
/// Utilities for the texts shown by the launcher.
/// </summary>
public static class StringUtils
{
    /// <summary>
    /// Returns whether the given text is null, empty or made of whitespaces
    /// only.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <returns>Whether the text is blank.</returns>
    public static bool IsBlank(string? text) =>
        string.IsNullOrWhiteSpace(text);

    /// <summary>
    /// Returns whether the given text holds at least one character that is not
    /// a whitespace.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <returns>Whether the text is not blank.</returns>
    public static bool IsNotBlank(string? text) => !IsBlank(text);

    /// <summary>
    /// Replaces the sequences of whitespaces of the given text with a single
    /// space.
    /// </summary>
    /// <param name="text">The text to normalize.</param>
    /// <returns>The normalized text.</returns>
    public static string NormalizeWhitespaces(string text)
    {
        var builder = new StringBuilder(text.Length);
        var lastWasSpace = false;

        foreach (var character in text)
        {
            if (char.IsWhiteSpace(character))
            {
                if (!lastWasSpace)
                    builder.Append(' ');

                lastWasSpace = true;
            }
            else
            {
                builder.Append(character);
                lastWasSpace = false;
            }
        }

        return builder.ToString();
    }

    /// <summary>
    /// Converts the first character of the given text into upper case.
    /// </summary>
    /// <param name="text">The text to convert.</param>
    /// <returns>The converted text.</returns>
    public static string CapitalizeFirst(string text)
    {
        if (string.IsNullOrEmpty(text))
            return text;

        return char.ToUpperInvariant(text[0]) + text[1..];
    }

    /// <summary>
    /// Returns whether the given text holds a digit.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <returns>Whether the text holds a digit.</returns>
    public static bool ContainsDigit(string text)
    {
        foreach (var character in text)
        {
            if (char.IsDigit(character))
                return true;
        }

        return false;
    }

    /// <summary>
    /// Returns the part of the given text that is before the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <returns>The part before the delimiter, or the whole text when the
    /// delimiter is missing.</returns>
    public static string SubstringBefore(string text, string delimiter)
    {
        var index = text.IndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? text : text[..index];
    }

    /// <summary>
    /// Returns the part of the given text that is before the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <param name="missingDelimiterValue">The value returned when the
    /// delimiter is missing.</param>
    /// <returns>The part before the delimiter.</returns>
    public static string SubstringBefore(string text, string delimiter, string missingDelimiterValue)
    {
        var index = text.IndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? missingDelimiterValue : text[..index];
    }

    /// <summary>
    /// Returns the part of the given text that is before the last occurrence of
    /// the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <returns>The part before the delimiter, or the whole text when the
    /// delimiter is missing.</returns>
    public static string SubstringBeforeLast(string text, string delimiter)
    {
        var index = text.LastIndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? text : text[..index];
    }

    /// <summary>
    /// Returns the part of the given text that is after the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <returns>The part after the delimiter, or the whole text when the
    /// delimiter is missing.</returns>
    public static string SubstringAfter(string text, string delimiter)
    {
        var index = text.IndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? text : text[(index + delimiter.Length)..];
    }

    /// <summary>
    /// Returns the part of the given text that is after the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <param name="missingDelimiterValue">The value returned when the
    /// delimiter is missing.</param>
    /// <returns>The part after the delimiter.</returns>
    public static string SubstringAfter(string text, string delimiter, string missingDelimiterValue)
    {
        var index = text.IndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? missingDelimiterValue : text[(index + delimiter.Length)..];
    }

    /// <summary>
    /// Returns the part of the given text that is after the last occurrence of
    /// the given delimiter.
    /// </summary>
    /// <param name="text">The text to search in.</param>
    /// <param name="delimiter">The delimiter to look for.</param>
    /// <returns>The part after the delimiter, or the whole text when the
    /// delimiter is missing.</returns>
    public static string SubstringAfterLast(string text, string delimiter)
    {
        var index = text.LastIndexOf(delimiter, StringComparison.Ordinal);
        return index < 0 ? text : text[(index + delimiter.Length)..];
    }

    /// <summary>
    /// Returns whether the given text starts with the given prefix and ends
    /// with the given suffix.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <param name="prefix">The expected prefix.</param>
    /// <param name="suffix">The expected suffix.</param>
    /// <returns>Whether the text is surrounded by the prefix and the suffix.</returns>
    public static bool IsSurrounded(string text, string prefix, string suffix) =>
        text.StartsWith(prefix, StringComparison.Ordinal) &&
        text.EndsWith(suffix, StringComparison.Ordinal);

    /// <summary>
    /// Removes the given prefix and suffix from the given text, when they are
    /// present.
    /// </summary>
    /// <param name="text">The text to trim.</param>
    /// <param name="prefix">The prefix to remove.</param>
    /// <param name="suffix">The suffix to remove.</param>
    /// <returns>The trimmed text.</returns>
    public static string RemoveSurrounding(string text, string prefix, string suffix) =>
        IsSurrounded(text, prefix, suffix) ? text[prefix.Length..^suffix.Length] : text;

    /// <summary>
    /// Adds the given prefix to the given text, when it is missing.
    /// </summary>
    /// <param name="text">The text to prefix.</param>
    /// <param name="prefix">The prefix to add.</param>
    /// <returns>The prefixed text.</returns>
    public static string AddPrefix(string text, string prefix) =>
        text.StartsWith(prefix, StringComparison.Ordinal) ? text : prefix + text;

    /// <summary>
    /// Adds the given suffix to the given text, when it is missing.
    /// </summary>
    /// <param name="text">The text to suffix.</param>
    /// <param name="suffix">The suffix to add.</param>
    /// <returns>The suffixed text.</returns>
    public static string AddSuffix(string text, string suffix) =>
        text.EndsWith(suffix, StringComparison.Ordinal) ? text : text + suffix;

    /// <summary>
    /// Removes the given prefix from the given text, when it is present.
    /// </summary>
    /// <param name="text">The text to trim.</param>
    /// <param name="prefix">The prefix to remove.</param>
    /// <returns>The trimmed text.</returns>
    public static string RemovePrefix(string text, string prefix) =>
        text.StartsWith(prefix, StringComparison.Ordinal) ? text[prefix.Length..] : text;

    /// <summary>
    /// Removes the given suffix from the given text, when it is present.
    /// </summary>
    /// <param name="text">The text to trim.</param>
    /// <param name="suffix">The suffix to remove.</param>
    /// <returns>The trimmed text.</returns>
    public static string RemoveSuffix(string text, string suffix) =>
        text.EndsWith(suffix, StringComparison.Ordinal) ? text[..^suffix.Length] : text;

    /// <summary>
    /// Returns whether the given text starts with the given prefix, ignoring
    /// the case of the letters.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <param name="prefix">The expected prefix.</param>
    /// <returns>Whether the text starts with the prefix.</returns>
    public static bool StartsWithIgnoreCase(string text, string prefix) =>
        text.StartsWith(prefix, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Returns whether the given text ends with the given suffix, ignoring the
    /// case of the letters.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <param name="suffix">The expected suffix.</param>
    /// <returns>Whether the text ends with the suffix.</returns>
    public static bool EndsWithIgnoreCase(string text, string suffix) =>
        text.EndsWith(suffix, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Returns whether the given text holds a Chinese character.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <returns>Whether the text holds a Chinese character.</returns>
    public static bool ContainsChinese(string text)
    {
        foreach (var character in text)
        {
            if (character >= 0x4E00 && character <= 0x9FFF)
                return true;
        }

        return false;
    }

    /// <summary>
    /// Returns the given character repeated the given number of times.
    /// </summary>
    /// <param name="character">The character to repeat.</param>
    /// <param name="count">The number of the repetitions.</param>
    /// <returns>The repeated character.</returns>
    public static string Repeats(char character, int count) =>
        count <= 0 ? string.Empty : new string(character, count);

    /// <summary>
    /// Truncates the given text to the given number of characters.
    /// </summary>
    /// <param name="text">The text to truncate.</param>
    /// <param name="limit">The maximum number of the characters of the
    /// result.</param>
    /// <returns>The truncated text.</returns>
    public static string Truncate(string text, int limit) =>
        text.Length <= limit ? text : text[..limit];

    /// <summary>
    /// Returns whether the given text holds only ASCII characters.
    /// </summary>
    /// <param name="text">The text to check.</param>
    /// <returns>Whether the text is an ASCII one.</returns>
    public static bool IsAscii(string text)
    {
        foreach (var character in text)
        {
            if (character > 0x7F)
                return false;
        }

        return true;
    }

    /// <summary>
    /// Splits the given text into its words, dropping the empty ones.
    /// </summary>
    /// <param name="text">The text to split.</param>
    /// <returns>The words of the text.</returns>
    public static IReadOnlyList<string> Tokenize(string text) =>
        text.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries);
}
