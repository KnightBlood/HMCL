using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text;

namespace HMCLCore.NET.Game;

/// <summary>
/// Manages Minecraft worlds saved in an instance's <c>saves/</c> directory:
/// listing, backing up and deleting worlds.
/// </summary>
public sealed class WorldManager
{
    private readonly string _savesDirectory;
    private readonly string _backupsDirectory;

    /// <summary>
    /// Creates a new <see cref="WorldManager"/> for the given instance.
    /// </summary>
    /// <param name="gameDirectory">The instance's game directory (contains <c>saves/</c>).</param>
    /// <param name="backupsDirectory">The directory world backups are written to.</param>
    public WorldManager(string gameDirectory, string backupsDirectory)
    {
        _savesDirectory = Path.Combine(gameDirectory, "saves");
        _backupsDirectory = backupsDirectory ?? string.Empty;
    }

    /// <summary>
    /// Lists all worlds found in <c>saves/</c>, sorted by name.
    /// </summary>
    /// <returns>A list of world infos, empty when no saves directory exists.</returns>
    public List<WorldInfo> GetWorlds()
    {
        var worlds = new List<WorldInfo>();
        if (!Directory.Exists(_savesDirectory))
            return worlds;

        foreach (var dir in Directory.GetDirectories(_savesDirectory))
        {
            var name = Path.GetFileName(dir);
            if (name == null)
                continue;

            var info = new DirectoryInfo(dir);
            worlds.Add(new WorldInfo
            {
                DirectoryName = name,
                DisplayName = ReadDisplayName(dir) ?? name,
                Size = DirectorySize(dir),
                LastModified = info.LastWriteTimeUtc,
                Path = dir,
                InUse = IsWorldLocked(name)
            });
        }

        return worlds.OrderBy(w => w.DisplayName, StringComparer.OrdinalIgnoreCase).ToList();
    }

    /// <summary>
    /// Returns whether the world is used by a running game, like the locked
    /// worlds of Java HMCL.
    /// </summary>
    /// <param name="directoryName">The world directory name inside <c>saves/</c>.</param>
    /// <returns>Whether the world is locked.</returns>
    public bool IsWorldLocked(string directoryName)
    {
        if (!Directory.Exists(_savesDirectory))
            return false;

        var lockFile = Path.Combine(_savesDirectory, directoryName, "session.lock");
        if (!File.Exists(lockFile))
            return false;

        try
        {
            // The game keeps the lock file open while the world is loaded; the
            // file can only be opened exclusively when the world is free.
            using var stream = File.Open(lockFile, FileMode.Open, FileAccess.ReadWrite, FileShare.None);
            return false;
        }
        catch (IOException)
        {
            return true;
        }
        catch (UnauthorizedAccessException)
        {
            return true;
        }
    }

    /// <summary>
    /// Backs up a world into the backups directory as a zip archive.
    /// </summary>
    /// <param name="directoryName">The world directory name inside <c>saves/</c>.</param>
    /// <returns>The path of the created backup zip, or null on failure.</returns>
    public string? BackupWorld(string directoryName)
    {
        if (!Directory.Exists(_savesDirectory))
            return null;

        var worldPath = Path.Combine(_savesDirectory, directoryName);
        if (!Directory.Exists(worldPath))
            return null;

        // The world cannot be backed up while the game is using it.
        if (IsWorldLocked(directoryName))
            return null;

        try
        {
            Directory.CreateDirectory(_backupsDirectory);

            // The name of the backup holds the time it was created at and the
            // number of the backups of the world, like the backups of Java
            // HMCL.
            var timestamp = DateTime.Now.ToString("yyyy-MM-dd_HH-mm-ss");
            var safeName = string.Join("_", directoryName.Split(Path.GetInvalidFileNameChars()));
            var count = CountBackups(directoryName);
            var suffix = count > 0 ? $" ({count})" : string.Empty;
            var backupPath = Path.Combine(_backupsDirectory, $"{timestamp}_{safeName}{suffix}.zip");

            ZipFile.CreateFromDirectory(worldPath, backupPath);
            return backupPath;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Lists the backups of the worlds of the instance.
    /// </summary>
    /// <returns>The backups, the newest first.</returns>
    public IReadOnlyList<WorldBackup> GetBackups()
    {
        var backups = new List<WorldBackup>();

        if (!Directory.Exists(_backupsDirectory))
            return backups;

        foreach (var file in Directory.GetFiles(_backupsDirectory, "*.zip"))
        {
            var name = Path.GetFileNameWithoutExtension(file);

            // The name of a backup starts with the time it was created at,
            // which is always nineteen characters long.
            if (name.Length <= 19)
                continue;

            if (!DateTime.TryParseExact(
                    name[..19], "yyyy-MM-dd_HH-mm-ss", System.Globalization.CultureInfo.InvariantCulture,
                    System.Globalization.DateTimeStyles.None, out var time))
            {
                continue;
            }

            var rest = name[20..];
            var count = 0;
            var space = rest.IndexOf(" (", StringComparison.Ordinal);
            if (space > 0 && rest.EndsWith(')'))
            {
                if (int.TryParse(rest[(space + 2)..^1], out var parsed))
                    count = parsed;

                rest = rest[..space];
            }

            var size = 0L;
            try
            {
                size = new FileInfo(file).Length;
            }
            catch
            {
                // The size of an unreadable file is unknown.
            }

            backups.Add(new WorldBackup
            {
                FilePath = file,
                FileName = Path.GetFileName(file),
                WorldName = rest,
                Time = time,
                Count = count,
                Size = size
            });
        }

        return backups.OrderByDescending(backup => backup.Time).ToList();
    }

    /// <summary>
    /// Deletes a backup of a world.
    /// </summary>
    /// <param name="backupPath">The path of the backup.</param>
    /// <returns>Whether the backup was deleted.</returns>
    public bool DeleteBackup(string backupPath)
    {
        if (string.IsNullOrEmpty(backupPath) || !File.Exists(backupPath))
            return false;

        try
        {
            File.Delete(backupPath);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Returns the number of the backups of the given world.
    /// </summary>
    /// <param name="directoryName">The world directory name inside <c>saves/</c>.</param>
    /// <returns>The number of the backups.</returns>
    private int CountBackups(string directoryName)
    {
        if (!Directory.Exists(_backupsDirectory))
            return 0;

        var safeName = string.Join("_", directoryName.Split(Path.GetInvalidFileNameChars()));
        var count = 0;

        try
        {
            foreach (var file in Directory.GetFiles(_backupsDirectory, $"*_{safeName}*.zip"))
                count++;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return count;
        }

        return count;
    }

    /// <summary>
    /// Deletes a world directory from <c>saves/</c>.
    /// </summary>
    public bool DeleteWorld(string directoryName)
    {
        if (!Directory.Exists(_savesDirectory))
            return false;

        var worldPath = Path.Combine(_savesDirectory, directoryName);
        if (!Directory.Exists(worldPath))
            return false;

        // The world cannot be deleted while the game is using it.
        if (IsWorldLocked(directoryName))
            return false;

        try
        {
            Directory.Delete(worldPath, recursive: true);
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Exports a world into a zip archive at the given path.
    /// </summary>
    /// <param name="directoryName">The world directory name inside <c>saves/</c>.</param>
    /// <param name="destinationPath">The path of the archive to create.</param>
    /// <returns>Whether the world was exported.</returns>
    public bool ExportWorld(string directoryName, string destinationPath)
    {
        if (!Directory.Exists(_savesDirectory))
            return false;

        var worldPath = Path.Combine(_savesDirectory, directoryName);
        if (!Directory.Exists(worldPath))
            return false;

        try
        {
            var directory = Path.GetDirectoryName(destinationPath);
            if (!string.IsNullOrEmpty(directory))
                Directory.CreateDirectory(directory);

            if (File.Exists(destinationPath))
                File.Delete(destinationPath);

            ZipFile.CreateFromDirectory(worldPath, destinationPath);
            return true;
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Imports a world archive into <c>saves/</c>.
    /// </summary>
    /// <param name="archivePath">The archive to import.</param>
    /// <param name="directoryName">
    /// The name of the imported world directory, or null to derive it from the archive.
    /// </param>
    /// <returns>The name of the imported world, or null when the import failed.</returns>
    public string? ImportWorld(string archivePath, string? directoryName = null)
    {
        if (!File.Exists(archivePath))
            return null;

        var name = directoryName;
        if (string.IsNullOrEmpty(name))
            name = Path.GetFileNameWithoutExtension(archivePath);

        if (string.IsNullOrEmpty(name))
            return null;

        var target = Path.Combine(_savesDirectory, name!);
        if (Directory.Exists(target))
            return null;

        try
        {
            Directory.CreateDirectory(_savesDirectory);
            ZipFile.ExtractToDirectory(archivePath, target);
            return name;
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }
        catch (InvalidDataException)
        {
            return null;
        }
    }

    private static long DirectorySize(string directory)
    {
        long size = 0;
        try
        {
            foreach (var file in Directory.GetFiles(directory, "*", SearchOption.AllDirectories))
            {
                try
                {
                    size += new FileInfo(file).Length;
                }
                catch
                {
                    // Ignore inaccessible files.
                }
            }
        }
        catch
        {
            // Ignore inaccessible directories.
        }

        return size;
    }

    private static string? ReadDisplayName(string directory)
    {
        // Minecraft stores the world display name in levelname.txt.
        var levelName = Path.Combine(directory, "levelname.txt");
        if (File.Exists(levelName))
        {
            try
            {
                var name = File.ReadAllText(levelName, Encoding.UTF8).Trim();
                if (!string.IsNullOrEmpty(name))
                    return name;
            }
            catch
            {
                // Fall back to directory name below.
            }
        }

        return null;
    }
}

/// <summary>
/// A backup of a Minecraft world.
/// </summary>
public sealed class WorldBackup
{
    /// <summary>
    /// The path of the archive holding the backup.
    /// </summary>
    public string FilePath { get; init; } = string.Empty;

    /// <summary>
    /// The name of the archive holding the backup.
    /// </summary>
    public string FileName { get; init; } = string.Empty;

    /// <summary>
    /// The name of the world the backup was made of.
    /// </summary>
    public string WorldName { get; init; } = string.Empty;

    /// <summary>
    /// The time the backup was created at.
    /// </summary>
    public DateTime Time { get; init; }

    /// <summary>
    /// The number of the backups of the world, like the backups of Java HMCL.
    /// </summary>
    public int Count { get; init; }

    /// <summary>
    /// The size of the backup in bytes.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// Returns a human-readable size string.
    /// </summary>
    public string SizeText
    {
        get
        {
            var bytes = Size;
            var units = new[] { "B", "KB", "MB", "GB", "TB" };
            var unit = 0;
            double value = bytes;
            while (value >= 1024 && unit < units.Length - 1)
            {
                value /= 1024;
                unit++;
            }

            return unit == 0 ? $"{bytes} {units[unit]}" : $"{value:F2} {units[unit]}";
        }
    }

    /// <summary>
    /// The time the backup was created at, shown to the user.
    /// </summary>
    public string TimeText => Time.ToString("yyyy-MM-dd HH:mm:ss", System.Globalization.CultureInfo.CurrentCulture);
}

/// <summary>
/// Information about a single Minecraft world.
/// </summary>
public sealed class WorldInfo
{
    /// <summary>
    /// The world directory name inside <c>saves/</c>.
    /// </summary>
    public string DirectoryName { get; set; } = string.Empty;

    /// <summary>
    /// The human-readable display name (from <c>levelname.txt</c>).
    /// </summary>
    public string DisplayName { get; set; } = string.Empty;

    /// <summary>
    /// The world size in bytes.
    /// </summary>
    public long Size { get; set; }

    /// <summary>
    /// The last modification time in UTC.
    /// </summary>
    public DateTime LastModified { get; set; }

    /// <summary>
    /// The full directory path.
    /// </summary>
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// Whether the world is used by a running game.
    /// </summary>
    public bool InUse { get; set; }

    /// <summary>
    /// Returns a human-readable size string.
    /// </summary>
    public string SizeText
    {
        get
        {
            var bytes = Size;
            var units = new[] { "B", "KB", "MB", "GB", "TB" };
            int unit = 0;
            double value = bytes;
            while (value >= 1024 && unit < units.Length - 1)
            {
                value /= 1024;
                unit++;
            }

            return unit == 0 ? $"{bytes} {units[unit]}" : $"{value:F2} {units[unit]}";
        }
    }
}
