using System;

namespace HMCLCore.NET.Download;

/// <summary>
/// The concurrency used by the downloads of the launcher.
/// </summary>
/// <remarks>
/// The number of the parallel downloads is chosen by the user in the settings
/// of the launcher; when it is not set, it is chosen from the number of the
/// processors of the machine.
/// </remarks>
public static class DownloadConcurrency
{
    private static int _limit;

    /// <summary>
    /// The number of the parallel downloads; zero means that the number is
    /// chosen automatically.
    /// </summary>
    public static int Limit
    {
        get => _limit;
        set => _limit = Math.Max(0, value);
    }

    /// <summary>
    /// The number of the parallel downloads actually used.
    /// </summary>
    public static int EffectiveLimit => Limit > 0
        ? Math.Clamp(Limit, 1, 64)
        : Math.Clamp(Environment.ProcessorCount * 2, 1, 16);

    /// <summary>
    /// Applies the download settings of the launcher.
    /// </summary>
    /// <param name="auto">Whether the number of the downloads is chosen automatically.</param>
    /// <param name="threads">The number of the downloads chosen by the user.</param>
    public static void Apply(bool auto, int threads)
        => Limit = auto ? 0 : Math.Clamp(threads, 1, 64);
}
