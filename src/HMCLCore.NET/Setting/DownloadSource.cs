namespace HMCLCore.NET.Setting;

/// <summary>
/// The preferred source of the downloads of the launcher.
/// </summary>
public enum DownloadSource
{
    /// <summary>
    /// The source is chosen automatically, following the region of the machine.
    /// </summary>
    Default,

    /// <summary>
    /// The official sources of the game are preferred.
    /// </summary>
    Official,

    /// <summary>
    /// The mirror sources of the game are preferred.
    /// </summary>
    Mirror
}
