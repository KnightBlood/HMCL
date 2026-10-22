using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// The base class of the modal dialogs of the launcher.
/// </summary>
/// <remarks>
/// The dialogs are built programmatically, so they do not depend on the
/// compiled XAML and can be created from the view models as well. Derived
/// classes add their content with <see cref="AddContent"/> and
/// <see cref="AddMessage"/>, create their buttons with
/// <see cref="CreateButton"/> and finally call <see cref="AttachButtons"/>.
/// </remarks>
public abstract class DialogWindow : Window
{
    /// <summary>
    /// The panel holding the content of the dialog.
    /// </summary>
    private readonly StackPanel _contentPanel;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="title">The title of the dialog window.</param>
    /// <param name="width">The width of the dialog window.</param>
    protected DialogWindow(string title, double width = 420)
    {
        Title = title;
        Width = width;
        SizeToContent = SizeToContent.Height;
        MinHeight = 120;
        CanResize = false;
        ShowInTaskbar = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _contentPanel = new StackPanel
        {
            Margin = new Thickness(16),
            Spacing = 12
        };

        ButtonPanel = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right
        };

        Content = _contentPanel;

        // The escape key closes the dialog, like the dialogs of Java HMCL.
        KeyDown += OnKeyDown;
    }

    /// <summary>
    /// Closes the dialog when the user presses the escape key.
    /// </summary>
    /// <param name="sender">The dialog.</param>
    /// <param name="e">The key event.</param>
    private void OnKeyDown(object? sender, Avalonia.Input.KeyEventArgs e)
    {
        if (e.Key != Avalonia.Input.Key.Escape)
            return;

        e.Handled = true;
        Close(false);
    }

    /// <summary>
    /// The panel holding the buttons of the dialog.
    /// </summary>
    protected StackPanel ButtonPanel { get; }

    /// <summary>
    /// Adds a control to the content of the dialog.
    /// </summary>
    /// <param name="control">The control to add.</param>
    protected void AddContent(Control control) => _contentPanel.Children.Add(control);

    /// <summary>
    /// Adds a wrapped message to the content of the dialog.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    protected void AddMessage(string message)
    {
        AddContent(new TextBlock
        {
            Text = message,
            TextWrapping = TextWrapping.Wrap
        });
    }

    /// <summary>
    /// Adds the button panel to the content of the dialog; it must be called
    /// after all the other content has been added.
    /// </summary>
    protected void AttachButtons() => _contentPanel.Children.Add(ButtonPanel);

    /// <summary>
    /// Creates a button of the dialog.
    /// </summary>
    /// <param name="text">The label of the button.</param>
    /// <param name="isPrimary">Whether the button is the primary action of the dialog.</param>
    /// <param name="action">The action executed when the button is clicked.</param>
    /// <returns>The created button.</returns>
    protected static Button CreateButton(string text, bool isPrimary, Action action)
    {
        var button = new Button
        {
            Content = text,
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            Background = new SolidColorBrush(Color.Parse(isPrimary ? "#B0413E" : "#757575")),
            Foreground = Brushes.White
        };

        button.Click += (_, _) => action();
        return button;
    }
}
