using System;
using System.Collections.Generic;

namespace HMCLCore.NET.Util;

/// <summary>
/// Helpers for selecting the native library classifier that matches the
/// current platform.
/// </summary>
/// <remarks>
/// Mojang publishes the native libraries of a version as separate library
/// entries whose classifier encodes the operating system and, optionally, the
/// CPU architecture, for example <c>natives-windows</c>,
/// <c>natives-windows-arm64</c> or <c>natives-macos-arm64</c>. The
/// compatibility rules of the version manifest only restrict the operating
/// system, so every Windows variant would be accepted on Windows; the
/// architecture therefore has to be derived from the classifier itself.
/// </remarks>
public static class NativeClassifier
{
    /// <summary>
    /// The native classifiers of the current platform, ordered by preference.
    /// The first entry is the classifier matching the current architecture
    /// exactly, the remaining entries are the fallbacks of the same platform.
    /// </summary>
    public static IReadOnlyList<string> Candidates { get; } = BuildCandidates();

    /// <summary>
    /// The preferred native classifier of the current platform, for example
    /// <c>natives-windows</c> on 64-bit Windows.
    /// </summary>
    public static string Current => Candidates.Count > 0 ? Candidates[0] : string.Empty;

    /// <summary>
    /// Returns whether the given classifier denotes a native library.
    /// </summary>
    /// <param name="classifier">The classifier to inspect; may be null.</param>
    /// <returns>Whether the classifier is a native classifier.</returns>
    public static bool IsNativeClassifier(string? classifier)
    {
        if (string.IsNullOrEmpty(classifier))
            return false;

        return classifier.StartsWith("natives-", StringComparison.OrdinalIgnoreCase)
            || classifier.StartsWith("native-", StringComparison.OrdinalIgnoreCase)
            || IsKnownOsName(StripOsPrefix(classifier));
    }

    /// <summary>
    /// Returns whether the given classifier targets the current platform, that
    /// is the current operating system and the current CPU architecture.
    /// </summary>
    /// <param name="classifier">The classifier to inspect; may be null.</param>
    /// <returns>Whether the classifier applies to the current platform.</returns>
    public static bool IsApplicableToCurrentPlatform(string? classifier)
    {
        if (!TryParse(classifier, out var os, out var arch))
            return false;

        if (os != Platform.CurrentOs)
            return false;

        var currentArch = Platform.SystemArch;
        if (arch == Arch.UNKNOWN)
            return currentArch == DefaultArch(os);

        return arch == currentArch;
    }

    /// <summary>
    /// Returns whether the given classifier targets the current operating
    /// system, regardless of the CPU architecture.
    /// </summary>
    /// <param name="classifier">The classifier to inspect; may be null.</param>
    /// <returns>Whether the classifier applies to the current operating system.</returns>
    public static bool MatchesOperatingSystem(string? classifier)
    {
        if (!TryParse(classifier, out var os, out _))
            return false;

        return os == Platform.CurrentOs;
    }

    /// <summary>
    /// Parses a native classifier into its operating system and architecture.
    /// </summary>
    /// <param name="classifier">The classifier to parse; may be null.</param>
    /// <param name="os">The operating system of the classifier.</param>
    /// <param name="arch">The architecture of the classifier, or
    /// <see cref="Arch.UNKNOWN"/> when the classifier does not restrict the
    /// architecture.</param>
    /// <returns>Whether the classifier could be parsed.</returns>
    public static bool TryParse(string? classifier, out OsType os, out Arch arch)
    {
        os = OsType.Unknown;
        arch = Arch.UNKNOWN;

        if (string.IsNullOrEmpty(classifier))
            return false;

        var value = StripOsPrefix(classifier.Trim());

        var separator = value.IndexOf('-');
        var osName = separator < 0 ? value : value[..separator];
        var archName = separator < 0 ? null : value[(separator + 1)..];

        os = ParseOsName(osName);
        if (os == OsType.Unknown)
            return false;

        if (!string.IsNullOrEmpty(archName))
            arch = ParseArchName(archName);

        return true;
    }

    /// <summary>
    /// Returns the architecture a native classifier without an architecture
    /// suffix refers to.
    /// </summary>
    /// <param name="os">The operating system of the classifier.</param>
    /// <returns>The default architecture of the operating system.</returns>
    public static Arch DefaultArch(OsType os) => os switch
    {
        OsType.Windows or OsType.Linux or OsType.MacOs or OsType.FreeBsd => Arch.X86_64,
        _ => Arch.UNKNOWN
    };

    /// <summary>
    /// Removes the <c>natives-</c> or <c>native-</c> prefix of a classifier.
    /// </summary>
    /// <param name="value">The classifier to strip.</param>
    /// <returns>The classifier without its native prefix.</returns>
    private static string StripOsPrefix(string value)
    {
        if (value.StartsWith("natives-", StringComparison.OrdinalIgnoreCase))
            return value["natives-".Length..];
        if (value.StartsWith("native-", StringComparison.OrdinalIgnoreCase))
            return value["native-".Length..];
        return value;
    }

    /// <summary>
    /// Returns whether the given name is one of the operating system names
    /// used by the native classifiers.
    /// </summary>
    /// <param name="value">The name to check.</param>
    /// <returns>Whether the name is a known operating system name.</returns>
    private static bool IsKnownOsName(string value) =>
        value.Equals("windows", StringComparison.OrdinalIgnoreCase)
        || value.Equals("linux", StringComparison.OrdinalIgnoreCase)
        || value.Equals("macos", StringComparison.OrdinalIgnoreCase)
        || value.Equals("osx", StringComparison.OrdinalIgnoreCase)
        || value.Equals("freebsd", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// Parses the operating system name of a native classifier.
    /// </summary>
    /// <param name="value">The name to parse.</param>
    /// <returns>The operating system, or <see cref="OsType.Unknown"/>.</returns>
    private static OsType ParseOsName(string value)
    {
        if (value.Equals("windows", StringComparison.OrdinalIgnoreCase))
            return OsType.Windows;
        if (value.Equals("linux", StringComparison.OrdinalIgnoreCase))
            return OsType.Linux;
        if (value.Equals("macos", StringComparison.OrdinalIgnoreCase)
            || value.Equals("osx", StringComparison.OrdinalIgnoreCase)
            || value.Equals("darwin", StringComparison.OrdinalIgnoreCase))
            return OsType.MacOs;
        if (value.Equals("freebsd", StringComparison.OrdinalIgnoreCase))
            return OsType.FreeBsd;
        return OsType.Unknown;
    }

    /// <summary>
    /// Parses the architecture name of a native classifier.
    /// </summary>
    /// <param name="value">The name to parse.</param>
    /// <returns>The architecture, or <see cref="Arch.UNKNOWN"/>.</returns>
    private static Arch ParseArchName(string value)
    {
        if (value.Equals("arm64hf", StringComparison.OrdinalIgnoreCase))
            return Arch.ARM64;

        return Platform.ParseArchName(value);
    }

    /// <summary>
    /// Builds the ordered list of the native classifiers of the current
    /// platform.
    /// </summary>
    /// <returns>The classifiers, most specific first.</returns>
    private static IReadOnlyList<string> BuildCandidates()
    {
        var os = Platform.CurrentOs;
        if (os == OsType.Unknown)
            return Array.Empty<string>();

        var osName = os switch
        {
            OsType.Windows => "windows",
            OsType.Linux => "linux",
            OsType.MacOs => "macos",
            OsType.FreeBsd => "freebsd",
            _ => "universal"
        };

        var arch = Platform.SystemArch;
        var archName = arch switch
        {
            Arch.X86_64 => "x86_64",
            Arch.ARM64 => "arm64",
            Arch.X86 => "x86",
            Arch.ARM32 => "arm32",
            _ => null
        };

        var candidates = new List<string>();
        if (archName != null)
        {
            candidates.Add($"natives-{osName}-{archName}");
            candidates.Add($"natives-{osName}-{(arch == Arch.X86_64 ? "64" : "32")}");
        }

        candidates.Add($"natives-{osName}");

        return candidates;
    }
}
