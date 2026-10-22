using System;
using System.Collections.Generic;
using System.IO;

namespace HMCLCore.NET.Game;

/// <summary>
/// Builds the library list for a game instance.
/// </summary>
public sealed class LibraryBuilder
{
    private readonly List<Library> _libraries = new();

    /// <summary>
    /// Adds a library to the build.
    /// </summary>
    public LibraryBuilder AddLibrary(Library library)
    {
        _libraries.Add(library);
        return this;
    }

    /// <summary>
    /// Adds multiple libraries.
    /// </summary>
    public LibraryBuilder AddLibraries(IEnumerable<Library> libraries)
    {
        foreach (var lib in libraries)
            _libraries.Add(lib);
        return this;
    }

    /// <summary>
    /// Builds the classpath string from all libraries.
    /// </summary>
    public string BuildClasspath(string librariesDirectory)
    {
        var paths = new List<string>();
        foreach (var library in _libraries)
        {
            if (!library.IsApplicableToCurrentOs())
                continue;

            var path = library.GetLocalPath(librariesDirectory);
            if (File.Exists(path))
                paths.Add(path);
        }
        return string.Join(Path.PathSeparator, paths);
    }

    /// <summary>
    /// Returns all libraries that apply to the current OS.
    /// </summary>
    public List<Library> GetApplicableLibraries()
    {
        var applicable = new List<Library>();
        foreach (var library in _libraries)
        {
            if (library.IsApplicableToCurrentOs())
                applicable.Add(library);
        }
        return applicable;
    }

    /// <summary>
    /// Returns whether any libraries require natives extraction.
    /// </summary>
    public bool RequiresNatives => _libraries.Exists(l => l.RequiresNatives);
}

/// <summary>
/// Represents an OS restriction for game arguments or libraries.
/// </summary>
public sealed class OSRestriction
{
    /// <summary>
    /// The OS name restriction.
    /// </summary>
    public string? Os { get; init; }

    /// <summary>
    /// The OS architecture restriction.
    /// </summary>
    public string? Arch { get; init; }

    /// <summary>
    /// Returns whether this restriction applies to the current OS.
    /// </summary>
    public bool IsApplicable()
    {
        if (Os != null)
        {
            var osName = HMCLCore.NET.Util.Platform.MojangOsName;
            if (!Os.Equals(osName, StringComparison.OrdinalIgnoreCase))
                return false;
        }

        if (Arch != null)
        {
            var arch = HMCLCore.NET.Util.Platform.GetBits(HMCLCore.NET.Util.Platform.CurrentArch) switch
            {
                HMCLCore.NET.Util.Bits.BIT_32 => "x86",
                HMCLCore.NET.Util.Bits.BIT_64 => "x64",
                _ => "unknown"
            };
            if (!Arch.Equals(arch, StringComparison.OrdinalIgnoreCase))
                return false;
        }

        return true;
    }
}

/// <summary>
/// Process priority for the game process.
/// </summary>
public enum ProcessPriority
{
    Idle,
    BelowNormal,
    Normal,
    AboveNormal,
    High,
    RealTime
}

/// <summary>
/// Quick play type for the game.
/// </summary>
public enum QuickPlayType
{
    None,
    SinglePlayer,
    Multiplayer,
    Realms
}

/// <summary>
/// Quick play option configuration.
/// </summary>
public sealed class QuickPlayOption
{
    /// <summary>
    /// The type of quick play.
    /// </summary>
    public QuickPlayType Type { get; init; } = QuickPlayType.None;

    /// <summary>
    /// The server address for multiplayer quick play.
    /// </summary>
    public string? ServerAddress { get; init; }

    /// <summary>
    /// The world name for single player quick play.
    /// </summary>
    public string? WorldName { get; init; }
}

/// <summary>
/// Renderer settings for Minecraft.
/// </summary>
public enum Renderer
{
    Auto,
    OpenGL,
    DirectX
}

/// <summary>
/// Java version constraint for game compatibility.
/// </summary>
public sealed class JavaVersionConstraint
{
    /// <summary>
    /// The minimum Java major version.
    /// </summary>
    public int MinMajor { get; init; }

    /// <summary>
    /// The maximum Java major version.
    /// </summary>
    public int? MaxMajor { get; init; }

    /// <summary>
    /// Returns whether the specified Java version satisfies this constraint.
    /// </summary>
    public bool IsSatisfiedBy(int javaMajorVersion)
    {
        if (javaMajorVersion < MinMajor)
            return false;
        if (MaxMajor.HasValue && javaMajorVersion > MaxMajor.Value)
            return false;
        return true;
    }
}
