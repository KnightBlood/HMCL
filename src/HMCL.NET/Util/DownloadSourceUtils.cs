using System;
using HMCLCore.NET.Download;
using HMCLCore.NET.Setting;

namespace HMCL.NET.Util;

/// <summary>
/// Applies the download sources of the settings to the downloads of the
/// launcher.
/// </summary>
public static class DownloadSourceUtils
{
    /// <summary>
    /// Applies the download sources chosen in the given settings.
    /// </summary>
    /// <param name="settings">The settings holding the chosen sources.</param>
    public static void Apply(LauncherSettings settings)
    {
        var versionListSource = settings.VersionListSource;
        var fileSource = settings.FileDownloadSource;

        // The download provider of the old versions of the launcher is
        // migrated to the new download sources.
        if (settings.DownloadProvider.Contains("BMCLAPI", StringComparison.OrdinalIgnoreCase) &&
            versionListSource == DownloadSource.Default &&
            fileSource == DownloadSource.Default)
        {
            versionListSource = DownloadSource.Mirror;
            fileSource = DownloadSource.Mirror;
        }

        MirrorSource.SetSources(versionListSource, fileSource);
    }
}
