using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// The kind of a blueprint of the game.
/// </summary>
public enum BlueprintType
{
    /// <summary>
    /// A blueprint of the WorldEdit format, stored in the ".schem" files.
    /// </summary>
    Schem,

    /// <summary>
    /// A blueprint of the NBT format, stored in the ".nbt" files.
    /// </summary>
    Nbt,

    /// <summary>
    /// A blueprint of the Litematic format, stored in the ".litematic" files.
    /// </summary>
    Litematic
}

/// <summary>
/// A blueprint of the game, like the structures saved by the players.
/// </summary>
public sealed class BlueprintInfo
{
    /// <summary>
    /// Creates the information of a blueprint.
    /// </summary>
    /// <param name="path">The path of the file holding the blueprint.</param>
    /// <param name="type">The kind of the blueprint.</param>
    /// <param name="size">The size of the file, in bytes.</param>
    /// <param name="modifiedAt">The date the file was modified at.</param>
    public BlueprintInfo(string path, BlueprintType type, long size, DateTime modifiedAt)
    {
        Path = path;
        Type = type;
        Size = size;
        ModifiedAt = modifiedAt;
    }

    /// <summary>
    /// The path of the file holding the blueprint.
    /// </summary>
    public string Path { get; }

    /// <summary>
    /// The name of the file holding the blueprint.
    /// </summary>
    public string FileName => System.IO.Path.GetFileName(Path);

    /// <summary>
    /// The kind of the blueprint.
    /// </summary>
    public BlueprintType Type { get; }

    /// <summary>
    /// The size of the file, in bytes.
    /// </summary>
    public long Size { get; }

    /// <summary>
    /// The date the file was modified at.
    /// </summary>
    public DateTime ModifiedAt { get; }

    /// <summary>
    /// The size of the file shown to the user.
    /// </summary>
    public string SizeText => Size >= 1024 * 1024
        ? $"{Size / 1024d / 1024d:F2} MB"
        : $"{Size / 1024d:F1} KB";

    /// <summary>
    /// The date the file was modified at, shown to the user.
    /// </summary>
    public string ModifiedAtText => ModifiedAt.ToString("yyyy-MM-dd HH:mm");

    /// <summary>
    /// The kind of the blueprint shown to the user.
    /// </summary>
    public string TypeText => Type switch
    {
        BlueprintType.Schem => "WorldEdit 蓝图",
        BlueprintType.Litematic => "Litematic 蓝图",
        _ => "NBT 蓝图"
    };
}

/// <summary>
/// Manages the blueprints of a game instance.
/// </summary>
public sealed class BlueprintManager
{
    /// <summary>
    /// The extensions of the files holding a blueprint.
    /// </summary>
    private static readonly (string Extension, BlueprintType Type)[] KnownExtensions =
    {
        (".schem", BlueprintType.Schem),
        (".litematic", BlueprintType.Litematic),
        (".nbt", BlueprintType.Nbt)
    };

    /// <summary>
    /// Creates a manager for the blueprints of the given game directory.
    /// </summary>
    /// <param name="gameDirectory">The game directory holding the blueprints.</param>
    public BlueprintManager(string gameDirectory)
    {
        GameDirectory = gameDirectory ?? throw new ArgumentNullException(nameof(gameDirectory));
    }

    /// <summary>
    /// The game directory of the instance.
    /// </summary>
    public string GameDirectory { get; }

    /// <summary>
    /// The directory holding the blueprints of the instance.
    /// </summary>
    public string BlueprintDirectory => Path.Combine(GameDirectory, "schematics");

    /// <summary>
    /// Returns the kind of the blueprint held by the given file.
    /// </summary>
    /// <param name="path">The path of the file.</param>
    /// <returns>The kind of the blueprint, or null when the file is not one.</returns>
    public static BlueprintType? GetType(string path)
    {
        var extension = Path.GetExtension(path);
        foreach (var (known, type) in KnownExtensions)
        {
            if (extension.Equals(known, StringComparison.OrdinalIgnoreCase))
                return type;
        }

        return null;
    }

    /// <summary>
    /// Returns the blueprints of the instance, the newest ones first.
    /// </summary>
    /// <returns>The blueprints of the instance.</returns>
    public IReadOnlyList<BlueprintInfo> GetBlueprints()
    {
        var directory = BlueprintDirectory;
        if (!Directory.Exists(directory))
            return Array.Empty<BlueprintInfo>();

        try
        {
            return Directory.GetFiles(directory)
                .Select(file => new FileInfo(file))
                .Where(info => GetType(info.Name) != null)
                .Select(info => new BlueprintInfo(
                    info.FullName,
                    GetType(info.Name)!.Value,
                    info.Length,
                    info.LastWriteTime))
                .OrderByDescending(blueprint => blueprint.ModifiedAt)
                .ToList();
        }
        catch (IOException)
        {
            return Array.Empty<BlueprintInfo>();
        }
        catch (UnauthorizedAccessException)
        {
            return Array.Empty<BlueprintInfo>();
        }
    }

    /// <summary>
    /// Imports a blueprint into the blueprints directory of the instance.
    /// </summary>
    /// <param name="sourcePath">The path of the imported file.</param>
    /// <returns>The imported blueprint, or null when the file is not a blueprint.</returns>
    public BlueprintInfo? Import(string sourcePath)
    {
        if (!File.Exists(sourcePath))
            return null;

        var type = GetType(sourcePath);
        if (type is null)
            return null;

        try
        {
            Directory.CreateDirectory(BlueprintDirectory);

            var target = Path.Combine(BlueprintDirectory, Path.GetFileName(sourcePath));
            File.Copy(sourcePath, target, true);

            var info = new FileInfo(target);
            return new BlueprintInfo(target, type.Value, info.Length, info.LastWriteTime);
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }
    }

    /// <summary>
    /// Deletes the given blueprint.
    /// </summary>
    /// <param name="blueprint">The blueprint to delete.</param>
    /// <returns>Whether the blueprint was deleted.</returns>
    public bool Remove(BlueprintInfo blueprint)
    {
        try
        {
            if (!File.Exists(blueprint.Path))
                return false;

            File.Delete(blueprint.Path);
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
}
