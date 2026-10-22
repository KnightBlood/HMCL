using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Text;

namespace HMCLCore.NET.Game;

/// <summary>
/// Exports the diagnostic files of a game session into a zip archive,
/// mirroring the Java HMCL log exporter: the game's own log files, the crash
/// reports, the launcher log and the launch script.
/// </summary>
public static class LogExporter
{
    /// <summary>
    /// The directories scanned for log files, relative to the game directory.
    /// </summary>
    private static readonly string[] LogDirectories = { "logs", "liteconfig" };

    /// <summary>
    /// Exports the logs of a game session.
    /// </summary>
    /// <param name="zipFilePath">The path of the zip archive to create.</param>
    /// <param name="gameDirectory">The game directory holding the logs and crash reports.</param>
    /// <param name="gameLog">The captured game output, written as "minecraft.log".</param>
    /// <param name="launcherLog">The launcher log, written as "hmcl.log".</param>
    /// <param name="launchScript">The launch script, written as "launch.bat" or "launch.sh".</param>
    /// <returns>The number of files written into the archive.</returns>
    public static int Export(
        string zipFilePath,
        string gameDirectory,
        IEnumerable<string>? gameLog = null,
        IEnumerable<string>? launcherLog = null,
        string? launchScript = null)
    {
        var directory = Path.GetDirectoryName(zipFilePath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        var written = 0;
        using var archive = ZipFile.Open(zipFilePath, ZipArchiveMode.Create);

        foreach (var relative in LogDirectories)
        {
            written += AddDirectory(archive, Path.Combine(gameDirectory, relative), relative);
        }

        written += AddDirectory(archive, gameDirectory, "runDirectory");

        written += AddDirectory(archive, Path.Combine(gameDirectory, "crash-reports"), "crash-reports");

        if (gameLog != null)
        {
            WriteText(archive, "minecraft.log", gameLog);
            written++;
        }

        if (launcherLog != null)
        {
            WriteText(archive, "hmcl.log", launcherLog);
            written++;
        }

        if (!string.IsNullOrEmpty(launchScript))
        {
            var name = OperatingSystemInfo.IsWindows ? "launch.bat" : "launch.sh";
            WriteText(archive, name, new[] { launchScript });
            written++;
        }

        return written;
    }

    /// <summary>
    /// Adds every log file of a directory to the archive.
    /// </summary>
    /// <param name="archive">The archive to write to.</param>
    /// <param name="sourceDirectory">The directory to scan.</param>
    /// <param name="archiveDirectory">The directory name used inside the archive.</param>
    /// <returns>The number of files added.</returns>
    private static int AddDirectory(ZipArchive archive, string sourceDirectory, string archiveDirectory)
    {
        if (!Directory.Exists(sourceDirectory))
            return 0;

        var written = 0;
        try
        {
            foreach (var file in Directory.GetFiles(sourceDirectory))
            {
                var extension = Path.GetExtension(file);
                if (!extension.Equals(".log", StringComparison.OrdinalIgnoreCase) &&
                    !extension.Equals(".txt", StringComparison.OrdinalIgnoreCase))
                {
                    continue;
                }

                var entry = archive.CreateEntry($"{archiveDirectory}/{Path.GetFileName(file)}", CompressionLevel.Optimal);
                using var stream = entry.Open();
                using var source = File.OpenRead(file);
                source.CopyTo(stream);
                written++;
            }
        }
        catch (IOException)
        {
            // A locked or unreadable log file must not abort the export.
        }
        catch (UnauthorizedAccessException)
        {
            // Same as above: skip files the launcher may not read.
        }

        return written;
    }

    /// <summary>
    /// Writes a text file into the archive.
    /// </summary>
    /// <param name="archive">The archive to write to.</param>
    /// <param name="name">The file name inside the archive.</param>
    /// <param name="lines">The lines to write.</param>
    private static void WriteText(ZipArchive archive, string name, IEnumerable<string> lines)
    {
        var entry = archive.CreateEntry(name, CompressionLevel.Optimal);
        using var stream = entry.Open();
        using var writer = new StreamWriter(stream, new UTF8Encoding(false));
        foreach (var line in lines)
            writer.WriteLine(line);
    }
}

/// <summary>
/// Small helper exposing the host platform information used by the exporter.
/// </summary>
public static class OperatingSystemInfo
{
    /// <summary>
    /// Whether the launcher runs on Windows.
    /// </summary>
    public static bool IsWindows =>
        OperatingSystem.IsWindows() || Environment.OSVersion.Platform == PlatformID.Win32NT;
}
