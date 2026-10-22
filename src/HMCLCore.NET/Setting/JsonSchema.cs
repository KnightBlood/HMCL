using System;
using System.Globalization;
using System.Text.Json;

namespace HMCLCore.NET.Setting;

/// <summary>
/// A schema marker of a JSON settings file, which describes the format of the
/// file and allows the launcher to migrate the files written by the older
/// versions of the launcher.
/// </summary>
/// <remarks>
/// The schema markers owned by the launcher use the URL form
/// <c>https://schemas.glavo.site/hmcl/&lt;id&gt;/&lt;version&gt;</c>.
/// </remarks>
public sealed class JsonSchema : IEquatable<JsonSchema>
{
    /// <summary>
    /// The name of the JSON property holding the schema marker.
    /// </summary>
    public const string PropertySchema = "$schema";

    /// <summary>
    /// The prefix of the schema URLs owned by the launcher.
    /// </summary>
    public const string UrlPrefix = "https://schemas.glavo.site/hmcl/";

    /// <summary>
    /// The raw schema string.
    /// </summary>
    public string Value { get; }

    /// <summary>
    /// The identifier of the schema, or null when the schema string is not a
    /// schema URL owned by the launcher.
    /// </summary>
    public string? Id { get; }

    /// <summary>
    /// The version of the schema, or null when the schema string is not a
    /// schema URL owned by the launcher.
    /// </summary>
    public SchemaVersion? Version { get; }

    /// <summary>
    /// Creates a schema from a raw schema string.
    /// </summary>
    /// <param name="value">The raw schema string.</param>
    public JsonSchema(string value)
    {
        Value = value ?? throw new ArgumentNullException(nameof(value));

        var parsed = ParseSchemaUrl(value);
        if (parsed is not null)
        {
            Id = parsed.Id;
            Version = parsed.Version;
        }
    }

    /// <summary>
    /// Whether the schema string is a schema URL owned by the launcher.
    /// </summary>
    public bool IsParsed => Version is not null;

    /// <summary>
    /// The raw schema string.
    /// </summary>
    public string Url => Value;

    /// <summary>
    /// Creates a schema of the launcher.
    /// </summary>
    /// <param name="id">The identifier of the schema.</param>
    /// <param name="version">The version of the schema.</param>
    /// <returns>The created schema.</returns>
    public static JsonSchema Of(string id, SchemaVersion version)
        => new($"{UrlPrefix}{id}/{version}");

    /// <summary>
    /// Creates a schema of the launcher.
    /// </summary>
    /// <param name="id">The identifier of the schema.</param>
    /// <param name="major">The major version of the schema.</param>
    /// <param name="minor">The minor version of the schema.</param>
    /// <param name="patch">The patch version of the schema.</param>
    /// <returns>The created schema.</returns>
    public static JsonSchema Of(string id, int major, int minor, int patch)
        => Of(id, new SchemaVersion(major, minor, patch));

    /// <summary>
    /// Checks whether a JSON file can be read and safely saved.
    /// </summary>
    /// <param name="element">The root of the JSON file.</param>
    /// <param name="expected">The schema supported by the current code.</param>
    /// <returns>The result of the check.</returns>
    public static CompatibilityResult Check(JsonElement element, JsonSchema expected)
    {
        ArgumentNullException.ThrowIfNull(expected);

        if (!expected.IsParsed)
            throw new ArgumentException("The expected schema must be parseable", nameof(expected));

        if (!element.TryGetProperty(PropertySchema, out var schemaElement))
            return CompatibilityResult.Missing();

        if (schemaElement.ValueKind != JsonValueKind.String)
            return CompatibilityResult.Invalid(schemaElement.GetRawText());

        var actual = new JsonSchema(schemaElement.GetString()!);
        if (!actual.IsParsed)
            return CompatibilityResult.Unparseable(actual);

        if (!string.Equals(actual.Id, expected.Id, StringComparison.Ordinal))
            return CompatibilityResult.UnexpectedId(actual);

        if (actual.Version!.Major != expected.Version!.Major)
            return CompatibilityResult.UnsupportedMajor(actual);

        if (actual.Version.Minor > expected.Version.Minor)
            return CompatibilityResult.ReadOnlyPreserveSchema(actual);

        if (actual.Version.Minor == expected.Version.Minor)
            return CompatibilityResult.ReadWritePreserveSchema(actual);

        return CompatibilityResult.ReadWrite(actual);
    }

    /// <summary>
    /// Parses a schema URL owned by the launcher.
    /// </summary>
    /// <param name="value">The raw schema string.</param>
    /// <returns>The parsed schema, or null when the string is not a schema URL.</returns>
    private static ParsedSchema? ParseSchemaUrl(string value)
    {
        if (!value.StartsWith(UrlPrefix, StringComparison.Ordinal))
            return null;

        var path = value.Substring(UrlPrefix.Length);
        var slash = path.IndexOf('/');
        if (slash <= 0 || slash != path.LastIndexOf('/') || slash == path.Length - 1)
            return null;

        var id = path.Substring(0, slash);
        if (!IsValidId(id))
            return null;

        var versionString = path.Substring(slash + 1);
        if (versionString.Length == 0)
            return null;

        if (!SchemaVersion.TryParse(versionString, out var version))
            return null;

        // Only the canonical form and the form without the patch number are
        // accepted.
        if (versionString != version.ToString() &&
            !(version.Patch == 0 && versionString == $"{version.Major}.{version.Minor}"))
            return null;

        return new ParsedSchema(id, version);
    }

    /// <summary>
    /// Returns whether a schema identifier is valid.
    /// </summary>
    /// <param name="id">The identifier to check.</param>
    /// <returns>Whether the identifier is valid.</returns>
    private static bool IsValidId(string id)
    {
        if (id.Length == 0)
            return false;

        if (id[0] < 'a' || id[0] > 'z')
            return false;

        for (var i = 1; i < id.Length; i++)
        {
            var ch = id[i];
            if (!((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9') || ch == '-'))
                return false;
        }

        return true;
    }

    /// <inheritdoc/>
    public bool Equals(JsonSchema? other)
        => other is not null && string.Equals(Value, other.Value, StringComparison.Ordinal);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as JsonSchema);

    /// <inheritdoc/>
    public override int GetHashCode() => Value.GetHashCode(StringComparison.Ordinal);

    /// <inheritdoc/>
    public override string ToString() => Value;

    /// <summary>
    /// A parsed schema URL.
    /// </summary>
    private sealed class ParsedSchema
    {
        public ParsedSchema(string id, SchemaVersion version)
        {
            Id = id;
            Version = version;
        }

        public string Id { get; }

        public SchemaVersion Version { get; }
    }

    /// <summary>
    /// The version of a schema.
    /// </summary>
    public sealed class SchemaVersion : IComparable<SchemaVersion>, IEquatable<SchemaVersion>
    {
        /// <summary>
        /// Creates a schema version.
        /// </summary>
        /// <param name="major">The major version.</param>
        /// <param name="minor">The minor version.</param>
        /// <param name="patch">The patch version.</param>
        public SchemaVersion(int major, int minor, int patch)
        {
            if (major < 0)
                throw new ArgumentOutOfRangeException(nameof(major));
            if (minor < 0)
                throw new ArgumentOutOfRangeException(nameof(minor));
            if (patch < 0)
                throw new ArgumentOutOfRangeException(nameof(patch));

            Major = major;
            Minor = minor;
            Patch = patch;
        }

        /// <summary>
        /// The major version.
        /// </summary>
        public int Major { get; }

        /// <summary>
        /// The minor version.
        /// </summary>
        public int Minor { get; }

        /// <summary>
        /// The patch version.
        /// </summary>
        public int Patch { get; }

        /// <summary>
        /// Parses a schema version string.
        /// </summary>
        /// <param name="version">
        /// The version string in the <c>major.minor</c> or
        /// <c>major.minor.patch</c> form.
        /// </param>
        /// <param name="result">The parsed version.</param>
        /// <returns>Whether the version string is valid.</returns>
        public static bool TryParse(string version, out SchemaVersion result)
        {
            result = new SchemaVersion(0, 0, 0);

            var firstDot = version.IndexOf('.');
            var secondDot = firstDot < 0 ? -1 : version.IndexOf('.', firstDot + 1);

            if (firstDot <= 0 ||
                firstDot == version.Length - 1 ||
                (secondDot >= 0 &&
                 (secondDot <= firstDot + 1 ||
                  secondDot != version.LastIndexOf('.') ||
                  secondDot == version.Length - 1)))
                return false;

            if (!TryParsePart(version, 0, firstDot, out var major))
                return false;

            if (secondDot >= 0)
            {
                if (!TryParsePart(version, firstDot + 1, secondDot, out var minor) ||
                    !TryParsePart(version, secondDot + 1, version.Length, out var patch))
                    return false;

                result = new SchemaVersion(major, minor, patch);
            }
            else
            {
                if (!TryParsePart(version, firstDot + 1, version.Length, out var minor))
                    return false;

                result = new SchemaVersion(major, minor, 0);
            }

            return true;
        }

        /// <summary>
        /// Parses a part of a schema version.
        /// </summary>
        private static bool TryParsePart(string version, int start, int end, out int result)
        {
            result = 0;

            for (var i = start; i < end; i++)
            {
                var ch = version[i];
                if (ch < '0' || ch > '9')
                    return false;
            }

            return int.TryParse(
                version.Substring(start, end - start), NumberStyles.None, CultureInfo.InvariantCulture, out result);
        }

        /// <summary>
        /// Compares the version with another one.
        /// </summary>
        /// <param name="other">The version to compare with.</param>
        /// <returns>The result of the comparison.</returns>
        public int CompareTo(SchemaVersion? other)
        {
            if (other is null)
                return 1;

            var result = Major.CompareTo(other.Major);
            if (result != 0)
                return result;

            result = Minor.CompareTo(other.Minor);
            if (result != 0)
                return result;

            return Patch.CompareTo(other.Patch);
        }

        /// <inheritdoc/>
        public bool Equals(SchemaVersion? other)
            => other is not null &&
               Major == other.Major && Minor == other.Minor && Patch == other.Patch;

        /// <inheritdoc/>
        public override bool Equals(object? obj) => Equals(obj as SchemaVersion);

        /// <inheritdoc/>
        public override int GetHashCode() => HashCode.Combine(Major, Minor, Patch);

        /// <inheritdoc/>
        public override string ToString()
            => $"{Major}.{Minor}.{Patch}";
    }

    /// <summary>
    /// The result of checking whether a JSON file can be read and safely saved.
    /// </summary>
    public sealed class CompatibilityResult
    {
        private CompatibilityResult(CompatibilityStatus status, JsonSchema? actual, string? invalidValue)
        {
            Status = status;
            Actual = actual;
            InvalidValue = invalidValue;

            Readable = status is CompatibilityStatus.ReadWrite or CompatibilityStatus.ReadWritePreserveSchema or CompatibilityStatus.ReadOnlyPreserveSchema;
            AllowSave = status is CompatibilityStatus.ReadWrite or CompatibilityStatus.ReadWritePreserveSchema;
            PreserveSchema = status is CompatibilityStatus.ReadWritePreserveSchema or CompatibilityStatus.ReadOnlyPreserveSchema;
        }

        /// <summary>
        /// The compatibility status of the file.
        /// </summary>
        public CompatibilityStatus Status { get; }

        /// <summary>
        /// Whether the file may be deserialized.
        /// </summary>
        public bool Readable { get; }

        /// <summary>
        /// Whether the file may be overwritten.
        /// </summary>
        public bool AllowSave { get; }

        /// <summary>
        /// Whether saving the file should preserve the original schema marker.
        /// </summary>
        public bool PreserveSchema { get; }

        /// <summary>
        /// The schema read from the file, or null when the file does not have a
        /// valid schema marker.
        /// </summary>
        public JsonSchema? Actual { get; }

        /// <summary>
        /// The raw text of an invalid schema marker.
        /// </summary>
        public string? InvalidValue { get; }

        /// <summary>
        /// Creates a readable and writable result.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult ReadWrite(JsonSchema actual)
            => new(CompatibilityStatus.ReadWrite, actual, null);

        /// <summary>
        /// Creates a readable and writable result that preserves the original
        /// schema marker.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult ReadWritePreserveSchema(JsonSchema actual)
            => new(CompatibilityStatus.ReadWritePreserveSchema, actual, null);

        /// <summary>
        /// Creates a readable result that does not allow the file to be
        /// overwritten.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult ReadOnlyPreserveSchema(JsonSchema actual)
            => new(CompatibilityStatus.ReadOnlyPreserveSchema, actual, null);

        /// <summary>
        /// Creates a result for a file without a schema marker.
        /// </summary>
        /// <returns>The created result.</returns>
        public static CompatibilityResult Missing() => new(CompatibilityStatus.Missing, null, null);

        /// <summary>
        /// Creates a result for a file with an invalid schema marker.
        /// </summary>
        /// <param name="invalidValue">The raw text of the schema marker.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult Invalid(string invalidValue)
            => new(CompatibilityStatus.Invalid, null, invalidValue);

        /// <summary>
        /// Creates a result for a file with a schema marker that is not a schema
        /// URL owned by the launcher.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult Unparseable(JsonSchema actual)
            => new(CompatibilityStatus.Unparseable, actual, null);

        /// <summary>
        /// Creates a result for a file with an unexpected schema identifier.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult UnexpectedId(JsonSchema actual)
            => new(CompatibilityStatus.UnexpectedId, actual, null);

        /// <summary>
        /// Creates a result for a file with an unsupported major schema version.
        /// </summary>
        /// <param name="actual">The schema read from the file.</param>
        /// <returns>The created result.</returns>
        public static CompatibilityResult UnsupportedMajor(JsonSchema actual)
            => new(CompatibilityStatus.UnsupportedMajor, actual, null);

        /// <summary>
        /// The compatibility status of a file.
        /// </summary>
        public enum CompatibilityStatus
        {
            /// <summary>
            /// The file matches an older compatible schema and may be rewritten.
            /// </summary>
            ReadWrite,

            /// <summary>
            /// The file may be rewritten, but the original schema marker should
            /// be preserved.
            /// </summary>
            ReadWritePreserveSchema,

            /// <summary>
            /// The file may be read, but it must not be overwritten.
            /// </summary>
            ReadOnlyPreserveSchema,

            /// <summary>
            /// The file does not have a schema marker.
            /// </summary>
            Missing,

            /// <summary>
            /// The schema marker of the file is not a string.
            /// </summary>
            Invalid,

            /// <summary>
            /// The schema marker of the file is not a schema URL owned by the
            /// launcher.
            /// </summary>
            Unparseable,

            /// <summary>
            /// The schema marker of the file describes another kind of file.
            /// </summary>
            UnexpectedId,

            /// <summary>
            /// The major version of the schema of the file is not supported.
            /// </summary>
            UnsupportedMajor
        }

        /// <summary>
        /// Whether the file with the given status may be deserialized.
        /// </summary>
        /// <param name="status">The status to check.</param>
        /// <returns>Whether the file may be deserialized.</returns>
        public static bool IsReadable(CompatibilityStatus status)
            => status is CompatibilityStatus.ReadWrite or CompatibilityStatus.ReadWritePreserveSchema or CompatibilityStatus.ReadOnlyPreserveSchema;

        /// <summary>
        /// Whether the file with the given status may be overwritten.
        /// </summary>
        /// <param name="status">The status to check.</param>
        /// <returns>Whether the file may be overwritten.</returns>
        public static bool IsSaveAllowed(CompatibilityStatus status)
            => status is CompatibilityStatus.ReadWrite or CompatibilityStatus.ReadWritePreserveSchema;

        /// <summary>
        /// Whether saving the file with the given status should preserve the
        /// original schema marker.
        /// </summary>
        /// <param name="status">The status to check.</param>
        /// <returns>Whether the schema marker should be preserved.</returns>
        public static bool ShouldPreserveSchema(CompatibilityStatus status)
            => status is CompatibilityStatus.ReadWritePreserveSchema or CompatibilityStatus.ReadOnlyPreserveSchema;
    }
}
