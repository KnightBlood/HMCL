using Avalonia;
using Avalonia.Controls;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page holding a switch, like the line toggle controls
/// of Java HMCL.
/// </summary>
public class LineToggle : Line
{
    /// <summary>
    /// Whether the switch is turned on.
    /// </summary>
    public static readonly StyledProperty<bool?> IsCheckedProperty =
        AvaloniaProperty.Register<LineToggle, bool?>(nameof(IsChecked));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public LineToggle()
    {
        // The switch shows no text, like the switches of Java HMCL.
        var toggle = new ToggleSwitch
        {
            OnContent = string.Empty,
            OffContent = string.Empty
        };

        PropertyLink.Link(this, IsCheckedProperty, toggle, ToggleSwitch.IsCheckedProperty);

        Content = toggle;
    }

    /// <summary>
    /// Whether the switch is turned on.
    /// </summary>
    public bool? IsChecked
    {
        get => GetValue(IsCheckedProperty);
        set => SetValue(IsCheckedProperty, value);
    }
}
