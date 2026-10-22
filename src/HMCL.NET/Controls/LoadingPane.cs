using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// The state of a loading pane.
/// </summary>
public enum LoadingState
{
    /// <summary>
    /// The content is being loaded.
    /// </summary>
    Loading,

    /// <summary>
    /// The content is loaded.
    /// </summary>
    Loaded,

    /// <summary>
    /// The content could not be loaded.
    /// </summary>
    Failed
}

/// <summary>
/// The pane shown while the content of a page is being loaded, and the page
/// shown when the content cannot be loaded. It mirrors the spinner pane and
/// the failure page of Java HMCL.
/// </summary>
public sealed class LoadingPane : UserControl
{
    /// <summary>
    /// The state of the pane.
    /// </summary>
    public static readonly StyledProperty<LoadingState> StateProperty =
        AvaloniaProperty.Register<LoadingPane, LoadingState>(nameof(State));

    /// <summary>
    /// The message shown while loading or after a failure.
    /// </summary>
    public static readonly StyledProperty<string?> MessageProperty =
        AvaloniaProperty.Register<LoadingPane, string?>(nameof(Message));

    private readonly ProgressBar _progress = new()
    {
        IsIndeterminate = true,
        Width = 220,
        HorizontalAlignment = HorizontalAlignment.Center
    };

    private readonly TextBlock _messageText = new()
    {
        TextWrapping = TextWrapping.Wrap,
        HorizontalAlignment = HorizontalAlignment.Center,
        Foreground = new SolidColorBrush(Color.Parse("#666666"))
    };

    private readonly TextBlock _errorText = new()
    {
        TextWrapping = TextWrapping.Wrap,
        HorizontalAlignment = HorizontalAlignment.Center,
        Foreground = new SolidColorBrush(Color.Parse("#C62828"))
    };

    private readonly Button _retryButton = new()
    {
        Content = Lang.Get("hmcl.loadingpane.terracotta_status_fatal_retry"),
        Padding = new Thickness(16, 6),
        HorizontalAlignment = HorizontalAlignment.Center
    };

    private readonly Panel _root = new();

    /// <summary>
    /// Creates the pane.
    /// </summary>
    public LoadingPane()
    {
        // The pane covers the content of the page while it is loaded, so the
        // background of the pane hides the content that is being replaced.
        Background = new SolidColorBrush(Color.Parse("#F5F5F5"));

        var loadingPanel = new StackPanel
        {
            Spacing = 12,
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center
        };

        loadingPanel.Children.Add(_progress);
        loadingPanel.Children.Add(_messageText);

        var failurePanel = new StackPanel
        {
            Spacing = 12,
            VerticalAlignment = VerticalAlignment.Center,
            HorizontalAlignment = HorizontalAlignment.Center
        };

        failurePanel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.loadingpane.836_"),
            FontSize = 16,
            FontWeight = FontWeight.Bold,
            HorizontalAlignment = HorizontalAlignment.Center
        });
        failurePanel.Children.Add(_errorText);
        failurePanel.Children.Add(_retryButton);

        var root = _root;
        root.Children.Add(loadingPanel);
        root.Children.Add(failurePanel);

        _retryButton.Click += (_, _) => RetryRequested?.Invoke(this, EventArgs.Empty);

        Content = root;
        State = LoadingState.Loading;
    }

    /// <summary>
    /// Raised when the user asks for the content to be loaded again.
    /// </summary>
    public event EventHandler? RetryRequested;

    /// <summary>
    /// The state of the pane.
    /// </summary>
    public LoadingState State
    {
        get => GetValue(StateProperty);
        set => SetValue(StateProperty, value);
    }

    /// <summary>
    /// The message shown while loading or after a failure.
    /// </summary>
    public string? Message
    {
        get => GetValue(MessageProperty);
        set => SetValue(MessageProperty, value);
    }

    /// <inheritdoc />
    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property == MessageProperty)
        {
            _messageText.Text = Message ?? string.Empty;
            _errorText.Text = Message ?? string.Empty;
            return;
        }

        if (change.Property != StateProperty)
            return;

        var children = _root.Children;
        var loading = children.Count > 0 ? children[0] : null;
        var failure = children.Count > 1 ? children[1] : null;

        if (loading is not null)
            loading.IsVisible = State == LoadingState.Loading;

        if (failure is not null)
            failure.IsVisible = State == LoadingState.Failed;

        // The pane is only shown while the content is missing.
        IsVisible = State != LoadingState.Loaded;
    }
}
