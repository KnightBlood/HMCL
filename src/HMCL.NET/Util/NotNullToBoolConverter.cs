using System;
using System.Globalization;
using Avalonia.Data.Converters;

namespace HMCL.NET.Util;

/// <summary>
/// Converts null to <c>true</c> and non-null to <c>false</c>, so that the
/// empty states of the launcher can be shown when nothing is selected.
public sealed class NotNullToBoolConverter : IValueConverter
{
    /// <inheritdoc />
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is null;

    /// <inheritdoc />
    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
