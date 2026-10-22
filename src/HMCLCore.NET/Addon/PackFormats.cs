using System;
using System.Collections.Generic;

namespace HMCLCore.NET.Addon;

/// <summary>
/// Describes how well a resource pack matches the Minecraft version of an instance.
/// </summary>
public enum ResourcePackCompatibility
{
    /// <summary>
    /// The pack can be used with the instance.
    /// </summary>
    Compatible = 0,

    /// <summary>
    /// The pack was built for a newer Minecraft version.
    /// </summary>
    TooNew = 1,

    /// <summary>
    /// The pack was built for an older Minecraft version.
    /// </summary>
    TooOld = 2,

    /// <summary>
    /// The pack metadata is malformed and cannot be interpreted.
    /// </summary>
    Invalid = 3,

    /// <summary>
    /// The pack does not contain a pack.mcmeta file.
    /// </summary>
    MissingPackMeta = 4,

    /// <summary>
    /// The Minecraft version of the instance is unknown, so no compatibility
    /// check could be performed.
    /// </summary>
    MissingGameMeta = 5
}

/// <summary>
/// Maps Minecraft versions to the resource pack format numbers they expect and
/// compares Minecraft version strings numerically.
/// </summary>
public static class ResourcePackFormats
{
    /// <summary>
    /// The oldest Minecraft version that supports resource packs.
    /// </summary>
    public const string LeastSupportedVersion = "1.6.1";

    /// <summary>
    /// The oldest Minecraft version that stores resource pack identifiers with
    /// the "file/" prefix in options.txt.
    /// </summary>
    public const string LeastNewOptionsVersion = "1.13";

    /// <summary>
    /// The known resource pack format numbers, ordered by Minecraft version.
    /// </summary>
    private static readonly (string Version, int Format)[] Formats =
    {
        ("1.6.1", 1),
        ("1.7", 2),
        ("1.8", 3),
        ("1.9", 4),
        ("1.11", 5),
        ("1.13", 6),
        ("1.17", 7),
        ("1.18", 8),
        ("1.19", 9),
        ("1.19.3", 10),
        ("1.19.4", 12),
        ("1.20", 15),
        ("1.20.2", 18),
        ("1.20.3", 26),
        ("1.20.5", 32),
        ("1.21", 34),
        ("1.21.2", 42),
        ("1.21.4", 46),
        ("1.21.5", 55),
        ("1.21.6", 63),
        ("1.21.7", 64)
    };

    /// <summary>
    /// Returns whether the given Minecraft version supports resource packs.
    /// </summary>
    /// <param name="minecraftVersion">The Minecraft version, or null when unknown.</param>
    /// <returns>Whether resource packs are supported.</returns>
    public static bool IsSupported(string? minecraftVersion) =>
        !string.IsNullOrWhiteSpace(minecraftVersion) &&
        Compare(minecraftVersion, LeastSupportedVersion) >= 0;

    /// <summary>
    /// Returns whether the given Minecraft version stores resource pack
    /// identifiers with the "file/" prefix.
    /// </summary>
    /// <param name="minecraftVersion">The Minecraft version, or null when unknown.</param>
    /// <returns>Whether the new options format is used.</returns>
    public static bool SupportsNewOptionsFormat(string? minecraftVersion) =>
        !string.IsNullOrWhiteSpace(minecraftVersion) &&
        Compare(minecraftVersion, LeastNewOptionsVersion) >= 0;

    /// <summary>
    /// Returns the resource pack format number expected by the given Minecraft version.
    /// </summary>
    /// <param name="minecraftVersion">The Minecraft version, or null when unknown.</param>
    /// <returns>The expected format number, or null when it cannot be determined.</returns>
    public static int? GetRequiredFormat(string? minecraftVersion)
    {
        if (string.IsNullOrWhiteSpace(minecraftVersion) || !IsReleaseVersion(minecraftVersion))
            return null;

        int? result = null;
        foreach (var (version, format) in Formats)
        {
            if (Compare(minecraftVersion, version) < 0)
                break;

            result = format;
        }

        return result;
    }

    /// <summary>
    /// Compares two Minecraft version strings numerically, so that 1.10 is
    /// greater than 1.9. Snapshot versions compare lower than every release.
    /// </summary>
    /// <param name="left">The first version.</param>
    /// <param name="right">The second version.</param>
    /// <returns>A negative value, zero or a positive value.</returns>
    public static int Compare(string left, string right)
    {
        var leftParts = Split(left);
        var rightParts = Split(right);

        for (var i = 0; i < Math.Max(leftParts.Length, rightParts.Length); i++)
        {
            var a = i < leftParts.Length ? leftParts[i] : 0;
            var b = i < rightParts.Length ? rightParts[i] : 0;
            if (a != b)
                return a.CompareTo(b);
        }

        return 0;
    }

    /// <summary>
    /// Returns whether the version is a release version, that is a purely
    /// numeric dotted version such as "1.20.1".
    /// </summary>
    /// <param name="version">The version string.</param>
    /// <returns>Whether the version is a release version.</returns>
    public static bool IsReleaseVersion(string version)
    {
        if (string.IsNullOrWhiteSpace(version))
            return false;

        foreach (var part in version.Split('.', StringSplitOptions.RemoveEmptyEntries))
        {
            if (part.Length == 0)
                return false;

            foreach (var ch in part)
            {
                if (!char.IsDigit(ch))
                    return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Splits a version string into its numeric components. Non numeric
    /// components are represented by -1 so that snapshots sort before releases.
    /// </summary>
    /// <param name="version">The version string.</param>
    /// <returns>The numeric components of the version.</returns>
    private static int[] Split(string version)
    {
        var parts = version.Split('.', StringSplitOptions.RemoveEmptyEntries);
        var result = new int[parts.Length];

        for (var i = 0; i < parts.Length; i++)
        {
            var span = parts[i];
            var digits = 0;
            while (digits < span.Length && char.IsDigit(span[digits]))
                digits++;

            result[i] = digits == 0 || !int.TryParse(span[..digits], out var value) ? -1 : value;
        }

        return result;
    }
}
