using System.Collections;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page holding a combo box, like the line select
/// controls of Java HMCL.
/// </summary>
public class LineSelect : Line
{
    /// <summary>
    /// The items offered by the combo box.
    /// </summary>
    public static readonly StyledProperty<IEnumerable?> ItemsSourceProperty =
        AvaloniaProperty.Register<LineSelect, IEnumerable?>(nameof(ItemsSource));

    /// <summary>
    /// The item chosen by the user.
    /// </summary>
    public static readonly StyledProperty<object?> SelectedItemProperty =
        AvaloniaProperty.Register<LineSelect, object?>(nameof(SelectedItem));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public LineSelect()
    {
        var comboBox = new ComboBox
        {
            MinWidth = 180,
            HorizontalAlignment = HorizontalAlignment.Left
        };

        PropertyLink.Copy(this, ItemsSourceProperty, comboBox, ItemsControl.ItemsSourceProperty);
        PropertyLink.Link(this, SelectedItemProperty, comboBox, ComboBox.SelectedItemProperty);

        Content = comboBox;
    }

    /// <summary>
    /// The items offered by the combo box.
    /// </summary>
    public IEnumerable? ItemsSource
    {
        get => GetValue(ItemsSourceProperty);
        set => SetValue(ItemsSourceProperty, value);
    }

    /// <summary>
    /// The item chosen by the user.
    /// </summary>
    public object? SelectedItem
    {
        get => GetValue(SelectedItemProperty);
        set => SetValue(SelectedItemProperty, value);
    }
}
