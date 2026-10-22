using System;
using System.Diagnostics;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Game;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;

namespace HMCL.NET.Views;

/// <summary>
/// The window listing the backups of the worlds of an instance, like the world
/// backups page of Java HMCL.
/// </summary>
public sealed class WorldBackupWindow : Window
{
    private readonly MainWindowViewModel _viewModel;

    private readonly ListBox _backupList = new() { HorizontalAlignment = HorizontalAlignment.Stretch };
    private readonly TextBlock _statusText = new()
    {
        Foreground = Brushes.Gray,
        FontSize = 12,
        TextWrapping = TextWrapping.Wrap
    };

    /// <summary>
    /// Creates the window.
    /// </summary>
    /// <param name="viewModel">The launcher view model owning the backups.</param>
    public WorldBackupWindow(MainWindowViewModel viewModel)
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

        Title = Lang.Get("world.backup", "世界备份");
        Width = 560;
        Height = 420;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _viewModel.RefreshWorldBackups();
        _backupList.ItemsSource = _viewModel.WorldBackups;

        var refreshButton = new Button
        {
            Content = Lang.Get("button.refresh", "刷新"),
            Padding = new Thickness(12, 6)
        };
        refreshButton.Click += (_, _) =>
        {
            _viewModel.RefreshWorldBackups();
            UpdateStatus();
        };

        var createButton = new Button
        {
            Content = Lang.Get("world.backup.create.new_one", "新建备份"),
            Padding = new Thickness(12, 6)
        };
        createButton.Click += (_, _) =>
        {
            _viewModel.BackupSelectedWorld();
            _viewModel.RefreshWorldBackups();
            UpdateStatus();
        };

        var revealButton = new Button
        {
            Content = Lang.Get("world.backup.reveal", "在文件夹中显示"),
            Padding = new Thickness(12, 6)
        };
        revealButton.Click += (_, _) => RevealSelected();

        var deleteButton = new Button
        {
            Content = Lang.Get("common.delete", "删除"),
            Padding = new Thickness(12, 6)
        };
        deleteButton.Click += async (_, _) => await DeleteSelectedAsync();

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(12, 6)
        };
        closeButton.Click += (_, _) => Close();

        var buttonRow = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            Children = { refreshButton, createButton, revealButton, deleteButton, closeButton }
        };

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };
        root.Children.Add(_backupList);
        root.Children.Add(_statusText);

        var content = new Grid { RowDefinitions = new RowDefinitions("*,Auto") };
        content.Children.Add(root);

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Margin = new Thickness(16, 0, 16, 16),
            Children = { buttonRow }
        };
        Grid.SetRow(buttons, 1);
        content.Children.Add(buttons);

        Content = content;

        UpdateStatus();

        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Shows the backup in the file manager of the system.
    /// </summary>
    private void RevealSelected()
    {
        if (_backupList.SelectedItem is not WorldBackup backup)
            return;

        try
        {
            var directory = Path.GetDirectoryName(backup.FilePath);
            if (!string.IsNullOrEmpty(directory))
                Process.Start(new ProcessStartInfo("explorer.exe", $"/select,\"{backup.FilePath}\"")
                {
                    UseShellExecute = true
                });
        }
        catch (Exception)
        {
            // The file manager cannot be opened.
        }
    }

    /// <summary>
    /// Asks the user to confirm and then deletes the selected backup.
    /// </summary>
    private async Task DeleteSelectedAsync()
    {
        if (_backupList.SelectedItem is not WorldBackup backup)
            return;

        var confirmed = await DialogService.ConfirmAsync(
            this,
            string.Format(
                Lang.Get("world.backup.delete.confirm", "确定要删除备份 {0} 吗？"),
                backup.FileName),
            Lang.Get("common.delete", "删除"));

        if (!confirmed)
            return;

        if (_viewModel.DeleteWorldBackup(backup))
            _statusText.Text = Lang.Get("world.backup.delete.success", "备份已删除");
        else
            _statusText.Text = Lang.Get("world.backup.delete.failed", "备份删除失败");

        UpdateStatus();
    }

    /// <summary>
    /// Updates the status line below the list of the backups.
    /// </summary>
    private void UpdateStatus()
    {
        _statusText.Text = _viewModel.WorldBackups.Count == 0
            ? Lang.Get("world.backup.empty", "暂无备份")
            : $"共 {_viewModel.WorldBackups.Count} 个备份";
    }
}
