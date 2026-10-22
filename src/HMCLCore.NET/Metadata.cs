using System;
using System.IO;

namespace HMCLCore.NET;

/// <summary>
/// Launcher metadata and environment constants.
/// </summary>
public static class Metadata
{
    /// <summary>
    /// The title of the launcher, used in the window title and in the logs.
    /// </summary>
    public const string TITLE = "Hello Minecraft! Launcher";

    /// <summary>
    /// The short title of the launcher.
    /// </summary>
    public const string SHORT_TITLE = "HMCL";

    /// <summary>
    /// The version of the launcher.
    /// </summary>
    public const string VERSION = "3.0";

    /// <summary>
    /// The AppUserModelID that groups the windows of the launcher on the
    /// Windows taskbar.
    /// </summary>
    public const string WINDOWS_APP_USER_MODEL_ID = "org.jackhuang.hmcl";

    /// <summary>
    /// The update channel the launcher follows unless the user selects another.
    /// </summary>
    public const string DEFAULT_UPDATE_CHANNEL = "stable";

    /// <summary>
    /// The directory the launcher was started from.
    /// </summary>
    public static readonly string CurrentDirectory = Directory.GetCurrentDirectory();

    /// <summary>
    /// The per-user home directory of the launcher (`~/.hmcl`).
    /// </summary>
    public static readonly string HMCLUserHome = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hmcl");

    /// <summary>
    /// The local data directory of the launcher, where caches and downloaded
    /// libraries are kept.
    /// </summary>
    public static readonly string HMCLLocalHome = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "hmcl");
}