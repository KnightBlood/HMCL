using System;
using Avalonia;
using Avalonia.Animation;
using Avalonia.Animation.Easings;
using Avalonia.Controls;
using Avalonia.Controls.Presenters;
using Avalonia.Controls.Shapes;
using Avalonia.Controls.Templates;
using Avalonia.Input;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Styling;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// An item of a navigation drawer, showing an icon, a title and an optional
/// subtitle, like the advanced list items of Java HMCL.
/// </summary>
public class AdvancedListItem : Button
{
    /// <summary>
    /// The title of the item.
    /// </summary>
    public static readonly StyledProperty<string?> TitleProperty =
        AvaloniaProperty.Register<AdvancedListItem, string?>(nameof(Title));

    /// <summary>
    /// The subtitle shown under the title of the item.
    /// </summary>
    public static readonly StyledProperty<string?> SubtitleProperty =
        AvaloniaProperty.Register<AdvancedListItem, string?>(nameof(Subtitle));

    /// <summary>
    /// The icon shown before the title of the item.
    /// </summary>
    public static readonly StyledProperty<Control?> IconProperty =
        AvaloniaProperty.Register<AdvancedListItem, Control?>(nameof(Icon));

    private readonly Ellipse _ripple = new()
    {
        Fill = new SolidColorBrush(Color.Parse("#22000000")),
        IsHitTestVisible = false,
        IsVisible = false
    };

    /// <summary>
    /// The opacity of the ripple shown when the item is pressed.
    /// </summary>
    public static readonly StyledProperty<double> RippleOpacityProperty =
        AvaloniaProperty.Register<AdvancedListItem, double>(nameof(RippleOpacity));

    /// <summary>
    /// The scale of the ripple shown when the item is pressed.
    /// </summary>
    public static readonly StyledProperty<double> RippleScaleProperty =
        AvaloniaProperty.Register<AdvancedListItem, double>(nameof(RippleScale));

    /// <summary>
    /// The opacity of the ripple shown when the item is pressed.
    /// </summary>
    public double RippleOpacity
    {
        get => GetValue(RippleOpacityProperty);
        set => SetValue(RippleOpacityProperty, value);
    }

    /// <summary>
    /// The scale of the ripple shown when the item is pressed.
    /// </summary>
    public double RippleScale
    {
        get => GetValue(RippleScaleProperty);
        set => SetValue(RippleScaleProperty, value);
    }

    private double _rippleX;
    private double _rippleY;
    private double _rippleRadius;

    /// <summary>
    /// Creates the item.
    /// </summary>
    public AdvancedListItem()
    {
        Template = new FuncControlTemplate<AdvancedListItem>((item, _) => BuildLayout(item));
    }

    /// <summary>
    /// The title of the item.
    /// </summary>
    public string? Title
    {
        get => GetValue(TitleProperty);
        set => SetValue(TitleProperty, value);
    }

    /// <summary>
    /// The subtitle shown under the title of the item.
    /// </summary>
    public string? Subtitle
    {
        get => GetValue(SubtitleProperty);
        set
        {
            SetValue(SubtitleProperty, value);
            HasSubtitle = !string.IsNullOrEmpty(value);
        }
    }

    /// <summary>
    /// Whether the item shows a subtitle under its title.
    /// </summary>
    public static readonly StyledProperty<bool> HasSubtitleProperty =
        AvaloniaProperty.Register<AdvancedListItem, bool>(nameof(HasSubtitle));

    /// <summary>
    /// Whether the item shows a subtitle under its title.
    /// </summary>
    public bool HasSubtitle
    {
        get => GetValue(HasSubtitleProperty);
        private set => SetValue(HasSubtitleProperty, value);
    }

    /// <summary>
    /// The icon shown before the title of the item.
    /// </summary>
    public Control? Icon
    {
        get => GetValue(IconProperty);
        set => SetValue(IconProperty, value);
    }

    /// <summary>
    /// Builds the layout of the item: the icon, the title with its subtitle
    /// and the content shown on the right side.
    /// </summary>
    /// <param name="item">The item being built.</param>
    /// <returns>The layout of the item.</returns>
    private static Control BuildLayout(AdvancedListItem item)
    {
        // The icon and the title share the first row, so that their vertical
        // centers match; the subtitle is shown under the title.
        var grid = new Grid
        {
            ColumnDefinitions = new ColumnDefinitions("Auto,*,Auto"),
            RowDefinitions = new RowDefinitions("Auto,Auto")
        };

        var icon = new ContentPresenter
        {
            // The icon always takes the same room, so that the titles of every
            // item of the list start at the same place.
            Width = 30,
            Height = 30,
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center,
            Margin = new Thickness(0, 0, 10, 0)
        };
        PropertyLink.Copy(item, IconProperty, icon, ContentPresenter.ContentProperty);
        Grid.SetColumn(icon, 0);
        Grid.SetRow(icon, 0);

        var title = new TextBlock
        {
            VerticalAlignment = VerticalAlignment.Center,
            TextTrimming = TextTrimming.CharacterEllipsis
        };
        PropertyLink.Copy(item, TitleProperty, title, TextBlock.TextProperty);
        Grid.SetColumn(title, 1);
        Grid.SetRow(title, 0);

        var subtitle = new TextBlock
        {
            VerticalAlignment = VerticalAlignment.Top,
            FontSize = 11,
            Foreground = new SolidColorBrush(Color.Parse("#888888")),
            TextTrimming = TextTrimming.CharacterEllipsis
        };
        PropertyLink.Copy(item, SubtitleProperty, subtitle, TextBlock.TextProperty);
        // The row of the subtitle is only used when the item has one.
        PropertyLink.Copy(item, HasSubtitleProperty, subtitle, IsVisibleProperty);
        Grid.SetColumn(subtitle, 1);
        Grid.SetRow(subtitle, 1);

        var trailing = new ContentPresenter
        {
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        PropertyLink.Copy(item, ContentProperty, trailing, ContentPresenter.ContentProperty);
        Grid.SetColumn(trailing, 2);
        Grid.SetRow(trailing, 0);

        grid.Children.Add(icon);
        grid.Children.Add(title);
        grid.Children.Add(subtitle);
        grid.Children.Add(trailing);

        // The border holds the background and the padding of the button, so
        // that the styles of the button keep working with the custom layout.
        var border = new Border
        {
            Child = grid
        };
        PropertyLink.Copy(item, PaddingProperty, border, Border.PaddingProperty);
        PropertyLink.Copy(item, BackgroundProperty, border, Border.BackgroundProperty);
        PropertyLink.Copy(item, BorderBrushProperty, border, Border.BorderBrushProperty);
        PropertyLink.Copy(item, BorderThicknessProperty, border, Border.BorderThicknessProperty);
        PropertyLink.Copy(item, CornerRadiusProperty, border, Border.CornerRadiusProperty);

        return border;
    }

    /// <inheritdoc />
    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        // The ripple starts at the point where the item was pressed. It is
        // drawn by the item itself, so that it never changes the layout of the
        // list it belongs to.
        var position = e.GetPosition(this);
        _rippleX = position.X;
        _rippleY = position.Y;
        _rippleRadius = Math.Max(Bounds.Width, Bounds.Height) * 1.2;

        RippleOpacity = 0.25;
        RippleScale = 0.1;

        // The animations can be disabled in the settings of the launcher.
        if (HMCL.NET.Util.AppearanceManager.AnimationsDisabled)
        {
            RippleOpacity = 0;
            return;
        }

        var duration = TimeSpan.FromMilliseconds(450);

        var grow = new Animation
        {
            Duration = duration,
            Easing = new CubicEaseOut(),
            Children =
            {
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleScaleProperty, Value = 1d } },
                    KeyTime = duration
                }
            }
        };

        var fade = new Animation
        {
            Duration = duration,
            Easing = new CubicEaseOut(),
            Children =
            {
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleOpacityProperty, Value = 0d } },
                    KeyTime = duration
                }
            }
        };

        _ = grow.RunAsync(this);
        _ = fade.RunAsync(this);
    }

    /// <inheritdoc />
    public override void Render(DrawingContext context)
    {
        base.Render(context);

        // The ripple is drawn over the content of the item.
        if (RippleOpacity <= 0 || _rippleRadius <= 0)
            return;

        var radius = _rippleRadius * RippleScale;
        if (radius <= 0)
            return;

        context.DrawEllipse(
            _ripple.Fill, null, new Point(_rippleX, _rippleY), radius, radius);
    }
}
