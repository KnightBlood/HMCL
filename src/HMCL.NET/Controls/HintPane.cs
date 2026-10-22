using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace HMCL.NET.Controls;

/// <summary>
/// The kind of a message shown by the hint pane.
/// </summary>
public enum HintType
{
    /// <summary>
    /// An ordinary message.
    /// </summary>
    Information,

    /// <summary>
    /// A message telling that an operation succeeded.
    /// </summary>
    Success,

    /// <summary>
    /// A message telling that something needs the attention of the user.
    /// </summary>
    Warning,

    /// <summary>
    /// A message telling that an operation failed.
    /// </summary>
    Error
}

/// <summary>
/// A message shown by the hint pane of the launcher.
/// </summary>
public sealed class HintInfo
{
    /// <summary>
    /// Creates a message.
    /// </summary>
    /// <param name="type">The kind of the message.</param>
    /// <param name="message">The text of the message.</param>
    public HintInfo(HintType type, string message)
    {
        Type = type;
        Message = message;
    }

    /// <summary>
    /// The kind of the message.
    /// </summary>
    public HintType Type { get; }

    /// <summary>
    /// The text of the message.
    /// </summary>
    public string Message { get; }
}

/// <summary>
/// The bar showing the messages of the launcher above its content, like the
/// hint pane of Java HMCL.
/// </summary>
public sealed class HintPane : Border
{
    /// <summary>
    /// The message shown by the pane.
    /// </summary>
    public static readonly StyledProperty<HintInfo?> HintProperty =
        AvaloniaProperty.Register<HintPane, HintInfo?>(nameof(Hint));

    private readonly TextBlock _text = new() { TextWrapping = TextWrapping.Wrap };
    private readonly Button _closeButton = new()
    {
        Content = "✕",
        Padding = new Thickness(8, 0),
        Background = Brushes.Transparent,
        BorderThickness = new Thickness(0),
        VerticalAlignment = VerticalAlignment.Center
    };

    /// <summary>
    /// Creates the pane.
    /// </summary>
    public HintPane()
    {
        var panel = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto") };
        panel.Children.Add(_text);
        Grid.SetColumn(_closeButton, 1);
        panel.Children.Add(_closeButton);

        Padding = new Thickness(12, 8);
        CornerRadius = new CornerRadius(4);
        Margin = new Thickness(12, 8, 12, 0);
        Child = panel;
        IsVisible = false;

        _closeButton.Click += (_, _) => Hint = null;
    }

    /// <summary>
    /// The message shown by the pane.
    /// </summary>
    public HintInfo? Hint
    {
        get => GetValue(HintProperty);
        set => SetValue(HintProperty, value);
    }

    /// <inheritdoc />
    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property != HintProperty)
            return;

        var hint = Hint;
        IsVisible = hint is not null && !string.IsNullOrEmpty(hint.Message);
        if (hint is null || !IsVisible)
            return;

        _text.Text = hint.Message;
        Background = GetBackground(hint.Type);
        _text.Foreground = Brushes.White;
    }

    /// <summary>
    /// Returns the background used for the given kind of message.
    /// </summary>
    /// <param name="type">The kind of the message.</param>
    /// <returns>The background of the pane.</returns>
    private static IBrush GetBackground(HintType type) => type switch
    {
        HintType.Success => new SolidColorBrush(Color.Parse("#2E7D32")),
        HintType.Warning => new SolidColorBrush(Color.Parse("#EF6C00")),
        HintType.Error => new SolidColorBrush(Color.Parse("#C62828")),
        _ => new SolidColorBrush(Color.Parse("#1565C0"))
    };
}
