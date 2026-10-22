using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;
using System.Text.RegularExpressions;

namespace HMCLCore.NET.Util.Versioning;

/// <summary>
/// The era a Minecraft version belongs to.
/// </summary>
public enum GameVersionType
{
    /// <summary>The versions before Minecraft Classic, for example <c>rd-132211</c>.</summary>
    PreClassic,

    /// <summary>The versions of Minecraft Classic, for example <c>c0.0.11a</c>.</summary>
    Classic,

    /// <summary>The versions of Minecraft Indev, for example <c>in-20100223</c>.</summary>
    Indev,

    /// <summary>The versions of Minecraft Infdev, for example <c>inf-20100618</c>.</summary>
    Infdev,

    /// <summary>The alpha versions of Minecraft, for example <c>a1.2.6</c>.</summary>
    Alpha,

    /// <summary>The beta versions of Minecraft, for example <c>b1.7.3</c>.</summary>
    Beta,

    /// <summary>The releases and the snapshots of Minecraft.</summary>
    New
}

/// <summary>
/// The version number of a Minecraft version, which knows the order of the
/// releases, the snapshots and the versions of the older eras.
/// </summary>
/// <remarks>
/// Ported from the Java implementation of HMCL.
/// </remarks>
public abstract class GameVersionNumber : IComparable<GameVersionNumber>
{
    /// <summary>
    /// The version string the version number was parsed from.
    /// </summary>
    public string Value { get; }

    /// <summary>
    /// The normalized form of the version string.
    /// </summary>
    public string Normalized { get; }

    /// <summary>
    /// Creates a new version number.
    /// </summary>
    /// <param name="value">The version string the version number was parsed from.</param>
    /// <param name="normalized">The normalized form of the version string.</param>
    protected GameVersionNumber(string value, string normalized)
    {
        Value = value;
        Normalized = normalized;
    }

    /// <summary>
    /// The era the version belongs to.
    /// </summary>
    public abstract GameVersionType Type { get; }

    /// <summary>
    /// Compares the version number with another version number of the same
    /// era.
    /// </summary>
    /// <param name="other">The version number to compare with.</param>
    /// <returns>The result of the comparison.</returns>
    protected abstract int CompareToImpl(GameVersionNumber other);

    /// <summary>
    /// Parses a version string into a version number.
    /// </summary>
    /// <param name="version">The version string to parse.</param>
    /// <returns>
    /// The version number of the version string; unknown versions are returned
    /// as a <see cref="Special"/> version number.
    /// </returns>
    public static GameVersionNumber AsGameVersion(string version)
    {
        version ??= string.Empty;

        VersionTable.EnsureInitialized();

        if (VersionTable.Specials.TryGetValue(version, out var special))
            return special;

        try
        {
            if (version.Length > 0)
            {
                switch (version[0])
                {
                    case 'r':
                    case 'a':
                    case 'b':
                    case 'c':
                    case 'i':
                        return Old.Parse(version);
                }

                if (version == "0.0")
                    return Release.Zero;

                if (version.Length >= 6 && version[2] == 'w')
                    return LegacySnapshot.Parse(version);

                return Release.Parse(version);
            }
        }
        catch (Exception)
        {
            // The version string does not match any known format.
        }

        return new Special(version, version);
    }

    /// <summary>
    /// Compares two version strings.
    /// </summary>
    /// <param name="version1">The first version string.</param>
    /// <param name="version2">The second version string.</param>
    /// <returns>
    /// A negative number, zero, or a positive number when the first version is
    /// lower than, equal to, or higher than the second one.
    /// </returns>
    public static int Compare(string version1, string version2)
        => AsGameVersion(version1).CompareTo(AsGameVersion(version2));

    /// <summary>
    /// Returns whether the version string is the version of a known Minecraft
    /// version.
    /// </summary>
    /// <param name="version">The version string to check.</param>
    /// <returns>Whether the version string is known.</returns>
    public static bool IsKnown(string version)
        => AsGameVersion(version) is not Special special || !special.IsUnknown;

    /// <summary>
    /// Returns whether the version is an April Fools' Day version.
    /// </summary>
    /// <param name="version">The version to check.</param>
    /// <returns>Whether the version is an April Fools' Day version.</returns>
    public static bool IsAprilFools(GameVersionNumber version)
    {
        switch (version)
        {
            case Special special:
            {
                var normalized = special.ToNormalizedString();
                return (!normalized.StartsWith("1.", StringComparison.Ordinal) &&
                        !string.Equals(normalized, "13w12~", StringComparison.Ordinal)) ||
                       string.Equals(normalized, "1.RV-Pre1", StringComparison.Ordinal);
            }

            case LegacySnapshot snapshot:
                return snapshot.IntValue == LegacySnapshot.ToInt(15, 14, 'a', false);

            default:
                return false;
        }
    }

    /// <summary>
    /// Returns the release the given snapshot belongs to.
    /// </summary>
    /// <param name="version">The version to get the release of.</param>
    /// <returns>
    /// The release of the version, or null when the version is a release
    /// itself or has no release.
    /// </returns>
    public static GameVersionNumber? GetReleaseOfSnapshot(GameVersionNumber version)
    {
        switch (version)
        {
            case Release release:
            {
                if (release.EaType == Release.ReleaseType.GA)
                    return null;

                if (release.Patch > 0)
                    return AsGameVersion($"{release.Major}.{release.Minor}.{release.Patch}");

                return AsGameVersion($"{release.Major}.{release.Minor}");
            }

            case LegacySnapshot snapshot:
            {
                var defaultVersions = VersionTable.DefaultGameVersions;
                for (var i = defaultVersions.Length - 1; i >= 0; i--)
                {
                    if (AsGameVersion(defaultVersions[i]) is not Release gaRelease)
                        continue;

                    if (gaRelease.CompareToSnapshot(snapshot) > 0)
                        return gaRelease;
                }

                break;
            }
        }

        return null;
    }

    /// <summary>
    /// Compares the version number with another one.
    /// </summary>
    /// <param name="other">The version number to compare with.</param>
    /// <returns>
    /// A negative number, zero, or a positive number when this version is
    /// lower than, equal to, or higher than the other one.
    /// </returns>
    public int CompareTo(GameVersionNumber? other)
    {
        if (other is null)
            return 1;

        if (Type != other.Type)
            return Type.CompareTo(other.Type);

        return CompareToImpl(other);
    }

    /// <summary>
    /// Compares the version number with the version of another version string.
    /// </summary>
    /// <param name="other">The version string to compare with.</param>
    /// <returns>The result of the comparison.</returns>
    public int CompareTo(string other) => CompareTo(AsGameVersion(other));

    /// <summary>
    /// Returns the normalized form of the version string.
    /// </summary>
    /// <returns>The normalized form of the version string.</returns>
    public string ToNormalizedString() => Normalized;

    /// <summary>
    /// Returns whether the version is at least the given version.
    /// </summary>
    /// <param name="releaseVersion">The release version to compare with.</param>
    /// <param name="snapshotVersion">The snapshot version to compare with.</param>
    /// <param name="strictReleaseVersion">
    /// Whether the release version is considered greater than its pre-release
    /// and release candidate versions.
    /// </param>
    /// <returns>Whether the version is at least the given version.</returns>
    public bool IsAtLeast(string releaseVersion, string snapshotVersion, bool strictReleaseVersion)
    {
        if (this is Release self)
        {
            var other = strictReleaseVersion
                ? Release.Parse(releaseVersion)
                : Release.ParseSimple(releaseVersion);

            return self.CompareToRelease(other) >= 0;
        }

        return CompareTo(LegacySnapshot.Parse(snapshotVersion)) >= 0;
    }

    /// <summary>
    /// Returns whether the version is at least the given version.
    /// </summary>
    /// <param name="releaseVersion">The release version to compare with.</param>
    /// <param name="snapshotVersion">The snapshot version to compare with.</param>
    /// <returns>Whether the version is at least the given version.</returns>
    public bool IsAtLeast(string releaseVersion, string snapshotVersion)
        => IsAtLeast(releaseVersion, snapshotVersion, false);

    /// <inheritdoc/>
    public override string ToString() => Value;

    /// <summary>
    /// A version of the eras before the first release of Minecraft, for
    /// example <c>a1.2.6</c> or <c>rd-132211</c>.
    /// </summary>
    public sealed class Old : GameVersionNumber
    {
        private readonly GameVersionType _type;
        private readonly VersionNumber _versionNumber;

        private Old(string value, GameVersionType type, VersionNumber versionNumber)
            : base(value, value)
        {
            _type = type;
            _versionNumber = versionNumber;
        }

        /// <summary>
        /// Parses a version of the older eras.
        /// </summary>
        /// <param name="value">The version string to parse.</param>
        /// <returns>The parsed version number.</returns>
        public static Old Parse(string value)
        {
            if (value.Length == 0)
                throw new ArgumentException("Empty old version number", nameof(value));

            GameVersionType type;
            var prefixLength = 1;
            switch (value[0])
            {
                case 'r':
                    if (!value.StartsWith("rd-", StringComparison.Ordinal))
                        throw new ArgumentException(value, nameof(value));

                    type = GameVersionType.PreClassic;
                    prefixLength = 3;
                    break;

                case 'i':
                    if (value.StartsWith("inf-", StringComparison.Ordinal))
                    {
                        type = GameVersionType.Infdev;
                        prefixLength = 4;
                    }
                    else if (value.StartsWith("in-", StringComparison.Ordinal))
                    {
                        type = GameVersionType.Indev;
                        prefixLength = 3;
                    }
                    else
                    {
                        throw new ArgumentException(value, nameof(value));
                    }
                    break;

                case 'a':
                    type = GameVersionType.Alpha;
                    break;

                case 'b':
                    type = GameVersionType.Beta;
                    break;

                case 'c':
                    type = GameVersionType.Classic;
                    break;

                default:
                    throw new ArgumentException(value, nameof(value));
            }

            if (value.Length < prefixLength + 1 || !char.IsDigit(value[prefixLength]))
                throw new ArgumentException(value, nameof(value));

            return new Old(value, type, VersionNumber.AsVersion(value.Substring(prefixLength)));
        }

        /// <inheritdoc/>
        public override GameVersionType Type => _type;

        /// <inheritdoc/>
        protected override int CompareToImpl(GameVersionNumber other)
            => _versionNumber.CompareTo(((Old)other)._versionNumber);
    }

    /// <summary>
    /// A release of Minecraft, for example <c>1.21.1</c>.
    /// </summary>
    public sealed class Release : GameVersionNumber
    {
        /// <summary>
        /// The smallest major version that is not the major version one.
        /// </summary>
        internal const int MinimumYearMajorVersion = 26;

        private static readonly Regex VersionPattern = new(
            @"\A(?<prefix>(?<major>1|[1-9]\d+)\.(?<minor>\d+)(\.(?<patch>[0-9]+))?)(?<suffix>.*)\z",
            RegexOptions.CultureInvariant);

        /// <summary>
        /// The version number of the version zero.
        /// </summary>
        public static readonly Release Zero = new(
            "0.0", "0.0",
            0, 0, 0,
            ReleaseType.Unknown, VersionNumber.Zero, Additional.None);

        /// <summary>
        /// The kind of a release.
        /// </summary>
        public enum ReleaseType
        {
            /// <summary>The release is of an unknown kind.</summary>
            Unknown,

            /// <summary>The release is a snapshot.</summary>
            Snapshot,

            /// <summary>The release is a pre-release.</summary>
            PreRelease,

            /// <summary>The release is a release candidate.</summary>
            ReleaseCandidate,

            /// <summary>The release is a general available release.</summary>
            GA
        }

        /// <summary>
        /// An additional property of a release.
        /// </summary>
        public enum Additional
        {
            /// <summary>The release has no additional property.</summary>
            None,

            /// <summary>The release is the unobfuscated variant of a version.</summary>
            Unobfuscated
        }

        /// <summary>
        /// The major version of the release.
        /// </summary>
        public int Major { get; }

        /// <summary>
        /// The minor version of the release.
        /// </summary>
        public int Minor { get; }

        /// <summary>
        /// The patch version of the release.
        /// </summary>
        public int Patch { get; }

        /// <summary>
        /// The kind of the release.
        /// </summary>
        public ReleaseType EaType { get; }

        /// <summary>
        /// The version of the early access part of the release.
        /// </summary>
        public VersionNumber EaVersion { get; }

        /// <summary>
        /// The additional property of the release.
        /// </summary>
        public Additional AdditionalInfo { get; }

        private Release(
            string value,
            string normalized,
            int major,
            int minor,
            int patch,
            ReleaseType eaType,
            VersionNumber eaVersion,
            Additional additional)
            : base(value, normalized)
        {
            Major = major;
            Minor = minor;
            Patch = patch;
            EaType = eaType;
            EaVersion = eaVersion;
            AdditionalInfo = additional;
        }

        /// <summary>
        /// Parses a release version.
        /// </summary>
        /// <param name="value">The version string to parse.</param>
        /// <returns>The parsed version number.</returns>
        public static Release Parse(string value)
        {
            var match = VersionPattern.Match(value);
            if (!match.Success)
                throw new ArgumentException(value, nameof(value));

            var major = int.Parse(match.Groups["major"].Value, CultureInfo.InvariantCulture);
            if (major != 1 && major < MinimumYearMajorVersion)
                throw new ArgumentException(value, nameof(value));

            var minor = int.Parse(match.Groups["minor"].Value, CultureInfo.InvariantCulture);

            var patchGroup = match.Groups["patch"];
            var patch = patchGroup.Success
                ? int.Parse(patchGroup.Value, CultureInfo.InvariantCulture)
                : 0;

            var suffix = match.Groups["suffix"].Value;
            var isLegacyRelease = major == 1;

            ReleaseType releaseType;
            VersionNumber eaVersion;
            var additional = Additional.None;
            var needNormalize = false;

            if (suffix.EndsWith("_unobfuscated", StringComparison.Ordinal))
            {
                suffix = suffix.Substring(0, suffix.Length - "_unobfuscated".Length);
                additional = Additional.Unobfuscated;
            }
            else if (suffix.EndsWith(" Unobfuscated", StringComparison.Ordinal))
            {
                needNormalize = true;
                suffix = suffix.Substring(0, suffix.Length - " Unobfuscated".Length);
                additional = Additional.Unobfuscated;
            }

            if (suffix.Length == 0)
            {
                releaseType = ReleaseType.GA;
                eaVersion = VersionNumber.Zero;
            }
            else if (suffix.StartsWith("-snapshot-", StringComparison.Ordinal))
            {
                releaseType = ReleaseType.Snapshot;
                eaVersion = VersionNumber.AsVersion(suffix.Substring("-snapshot-".Length));
            }
            else if (suffix.StartsWith(" Snapshot ", StringComparison.Ordinal))
            {
                needNormalize = true;
                releaseType = ReleaseType.Snapshot;
                eaVersion = VersionNumber.AsVersion(suffix.Substring(" Snapshot ".Length));
            }
            else if (suffix.StartsWith("-pre-", StringComparison.Ordinal))
            {
                if (isLegacyRelease)
                    needNormalize = true;
                releaseType = ReleaseType.PreRelease;
                eaVersion = VersionNumber.AsVersion(suffix.Substring("-pre-".Length));
            }
            else if (suffix.StartsWith("-pre", StringComparison.Ordinal))
            {
                if (!isLegacyRelease)
                    needNormalize = true;
                releaseType = ReleaseType.PreRelease;
                eaVersion = VersionNumber.AsVersion(suffix.Substring("-pre".Length));
            }
            else if (suffix.StartsWith(" Pre-Release ", StringComparison.Ordinal))
            {
                needNormalize = true;
                releaseType = ReleaseType.PreRelease;
                eaVersion = VersionNumber.AsVersion(suffix.Substring(" Pre-Release ".Length));
            }
            else if (suffix.StartsWith(" Pre-release ", StringComparison.Ordinal))
            {
                needNormalize = true;
                releaseType = ReleaseType.PreRelease;
                eaVersion = VersionNumber.AsVersion(suffix.Substring(" Pre-release ".Length));
            }
            else if (suffix.StartsWith("-rc-", StringComparison.Ordinal))
            {
                if (isLegacyRelease)
                    needNormalize = true;
                releaseType = ReleaseType.ReleaseCandidate;
                eaVersion = VersionNumber.AsVersion(suffix.Substring("-rc-".Length));
            }
            else if (suffix.StartsWith("-rc", StringComparison.Ordinal))
            {
                if (!isLegacyRelease)
                    needNormalize = true;
                releaseType = ReleaseType.ReleaseCandidate;
                eaVersion = VersionNumber.AsVersion(suffix.Substring("-rc".Length));
            }
            else if (suffix.StartsWith(" Release Candidate ", StringComparison.Ordinal))
            {
                needNormalize = true;
                releaseType = ReleaseType.ReleaseCandidate;
                eaVersion = VersionNumber.AsVersion(suffix.Substring(" Release Candidate ".Length));
            }
            else
            {
                throw new ArgumentException(value, nameof(value));
            }

            string normalized;
            if (needNormalize)
            {
                var builder = new StringBuilder(value.Length);
                builder.Append(match.Groups["prefix"].Value);
                if (releaseType != ReleaseType.GA)
                {
                    builder.Append(isLegacyRelease ? LegacyInfix(releaseType) : NewInfix(releaseType));
                    builder.Append(eaVersion);
                }
                builder.Append(AdditionalSuffix(additional));
                normalized = builder.ToString();
            }
            else
            {
                normalized = value;
            }

            return new Release(value, normalized, major, minor, patch, releaseType, eaVersion, additional);
        }

        /// <summary>
        /// Parses a release version of the simple format, which does not
        /// describe the kind of the release.
        /// </summary>
        /// <param name="value">The version string to parse.</param>
        /// <returns>The parsed version number.</returns>
        public static Release ParseSimple(string value)
        {
            var majorLength = GetNumberLength(value, 0);
            if (majorLength == 0 || value.Length < majorLength + 2 || value[majorLength] != '.')
                throw new ArgumentException(value, nameof(value));

            var major = int.Parse(value.Substring(0, majorLength), CultureInfo.InvariantCulture);
            if (major != 1 && major < MinimumYearMajorVersion)
                throw new ArgumentException(value, nameof(value));

            var minorOffset = majorLength + 1;

            var minorLength = GetNumberLength(value, minorOffset);
            if (minorLength == 0)
                throw new ArgumentException(value, nameof(value));

            try
            {
                var minor = int.Parse(
                    value.Substring(minorOffset, minorLength), CultureInfo.InvariantCulture);
                var patch = 0;

                if (minorOffset + minorLength < value.Length)
                {
                    var patchOffset = minorOffset + minorLength + 1;

                    if (patchOffset >= value.Length || value[patchOffset - 1] != '.')
                        throw new ArgumentException(value, nameof(value));

                    patch = int.Parse(value.Substring(patchOffset), CultureInfo.InvariantCulture);
                }

                return new Release(
                    value, value, major, minor, patch,
                    ReleaseType.Unknown, VersionNumber.Zero, Additional.None);
            }
            catch (FormatException)
            {
                throw new ArgumentException(value, nameof(value));
            }
        }

        /// <summary>
        /// Compares the release with another release.
        /// </summary>
        /// <param name="other">The release to compare with.</param>
        /// <returns>The result of the comparison.</returns>
        public int CompareToRelease(Release other)
        {
            var result = Major.CompareTo(other.Major);
            if (result != 0)
                return result;

            result = Minor.CompareTo(other.Minor);
            if (result != 0)
                return result;

            result = Patch.CompareTo(other.Patch);
            if (result != 0)
                return result;

            result = EaType.CompareTo(other.EaType);
            if (result != 0)
                return result;

            result = EaVersion.CompareTo(other.EaVersion);
            if (result != 0)
                return result;

            return AdditionalInfo.CompareTo(other.AdditionalInfo);
        }

        /// <summary>
        /// Compares the release with a snapshot.
        /// </summary>
        /// <param name="other">The snapshot to compare with.</param>
        /// <returns>The result of the comparison.</returns>
        public int CompareToSnapshot(LegacySnapshot other)
        {
            if (Major == 0)
                return -1;

            if (Major == 1)
            {
                var index = Array.BinarySearch(VersionTable.SnapshotInts, other.IntValue);
                if (index >= 0)
                    return CompareToRelease(VersionTable.SnapshotPrev[index]) <= 0 ? -1 : 1;

                index = -(index + 1);
                if (index == VersionTable.SnapshotInts.Length)
                    return -1;

                return CompareToRelease(VersionTable.SnapshotPrev[index]) <= 0 ? -1 : 1;
            }

            return 1;
        }

        /// <inheritdoc/>
        public override GameVersionType Type => GameVersionType.New;

        /// <inheritdoc/>
        protected override int CompareToImpl(GameVersionNumber other)
        {
            switch (other)
            {
                case Release release:
                    return CompareToRelease(release);

                case LegacySnapshot snapshot:
                    return CompareToSnapshot(snapshot);

                case Special special:
                    return -special.CompareToReleaseOrSnapshot(this);

                default:
                    throw new InvalidOperationException(other.GetType().Name);
            }
        }

        /// <summary>
        /// Returns the length of the number at the given offset.
        /// </summary>
        /// <param name="value">The string to read.</param>
        /// <param name="offset">The offset to start reading at.</param>
        /// <returns>The length of the number.</returns>
        private static int GetNumberLength(string value, int offset)
        {
            var current = offset;
            while (current < value.Length)
            {
                var ch = value[current];
                if (ch < '0' || ch > '9')
                    break;

                current++;
            }

            return current - offset;
        }

        /// <summary>
        /// Returns the infix of the kind of a release used by the versions
        /// with the major version one.
        /// </summary>
        /// <param name="type">The kind of the release.</param>
        /// <returns>The infix of the kind of the release.</returns>
        private static string LegacyInfix(ReleaseType type) => type switch
        {
            ReleaseType.Snapshot => "-snapshot-",
            ReleaseType.PreRelease => "-pre",
            ReleaseType.ReleaseCandidate => "-rc",
            _ => string.Empty
        };

        /// <summary>
        /// Returns the infix of the kind of a release used by the versions with
        /// a year as their major version.
        /// </summary>
        /// <param name="type">The kind of the release.</param>
        /// <returns>The infix of the kind of the release.</returns>
        private static string NewInfix(ReleaseType type) => type switch
        {
            ReleaseType.Snapshot => "-snapshot-",
            ReleaseType.PreRelease => "-pre-",
            ReleaseType.ReleaseCandidate => "-rc-",
            _ => string.Empty
        };

        /// <summary>
        /// Returns the suffix of an additional property of a release.
        /// </summary>
        /// <param name="additional">The additional property.</param>
        /// <returns>The suffix of the additional property.</returns>
        private static string AdditionalSuffix(Additional additional) => additional switch
        {
            Additional.Unobfuscated => "_unobfuscated",
            _ => string.Empty
        };
    }

    /// <summary>
    /// A snapshot of Minecraft, for example <c>25w46a</c>.
    /// </summary>
    public sealed class LegacySnapshot : GameVersionNumber
    {
        /// <summary>
        /// The numeric representation of the snapshot.
        /// </summary>
        public int IntValue { get; }

        private LegacySnapshot(string value, string normalized, int year, int week, char suffix, bool unobfuscated)
            : base(value, normalized)
        {
            IntValue = ToInt(year, week, suffix, unobfuscated);
        }

        /// <summary>
        /// Parses a snapshot version.
        /// </summary>
        /// <param name="value">The version string to parse.</param>
        /// <returns>The parsed version number.</returns>
        public static LegacySnapshot Parse(string value)
        {
            if (value.Length < 6 || value[2] != 'w')
                throw new ArgumentException(value, nameof(value));

            int prefixLength;
            bool unobfuscated;
            string normalized;
            if (value.EndsWith("_unobfuscated", StringComparison.Ordinal))
            {
                prefixLength = value.Length - "_unobfuscated".Length;
                unobfuscated = true;
                normalized = value;
            }
            else if (value.EndsWith(" Unobfuscated", StringComparison.Ordinal))
            {
                prefixLength = value.Length - " Unobfuscated".Length;
                unobfuscated = true;
                normalized = value.Substring(0, prefixLength) + "_unobfuscated";
            }
            else
            {
                prefixLength = value.Length;
                unobfuscated = false;
                normalized = value;
            }

            if (prefixLength != 6)
                throw new ArgumentException(value, nameof(value));

            int year;
            int week;
            try
            {
                year = int.Parse(value.Substring(0, 2), CultureInfo.InvariantCulture);
                week = int.Parse(value.Substring(3, 2), CultureInfo.InvariantCulture);
            }
            catch (FormatException)
            {
                throw new ArgumentException(value, nameof(value));
            }

            if (year >= Release.MinimumYearMajorVersion)
                throw new ArgumentException(value, nameof(value));

            var suffix = value[5];
            if (suffix < 'a' || suffix > 'z')
                throw new ArgumentException(value, nameof(value));

            return new LegacySnapshot(value, normalized, year, week, suffix, unobfuscated);
        }

        /// <summary>
        /// Converts a snapshot into its numeric representation.
        /// </summary>
        /// <param name="year">The year of the snapshot.</param>
        /// <param name="week">The week of the snapshot.</param>
        /// <param name="suffix">The suffix of the snapshot.</param>
        /// <param name="unobfuscated">Whether the snapshot is unobfuscated.</param>
        /// <returns>The numeric representation of the snapshot.</returns>
        public static int ToInt(int year, int week, char suffix, bool unobfuscated)
            => (year << 24) | (week << 16) | (suffix << 8) | (unobfuscated ? 1 : 0);

        /// <summary>
        /// The year of the snapshot.
        /// </summary>
        public int Year => (IntValue >> 24) & 0xff;

        /// <summary>
        /// The week of the snapshot.
        /// </summary>
        public int Week => (IntValue >> 16) & 0xff;

        /// <summary>
        /// The suffix of the snapshot.
        /// </summary>
        public char Suffix => (char)((IntValue >> 8) & 0xff);

        /// <summary>
        /// Whether the snapshot is the unobfuscated variant of a version.
        /// </summary>
        public bool IsUnobfuscated => (IntValue & 0b00000001) != 0;

        /// <inheritdoc/>
        public override GameVersionType Type => GameVersionType.New;

        /// <inheritdoc/>
        protected override int CompareToImpl(GameVersionNumber other)
        {
            switch (other)
            {
                case Release otherRelease:
                    return -otherRelease.CompareToSnapshot(this);

                case LegacySnapshot otherSnapshot:
                    return IntValue.CompareTo(otherSnapshot.IntValue);

                case Special otherSpecial:
                    return -otherSpecial.CompareToReleaseOrSnapshot(this);

                default:
                    throw new InvalidOperationException(other.GetType().Name);
            }
        }
    }

    /// <summary>
    /// A version that does not match any of the known formats, for example the
    /// versions of the April Fools' Day.
    /// </summary>
    public sealed class Special : GameVersionNumber
    {
        /// <summary>
        /// The version the special version is based on, if it is known.
        /// </summary>
        internal GameVersionNumber? Prev { get; set; }

        private VersionNumber? _versionNumber;

        internal Special(string value, string normalized)
            : base(value, normalized)
        {
        }

        /// <summary>
        /// Whether the version is unknown.
        /// </summary>
        public bool IsUnknown => Prev is null;

        /// <inheritdoc/>
        public override GameVersionType Type => GameVersionType.New;

        /// <summary>
        /// Returns the version the special version is based on.
        /// </summary>
        /// <returns>The version the special version is based on.</returns>
        public VersionNumber AsVersionNumber()
            => _versionNumber ??= VersionNumber.AsVersion(Normalized);

        /// <summary>
        /// Returns the version the special version is based on, skipping the
        /// other special versions.
        /// </summary>
        /// <returns>The version the special version is based on.</returns>
        internal GameVersionNumber GetPrevNormalVersion()
        {
            var version = Prev;
            while (version is Special special)
                version = special.Prev;

            return version ?? throw new InvalidOperationException("version: " + Value);
        }

        /// <summary>
        /// Compares the special version with a release or a snapshot.
        /// </summary>
        /// <param name="other">The version to compare with.</param>
        /// <returns>The result of the comparison.</returns>
        internal int CompareToReleaseOrSnapshot(GameVersionNumber other)
        {
            if (IsUnknown)
                return 1;

            if (GetPrevNormalVersion().CompareTo(other) >= 0)
                return 1;

            return -1;
        }

        /// <summary>
        /// Compares the special version with another special version.
        /// </summary>
        /// <param name="other">The special version to compare with.</param>
        /// <returns>The result of the comparison.</returns>
        internal int CompareToSpecial(Special other)
        {
            if (IsUnknown)
                return other.IsUnknown ? AsVersionNumber().CompareTo(other.AsVersionNumber()) : 1;

            if (other.IsUnknown)
                return -1;

            if (string.Equals(Normalized, other.Normalized, StringComparison.Ordinal))
                return 0;

            var result = GetPrevNormalVersion().CompareTo(other.GetPrevNormalVersion());
            if (result != 0)
                return result;

            var version = Prev;
            while (version is Special special)
            {
                // The aliases have the same normalized name, but are distinct
                // objects.
                if (ReferenceEquals(version, other))
                    return 1;

                version = special.Prev;
            }

            return -1;
        }

        /// <inheritdoc/>
        protected override int CompareToImpl(GameVersionNumber other)
        {
            switch (other)
            {
                case Release:
                case LegacySnapshot:
                    return CompareToReleaseOrSnapshot(other);

                case Special special:
                    return CompareToSpecial(special);

                default:
                    throw new InvalidOperationException(other.GetType().Name);
            }
        }
    }

    /// <summary>
    /// The table of the known Minecraft versions, which is built from the
    /// resources of the launcher.
    /// </summary>
    private static class VersionTable
    {
        private static readonly object SyncRoot = new();
        private static bool _initialized;

        /// <summary>
        /// The special versions, keyed by their version string.
        /// </summary>
        public static Dictionary<string, Special> Specials { get; } = new();

        /// <summary>
        /// The releases of Minecraft, newest first.
        /// </summary>
        public static string[] DefaultGameVersions { get; private set; } = Array.Empty<string>();

        /// <summary>
        /// The numeric representations of the snapshots, in ascending order.
        /// </summary>
        public static int[] SnapshotInts { get; private set; } = Array.Empty<int>();

        /// <summary>
        /// The releases the snapshots were published before.
        /// </summary>
        public static Release[] SnapshotPrev { get; private set; } = Array.Empty<Release>();

        /// <summary>
        /// Initializes the table of the known versions.
        /// </summary>
        public static void EnsureInitialized()
        {
            if (_initialized)
                return;

            lock (SyncRoot)
            {
                if (_initialized)
                    return;

                _initialized = true;

                var defaultGameVersions = new List<string>();
                var snapshots = new List<int>();
                var snapshotPrev = new List<Release>();

                Release? currentRelease = null;
                GameVersionNumber? prev = null;

                foreach (var line in ReadResourceLines("versions.txt"))
                {
                    if (line.Length == 0)
                        continue;

                    var version = AsGameVersion(line);

                    if (currentRelease is null)
                        currentRelease = version as Release;

                    switch (version)
                    {
                        case LegacySnapshot snapshot:
                            snapshots.Add(snapshot.IntValue);
                            snapshotPrev.Add(currentRelease!);
                            break;

                        case Release release:
                            currentRelease = release;

                            if (release.EaType == Release.ReleaseType.GA &&
                                release.AdditionalInfo == Release.Additional.None)
                            {
                                defaultGameVersions.Insert(0, release.Value);
                            }
                            break;

                        case Special special:
                            special.Prev = prev;
                            Specials[special.Value] = special;
                            break;
                    }

                    prev = version;
                }

                foreach (var line in ReadResourceLines("version-alias.csv"))
                {
                    if (line.Length == 0)
                        continue;

                    var parts = line.Split(',');
                    if (parts.Length < 2)
                        continue;

                    var normalized = parts[0];
                    if (!Specials.TryGetValue(normalized, out var normalizedVersion))
                        continue;

                    for (var i = 1; i < parts.Length; i++)
                    {
                        var version = new Special(parts[i], normalized)
                        {
                            Prev = normalizedVersion.Prev
                        };
                        Specials[parts[i]] = version;
                    }
                }

                DefaultGameVersions = defaultGameVersions.ToArray();
                SnapshotInts = snapshots.ToArray();
                SnapshotPrev = snapshotPrev.ToArray();
            }
        }

        /// <summary>
        /// Reads the lines of an embedded resource of the launcher.
        /// </summary>
        /// <param name="name">The name of the resource file.</param>
        /// <returns>The lines of the resource file.</returns>
        private static IEnumerable<string> ReadResourceLines(string name)
        {
            var assembly = typeof(VersionTable).Assembly;
            var resourceName = assembly.GetManifestResourceNames()
                .FirstOrDefault(candidate => candidate.EndsWith(name, StringComparison.Ordinal));

            if (resourceName is null)
                yield break;

            using var stream = assembly.GetManifestResourceStream(resourceName);
            if (stream is null)
                yield break;

            using var reader = new StreamReader(stream, Encoding.ASCII);
            while (reader.ReadLine() is { } line)
                yield return line;
        }
    }
}
