using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The result of the game directory editor dialog.
/// </summary>
public sealed class GameDirectoryDialogResult
{
    /// <summary>
    /// Creates a dialog result.
    /// </summary>
    /// <param name="name">The custom display name, or null when unnamed.</param>
    /// <param name="path">The root path of the game directory.</param>
    /// <param name="useRelativePath">
    /// Whether the path is relative to the directory of the launcher.
    /// </param>
    public GameDirectoryDialogResult(string? name, string path, bool useRelativePath = false)
    {
        Name = name;
        Path = path;
        UseRelativePath = useRelativePath;
    }

    /// <summary>
    /// The custom display name, or null when unnamed.
    /// </summary>
    public string? Name { get; }

    /// <summary>
    /// The root path of the game directory.
    /// </summary>
    public string Path { get; }

    /// <summary>
    /// Whether the path is relative to the directory of the launcher.
    /// </summary>
    public bool UseRelativePath { get; }
}

/// <summary>
/// A small modal dialog collecting the name and path of a game directory.
/// Built programmatically to avoid XAML compilation dependencies.
/// </summary>
public sealed class GameDirectoryDialog : Window
{
    private readonly TextBox _nameBox;
    private readonly TextBox _pathBox;
    private readonly CheckBox _relativePathBox;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="name">The current custom display name, or null.</param>
    /// <param name="path">The current root path.</param>
    /// <param name="useRelativePath">
    /// Whether the path is relative to the directory of the launcher.
    /// </param>
    public GameDirectoryDialog(string? name, string path, bool useRelativePath = false)
    {
        Title = Lang.Get("hmcl.gamedirectorydialog.910_");
        Width = 460;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _nameBox = new TextBox
        {
            Text = name ?? string.Empty,
            PlaceholderText = Lang.Get("hmcl.gamedirectorydialog.912_")
        };

        _pathBox = new TextBox
        {
            Text = path ?? string.Empty,
            PlaceholderText = Lang.Get("hmcl.gamedirectorydialog.914_")
        };

        _relativePathBox = new CheckBox
        {
            Content = Lang.Get("game_directory.use_relative_path", "使用相对于启动器目录的路径"),
            IsChecked = useRelativePath
        };

        var okButton = CreateDialogButton(Lang.Get("hmcl.gamedirectorydialog.button_ok"), "#2B579A");
        okButton.Click += (_, _) => Close(BuildResult());

        var cancelButton = CreateDialogButton(Lang.Get("hmcl.gamedirectorydialog.button_cancel"), "#757575");
        cancelButton.Click += (_, _) => Close(null);

        var buttonPanel = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        buttonPanel.Children.Add(okButton);
        buttonPanel.Children.Add(cancelButton);

        var root = new StackPanel
        {
            Margin = new Thickness(16),
            Spacing = 12
        };
        root.Children.Add(new TextBlock { Text = Lang.Get("hmcl.gamedirectorydialog.mods_name"), FontWeight = FontWeight.Bold });
        root.Children.Add(_nameBox);
        root.Children.Add(new TextBlock { Text = Lang.Get("hmcl.gamedirectorydialog.922_"), FontWeight = FontWeight.Bold });
        root.Children.Add(_pathBox);
        root.Children.Add(_relativePathBox);
        root.Children.Add(buttonPanel);

        Content = root;
        this.EnableEscapeToClose();
        _pathBox.Focus();
    }

    /// <summary>
    /// Builds the result from the current input, or null when the path is empty.
    /// </summary>
    private GameDirectoryDialogResult? BuildResult()
    {
        var path = _pathBox.Text?.Trim() ?? string.Empty;
        if (path.Length == 0)
            return null;

        var name = _nameBox.Text?.Trim();
        return new GameDirectoryDialogResult(
            string.IsNullOrEmpty(name) ? null : name, path, _relativePathBox.IsChecked == true);
    }

    /// <summary>
    /// Builds a consistently styled dialog button.
    /// </summary>
    private static Button CreateDialogButton(string content, string backgroundHex)
    {
        return new Button
        {
            Content = content,
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            Background = new SolidColorBrush(Color.Parse(backgroundHex)),
            Foreground = Brushes.White
        };
    }
}
