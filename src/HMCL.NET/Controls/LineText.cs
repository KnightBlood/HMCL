using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page holding a text field, like the line text controls
/// of Java HMCL.
/// </summary>
public class LineText : Line
{
    /// <summary>
    /// The text edited by the user.
    /// </summary>
    public static readonly StyledProperty<string?> TextProperty =
        AvaloniaProperty.Register<LineText, string?>(nameof(Text));

    /// <summary>
    /// The hint shown in the empty text field.
    /// </summary>
    public static readonly StyledProperty<string?> PlaceholderTextProperty =
        AvaloniaProperty.Register<LineText, string?>(nameof(PlaceholderText));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public LineText()
    {
        var textBox = new TextBox
        {
            MinWidth = 180,
            HorizontalAlignment = HorizontalAlignment.Left
        };

        PropertyLink.Link(this, TextProperty, textBox, TextBox.TextProperty);
        PropertyLink.Copy(this, PlaceholderTextProperty, textBox, TextBox.PlaceholderTextProperty);

        Content = textBox;
    }

    /// <summary>
    /// The text edited by the user.
    /// </summary>
    public string? Text
    {
        get => GetValue(TextProperty);
        set => SetValue(TextProperty, value);
    }

    /// <summary>
    /// The hint shown in the empty text field.
    /// </summary>
    public string? PlaceholderText
    {
        get => GetValue(PlaceholderTextProperty);
        set => SetValue(PlaceholderTextProperty, value);
    }
}
