namespace HMCLCore.NET.Modpack;

/// <summary>
/// The type of the modpack exported by the launcher.
/// </summary>
public enum ModpackExportType
{
    /// <summary>
    /// A Modrinth modpack, stored in a <c>.mrpack</c> archive.
    /// </summary>
    Modrinth,

    /// <summary>
    /// A CurseForge modpack, stored in a <c>.zip</c> archive.
    /// </summary>
    CurseForge,

    /// <summary>
    /// A MCBBS modpack, stored in a <c>.zip</c> archive.
    /// </summary>
    Mcbbs,

    /// <summary>
    /// A server pack, stored in a <c>.zip</c> archive holding the files of the
    /// instance, ready to be used by a server.
    /// </summary>
    Server
}
