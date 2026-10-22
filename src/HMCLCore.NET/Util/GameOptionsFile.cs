using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace HMCLCore.NET.Util;

/// <summary>
/// Reads and writes the "options.txt" file of a game instance. The file stores
/// one "key:value" pair per line; list values use the JSON-like notation
/// Minecraft uses, e.g. <c>["file/a.zip","file/b.zip"]</c>.
/// </summary>
public sealed class GameOptionsFile
{
    private readonly List<string> _lines = new();
    private readonly string? _path;

    /// <summary>
    /// Creates an empty options file that is not backed by a file on disk.
    /// </summary>
    public GameOptionsFile()
    {
    }

    private GameOptionsFile(string path)
    {
        _path = path;
        ReadLines(path);
    }

    /// <summary>
    /// Loads the options file from disk. A missing file yields empty options.
    /// </summary>
    /// <param name="path">The path of options.txt.</param>
    /// <returns>The loaded options file.</returns>
    public static GameOptionsFile Load(string path)
    {
        var options = new GameOptionsFile(path);
        return options;
    }

    /// <summary>
    /// Whether the options are backed by a file on disk.
    /// </summary>
    public bool HasFile => _path != null;

    /// <summary>
    /// Returns the value of an option, or null when it is not present.
    /// </summary>
    /// <param name="key">The option key.</param>
    /// <returns>The option value, or null.</returns>
    public string? Get(string key)
    {
        foreach (var line in _lines)
        {
            var index = line.IndexOf(':');
            if (index <= 0)
                continue;

            if (string.Equals(line[..index], key, StringComparison.Ordinal))
                return line[(index + 1)..];
        }

        return null;
    }

    /// <summary>
    /// Sets an option, appending it when it is not present yet.
    /// </summary>
    /// <param name="key">The option key.</param>
    /// <param name="value">The option value.</param>
    public void Set(string key, string value)
    {
        var entry = $"{key}:{value}";
        for (var i = 0; i < _lines.Count; i++)
        {
            var line = _lines[i];
            var index = line.IndexOf(':');
            if (index <= 0)
                continue;

            if (string.Equals(line[..index], key, StringComparison.Ordinal))
            {
                _lines[i] = entry;
                return;
            }
        }

        _lines.Add(entry);
    }

    /// <summary>
    /// Reads a list option.
    /// </summary>
    /// <param name="key">The option key.</param>
    /// <returns>The values of the list, empty when the option is missing.</returns>
    public IReadOnlyList<string> GetStringList(string key)
    {
        var raw = Get(key);
        if (string.IsNullOrEmpty(raw))
            return Array.Empty<string>();

        return ParseStringList(raw);
    }

    /// <summary>
    /// Writes a list option.
    /// </summary>
    /// <param name="key">The option key.</param>
    /// <param name="values">The values of the list.</param>
    public void SetStringList(string key, IEnumerable<string> values)
    {
        Set(key, FormatStringList(values));
    }

    /// <summary>
    /// Writes the options back to disk. Does nothing when the options are not
    /// backed by a file.
    /// </summary>
    public void Save()
    {
        if (_path == null)
            return;

        var directory = System.IO.Path.GetDirectoryName(_path);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        File.WriteAllText(_path, string.Join(Environment.NewLine, _lines), new UTF8Encoding(false));
    }

    /// <summary>
    /// Reads the raw lines of the options file.
    /// </summary>
    /// <param name="path">The path of options.txt.</param>
    private void ReadLines(string path)
    {
        if (!File.Exists(path))
            return;

        try
        {
            _lines.AddRange(File.ReadAllLines(path));
        }
        catch (IOException)
        {
            // A locked or unreadable options file leaves the options empty.
        }
    }

    /// <summary>
    /// Parses a Minecraft string list such as <c>["a","b"]</c>.
    /// </summary>
    /// <param name="raw">The raw option value.</param>
    /// <returns>The parsed values.</returns>
    private static IReadOnlyList<string> ParseStringList(string raw)
    {
        var values = new List<string>();
        var builder = new StringBuilder();
        var inQuotes = false;
        var escaped = false;

        foreach (var ch in raw)
        {
            if (escaped)
            {
                builder.Append(ch);
                escaped = false;
                continue;
            }

            switch (ch)
            {
                case '\\' when inQuotes:
                    escaped = true;
                    break;
                case '"':
                    inQuotes = !inQuotes;
                    break;
                case ',' when !inQuotes:
                    AddValue(values, builder);
                    break;
                default:
                    if (inQuotes)
                        builder.Append(ch);
                    break;
            }
        }

        AddValue(values, builder);
        return values;
    }

    /// <summary>
    /// Appends a parsed value to the list when it is not empty.
    /// </summary>
    /// <param name="values">The list to append to.</param>
    /// <param name="builder">The builder holding the current value.</param>
    private static void AddValue(List<string> values, StringBuilder builder)
    {
        var value = builder.ToString();
        builder.Clear();
        if (value.Length > 0)
            values.Add(value);
    }

    /// <summary>
    /// Formats a list of values the way Minecraft stores them.
    /// </summary>
    /// <param name="values">The values to format.</param>
    /// <returns>The formatted option value.</returns>
    private static string FormatStringList(IEnumerable<string> values)
    {
        var builder = new StringBuilder("[");
        var first = true;

        foreach (var value in values)
        {
            if (!first)
                builder.Append(',');

            first = false;
            builder.Append('"');
            builder.Append(value.Replace("\\", "\\\\").Replace("\"", "\\\""));
            builder.Append('"');
        }

        builder.Append(']');
        return builder.ToString();
    }
}
