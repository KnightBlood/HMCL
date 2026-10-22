using System;
using System.Globalization;
using Avalonia.Data.Converters;

namespace HMCL.NET.Util;

/// <summary>
/// Converts null to <c>false</c> and non-null to <c>true</c>.
/// </summary>
public sealed class NullToBoolConverter : IValueConverter
{
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is not null;

    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
