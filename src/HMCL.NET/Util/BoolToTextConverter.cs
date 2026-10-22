using System;
using System.Globalization;
using Avalonia.Data.Converters;

namespace HMCL.NET.Util;

/// <summary>
/// Converts a boolean into a short Chinese label, used to show the enabled
/// state of resource packs and data packs in the list views.
/// </summary>
public sealed class BoolToTextConverter : IValueConverter
{
    /// <summary>
    /// The shared instance of the converter.
    /// </summary>
    public static BoolToTextConverter Instance { get; } = new();

    /// <summary>
    /// The text shown when the value is true.
    /// </summary>
    public string TrueText { get; set; } = Lang.Get("hmcl.booltotextconverter.828_");

    /// <summary>
    /// The text shown when the value is false.
    /// </summary>
    public string FalseText { get; set; } = Lang.Get("hmcl.booltotextconverter.830_");

    /// <summary>
    /// Converts the value to its label.
    /// </summary>
    /// <param name="value">The value to convert.</param>
    /// <param name="targetType">The requested target type.</param>
    /// <param name="parameter">An optional converter parameter overriding the labels.</param>
    /// <param name="culture">The culture used for the conversion.</param>
    /// <returns>The label of the value.</returns>
    public object? Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
    {
        var enabled = value is true;
        if (parameter is string text)
            return enabled ? text : string.Empty;

        return enabled ? TrueText : FalseText;
    }

    /// <summary>
    /// Conversion back is not supported.
    /// </summary>
    /// <param name="value">The value to convert.</param>
    /// <param name="targetType">The requested target type.</param>
    /// <param name="parameter">The converter parameter.</param>
    /// <param name="culture">The culture used for the conversion.</param>
    /// <returns>Never returns.</returns>
    public object? ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => throw new NotSupportedException();
}
