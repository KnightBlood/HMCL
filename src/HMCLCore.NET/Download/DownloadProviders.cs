using System;

namespace HMCLCore.NET.Download;

/// <summary>
/// Download provider for Mojang's official servers.
/// </summary>
public sealed class MojangDownloadProvider : DownloadProvider
{
    /// <summary>
    /// The name of this download provider.
    /// </summary>
    public override string Name => "Mojang Official";

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public override bool CanHandle(Uri url)
    {
        var host = url.Host.ToLowerInvariant();
        return host.Contains("mojang.com") ||
               host.Contains("minecraft.net") ||
               host.Contains("launcher.mojang.com") ||
               host.Contains("resources.download.minecraft.net");
    }
}

/// <summary>
/// Download provider for the BMCLAPI mirror.
/// </summary>
public sealed class BmclapiDownloadProvider : DownloadProvider
{
    /// <summary>
    /// The name of this download provider.
    /// </summary>
    public override string Name => "BMCLAPI";

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public override bool CanHandle(Uri url)
    {
        var host = url.Host.ToLowerInvariant();
        return host.Contains("bmclapi") || host.Contains("bmc");
    }
}

/// <summary>
/// Download provider for CurseForge.
/// </summary>
public sealed class CurseForgeDownloadProvider : DownloadProvider
{
    /// <summary>
    /// The name of this download provider.
    /// </summary>
    public override string Name => "CurseForge";

    /// <summary>
    /// The user agent sent with the requests of the provider; CurseForge
    /// rejects the requests without one.
    /// </summary>
    protected override string? UserAgent => "HMCL.NET/1.0";

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public override bool CanHandle(Uri url)
    {
        var host = url.Host.ToLowerInvariant();
        return host.Contains("curseforge.com") ||
               host.Contains("mediaforgeh.com") ||
               host.Contains("edge.forgecdn.net");
    }
}

/// <summary>
/// Download provider for Modrinth.
/// </summary>
public sealed class ModrinthDownloadProvider : DownloadProvider
{
    /// <summary>
    /// The name of this download provider.
    /// </summary>
    public override string Name => "Modrinth";

    /// <summary>
    /// The user agent sent with the requests of the provider; Modrinth rejects
    /// the requests without one.
    /// </summary>
    protected override string? UserAgent => "HMCL.NET/1.0";

    /// <summary>
    /// Returns whether this provider can handle the specified URL.
    /// </summary>
    /// <param name="url">The address of the file.</param>
    /// <returns>Whether the provider handles the address.</returns>
    public override bool CanHandle(Uri url)
    {
        var host = url.Host.ToLowerInvariant();
        return host.Contains("modrinth.com") ||
               host.Contains("modrinth");
    }
}
