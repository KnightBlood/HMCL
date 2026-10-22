using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page holding a number field, like the line number
/// controls of Java HMCL.
/// </summary>
public class LineNumber : Line
{
    /// <summary>
    /// The value edited by the user.
    /// </summary>
    public static readonly StyledProperty<decimal?> ValueProperty =
        AvaloniaProperty.Register<LineNumber, decimal?>(nameof(Value));

    /// <summary>
    /// The smallest value accepted by the field.
    /// </summary>
    public static readonly StyledProperty<decimal> MinimumProperty =
        AvaloniaProperty.Register<LineNumber, decimal>(nameof(Minimum));

    /// <summary>
    /// The biggest value accepted by the field.
    /// </summary>
    public static readonly StyledProperty<decimal> MaximumProperty =
        AvaloniaProperty.Register<LineNumber, decimal>(nameof(Maximum));

    /// <summary>
    /// The step of the field.
    /// </summary>
    public static readonly StyledProperty<decimal> IncrementProperty =
        AvaloniaProperty.Register<LineNumber, decimal>(nameof(Increment));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public LineNumber()
    {
        var numeric = new NumericUpDown
        {
            MinWidth = 140,
            HorizontalAlignment = HorizontalAlignment.Left,
            FormatString = "0"
        };

        PropertyLink.Link(this, ValueProperty, numeric, NumericUpDown.ValueProperty);
        PropertyLink.Copy(this, MinimumProperty, numeric, NumericUpDown.MinimumProperty);
        PropertyLink.Copy(this, MaximumProperty, numeric, NumericUpDown.MaximumProperty);
        PropertyLink.Copy(this, IncrementProperty, numeric, NumericUpDown.IncrementProperty);

        Content = numeric;
    }

    /// <summary>
    /// The value edited by the user.
    /// </summary>
    public decimal? Value
    {
        get => GetValue(ValueProperty);
        set => SetValue(ValueProperty, value);
    }

    /// <summary>
    /// The smallest value accepted by the field.
    /// </summary>
    public decimal Minimum
    {
        get => GetValue(MinimumProperty);
        set => SetValue(MinimumProperty, value);
    }

    /// <summary>
    /// The biggest value accepted by the field.
    /// </summary>
    public decimal Maximum
    {
        get => GetValue(MaximumProperty);
        set => SetValue(MaximumProperty, value);
    }

    /// <summary>
    /// The step of the field.
    /// </summary>
    public decimal Increment
    {
        get => GetValue(IncrementProperty);
        set => SetValue(IncrementProperty, value);
    }
}
