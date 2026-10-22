using System;

namespace HMCLCore.NET.Game;

/// <summary>
/// Represents a Minecraft game instance/profile.
/// </summary>
public sealed class GameInstance
{
    /// <summary>
    /// The instance ID.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The profile name (display name).
    /// </summary>
    public string Profile { get; init; } = string.Empty;

    /// <summary>
    /// The Minecraft version (e.g., "1.20.1").
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The game directory path.
    /// </summary>
    public string GameDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The instance type (release, snapshot, etc.).
    /// </summary>
    public string Type { get; init; } = "release";

    /// <summary>
    /// The last time the instance was launched.
    /// </summary>
    public DateTime LastLaunch { get; init; }

    /// <summary>
    /// The mod loader inferred from the main class (Vanilla, Fabric, Quilt,
    /// Forge or NeoForge).
    /// </summary>
    public string Loader { get; init; } = "Vanilla";

    /// <summary>
    /// The version of the mod loader of the instance, when it is known.
    /// </summary>
    public string LoaderVersion { get; init; } = string.Empty;

    /// <summary>
    /// The last launch time formatted for display.
    /// </summary>
    public string LastPlayedText => LastLaunch == default
        ? "从未启动"
        : $"最后启动 {LastLaunch.ToLocalTime():yyyy-MM-dd HH:mm}";

    /// <summary>
    /// Whether this is the default/primary instance.
    /// </summary>
    public bool IsDefault { get; init; }

    /// <summary>
    /// The path of the custom icon of the instance, or null when the instance
    /// does not have one.
    /// </summary>
    public string? IconPath { get; init; }

    /// <summary>
    /// Returns a display-friendly name for the instance.
    /// </summary>
    public string GetDisplayName() => string.IsNullOrEmpty(Profile) ? Id : Profile;

    public override string ToString() => GetDisplayName();
}

/// <summary>
/// Unique identifier for a game instance.
/// </summary>
public sealed class GameInstanceID : IEquatable<GameInstanceID>
{
    /// <summary>
    /// The instance ID string value.
    /// </summary>
    public string Value { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new GameInstanceID from a string.
    /// </summary>
    public static GameInstanceID FromString(string value) => new() { Value = value };

    public bool Equals(GameInstanceID? other) => other != null && Value == other.Value;

    public override bool Equals(object? obj) => Equals(obj as GameInstanceID);

    public override int GetHashCode() => Value.GetHashCode();

    public override string ToString() => Value;

    public static bool operator ==(GameInstanceID? left, GameInstanceID? right) =>
        left?.Equals(right) ?? right is null;

    public static bool operator !=(GameInstanceID? left, GameInstanceID? right) =>
        !(left == right);
}

/// <summary>
/// Represents a Minecraft game version, including type and release info.
/// </summary>
public sealed class GameVersion
{
    /// <summary>
    /// The version ID (e.g., "1.20.1").
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The version type (release, snapshot, beta, alpha).
    /// </summary>
    public string Type { get; init; } = "release";

    /// <summary>
    /// The release time.
    /// </summary>
    public DateTime ReleaseTime { get; init; }

    /// <summary>
    /// Whether this is a release version.
    /// </summary>
    public bool IsRelease => Type == "release";

    /// <summary>
    /// Whether this is a snapshot version.
    /// </summary>
    public bool IsSnapshot => Type == "snapshot";
}

/// <summary>
/// Options for launching Minecraft.
/// </summary>
public sealed class LaunchOptions
{
    /// <summary>
    /// The username.
    /// </summary>
    public string Username { get; init; } = string.Empty;

    /// <summary>
    /// The UUID (without hyphens).
    /// </summary>
    public string Uuid { get; init; } = string.Empty;

    /// <summary>
    /// The access token.
    /// </summary>
    public string AccessToken { get; init; } = string.Empty;

    /// <summary>
    /// The client token.
    /// </summary>
    public string ClientToken { get; init; } = string.Empty;

    /// <summary>
    /// The Java executable path.
    /// </summary>
    public string JavaPath { get; init; } = "java";

    /// <summary>
    /// The JVM arguments.
    /// </summary>
    public System.Collections.Generic.List<string> JvmArguments { get; init; } = new();

    /// <summary>
    /// The game arguments.
    /// </summary>
    public System.Collections.Generic.List<string> GameArguments { get; init; } = new();

    /// <summary>
    /// The game directory.
    /// </summary>
    public string GameDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The assets directory.
    /// </summary>
    public string AssetsDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The assets index name.
    /// </summary>
    public string? AssetsIndexName { get; init; }

    /// <summary>
    /// The memory allocation in MB.
    /// </summary>
    public int Memory { get; init; } = 2048;

    /// <summary>
    /// The version name.
    /// </summary>
    public string VersionName { get; init; } = string.Empty;

    /// <summary>
    /// The version type.
    /// </summary>
    public string VersionType { get; init; } = "release";

    /// <summary>
    /// The main class.
    /// </summary>
    public string MainClass { get; init; } = "net.minecraft.client.main.Main";

    /// <summary>
    /// The classpath.
    /// </summary>
    public string Classpath { get; init; } = string.Empty;

    /// <summary>
    /// The natives directory.
    /// </summary>
    public string NativesDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The server address for multiplayer.
    /// </summary>
    public string? ServerAddress { get; init; }

    /// <summary>
    /// The server port for multiplayer.
    /// </summary>
    public int ServerPort { get; init; } = 25565;

    /// <summary>
    /// Whether to enable the demo mode.
    /// </summary>
    public bool Demo { get; init; }

    /// <summary>
    /// Whether to enable quick play.
    /// </summary>
    public bool QuickPlay { get; init; }

    /// <summary>
    /// Returns the JVM arguments as a single string.
    /// </summary>
    public string JvmArgumentsString => string.Join(" ", JvmArguments);

    /// <summary>
    /// Returns the game arguments as a single string.
    /// </summary>
    public string GameArgumentsString => string.Join(" ", GameArguments);
}

/// <summary>
/// Represents a library dependency for Minecraft.
/// </summary>
public sealed class Library
{
    /// <summary>
    /// The name of the library (groupId:artifactId:version).
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The URL to download the library from, if custom.
    /// </summary>
    public string? Url { get; init; }

    /// <summary>
    /// The downloads info for this library.
    /// </summary>
    public LibraryDownloads? Downloads { get; init; }

    /// <summary>
    /// Rules that determine when this library is loaded (e.g., only on Windows).
    /// </summary>
    public System.Collections.Generic.List<CompatibilityRule>? Rules { get; init; }

    /// <summary>
    /// The native classifiers for this library.
    /// </summary>
    public System.Collections.Generic.List<string>? Natives { get; init; }

    /// <summary>
    /// Extract rules for this library.
    /// </summary>
    public System.Collections.Generic.List<ExtractRule>? Extract { get; init; }

    /// <summary>
    /// Returns whether this library should be loaded on the current OS.
    /// </summary>
    /// <remarks>
    /// Native libraries are published per operating system and CPU
    /// architecture. The compatibility rules of the manifest only restrict
    /// the operating system, therefore the architecture of a native library
    /// is additionally verified against its classifier.
    /// </remarks>
    public bool IsApplicableToCurrentOs()
    {
        if (Classifier is { } classifier
            && HMCLCore.NET.Util.NativeClassifier.IsNativeClassifier(classifier)
            && !HMCLCore.NET.Util.NativeClassifier.IsApplicableToCurrentPlatform(classifier))
        {
            return false;
        }

        if (Rules == null || Rules.Count == 0)
            return true;

        var applicable = false;
        foreach (var rule in Rules)
        {
            if (rule.IsApplicable())
                applicable = rule.Action == "allow";
        }
        return applicable;
    }

    /// <summary>
    /// Returns whether this library requires natives.
    /// </summary>
    public bool RequiresNatives => Natives != null && Natives.Count > 0;

    /// <summary>
    /// The classifier of this library, that is the fourth part of its name
    /// (<c>group:artifact:version:classifier</c>), or null when the library
    /// has no classifier.
    /// </summary>
    /// <remarks>
    /// Mojang publishes the native libraries of a version as separate library
    /// entries carrying a classifier such as <c>natives-windows</c>.
    /// </remarks>
    public string? Classifier
    {
        get
        {
            var parts = Name.Split(':');
            return parts.Length >= 4 && !string.IsNullOrEmpty(parts[3]) ? parts[3] : null;
        }
    }

    /// <summary>
    /// Returns whether this library holds native libraries.
    /// </summary>
    public bool IsNative => Classifier is { } classifier
        && HMCLCore.NET.Util.NativeClassifier.IsNativeClassifier(classifier);

    /// <summary>
    /// The group of this library, that is the first part of its name.
    /// </summary>
    public string GroupId
    {
        get
        {
            var parts = Name.Split(':');
            return parts.Length >= 1 ? parts[0] : string.Empty;
        }
    }

    /// <summary>
    /// The artifact of this library, that is the second part of its name.
    /// </summary>
    public string ArtifactId
    {
        get
        {
            var parts = Name.Split(':');
            return parts.Length >= 2 ? parts[1] : string.Empty;
        }
    }

    /// <summary>
    /// The version of this library, that is the third part of its name.
    /// </summary>
    public string Version
    {
        get
        {
            var parts = Name.Split(':');
            return parts.Length >= 3 ? parts[2] : string.Empty;
        }
    }

    /// <summary>
    /// Returns whether this library is the library of the given group and
    /// artifact.
    /// </summary>
    /// <param name="groupId">The group of the library.</param>
    /// <param name="artifactId">The artifact of the library.</param>
    /// <returns>Whether the library is the given one.</returns>
    public bool Is(string groupId, string artifactId)
        => string.Equals(GroupId, groupId, StringComparison.OrdinalIgnoreCase) &&
           string.Equals(ArtifactId, artifactId, StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Returns the local path where this library should be stored.
    /// </summary>
    /// <param name="librariesDirectory">The root directory of the libraries.</param>
    /// <returns>The absolute path of the library file.</returns>
    public string GetLocalPath(string librariesDirectory)
    {
        var parts = Name.Split(':');
        if (parts.Length >= 3)
        {
            var groupId = parts[0].Replace('.', System.IO.Path.DirectorySeparatorChar);
            var artifactId = parts[1];
            var version = parts[2];
            var classifier = parts.Length >= 4 && !string.IsNullOrEmpty(parts[3]) ? $"-{parts[3]}" : string.Empty;
            return System.IO.Path.Combine(
                librariesDirectory, groupId, artifactId, version, $"{artifactId}-{version}{classifier}.jar");
        }
        return System.IO.Path.Combine(librariesDirectory, Name.Replace(':', System.IO.Path.DirectorySeparatorChar) + ".jar");
    }

    /// <summary>
    /// Returns whether this library is applicable to the current platform.
    /// </summary>
    public bool AppliesToCurrentPlatform() => IsApplicableToCurrentOs();
}

/// <summary>
/// Downloads info for a library.
/// </summary>
public sealed class LibraryDownloads
{
    /// <summary>
    /// The artifact download info.
    /// </summary>
    public DownloadInfo? Artifact { get; init; }

    /// <summary>
    /// The classifiers download info (e.g., for natives).
    /// </summary>
    public System.Collections.Generic.Dictionary<string, DownloadInfo>? Classifiers { get; init; }
}

/// <summary>
/// Download information for a file.
/// </summary>
public sealed class DownloadInfo
{
    /// <summary>
    /// The URL to download from.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The SHA1 hash of the file.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// The size of the file in bytes.
    /// </summary>
    public long Size { get; init; }
}

/// <summary>
/// Compatibility rule for libraries.
/// </summary>
public sealed class CompatibilityRule
{
    /// <summary>
    /// The action of the rule ("allow" or "disallow").
    /// </summary>
    public string Action { get; init; } = "allow";

    /// <summary>
    /// The OS conditions for this rule.
    /// </summary>
    public OsRule? Os { get; init; }

    /// <summary>
    /// Returns whether this rule is applicable to the current OS.
    /// </summary>
    public bool IsApplicable()
    {
        if (Os == null)
            return true;

        var osName = HMCLCore.NET.Util.Platform.MojangOsName;
        var arch = HMCLCore.NET.Util.Platform.GetBits(HMCLCore.NET.Util.Platform.CurrentArch) switch
        {
            HMCLCore.NET.Util.Bits.BIT_32 => "x86",
            HMCLCore.NET.Util.Bits.BIT_64 => "x64",
            _ => "unknown"
        };

        if (Os.Name != null && !Os.Name.Equals(osName, StringComparison.OrdinalIgnoreCase))
            return false;

        if (Os.Arch != null && !Os.Arch.Equals(arch, StringComparison.OrdinalIgnoreCase))
            return false;

        return true;
    }

    /// <summary>
    /// Operating system conditions for a rule.
    /// </summary>
    public sealed class OsRule
    {
        /// <summary>
        /// The OS name (windows, linux, macos).
        /// </summary>
        public string? Name { get; init; }

        /// <summary>
        /// The OS architecture (x86, x64).
        /// </summary>
        public string? Arch { get; init; }
    }
}

/// <summary>
/// Extract rule for library natives.
/// </summary>
public sealed class ExtractRule
{
    /// <summary>
    /// The pattern to match files to extract (or exclude, see <see cref="Exclude"/>).
    /// </summary>
    public string Pattern { get; init; } = string.Empty;

    /// <summary>
    /// Whether files matching <see cref="Pattern"/> are excluded from extraction.
    /// </summary>
    public bool Exclude { get; init; }
}
