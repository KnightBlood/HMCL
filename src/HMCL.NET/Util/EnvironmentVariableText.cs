using System;
using System.Collections.Generic;
using System.Text;

namespace HMCL.NET.Util;

/// <summary>
/// Converts between a dictionary of environment variables and the plain
/// "KEY=VALUE" per line text used by the settings UI.
/// </summary>
public static class EnvironmentVariableText
{
    /// <summary>
    /// Parses environment variables from a multi-line "KEY=VALUE" text.
    /// Blank lines and lines starting with '#' are ignored; entries without
    /// '=' are skipped. The first '=' separates the name from the value so
    /// values may themselves contain '='.
    /// </summary>
    /// <param name="text">The text to parse.</param>
    /// <returns>The parsed variables, keyed case-insensitively.</returns>
    public static Dictionary<string, string> Parse(string? text)
    {
        var result = new Dictionary<string, string>(StringComparer.OrdinalIgnoreCase);
        if (string.IsNullOrWhiteSpace(text))
            return result;

        foreach (var rawLine in text.Split('\n'))
        {
            var line = rawLine.Trim('\r', ' ', '\t');
            if (line.Length == 0 || line[0] == '#')
                continue;

            var separator = line.IndexOf('=');
            if (separator <= 0)
                continue;

            var key = line[..separator].Trim();
            var value = line[(separator + 1)..];
            if (key.Length == 0)
                continue;

            result[key] = value;
        }

        return result;
    }

    /// <summary>
    /// Formats environment variables as a multi-line "KEY=VALUE" text.
    /// </summary>
    /// <param name="variables">The variables to format.</param>
    /// <returns>The formatted text, or an empty string when there are no variables.</returns>
    public static string Format(IReadOnlyDictionary<string, string>? variables)
    {
        if (variables is null || variables.Count == 0)
            return string.Empty;

        var builder = new StringBuilder();
        foreach (var kvp in variables)
        {
            if (builder.Length > 0)
                builder.Append('\n');
            builder.Append(kvp.Key).Append('=').Append(kvp.Value);
        }

        return builder.ToString();
    }
}
