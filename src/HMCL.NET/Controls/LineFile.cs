using System;
using System.Collections.Generic;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Platform.Storage;
using HMCL.NET.Util;

namespace HMCL.NET.Controls;

/// <summary>
/// A row of a settings page holding a text field and a button opening a file
/// chooser, like the line file chooser controls of Java HMCL.
/// </summary>
public class LineFile : Line
{
    /// <summary>
    /// The path of the chosen file.
    /// </summary>
    public static readonly StyledProperty<string?> FilePathProperty =
        AvaloniaProperty.Register<LineFile, string?>(nameof(FilePath));

    /// <summary>
    /// The hint shown in the empty text field.
    /// </summary>
    public static readonly StyledProperty<string?> PlaceholderTextProperty =
        AvaloniaProperty.Register<LineFile, string?>(nameof(PlaceholderText));

    /// <summary>
    /// The label of the button opening the file chooser.
    /// </summary>
    public static readonly StyledProperty<string> BrowseButtonTextProperty =
        AvaloniaProperty.Register<LineFile, string>(nameof(BrowseButtonText), Lang.Get("hmcl.linefile.832_"));

    /// <summary>
    /// The label of the button clearing the chosen file; the button is hidden
    /// when the label is empty.
    /// </summary>
    public static readonly StyledProperty<string> ClearButtonTextProperty =
        AvaloniaProperty.Register<LineFile, string>(nameof(ClearButtonText));

    /// <summary>
    /// The kinds of the files offered by the file chooser; an empty list
    /// offers every file.
    /// </summary>
    public static readonly StyledProperty<IReadOnlyList<FilePickerFileType>?> FileTypesProperty =
        AvaloniaProperty.Register<LineFile, IReadOnlyList<FilePickerFileType>?>(nameof(FileTypes));

    /// <summary>
    /// Creates the row.
    /// </summary>
    public LineFile()
    {
        var panel = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8
        };

        var textBox = new TextBox
        {
            MinWidth = 200,
            HorizontalAlignment = HorizontalAlignment.Left
        };
        PropertyLink.Link(this, FilePathProperty, textBox, TextBox.TextProperty);
        PropertyLink.Copy(this, PlaceholderTextProperty, textBox, TextBox.PlaceholderTextProperty);

        var button = new Button();
        PropertyLink.Copy(this, BrowseButtonTextProperty, button, Button.ContentProperty);
        button.Click += OnBrowseClick;

        var clearButton = new Button();
        PropertyLink.Copy(this, ClearButtonTextProperty, clearButton, Button.ContentProperty);
        // The button clearing the file is hidden when it has no label.
        PropertyLink.Watch(this, ClearButtonTextProperty, text => clearButton.IsVisible = !string.IsNullOrEmpty(text));
        clearButton.Click += OnClearClick;

        panel.Children.Add(textBox);
        panel.Children.Add(button);
        panel.Children.Add(clearButton);

        Content = panel;
    }

    /// <summary>
    /// The path of the chosen file.
    /// </summary>
    public string? FilePath
    {
        get => GetValue(FilePathProperty);
        set => SetValue(FilePathProperty, value);
    }

    /// <summary>
    /// The hint shown in the empty text field.
    /// </summary>
    public string? PlaceholderText
    {
        get => GetValue(PlaceholderTextProperty);
        set => SetValue(PlaceholderTextProperty, value);
    }

    /// <summary>
    /// The label of the button opening the file chooser.
    /// </summary>
    public string BrowseButtonText
    {
        get => GetValue(BrowseButtonTextProperty);
        set => SetValue(BrowseButtonTextProperty, value);
    }

    /// <summary>
    /// The label of the button clearing the chosen file; the button is hidden
    /// when the label is empty.
    /// </summary>
    public string ClearButtonText
    {
        get => GetValue(ClearButtonTextProperty);
        set => SetValue(ClearButtonTextProperty, value);
    }

    /// <summary>
    /// The kinds of the files offered by the file chooser; an empty list
    /// offers every file.
    /// </summary>
    public IReadOnlyList<FilePickerFileType>? FileTypes
    {
        get => GetValue(FileTypesProperty);
        set => SetValue(FileTypesProperty, value);
    }

    /// <summary>
    /// Clears the chosen file.
    /// </summary>
    /// <param name="sender">The button clearing the file.</param>
    /// <param name="e">The click event.</param>
    private void OnClearClick(object? sender, RoutedEventArgs e)
    {
        FilePath = string.Empty;
    }

    /// <summary>
    /// Opens the file chooser of the system and stores the chosen file.
    /// </summary>
    /// <param name="sender">The button opening the chooser.</param>
    /// <param name="e">The click event.</param>
    private async void OnBrowseClick(object? sender, RoutedEventArgs e)
    {
        var topLevel = TopLevel.GetTopLevel(this);
        if (topLevel is null)
            return;

        try
        {
            var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
            {
                Title = BrowseButtonText,
                FileTypeFilter = FileTypes,
                AllowMultiple = false
            });

            if (files.Count > 0)
                FilePath = files[0].Path.LocalPath;
        }
        catch (Exception)
        {
            // The file chooser is not available; the path can still be typed in
            // the text field.
        }
    }
}
