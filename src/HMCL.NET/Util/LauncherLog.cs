using System;
using System.IO;
using System.Text;

namespace HMCL.NET.Util;

/// <summary>
/// Writes the log of the launcher into a file, like the log of Java HMCL.
/// </summary>
public static class LauncherLog
{
    private static readonly object _lock = new();
    private static string? _path;

    /// <summary>
    /// The path of the log file, or null when the log is disabled.
    /// </summary>
    public static string? Path => _path;

    /// <summary>
    /// Enables the log file at the given path.
    /// </summary>
    /// <param name="path">The path of the log file.</param>
    public static void Enable(string path)
    {
        lock (_lock)
            _path = path;
    }

    /// <summary>
    /// Writes a line into the log file.
    /// </summary>
    /// <param name="level">The level of the message.</param>
    /// <param name="message">The message of the log entry.</param>
    public static void Write(string level, string message)
    {
        var path = _path;
        if (string.IsNullOrEmpty(path))
            return;

        try
        {
            lock (_lock)
            {
                var directory = System.IO.Path.GetDirectoryName(path);
                if (!string.IsNullOrEmpty(directory))
                    Directory.CreateDirectory(directory);

                File.AppendAllText(
                    path,
                    $"[{DateTime.Now:yyyy-MM-dd HH:mm:ss}] [{level}] {message}{Environment.NewLine}",
                    Encoding.UTF8);
            }
        }
        catch (IOException)
        {
            // A failing log file must not break the launcher.
        }
        catch (UnauthorizedAccessException)
        {
            // Same as above: the launcher may not write into the log file.
        }
    }

    /// <summary>
    /// Writes an informational line into the log file.
    /// </summary>
    /// <param name="message">The message of the log entry.</param>
    public static void Info(string message) => Write("INFO", message);

    /// <summary>
    /// Writes a warning line into the log file.
    /// </summary>
    /// <param name="message">The message of the log entry.</param>
    public static void Warn(string message) => Write("WARN", message);

    /// <summary>
    /// Writes an error line into the log file.
    /// </summary>
    /// <param name="message">The message of the log entry.</param>
    /// <param name="exception">The exception that caused the error.</param>
    public static void Error(string message, Exception? exception = null)
    {
        Write("ERROR", exception is null ? message : $"{message}: {exception.Message}");
    }
}
