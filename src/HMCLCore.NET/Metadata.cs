using System;
using System.IO;

namespace HMCLCore.NET;

/// <summary>
/// Launcher metadata and environment constants.
/// </summary>
public static class Metadata
{
    public const string TITLE = "Hello Minecraft! Launcher";
    public const string FULL_TITLE = "Hello Minecraft! Launcher (HMCL)";
    public const string SHORT_TITLE = "HMCL";
    public const string VERSION = "3.0";

    public const string WINDOWS_APP_USER_MODEL_ID = "org.jackhuang.hmcl";
    public const string DEFAULT_UPDATE_CHANNEL = "stable";

    public static readonly string CurrentDirectory = Directory.GetCurrentDirectory();
    public static readonly string HMCLUserHome = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hmcl");
    public static readonly string HMCLLocalHome = Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "hmcl");
}