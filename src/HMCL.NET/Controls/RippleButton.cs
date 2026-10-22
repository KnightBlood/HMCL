using System;
using Avalonia;
using Avalonia.Animation;
using Avalonia.Animation.Easings;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Media;
using Avalonia.Rendering;
using Avalonia.Styling;

namespace HMCL.NET.Controls;

/// <summary>
/// A button showing a ripple around the point where it was pressed, like the
/// ripple container of Java HMCL.
/// </summary>
public sealed class RippleButton : Button
{
    /// <summary>
    /// The radius of the ripple.
    /// </summary>
    public static readonly StyledProperty<double> RippleRadiusProperty =
        AvaloniaProperty.Register<RippleButton, double>(nameof(RippleRadius));

    /// <summary>
    /// The opacity of the ripple.
    /// </summary>
    public static readonly StyledProperty<double> RippleOpacityProperty =
        AvaloniaProperty.Register<RippleButton, double>(nameof(RippleOpacity));

    /// <summary>
    /// The color of the ripple.
    /// </summary>
    public static readonly StyledProperty<IBrush?> RippleBrushProperty =
        AvaloniaProperty.Register<RippleButton, IBrush?>(nameof(RippleBrush));

    private double _rippleX;
    private double _rippleY;

    /// <summary>
    /// Creates the button.
    /// </summary>
    public RippleButton()
    {
        RippleBrush = new SolidColorBrush(Color.Parse("#33000000"));
        RippleOpacity = 0;
        RippleRadius = 0;
    }

    /// <summary>
    /// The radius of the ripple.
    /// </summary>
    public double RippleRadius
    {
        get => GetValue(RippleRadiusProperty);
        set => SetValue(RippleRadiusProperty, value);
    }

    /// <summary>
    /// The opacity of the ripple.
    /// </summary>
    public double RippleOpacity
    {
        get => GetValue(RippleOpacityProperty);
        set => SetValue(RippleOpacityProperty, value);
    }

    /// <summary>
    /// The color of the ripple.
    /// </summary>
    public IBrush? RippleBrush
    {
        get => GetValue(RippleBrushProperty);
        set => SetValue(RippleBrushProperty, value);
    }

    /// <inheritdoc />
    protected override void OnPointerPressed(PointerPressedEventArgs e)
    {
        base.OnPointerPressed(e);

        // The ripple starts at the point where the button was pressed.
        var position = e.GetPosition(this);
        _rippleX = position.X;
        _rippleY = position.Y;

        var distance = Math.Sqrt(
            Math.Max(0, _rippleX * _rippleX + _rippleY * _rippleY));
        var maxRadius = Math.Sqrt(
            Math.Max(0, (Bounds.Width - _rippleX) * (Bounds.Width - _rippleX) +
                      (Bounds.Height - _rippleY) * (Bounds.Height - _rippleY)));

        StartAnimation(maxRadius > distance ? maxRadius : distance);
    }

    /// <summary>
    /// Starts the animation of the ripple.
    /// </summary>
    /// <param name="maxRadius">The radius the ripple grows to.</param>
    private void StartAnimation(double maxRadius)
    {
        // The animations can be disabled in the settings of the launcher.
        if (HMCL.NET.Util.AppearanceManager.AnimationsDisabled)
            return;

        var duration = TimeSpan.FromMilliseconds(450);

        var animation = new Animation
        {
            Duration = duration,
            Easing = new CubicEaseOut(),
            FillMode = FillMode.Forward,
            Children =
            {
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleRadiusProperty, Value = 0d } },
                    KeyTime = TimeSpan.Zero
                },
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleRadiusProperty, Value = maxRadius } },
                    KeyTime = duration
                }
            }
        };

        var opacity = new Animation
        {
            Duration = duration,
            Easing = new CubicEaseOut(),
            FillMode = FillMode.Forward,
            Children =
            {
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleOpacityProperty, Value = 0.25d } },
                    KeyTime = TimeSpan.Zero
                },
                new KeyFrame
                {
                    Setters = { new Setter { Property = RippleOpacityProperty, Value = 0d } },
                    KeyTime = duration
                }
            }
        };

        _ = animation.RunAsync(this);
        _ = opacity.RunAsync(this);
    }

    /// <inheritdoc />
    public override void Render(DrawingContext context)
    {
        base.Render(context);

        if (RippleOpacity <= 0 || RippleRadius <= 0 || RippleBrush is null)
            return;

        context.DrawEllipse(RippleBrush, null, new Point(_rippleX, _rippleY), RippleRadius, RippleRadius);
    }
}
