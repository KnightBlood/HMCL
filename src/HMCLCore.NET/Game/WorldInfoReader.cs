using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using HMCLCore.NET.Nbt;

namespace HMCLCore.NET.Game;

/// <summary>
/// The information about the player stored in the level data of a world.
/// </summary>
public sealed class PlayerDetails
{
    /// <summary>
    /// The game mode of the player: survival, creative, adventure or hardcore.
    /// </summary>
    public string GameMode { get; init; } = "survival";

    /// <summary>
    /// The health of the player.
    /// </summary>
    public double Health { get; init; }

    /// <summary>
    /// The hunger level of the player.
    /// </summary>
    public int FoodLevel { get; init; }

    /// <summary>
    /// The saturation of the player.
    /// </summary>
    public float Saturation { get; init; }

    /// <summary>
    /// The experience level of the player.
    /// </summary>
    public int ExperienceLevel { get; init; }

    /// <summary>
    /// The position of the player.
    /// </summary>
    public IReadOnlyList<double> Position { get; init; } = Array.Empty<double>();

    /// <summary>
    /// The position of the spawn point of the world.
    /// </summary>
    public IReadOnlyList<double> SpawnPosition { get; init; } = Array.Empty<double>();

    /// <summary>
    /// The position of the last death of the player, when it is known.
    /// </summary>
    public IReadOnlyList<double> LastDeathPosition { get; init; } = Array.Empty<double>();
}

/// <summary>
/// The information about a world, read from its level data.
/// </summary>
public sealed class WorldDetails
{
    /// <summary>
    /// The name of the world.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The version of the game the world was played with.
    /// </summary>
    public string GameVersion { get; init; } = string.Empty;

    /// <summary>
    /// The date the world was played at the last time.
    /// </summary>
    public DateTime? LastPlayed { get; init; }

    /// <summary>
    /// The seed of the world.
    /// </summary>
    public long? Seed { get; init; }

    /// <summary>
    /// The time the world was played, in ticks.
    /// </summary>
    public long Time { get; init; }

    /// <summary>
    /// The difficulty of the world: peaceful, easy, normal or hard.
    /// </summary>
    public string Difficulty { get; init; } = "normal";

    /// <summary>
    /// Whether the commands are allowed in the world.
    /// </summary>
    public bool AllowCheats { get; init; }

    /// <summary>
    /// Whether the structures are generated in the world.
    /// </summary>
    public bool GenerateStructures { get; init; } = true;

    /// <summary>
    /// The position of the spawn point of the world.
    /// </summary>
    public IReadOnlyList<double> SpawnPosition { get; init; } = Array.Empty<double>();

    /// <summary>
    /// The information about the player of the world, when it is known.
    /// </summary>
    public PlayerDetails? Player { get; init; }
}

/// <summary>
/// Reads the information about a world from its level data, like the world
/// information page of Java HMCL.
/// </summary>
public static class WorldInfoReader
{
    /// <summary>
    /// Reads the information about the world stored in the given directory.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <returns>
    /// The information about the world, or null when the level data cannot be
    /// read.
    /// </returns>
    public static WorldDetails? Read(string worldDirectory)
    {
        try
        {
            var levelData = Path.Combine(worldDirectory, "level.dat");
            if (!File.Exists(levelData))
                return null;

            var file = NbtFile.Load(levelData);
            if (file?.Root is not NbtCompoundTag root)
                return null;

            var data = GetCompound(root, "Data");
            var player = GetCompound(root, "Player");

            var details = new WorldDetails
            {
                Name = GetString(data, "LevelName") ?? Path.GetFileName(worldDirectory) ?? string.Empty,
                GameVersion = GetVersion(data),
                LastPlayed = GetDate(data),
                Seed = GetSeed(data),
                Time = GetLong(data, "Time") ?? 0,
                Difficulty = GetDifficulty(data),
                AllowCheats = GetBoolean(data, "allowCommands") ?? false,
                GenerateStructures = GetGenerateStructures(data) ?? true,
                SpawnPosition = GetDoubles(data, "SpawnX", "SpawnY", "SpawnZ"),
                Player = player is null ? null : ReadPlayer(player)
            };

            return details;
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException or InvalidOperationException or FormatException)
        {
            return null;
        }
    }

    /// <summary>
    /// Reads the information about the player of a world.
    /// </summary>
    /// <param name="player">The player data of the level data.</param>
    /// <returns>The information about the player.</returns>
    private static PlayerDetails ReadPlayer(NbtCompoundTag player) => new()
    {
        GameMode = GetGameMode(GetInt(player, "GameType") ?? 0),
        Health = GetDouble(player, "Health") ?? 0,
        FoodLevel = GetInt(player, "Food") ?? 0,
        Saturation = GetFloat(player, "Saturation") ?? 0,
        ExperienceLevel = GetInt(player, "XPLevel") ?? 0,
        Position = GetDoubleList(player, "Pos"),
        SpawnPosition = GetDoubles(player, "SpawnX", "SpawnY", "SpawnZ"),
        LastDeathPosition = GetDoubleList(player, "LastDeathPos")
    };

    /// <summary>
    /// Returns the compound tag with the given name, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The compound tag, or null when it is missing.</returns>
    private static NbtCompoundTag? GetCompound(NbtCompoundTag? compound, string name)
        => compound is not null && compound.Children.TryGetValue(name, out var tag) ? tag as NbtCompoundTag : null;

    /// <summary>
    /// Returns the string value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static string? GetString(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        return compound.Children.TryGetValue(name, out var tag) && tag is NbtStringTag text
            ? text.Value
            : null;
    }

    /// <summary>
    /// Returns the integer value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static int? GetInt(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        if (!compound.Children.TryGetValue(name, out var tag))
            return null;

        return tag switch
        {
            NbtIntTag intValue => intValue.Value,
            NbtShortTag shortValue => shortValue.Value,
            NbtByteTag byteValue => byteValue.Value,
            NbtLongTag longValue => (int)longValue.Value,
            _ => null
        };
    }

    /// <summary>
    /// Returns the long value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static long? GetLong(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        if (!compound.Children.TryGetValue(name, out var tag))
            return null;

        return tag switch
        {
            NbtLongTag longValue => longValue.Value,
            NbtIntTag intValue => intValue.Value,
            NbtShortTag shortValue => shortValue.Value,
            NbtByteTag byteValue => byteValue.Value,
            _ => null
        };
    }

    /// <summary>
    /// Returns the floating point value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static double? GetDouble(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        if (!compound.Children.TryGetValue(name, out var tag))
            return null;

        return tag switch
        {
            NbtDoubleTag doubleValue => doubleValue.Value,
            NbtFloatTag floatValue => floatValue.Value,
            NbtIntTag intValue => intValue.Value,
            NbtLongTag longValue => longValue.Value,
            _ => null
        };
    }

    /// <summary>
    /// Returns the single precision value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static float? GetFloat(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        if (!compound.Children.TryGetValue(name, out var tag))
            return null;

        return tag switch
        {
            NbtFloatTag floatValue => floatValue.Value,
            NbtDoubleTag doubleValue => (float)doubleValue.Value,
            NbtIntTag intValue => intValue.Value,
            _ => null
        };
    }

    /// <summary>
    /// Returns the boolean value of a member, or null.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null.</returns>
    private static bool? GetBoolean(NbtCompoundTag? compound, string name)
    {
        if (compound is null)
            return null;

        if (!compound.Children.TryGetValue(name, out var tag))
            return null;

        return tag switch
        {
            NbtByteTag byteValue => byteValue.Value != 0,
            NbtIntTag intValue => intValue.Value != 0,
            NbtLongTag longValue => longValue.Value != 0,
            _ => null
        };
    }

    /// <summary>
    /// Returns the values of three members, as the coordinates of a position.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="x">The name of the member holding the first coordinate.</param>
    /// <param name="y">The name of the member holding the second coordinate.</param>
    /// <param name="z">The name of the member holding the third coordinate.</param>
    /// <returns>The position, empty when it is unknown.</returns>
    private static IReadOnlyList<double> GetDoubles(NbtCompoundTag? compound, string x, string y, string z)
    {
        var dx = GetDouble(compound, x);
        var dy = GetDouble(compound, y);
        var dz = GetDouble(compound, z);

        if (dx is null || dy is null || dz is null)
            return Array.Empty<double>();

        return new[] { dx.Value, dy.Value, dz.Value };
    }

    /// <summary>
    /// Returns the values of a list of doubles, empty when it is unknown.
    /// </summary>
    /// <param name="compound">The compound to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The values of the list.</returns>
    private static IReadOnlyList<double> GetDoubleList(NbtCompoundTag? compound, string name)
    {
        if (compound is null || !compound.Children.TryGetValue(name, out var tag) ||
            tag is not NbtListTag list)
        {
            return Array.Empty<double>();
        }

        var values = new List<double>(list.Children.Count);
        foreach (var child in list.Children)
        {
            if (child is NbtDoubleTag doubleValue)
                values.Add(doubleValue.Value);
            else if (child is NbtFloatTag floatValue)
                values.Add(floatValue.Value);
        }

        return values;
    }

    /// <summary>
    /// Returns the version of the game the world was played with.
    /// </summary>
    /// <param name="data">The data of the level data.</param>
    /// <returns>The version of the game.</returns>
    private static string GetVersion(NbtCompoundTag? data)
    {
        if (data is null)
            return string.Empty;

        if (GetCompound(data, "Version") is { } version)
        {
            var name = GetString(version, "Name");
            var id = GetInt(version, "Id");
            if (!string.IsNullOrEmpty(name))
                return name!;

            if (id is not null)
                return id.Value.ToString();
        }

        return string.Empty;
    }

    /// <summary>
    /// Returns the date the world was played at the last time.
    /// </summary>
    /// <param name="data">The data of the level data.</param>
    /// <returns>The date, or null when it is unknown.</returns>
    private static DateTime? GetDate(NbtCompoundTag? data)
    {
        var value = GetLong(data, "LastPlayed");
        if (value is null || value.Value <= 0)
            return null;

        return DateTimeOffset.FromUnixTimeMilliseconds(value.Value).LocalDateTime;
    }

    /// <summary>
    /// Returns the seed of the world.
    /// </summary>
    /// <param name="data">The data of the level data.</param>
    /// <returns>The seed, or null when it is unknown.</returns>
    private static long? GetSeed(NbtCompoundTag? data)
    {
        if (GetCompound(data, "WorldGenSettings") is { } settings)
        {
            var seed = GetLong(settings, "seed");
            if (seed is not null)
                return seed;
        }

        return null;
    }

    /// <summary>
    /// Returns the difficulty of the world.
    /// </summary>
    /// <param name="data">The data of the level data.</param>
    /// <returns>The difficulty of the world.</returns>
    private static string GetDifficulty(NbtCompoundTag? data)
        => (GetInt(data, "Difficulty") ?? 2) switch
        {
            0 => "peaceful",
            1 => "easy",
            3 => "hard",
            _ => "normal"
        };

    /// <summary>
    /// Returns whether the structures are generated in the world.
    /// </summary>
    /// <param name="data">The data of the level data.</param>
    /// <returns>Whether the structures are generated, or null when it is unknown.</returns>
    private static bool? GetGenerateStructures(NbtCompoundTag? data)
    {
        if (GetCompound(data, "WorldGenSettings") is { } settings)
        {
            var generate = GetBoolean(settings, "generate_features");
            if (generate is not null)
                return generate;
        }

        return null;
    }

    /// <summary>
    /// Returns the name of the game mode of a player.
    /// </summary>
    /// <param name="gameType">The numeric game mode.</param>
    /// <returns>The name of the game mode.</returns>
    private static string GetGameMode(int gameType) => gameType switch
    {
        1 => "creative",
        2 => "adventure",
        3 => "spectator",
        _ => "survival"
    };
}
