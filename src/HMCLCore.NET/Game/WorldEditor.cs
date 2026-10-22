using System;
using System.IO;
using System.Linq;
using HMCLCore.NET.Nbt;

namespace HMCLCore.NET.Game;

/// <summary>
/// The difficulty of a Minecraft world.
/// </summary>
public enum WorldDifficulty
{
    /// <summary>
    /// The peaceful difficulty, in which no hostile mobs are spawned.
    /// </summary>
    Peaceful = 0,

    /// <summary>
    /// The easy difficulty.
    /// </summary>
    Easy = 1,

    /// <summary>
    /// The normal difficulty.
    /// </summary>
    Normal = 2,

    /// <summary>
    /// The hard difficulty.
    /// </summary>
    Hard = 3
}

/// <summary>
/// Edits the data of the Minecraft worlds: their icon, their name and their
/// difficulty, like the world information page of Java HMCL.
/// </summary>
public static class WorldEditor
{
    /// <summary>
    /// The name of the file holding the icon of a world.
    /// </summary>
    public const string IconFileName = "icon.png";

    /// <summary>
    /// The name of the file holding the name of a world.
    /// </summary>
    public const string LevelNameFileName = "levelname.txt";

    /// <summary>
    /// The name of the file holding the data of a world.
    /// </summary>
    public const string LevelDataFileName = "level.dat";

    /// <summary>
    /// Copies the given image into the icon of the world.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <param name="iconPath">The path of the image used as the icon.</param>
    /// <returns>Whether the icon was changed.</returns>
    public static bool SetIcon(string worldDirectory, string iconPath)
    {
        if (!Directory.Exists(worldDirectory) || !File.Exists(iconPath))
            return false;

        try
        {
            var target = Path.Combine(worldDirectory, IconFileName);
            File.Copy(iconPath, target, true);
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Removes the icon of the world.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <returns>Whether the icon was removed.</returns>
    public static bool ClearIcon(string worldDirectory)
    {
        if (!Directory.Exists(worldDirectory))
            return false;

        try
        {
            var target = Path.Combine(worldDirectory, IconFileName);
            if (File.Exists(target))
                File.Delete(target);

            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Copies a world into a new one, like the duplication of Java HMCL.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <param name="newName">
    /// The name of the new world; the name of the directory of the copy is
    /// derived from it.
    /// </param>
    /// <returns>The directory of the copy, or null when it could not be made.</returns>
    public static string? Duplicate(string worldDirectory, string newName)
    {
        if (!Directory.Exists(worldDirectory) || string.IsNullOrWhiteSpace(newName))
            return null;

        var name = newName.Trim();
        var directoryName = string.Join("_", name.Split(Path.GetInvalidFileNameChars()))
            .Trim();
        if (string.IsNullOrEmpty(directoryName))
            return null;

        var parent = Path.GetDirectoryName(worldDirectory.TrimEnd(Path.DirectorySeparatorChar));
        if (string.IsNullOrEmpty(parent))
            return null;

        var target = Path.Combine(parent, directoryName);
        if (Directory.Exists(target))
            return null;

        try
        {
            CopyDirectory(worldDirectory, target);

            // The name of the copy is shown by the game, so it is written both
            // into the text file and into the data of the world.
            File.WriteAllText(Path.Combine(target, LevelNameFileName), name);

            SetLevelName(target, name);
            return target;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException or InvalidDataException)
        {
            return null;
        }
    }

    /// <summary>
    /// Changes the difficulty of a world and locks it, so that it cannot be
    /// changed from the game.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <param name="difficulty">The new difficulty of the world.</param>
    /// <param name="locked">Whether the difficulty is locked.</param>
    /// <returns>Whether the difficulty was changed.</returns>
    public static bool SetDifficulty(string worldDirectory, WorldDifficulty difficulty, bool locked)
    {
        var levelData = Path.Combine(worldDirectory, LevelDataFileName);
        if (!File.Exists(levelData))
            return false;

        try
        {
            var file = NbtFile.Load(levelData);
            if (file?.Root is not NbtCompoundTag root)
                return false;

            if (root.Children.TryGetValue("Data", out var dataTag) && dataTag is NbtCompoundTag data)
            {
                // The worlds created before 26.1-snapshot-6 store the difficulty
                // and its lock in the data of the world.
                if (data.Children.TryGetValue("difficulty_settings", out var settingsTag) &&
                    settingsTag is NbtCompoundTag settings)
                {
                    settings.Children["difficulty"] = new NbtStringTag
                    {
                        Value = GetDifficultyName(difficulty)
                    };
                    settings.Children["locked"] = new NbtByteTag { Value = locked ? (byte)1 : (byte)0 };
                }
                else
                {
                    data.Children["Difficulty"] = new NbtByteTag { Value = (byte)difficulty };
                    data.Children["DifficultyLocked"] = new NbtByteTag { Value = locked ? (byte)1 : (byte)0 };
                }

                file.Save(levelData);
                return true;
            }

            return false;
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException or UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Reads whether the difficulty of a world is locked.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <returns>Whether the difficulty is locked, or null when it is unknown.</returns>
    public static bool? IsDifficultyLocked(string worldDirectory)
    {
        var levelData = Path.Combine(worldDirectory, LevelDataFileName);
        if (!File.Exists(levelData))
            return null;

        try
        {
            var file = NbtFile.Load(levelData);
            if (file?.Root is not NbtCompoundTag root ||
                !root.Children.TryGetValue("Data", out var dataTag) ||
                dataTag is not NbtCompoundTag data)
            {
                return null;
            }

            if (data.Children.TryGetValue("difficulty_settings", out var settingsTag) &&
                settingsTag is NbtCompoundTag settings &&
                settings.Children.TryGetValue("locked", out var lockedTag) &&
                lockedTag is NbtByteTag lockedValue)
            {
                return lockedValue.Value != 0;
            }

            if (data.Children.TryGetValue("DifficultyLocked", out var tag) && tag is NbtByteTag value)
                return value.Value != 0;

            return null;
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException)
        {
            return null;
        }
    }

    /// <summary>
    /// Writes the name of a world into its data.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <param name="name">The new name of the world.</param>
    private static void SetLevelName(string worldDirectory, string name)
    {
        var levelData = Path.Combine(worldDirectory, LevelDataFileName);
        if (!File.Exists(levelData))
            return;

        try
        {
            var file = NbtFile.Load(levelData);
            if (file?.Root is not NbtCompoundTag root)
                return;

            if (root.Children.TryGetValue("Data", out var dataTag) && dataTag is NbtCompoundTag data)
            {
                data.Children["LevelName"] = new NbtStringTag { Value = name };
                file.Save(levelData);
            }
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException)
        {
            // The name of the world stays unchanged when its data cannot be
            // written.
        }
    }

    /// <summary>
    /// Returns the name of a difficulty used by the newer versions of the
    /// game.
    /// </summary>
    /// <param name="difficulty">The difficulty.</param>
    /// <returns>The name of the difficulty.</returns>
    private static string GetDifficultyName(WorldDifficulty difficulty) => difficulty switch
    {
        WorldDifficulty.Peaceful => "peaceful",
        WorldDifficulty.Easy => "easy",
        WorldDifficulty.Hard => "hard",
        _ => "normal"
    };

    /// <summary>
    /// Copies a directory and all its files into another one.
    /// </summary>
    /// <param name="source">The directory copied.</param>
    /// <param name="target">The directory the files are copied into.</param>
    private static void CopyDirectory(string source, string target)
    {
        Directory.CreateDirectory(target);

        foreach (var file in Directory.GetFiles(source))
            File.Copy(file, Path.Combine(target, Path.GetFileName(file)), true);

        foreach (var directory in Directory.GetDirectories(source))
        {
            var name = Path.GetFileName(directory);
            if (name is null)
                continue;

            CopyDirectory(directory, Path.Combine(target, name));
        }
    }
}
