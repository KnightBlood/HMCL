using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Templates;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// The title of a group of items of a navigation drawer, with a line under it,
/// like the class titles of Java HMCL.
/// </summary>
public class ClassTitle : ContentControl
{
    /// <summary>
    /// The title of the group.
    /// </summary>
    public static readonly StyledProperty<string?> TitleProperty =
        AvaloniaProperty.Register<ClassTitle, string?>(nameof(Title));

    /// <summary>
    /// The help shown under the title of the group.
    /// </summary>
    public static readonly StyledProperty<string?> SubtitleProperty =
        AvaloniaProperty.Register<ClassTitle, string?>(nameof(Subtitle));

    /// <summary>
    /// Creates the title.
    /// </summary>
    public ClassTitle()
    {
        Template = new FuncControlTemplate<ClassTitle>((title, _) => BuildLayout(title));
    }

    /// <summary>
    /// The title of the group.
    /// </summary>
    public string? Title
    {
        get => GetValue(TitleProperty);
        set => SetValue(TitleProperty, value);
    }

    /// <summary>
    /// The help shown under the title of the group.
    /// </summary>
    public string? Subtitle
    {
        get => GetValue(SubtitleProperty);
        set => SetValue(SubtitleProperty, value);
    }

    /// <summary>
    /// Builds the layout of the title: the text of the title, the node shown
    /// on its right side and the line under them.
    /// </summary>
    /// <param name="title">The title being built.</param>
    /// <returns>The layout of the title.</returns>
    private static Control BuildLayout(ClassTitle title)
    {
        var grid = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("*,Auto"),
            RowDefinitions = new RowDefinitions("Auto,Auto,Auto")
        };

        var label = new TextBlock
        {
            FontSize = 12,
            FontWeight = FontWeight.Normal,
            Foreground = new SolidColorBrush(Color.Parse("#888888"))
        };
        PropertyLink.Copy(title, TitleProperty, label, TextBlock.TextProperty);
        Grid.SetColumn(label, 0);

        var subtitle = new TextBlock
        {
            FontSize = 11,
            Foreground = new SolidColorBrush(Color.Parse("#AAAAAA")),
            TextWrapping = TextWrapping.Wrap
        };
        PropertyLink.Copy(title, SubtitleProperty, subtitle, TextBlock.TextProperty);
        Grid.SetColumn(subtitle, 0);
        Grid.SetRow(subtitle, 1);
        Grid.SetColumnSpan(subtitle, 2);

        var trailing = new ContentPresenter
        {
            HorizontalAlignment = HorizontalAlignment.Right,
            VerticalAlignment = VerticalAlignment.Center
        };
        PropertyLink.Copy(title, ContentProperty, trailing, ContentPresenter.ContentProperty);
        Grid.SetColumn(trailing, 1);

        var line = new Border
        {
            Height = 1,
            Background = new SolidColorBrush(Color.Parse("#DDDDDD")),
            Margin = new Thickness(0, 4, 0, 0)
        };
        Grid.SetRow(line, 2);
        Grid.SetColumnSpan(line, 2);

        grid.Children.Add(label);
        grid.Children.Add(subtitle);
        grid.Children.Add(trailing);
        grid.Children.Add(line);

        return grid;
    }
}
