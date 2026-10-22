using System;
using System.Collections.Generic;
using HMCLCore.NET.Setting;

namespace HMCLCore.NET.Game;

/// <summary>
/// Represents a Minecraft instance manifest, including version info and assets.
/// </summary>
public sealed class GameInstanceManifest
{
    /// <summary>
    /// The version ID (e.g., "1.20.1").
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The last time this instance was launched, in UTC.
    /// </summary>
    public DateTime LastPlayed { get; set; }

    /// <summary>
    /// The main class of the game.
    /// </summary>
    public string MainClass { get; init; } = "net.minecraft.client.main.Main";

    /// <summary>
    /// The minimum Java version required.
    /// </summary>
    public int JavaVersion { get; init; }

    /// <summary>
    /// The arguments for launching the game.
    /// </summary>
    public LaunchArguments Arguments { get; init; } = new();

    /// <summary>
    /// The libraries required by this version.
    /// </summary>
    public List<Library> Libraries { get; init; } = new();

    /// <summary>
    /// The asset index information.
    /// </summary>
    public AssetIndexInfo? AssetIndex { get; init; }

    /// <summary>
    /// The logging configuration.
    /// </summary>
    public LoggingInfo? LoggingConfig { get; init; }

    /// <summary>
    /// The downloadable game files (primarily the client JAR).
    /// </summary>
    public GameDownloads? Downloads { get; init; }

    /// <summary>
    /// Whether this version allows demo mode.
    /// </summary>
    public bool Demo { get; init; }

    /// <summary>
    /// Whether custom resolution is allowed.
    /// </summary>
    public bool CustomResolution { get; init; }

    /// <summary>
    /// Instance-specific game settings (overrides the global preset).
    /// </summary>
    public GameSettingsInstance GameSettings { get; set; } = new();

    /// <summary>
    /// The kind of icon shown for the instance.
    /// </summary>
    public GameInstanceIconType IconType { get; set; } = GameInstanceIconType.Default;

    /// <summary>
    /// The patches that were applied to this manifest, for example the patch
    /// of a mod loader.
    /// </summary>
    public List<GameInstancePatch> Patches { get; init; } = new();

    /// <summary>
    /// Returns whether this manifest is valid for launching.
    /// </summary>
    public bool IsValid => !string.IsNullOrEmpty(Version) && Libraries.Count > 0;

    /// <summary>
    /// Applies the patches of the manifest in the order of their priority.
    /// </summary>
    /// <returns>
    /// The manifest with all of its patches applied, or the manifest itself
    /// when it has no patches.
    /// </returns>
    public GameInstanceManifest ApplyPatches()
    {
        if (Patches.Count == 0)
            return this;

        var result = this;
        foreach (var patch in Patches.OrderBy(patch => patch.EffectivePriority))
            result = patch.ApplyTo(result);

        return result;
    }

    /// <summary>
    /// Asset index information.
    /// </summary>
    public sealed class AssetIndexInfo
    {
        /// <summary>
        /// The asset index ID (e.g., "1.20.1") passed to the game via --assetIndex.
        /// </summary>
        public string Id { get; init; } = string.Empty;

        /// <summary>
        /// The SHA1 hash of the asset index.
        /// </summary>
        public string Sha1 { get; init; } = string.Empty;

        /// <summary>
        /// The size of the asset index.
        /// </summary>
        public long Size { get; init; }

        /// <summary>
        /// The total size of all assets.
        /// </summary>
        public long TotalSize { get; init; }

        /// <summary>
        /// The URL to download the asset index.
        /// </summary>
        public string Url { get; init; } = string.Empty;
    }

    /// <summary>
    /// Launch arguments for Minecraft.
    /// </summary>
    public sealed class LaunchArguments
    {
        /// <summary>
        /// The JVM arguments.
        /// </summary>
        public List<string> Jvm { get; init; } = new();

        /// <summary>
        /// The game arguments.
        /// </summary>
        public List<string> Game { get; init; } = new();
    }

    /// <summary>
    /// Downloadable game files layer of the manifest ("downloads" object).
    /// </summary>
    public sealed class GameDownloads
    {
        /// <summary>
        /// The client JAR file info.
        /// </summary>
        public ClientFile? Client { get; init; }
    }

    /// <summary>
    /// Info for a single downloadable file (client JAR / server JAR).
    /// </summary>
    public sealed class ClientFile
    {
        /// <summary>
        /// The download URL.
        /// </summary>
        public string Url { get; init; } = string.Empty;

        /// <summary>
        /// The SHA1 hash of the file.
        /// </summary>
        public string Sha1 { get; init; } = string.Empty;

        /// <summary>
        /// The file size in bytes.
        /// </summary>
        public long Size { get; init; }
    }

    /// <summary>
    /// Logging configuration.
    /// </summary>
    public sealed class Logging
    {
        /// <summary>
        /// The log file path.
        /// </summary>
        public string? File { get; init; }

        /// <summary>
        /// The logging type.
        /// </summary>
        public string? Type { get; init; }
    }
}

/// <summary>
/// Represents a Minecraft asset index.
/// </summary>
public sealed class AssetIndex
{
    /// <summary>
    /// The SHA1 hash of the asset index.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// The size of the asset index file.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// The total size of all assets referenced by this index.
    /// </summary>
    public long TotalSize { get; init; }

    /// <summary>
    /// The URL to download the asset index.
    /// </summary>
    public string Url { get; init; } = string.Empty;
}

/// <summary>
/// Represents a Minecraft world.
/// </summary>
public sealed class World
{
    /// <summary>
    /// The world directory name.
    /// </summary>
    public string DirectoryName { get; init; } = string.Empty;

    /// <summary>
    /// The world level name (display name).
    /// </summary>
    public string LevelName { get; init; } = string.Empty;

    /// <summary>
    /// The world icon path.
    /// </summary>
    public string? IconPath { get; init; }

    /// <summary>
    /// The last time the world was played.
    /// </summary>
    public DateTime LastPlayed { get; init; } = DateTime.UtcNow;

    /// <summary>
    /// The world size in bytes.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// Whether this world is locked by another process.
    /// </summary>
    public bool IsLocked { get; init; }

    /// <summary>
    /// The world seed.
    /// </summary>
    public string? Seed { get; init; }

    /// <summary>
    /// Returns a display string for the world.
    /// </summary>
    public override string ToString() => LevelName;
}

/// <summary>
/// Exception thrown when a world is locked by another process.
/// </summary>
public sealed class WorldLockedException : Exception
{
    public WorldLockedException(string message) : base(message) { }
    public WorldLockedException(string message, Exception innerException) : base(message, innerException) { }
}

/// <summary>
/// Represents a Minecraft logging configuration.
/// </summary>
public sealed class LoggingInfo
{
    /// <summary>
    /// The logging type (e.g., "log4j2").
    /// </summary>
    public string Type { get; init; } = string.Empty;

    /// <summary>
    /// The logging file path.
    /// </summary>
    public string? File { get; init; }

    /// <summary>
    /// Returns whether logging is configured.
    /// </summary>
    public bool IsConfigured => !string.IsNullOrEmpty(Type);
}
