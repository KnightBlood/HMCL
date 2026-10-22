using System;
using System.Collections.Generic;
using System.Globalization;
using System.Numerics;
using System.Text;

namespace HMCLCore.NET.Util.Versioning;

/// <summary>
/// A comparable version number, following the version order specification of
/// Maven.
/// </summary>
/// <remarks>
/// Ported from the Java implementation of HMCL, which itself is a copy of
/// <c>org.apache.maven.artifact.versioning.ComparableVersion</c>.
/// </remarks>
public sealed class VersionNumber : IComparable<VersionNumber>, IEquatable<VersionNumber>
{
    /// <summary>
    /// The version number zero.
    /// </summary>
    public static readonly VersionNumber Zero = AsVersion("0");

    /// <summary>
    /// The characters that separate the parts of a version number.
    /// </summary>
    private const string Separators = "!\"#$%&'()*+,-/:;<=>?@[\\]^_`{|}~";

    /// <summary>
    /// The maximum length of a number that is stored in a 64 bit integer.
    /// </summary>
    private const int MaxLongItemLength = 18;

    private readonly string _value;
    private readonly ListItem _items;
    private readonly string _canonical;

    private VersionNumber(string version)
    {
        _value = version ?? throw new ArgumentNullException(nameof(version));

        var list = new ListItem();
        _items = list;

        var stack = new Stack<ListItem>();
        stack.Push(list);

        var isDigit = false;
        var startIndex = 0;

        for (var i = 0; i < version.Length; i++)
        {
            var c = version[i];

            if (c == '.')
            {
                list.Add(i == startIndex ? LongItem.Zero : ParseItem(version.Substring(startIndex, i - startIndex)));
                startIndex = i + 1;
            }
            else if (Separators.IndexOf(c) >= 0)
            {
                list.Add(i == startIndex ? LongItem.Zero : ParseItem(version.Substring(startIndex, i - startIndex)));
                startIndex = i + 1;

                var separated = new ListItem(c);
                list.Add(separated);
                stack.Push(separated);
                list = separated;
            }
            else if (c >= '0' && c <= '9')
            {
                if (!isDigit && i > startIndex)
                {
                    list.Add(ParseItem(version.Substring(startIndex, i - startIndex)));
                    startIndex = i;

                    var nested = new ListItem();
                    list.Add(nested);
                    stack.Push(nested);
                    list = nested;
                }

                isDigit = true;
            }
            else
            {
                if (isDigit && i > startIndex)
                {
                    list.Add(ParseItem(version.Substring(startIndex, i - startIndex)));
                    startIndex = i;

                    var nested = new ListItem();
                    list.Add(nested);
                    stack.Push(nested);
                    list = nested;
                }

                isDigit = false;
            }
        }

        if (version.Length > startIndex)
            list.Add(ParseItem(version.Substring(startIndex)));

        while (stack.Count > 0)
        {
            list = stack.Pop();
            list.Normalize();
        }

        _canonical = _items.ToString();
    }

    /// <summary>
    /// Parses a version string into a version number.
    /// </summary>
    /// <param name="version">The version string to parse.</param>
    /// <returns>The parsed version number.</returns>
    public static VersionNumber AsVersion(string version) => new(version);

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
        => AsVersion(version1).CompareTo(AsVersion(version2));

    /// <summary>
    /// Normalizes a version string to its canonical form.
    /// </summary>
    /// <param name="str">The version string to normalize.</param>
    /// <returns>The canonical form of the version string.</returns>
    public static string Normalize(string str) => AsVersion(str).GetCanonical();

    /// <summary>
    /// Returns whether the version string consists of numbers separated by
    /// dots only.
    /// </summary>
    /// <param name="version">The version string to check.</param>
    /// <returns>Whether the version string is a simple version number.</returns>
    public static bool IsIntVersionNumber(string version)
    {
        if (string.IsNullOrEmpty(version))
            return false;

        var idx = 0;
        var cont = true;
        do
        {
            var dotIndex = version.IndexOf('.', idx);
            if (dotIndex == idx || dotIndex == version.Length - 1)
                return false;

            int endIndex;
            if (dotIndex < 0)
            {
                cont = false;
                endIndex = version.Length;
            }
            else
            {
                endIndex = dotIndex;
            }

            if (endIndex - idx > 9)
                return false;

            for (var i = idx; i < endIndex; i++)
            {
                var ch = version[i];
                if (ch < '0' || ch > '9')
                    return false;
            }

            idx = endIndex + 1;
        }
        while (cont);

        return true;
    }

    /// <summary>
    /// Returns the version string the version number was parsed from.
    /// </summary>
    /// <returns>The version string of the version number.</returns>
    public string GetVersion() => _value;

    /// <summary>
    /// Returns the canonical form of the version number.
    /// </summary>
    /// <returns>The canonical form of the version number.</returns>
    public string GetCanonical() => _canonical;

    /// <summary>
    /// Compares the version number with another one.
    /// </summary>
    /// <param name="other">The version number to compare with.</param>
    /// <returns>
    /// A negative number, zero, or a positive number when this version is
    /// lower than, equal to, or higher than the other one.
    /// </returns>
    public int CompareTo(VersionNumber? other)
    {
        if (other is null)
            return 1;

        return _items.CompareTo(other._items);
    }

    /// <summary>
    /// Returns whether the version number is equal to another one.
    /// </summary>
    /// <param name="other">The version number to compare with.</param>
    /// <returns>Whether both version numbers have the same canonical form.</returns>
    public bool Equals(VersionNumber? other)
        => other is not null && string.Equals(_canonical, other._canonical, StringComparison.Ordinal);

    /// <inheritdoc/>
    public override bool Equals(object? obj) => Equals(obj as VersionNumber);

    /// <inheritdoc/>
    public override int GetHashCode() => _canonical.GetHashCode(StringComparison.Ordinal);

    /// <inheritdoc/>
    public override string ToString() => _value;

    /// <summary>
    /// Parses a part of a version number into an item.
    /// </summary>
    /// <param name="buf">The part of the version number to parse.</param>
    /// <returns>The item of the part.</returns>
    private static Item ParseItem(string buf)
    {
        var numberLength = 0;
        var leadingZero = true;
        foreach (var ch in buf)
        {
            if (ch >= '0' && ch <= '9')
            {
                if (ch != '0')
                    leadingZero = false;

                if (!leadingZero)
                    numberLength++;
            }
            else
            {
                return new StringItem(buf);
            }
        }

        if (numberLength == 0)
            return LongItem.Zero;
        if (numberLength <= MaxLongItemLength)
            return new LongItem(long.Parse(buf, CultureInfo.InvariantCulture));

        return new BigIntegerItem(buf);
    }

    /// <summary>
    /// An item of a version number.
    /// </summary>
    private abstract class Item
    {
        /// <summary>The type of a number item.</summary>
        public const int LongItemType = 0;

        /// <summary>The type of a big number item.</summary>
        public const int BigIntegerItemType = 1;

        /// <summary>The type of a string item.</summary>
        public const int StringItemType = 2;

        /// <summary>The type of a list item.</summary>
        public const int ListItemType = 3;

        /// <summary>
        /// The type of the item.
        /// </summary>
        public abstract int Type { get; }

        /// <summary>
        /// Whether the item is equal to the implicit item of the following
        /// part of the version number.
        /// </summary>
        public abstract bool IsNull { get; }

        /// <summary>
        /// Compares the item with another one.
        /// </summary>
        /// <param name="item">The item to compare with, or null.</param>
        /// <returns>The result of the comparison.</returns>
        public abstract int CompareTo(Item? item);

        /// <summary>
        /// Appends the canonical form of the item to a buffer.
        /// </summary>
        /// <param name="buffer">The buffer to append to.</param>
        public abstract void AppendTo(StringBuilder buffer);
    }

    /// <summary>
    /// A numeric item of a version number.
    /// </summary>
    private sealed class LongItem : Item
    {
        /// <summary>
        /// The item of the number zero.
        /// </summary>
        public static readonly LongItem Zero = new(0L);

        private readonly long _value;

        public LongItem(long value) => _value = value;

        public override int Type => LongItemType;

        public override bool IsNull => _value == 0L;

        public override int CompareTo(Item? item)
        {
            if (item is null)
                return _value == 0L ? 0 : 1;

            return item.Type switch
            {
                LongItemType => _value.CompareTo(((LongItem)item)._value),
                BigIntegerItemType => -1,
                StringItemType => 1,
                ListItemType => 1,
                _ => throw new InvalidOperationException("invalid item")
            };
        }

        public override void AppendTo(StringBuilder buffer)
            => buffer.Append(_value.ToString(CultureInfo.InvariantCulture));

        public override string ToString() => _value.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// A numeric item of a version number that does not fit into a 64 bit
    /// integer.
    /// </summary>
    private sealed class BigIntegerItem : Item
    {
        private readonly BigInteger _value;

        public BigIntegerItem(string value) => _value = BigInteger.Parse(value, CultureInfo.InvariantCulture);

        public override int Type => BigIntegerItemType;

        public override bool IsNull => false;

        public override int CompareTo(Item? item)
        {
            if (item is null)
                return 1;

            return item.Type switch
            {
                LongItemType => 1,
                BigIntegerItemType => _value.CompareTo(((BigIntegerItem)item)._value),
                StringItemType => 1,
                ListItemType => 1,
                _ => throw new InvalidOperationException("invalid item")
            };
        }

        public override void AppendTo(StringBuilder buffer)
            => buffer.Append(_value.ToString(CultureInfo.InvariantCulture));

        public override string ToString() => _value.ToString(CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// A qualifier of a version number.
    /// </summary>
    private sealed class StringItem : Item
    {
        private static readonly string[] PreQualifiers = { "alpha", "beta", "pre", "rc", "experimental" };

        private readonly string _value;
        private readonly bool _pre;

        public StringItem(string value)
        {
            _value = value;

            var trimmed = value.Trim().ToLowerInvariant();
            _pre = PreQualifiers.Any(qualifier => trimmed.StartsWith(qualifier, StringComparison.Ordinal));
        }

        public override int Type => StringItemType;

        public override bool IsNull => _value.Length == 0;

        public override int CompareTo(Item? item)
        {
            if (item is null)
                return _pre ? -1 : 1;

            return item.Type switch
            {
                LongItemType => -1,
                BigIntegerItemType => -1,
                StringItemType => string.CompareOrdinal(_value, ((StringItem)item)._value),
                ListItemType => -1,
                _ => throw new InvalidOperationException("invalid item")
            };
        }

        public override void AppendTo(StringBuilder buffer) => buffer.Append(_value);

        public override string ToString() => _value;
    }

    /// <summary>
    /// A list of items of a version number.
    /// </summary>
    private sealed class ListItem : Item, IEnumerable<Item>
    {
        private readonly List<Item> _items = new();
        private readonly char? _separator;

        public ListItem() => _separator = null;

        public ListItem(char separator) => _separator = separator;

        public int Count => _items.Count;

        public Item this[int index] => _items[index];

        public override int Type => ListItemType;

        public override bool IsNull => _items.Count == 0;

        /// <summary>
        /// Adds an item to the list.
        /// </summary>
        /// <param name="item">The item to add.</param>
        public void Add(Item item) => _items.Add(item);

        /// <summary>
        /// Removes the trailing items that are equal to the implicit item of
        /// the following part of the version number.
        /// </summary>
        public void Normalize()
        {
            for (var i = _items.Count - 1; i >= 0; i--)
            {
                var lastItem = _items[i];

                if (lastItem.IsNull)
                    _items.RemoveAt(i);
                else if (lastItem is not ListItem)
                    break;
            }
        }

        public override int CompareTo(Item? item)
        {
            if (item is null)
            {
                if (_items.Count == 0)
                    return 0;

                return _items[0].CompareTo(null);
            }

            switch (item.Type)
            {
                case LongItemType:
                case BigIntegerItemType:
                    return -1;

                case StringItemType:
                    return 1;

                case ListItemType:
                {
                    var other = (ListItem)item;
                    var count = Math.Max(_items.Count, other._items.Count);
                    for (var i = 0; i < count; i++)
                    {
                        var left = i < _items.Count ? _items[i] : null;
                        var right = i < other._items.Count ? other._items[i] : null;

                        var result = left is null
                            ? (right is null ? 0 : -1 * right.CompareTo(left))
                            : left.CompareTo(right);

                        if (result != 0)
                            return result;
                    }

                    return 0;
                }

                default:
                    throw new InvalidOperationException("invalid item");
            }
        }

        public override void AppendTo(StringBuilder buffer)
        {
            if (_separator.HasValue)
                buffer.Append(_separator.Value);

            var initLength = buffer.Length;

            foreach (var item in _items)
            {
                if (buffer.Length > initLength && item is not ListItem)
                    buffer.Append('.');
                item.AppendTo(buffer);
            }
        }

        public IEnumerator<Item> GetEnumerator() => _items.GetEnumerator();

        System.Collections.IEnumerator System.Collections.IEnumerable.GetEnumerator() => _items.GetEnumerator();

        public override string ToString()
        {
            var buffer = new StringBuilder();
            AppendTo(buffer);
            return buffer.ToString();
        }
    }
}
