using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Modpack;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The dialog letting the user choose the optional files of a modpack which
/// are installed, like the optional files page of Java HMCL.
/// </summary>
public sealed class OptionalFilesWindow : Window
{
    private readonly List<CheckBox> _checkBoxes = new();

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="optionalFiles">The optional files of the modpack.</param>
    public OptionalFilesWindow(IReadOnlyList<ModpackOptionalFile> optionalFiles)
    {
        Title = Lang.Get("install.optional_files", "可选文件");
        Width = 520;
        Height = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;
        SizeToContent = SizeToContent.Height;

        var list = new StackPanel { Spacing = 4 };
        foreach (var file in optionalFiles)
        {
            var checkBox = new CheckBox
            {
                Content = file.FileName,
                IsChecked = true,
                Tag = file.Key
            };

            _checkBoxes.Add(checkBox);
            list.Children.Add(checkBox);
        }

        var installButton = new Button
        {
            Content = Lang.Get("button.install", "安装"),
            Padding = new Thickness(16, 6),
            MinWidth = 90
        };
        installButton.Click += (_, _) => Close(true);

        var cancelButton = new Button
        {
            Content = Lang.Get("common.cancel", "取消"),
            Padding = new Thickness(16, 6),
            MinWidth = 90
        };
        cancelButton.Click += (_, _) => Close(false);

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 10,
            Children = { cancelButton, installButton }
        };

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };
        root.Children.Add(new TextBlock
        {
            Text = Lang.Get("install.optional_files.desc", "选择要安装的可选文件。"),
            FontSize = 12,
            Foreground = Brushes.Gray,
            TextWrapping = TextWrapping.Wrap
        });
        root.Children.Add(new ScrollViewer
        {
            Content = list,
            MaxHeight = 280
        });
        root.Children.Add(buttons);

        Content = root;

        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Returns the keys of the optional files the user chose not to install.
    /// </summary>
    /// <returns>The keys of the excluded files.</returns>
    public IReadOnlySet<string> GetExcludedFiles()
        => _checkBoxes
            .Where(checkBox => checkBox.IsChecked != true)
            .Select(checkBox => checkBox.Tag as string ?? string.Empty)
            .Where(key => !string.IsNullOrEmpty(key))
            .ToHashSet(StringComparer.OrdinalIgnoreCase);
}
