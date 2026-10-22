using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Templates;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page, showing a title on the left and the edited value
/// on the right, like the line controls of Java HMCL.
/// </summary>
/// <remarks>
/// The content of the row is the control shown on its right side; the title
/// and the subtitle of the row are shown on its left side.
/// </remarks>
public class Line : ContentControl
{
    /// <summary>
    /// The title of the row.
    /// </summary>
    public static readonly StyledProperty<string?> TitleProperty =
        AvaloniaProperty.Register<Line, string?>(nameof(Title));

    /// <summary>
    /// The subtitle shown under the title of the row.
    /// </summary>
    public static readonly StyledProperty<string?> SubtitleProperty =
        AvaloniaProperty.Register<Line, string?>(nameof(Subtitle));

    /// <summary>
    /// The node shown before the title of the row, usually an icon.
    /// </summary>
    public static readonly StyledProperty<object?> LeadingProperty =
        AvaloniaProperty.Register<Line, object?>(nameof(Leading));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public Line()
    {
        MinHeight = 48;
        HorizontalContentAlignment = HorizontalAlignment.Stretch;
        Template = new FuncControlTemplate<Line>((line, _) => BuildLayout(line));
    }

    /// <summary>
    /// The title of the row.
    /// </summary>
    public string? Title
    {
        get => GetValue(TitleProperty);
        set => SetValue(TitleProperty, value);
    }

    /// <summary>
    /// The subtitle shown under the title of the row.
    /// </summary>
    public string? Subtitle
    {
        get => GetValue(SubtitleProperty);
        set => SetValue(SubtitleProperty, value);
    }

    /// <summary>
    /// The node shown before the title of the row, usually an icon.
    /// </summary>
    public object? Leading
    {
        get => GetValue(LeadingProperty);
        set => SetValue(LeadingProperty, value);
    }

    /// <summary>
    /// Builds the layout of the row: the leading node, the title with its
    /// subtitle and the content of the row.
    /// </summary>
    /// <param name="line">The row being built.</param>
    /// <returns>The layout of the row.</returns>
    private static Control BuildLayout(Line line)
    {
        // The columns are proportional, so that the row keeps its shape when
        // the window is resized: an automatic column would take the whole
        // width of the title and squeeze the content out of the row.
        var grid = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("Auto,2*,3*")
        };

        var leading = new ContentPresenter
        {
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(0, 0, 12, 0)
        };
        PropertyLink.Copy(line, LeadingProperty, leading, ContentPresenter.ContentProperty);
        Grid.SetColumn(leading, 0);

        var titles = new StackPanel
        {
            VerticalAlignment = VerticalAlignment.Center,
            Margin = new Thickness(0, 0, 16, 0)
        };

        var title = new TextBlock { TextWrapping = TextWrapping.Wrap };
        PropertyLink.Copy(line, TitleProperty, title, TextBlock.TextProperty);
        titles.Children.Add(title);

        var subtitle = new TextBlock
        {
            FontSize = 11,
            Foreground = new SolidColorBrush(Color.Parse("#888888")),
            TextWrapping = TextWrapping.Wrap
        };
        PropertyLink.Copy(line, SubtitleProperty, subtitle, TextBlock.TextProperty);
        titles.Children.Add(subtitle);

        Grid.SetColumn(titles, 1);

        var content = new ContentPresenter
        {
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        PropertyLink.Copy(line, ContentProperty, content, ContentPresenter.ContentProperty);
        Grid.SetColumn(content, 2);

        grid.Children.Add(leading);
        grid.Children.Add(titles);
        grid.Children.Add(content);

        return grid;
    }
}
