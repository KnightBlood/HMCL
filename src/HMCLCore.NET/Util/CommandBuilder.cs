using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace HMCLCore.NET.Util;

/// <summary>
/// Builds command lines for launching processes.
/// Ported from Java CommandBuilder.
/// </summary>
public sealed class CommandBuilder
{
    private readonly List<string> _parts = new();

    /// <summary>
    /// Appends a part to the command line.
    /// </summary>
    public CommandBuilder Append(string part)
    {
        _parts.Add(part);
        return this;
    }

    /// <summary>
    /// Appends a part to the command line with proper quoting if needed.
    /// </summary>
    public CommandBuilder AppendQuoted(string part)
    {
        if (string.IsNullOrEmpty(part))
        {
            _parts.Add("\"\"");
            return this;
        }

        if (part.Contains(' ') || part.Contains('"'))
        {
            var escaped = part.Replace("\"", "\\\"");
            _parts.Add($"\"{escaped}\"");
        }
        else
        {
            _parts.Add(part);
        }

        return this;
    }

    /// <summary>
    /// Returns the built command line as a single string.
    /// </summary>
    public override string ToString()
    {
        var sb = new StringBuilder();
        for (int i = 0; i < _parts.Count; i++)
        {
            if (i > 0)
                sb.Append(' ');
            sb.Append(_parts[i]);
        }
        return sb.ToString();
    }

    /// <summary>
    /// Returns the command line parts as a list.
    /// </summary>
    public IReadOnlyList<string> Parts => _parts.AsReadOnly();

    /// <summary>
    /// Splits a command line into its parts, honoring the double quotes.
    /// </summary>
    /// <param name="command">The command line to split.</param>
    /// <returns>The parts of the command line.</returns>
    public static IReadOnlyList<string> Parse(string? command)
    {
        var parts = new List<string>();
        if (string.IsNullOrEmpty(command))
            return parts;

        var current = new StringBuilder();
        var inQuotes = false;
        var hasContent = false;

        foreach (var c in command)
        {
            if (c == '"')
            {
                inQuotes = !inQuotes;
                hasContent = true;
                continue;
            }

            if (!inQuotes && char.IsWhiteSpace(c))
            {
                if (hasContent)
                {
                    parts.Add(current.ToString());
                    current.Clear();
                    hasContent = false;
                }

                continue;
            }

            current.Append(c);
            hasContent = true;
        }

        if (hasContent)
            parts.Add(current.ToString());

        return parts;
    }
}