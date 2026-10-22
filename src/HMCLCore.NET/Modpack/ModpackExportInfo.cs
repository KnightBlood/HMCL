namespace HMCLCore.NET.Modpack;

/// <summary>
/// The information of an exported modpack, shown by the export wizard and
/// written into the manifest of the exported pack.
/// </summary>
public sealed class ModpackExportInfo
{
    /// <summary>
    /// The name of the modpack.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// The version of the modpack.
    /// </summary>
    public string Version { get; set; } = "1.0";

    /// <summary>
    /// The author of the modpack.
    /// </summary>
    public string Author { get; set; } = string.Empty;

    /// <summary>
    /// The description of the modpack.
    /// </summary>
    public string Description { get; set; } = string.Empty;

    /// <summary>
    /// The address of the page of the modpack.
    /// </summary>
    public string Url { get; set; } = string.Empty;

    /// <summary>
    /// The address of the file API of the modpack.
    /// </summary>
    public string FileApi { get; set; } = string.Empty;

    /// <summary>
    /// The minimum memory of the game, in mebibytes; zero means that the
    /// memory of the game is not limited.
    /// </summary>
    public int MinMemory { get; set; }

    /// <summary>
    /// The arguments added to the command line of the game.
    /// </summary>
    public string LaunchArguments { get; set; } = string.Empty;

    /// <summary>
    /// The arguments added to the command line of the Java runtime.
    /// </summary>
    public string JavaArguments { get; set; } = string.Empty;

    /// <summary>
    /// Whether the modpack has to be updated even when its version did not
    /// change.
    /// </summary>
    public bool ForceUpdate { get; set; }

    /// <summary>
    /// Whether the launcher is packed with the modpack.
    /// </summary>
    public bool PackWithLauncher { get; set; }

    /// <summary>
    /// Whether the files of the modpack are not published on the file API.
    /// </summary>
    public bool NoCreateRemoteFiles { get; set; }

    /// <summary>
    /// Whether the files of CurseForge are not published on the file API.
    /// </summary>
    public bool SkipCurseForgeRemoteFiles { get; set; }
}
