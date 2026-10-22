using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Instance folders page — lists common game folders and opens them in Explorer.
/// Data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class InstanceFoldersPage : UserControl
{
    /// <summary>
    /// Creates a new InstanceFoldersPage.
    /// </summary>
    public InstanceFoldersPage()
    {
        InitializeComponent();

        if (DataContext is MainWindowViewModel vm && vm.SelectedInstance is { } instance)
        {
            var folders = new List<FolderEntry>
            {
                new() { Name = Lang.Get("hmcl.instancefolderspage.axaml.988_"), Path = instance.GameDirectory },
                new() { Name = "mods", Path = Path.Combine(instance.GameDirectory, "mods") },
                new() { Name = "resourcepacks", Path = Path.Combine(instance.GameDirectory, "resourcepacks") },
                new() { Name = "datapacks", Path = Path.Combine(instance.GameDirectory, "datapacks") },
                new() { Name = "shaderpacks", Path = Path.Combine(instance.GameDirectory, "shaderpacks") },
                new() { Name = "screenshots", Path = Path.Combine(instance.GameDirectory, "screenshots") },
                new() { Name = "saves", Path = Path.Combine(instance.GameDirectory, "saves") },
                new() { Name = "config", Path = Path.Combine(instance.GameDirectory, "config") },
                new() { Name = "logs", Path = Path.Combine(instance.GameDirectory, "logs") },
            };
            FolderList.ItemsSource = folders;
        }
    }

    private void OnOpenFolderClick(object? sender, RoutedEventArgs e)
    {
        if (sender is Button button && button.CommandParameter is string path)
        {
            try
            {
                if (!Directory.Exists(path))
                    Directory.CreateDirectory(path);
                Process.Start(new ProcessStartInfo
                {
                    FileName = path,
                    UseShellExecute = true,
                });
            }
            catch
            {
                // Ignore failures opening folders
            }
        }
    }
}
