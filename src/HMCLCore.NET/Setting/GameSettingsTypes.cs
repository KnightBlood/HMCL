using System;
using System.Collections.Generic;
using System.Security.Cryptography;
using System.Text;

namespace HMCLCore.NET.Setting;

/// <summary>
/// Selects how HMCL resolves the Java runtime used to launch a game.
/// </summary>
public enum JavaVersionType
{
    /// <summary>
    /// Automatically selects a suitable Java runtime.
    /// </summary>
    Auto = 0,

    /// <summary>
    /// Selects an installed Java runtime by major version.
    /// </summary>
    Version = 1,

    /// <summary>
    /// Selects a runtime from detected Java runtimes.
    /// </summary>
    Detected = 2,

    /// <summary>
    /// Uses a user-provided Java executable path.
    /// </summary>
    Custom = 3
}

/// <summary>
/// Process priority for the launched game.
/// </summary>
public enum ProcessPriority
{
    High,
    AboveNormal,
    Normal,
    BelowNormal,
    Low
}

/// <summary>
/// Launcher visibility behavior after the game starts.
/// </summary>
public enum LauncherVisibility
{
    Hide,
    Close,
    Keep,
    HideAndReopen,
    CloseAndReopen
}

/// <summary>
/// Graphics rendering backend preference.
/// </summary>
public enum GraphicsBackend
{
    Auto,
    OpenGL,
    Vulkan,
    Software
}

/// <summary>
/// Quick play mode controlling how the game enters a session directly on launch.
/// </summary>
public enum QuickPlayMode
{
    /// <summary>
    /// No quick play; the game shows the title screen.
    /// </summary>
    None = 0,

    /// <summary>
    /// Directly enters the single player save given by the quick play argument.
    /// </summary>
    Singleplayer = 1,

    /// <summary>
    /// Directly joins the multiplayer server given by the quick play argument.
    /// </summary>
    Multiplayer = 2,

    /// <summary>
    /// Directly enters the Realms world given by the quick play argument.
    /// </summary>
    Realms = 3
}

/// <summary>
/// A reference to a Java runtime detected by HMCL.
/// </summary>
public sealed record DetectedJava(
    string Version,
    string PathHash)
{
    /// <summary>
    /// An empty detected Java reference.
    /// </summary>
    public static readonly DetectedJava Empty = new(string.Empty, string.Empty);

    /// <summary>
    /// Computes a stable hash for the given Java executable path.
    /// </summary>
    public static string HashExistingPath(string? path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return string.Empty;

        try
        {
            var bytes = Encoding.UTF8.GetBytes(path);
            var hash = SHA1.HashData(bytes);
            return Convert.ToHexString(hash);
        }
        catch
        {
            return string.Empty;
        }
    }
}

/// <summary>
/// Base class for game settings that support inheritance with per-property overrides.
/// </summary>
public abstract class GameSettingsBase
{
    /// <summary>
    /// The set of property names that are explicitly overridden by this settings object.
    /// </summary>
    public HashSet<string> OverrideProperties { get; set; } = new(StringComparer.Ordinal);

    /// <summary>
    /// Marks the specified property as overridden.
    /// </summary>
    protected void MarkOverridden(string propertyName)
    {
        if (!string.IsNullOrEmpty(propertyName))
            OverrideProperties.Add(propertyName);
    }

    /// <summary>
    /// Determines whether the specified property is overridden by this settings object.
    /// </summary>
    public bool IsOverridden(string propertyName) => OverrideProperties.Contains(propertyName);
}
