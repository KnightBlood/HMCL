using System;
using System.Globalization;
using Avalonia.Data.Converters;
using HMCL.NET.Util;

namespace HMCL.NET.Util;

/// <summary>
/// Converts the kind of a version of the game into the text shown to the
/// user, like the tags of the versions of Java HMCL.
public sealed class VersionTypeConverter : IValueConverter
{
    /// <inheritdoc />
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => (value as string) switch
        {
            "release" => Lang.Get("instance.game.release", "Release"),
            "snapshot" => Lang.Get("instance.game.snapshot", "Snapshot"),
            "old_beta" => Lang.Get("instance.game.old", "Old"),
            "old_alpha" => Lang.Get("instance.game.old", "Old"),
            _ => value?.ToString() ?? string.Empty
        };

    /// <inheritdoc />
    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
