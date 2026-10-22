using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;
using HMCL.NET.Views.Wizard;

namespace HMCL.NET.Views;

/// <summary>
/// The instance management page listing installed game instances
/// and allowing creation and deletion. Bindings resolve against the
/// owning <see cref="MainWindowViewModel"/> data context.
/// </summary>
public partial class InstancesPage : UserControl
{
    /// <summary>
    /// Creates a new InstancesPage.
    /// </summary>
    public InstancesPage()
    {
        InitializeComponent();
        CreateInstanceButton.Click += OnCreateInstanceClick;
        QuickCreateInstanceButton.Click += OnQuickCreateInstanceClick;
        DeleteInstanceButton.Click += OnDeleteInstanceClick;
        ManageModsButton.Click += OnManageModsClick;
        RenameInstanceButton.Click += OnRenameInstanceClick;
        BackupInstanceButton.Click += OnBackupInstanceClick;
        ExportInstanceButton.Click += OnExportInstanceClick;
        RestoreInstanceButton.Click += OnRestoreInstanceClick;
        InstallModpackButton.Click += OnInstallModpackClick;
        InstallLoaderButton.Click += OnInstallLoaderClick;

        // Context menu entries reuse the same handlers as the action buttons.
        LaunchInstanceMenuItem.Click += OnLaunchInstanceClick;
        ModsInstanceMenuItem.Click += OnManageModsClick;
        BackupInstanceMenuItem.Click += OnBackupInstanceClick;
        ExportInstanceMenuItem.Click += OnExportInstanceClick;
        GameSettingsInstanceButton.Click += OnGameSettingsInstanceClick;
        GameSettingsInstanceMenuItem.Click += OnGameSettingsInstanceClick;
        WorldsInstanceButton.Click += OnWorldsInstanceClick;
        WorldsInstanceMenuItem.Click += OnWorldsInstanceClick;
        RenameInstanceMenuItem.Click += OnRenameInstanceClick;
        DeleteInstanceMenuItem.Click += OnDeleteInstanceClick;
        VerifyInstanceButton.Click += OnVerifyInstanceClick;
        VerifyInstanceMenuItem.Click += OnVerifyInstanceClick;
        AddDirectoryButton.Click += OnAddDirectoryClick;
        EditDirectoryButton.Click += OnEditDirectoryClick;
        RemoveDirectoryButton.Click += OnRemoveDirectoryClick;
        UpdateModpackMenuItem.Click += OnUpdateModpackClick;
        SetIconInstanceButton.Click += OnSetIconClick;
        SetIconInstanceMenuItem.Click += OnSetIconClick;
        RemoveIconInstanceMenuItem.Click += OnRemoveIconClick;

        // Mod files dropped onto the page are installed into the selected instance.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);
    }

    /// <summary>
    /// Installs the mod files dropped onto the page.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var mods = files.Where(f => f.EndsWith(".jar", StringComparison.OrdinalIgnoreCase)).ToList();
        if (mods.Count == 0)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.instancespage.axaml.990_");
            return;
        }

        viewModel.InstallModsIntoSelectedInstance(mods);
    }

    /// <summary>
    /// Opens a file picker and stores the chosen image as the instance icon.
    /// </summary>
    private async void OnSetIconClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.instancespage.axaml.992_"),
                AllowMultiple = false,
                FileTypeFilter = new[]
                {
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.994_"))
                    {
                        Patterns = new[] { "*.png", "*.jpg", "*.jpeg", "*.bmp" }
                    },
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.996_")) { Patterns = new[] { "*" } }
                }
            });

        if (files.Count == 0)
            return;

        if (files[0].Path.Scheme == "file")
            viewModel.SetSelectedInstanceIcon(files[0].Path.LocalPath);
    }

    /// <summary>
    /// Removes the custom icon of the selected instance.
    /// </summary>
    private void OnRemoveIconClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
            viewModel.RemoveSelectedInstanceIcon();
    }

    /// <summary>
    /// Opens a file dialog and updates the selected instance to the chosen
    /// modpack version.
    /// </summary>
    private async void OnUpdateModpackClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.instancespage.axaml.998_"),
                AllowMultiple = false,
                FileTypeFilter = new[]
                {
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.modpack")) { Patterns = new[] { "*.zip", "*.mrpack" } },
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.1002_")) { Patterns = new[] { "*" } }
                }
            });

        if (files.Count == 0)
            return;

        var storageFile = files[0];
        if (storageFile.Path.Scheme == "file")
            await viewModel.UpdateModpackAsync(storageFile.Path.LocalPath);
    }

    /// <summary>
    /// Opens the game directory editor and adds the entered directory.
    /// </summary>
    private async void OnAddDirectoryClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var dialog = new GameDirectoryDialog(null, string.Empty);
        var result = await dialog.ShowDialog<GameDirectoryDialogResult?>(owner);
        if (result != null)
            viewModel.AddGameDirectory(result.Path, result.Name, result.UseRelativePath);
    }

    /// <summary>
    /// Opens the game directory editor and applies the changes to the selected directory.
    /// </summary>
    private async void OnEditDirectoryClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedGameDirectory: { } directory } viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var dialog = new GameDirectoryDialog(directory.Name, directory.Path, directory.UseRelativePath);
        var result = await dialog.ShowDialog<GameDirectoryDialogResult?>(owner);
        if (result != null)
            viewModel.UpdateGameDirectory(directory, result.Name, result.Path, result.UseRelativePath);
    }

    /// <summary>
    /// Removes the selected game directory.
    /// </summary>
    private async void OnRemoveDirectoryClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedGameDirectory: { } directory } viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var confirmed = await DialogService.ConfirmAsync(
            owner,
            $"确定要删除游戏目录“{directory.Name}”吗？",
            Lang.Get("hmcl.instancespage.axaml.1004_"));

        if (!confirmed)
            return;

        viewModel.RemoveGameDirectory(directory);
    }

    /// <summary>
    /// Verifies the selected instance's files and repairs missing/corrupted entries.
    /// </summary>
    private async void OnVerifyInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
            await viewModel.VerifyAndRepairSelectedInstanceAsync();
    }

    /// <summary>
    /// Launches the currently selected instance with the selected account.
    /// </summary>
    private async void OnLaunchInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is MainWindowViewModel viewModel)
            await viewModel.LaunchAsync();
    }

    /// <summary>
    /// Opens the game settings editor for the selected instance.
    /// </summary>
    private async void OnGameSettingsInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var instance = viewModel.SelectedInstance;
        if (instance is null)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.instancespage.axaml.1006_");
            return;
        }

        if (TopLevel.GetTopLevel(this) is Window owner)
        {
            var dialog = new GameSettingsDialog(instance, viewModel);
            await dialog.ShowDialog(owner);
        }
    }

    /// <summary>
    /// Launches the instance bound to the clicked row's play button.
    /// </summary>
    private async void OnRowLaunchClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (sender is Button { DataContext: HMCLCore.NET.Game.GameInstance instance })
        {
            viewModel.SelectedInstance = instance;
            await viewModel.LaunchAsync();
        }
    }

    /// <summary>
    /// Opens the loader installation dialog for the selected instance.
    /// </summary>
    private async void OnInstallLoaderClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedInstance: { } instance } viewModel)
            return;

        var dialog = new InstallLoaderDialog(instance, viewModel);
        if (TopLevel.GetTopLevel(this) is Window owner)
        {
            var installed = await dialog.ShowDialog<bool>(owner);
            if (installed)
                viewModel.StatusMessage = $"已为实例 {instance.Id} 安装加载器";
        }
    }

    /// <summary>
    /// Opens a small input dialog and renames the selected instance to the
    /// entered, sanitized name.
    /// </summary>
    private async void OnRenameInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedInstance: { } instance } viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var newName = await DialogService.InputAsync(
            owner,
            Lang.Get("hmcl.instancespage.axaml.1008_"),
            Lang.Get("hmcl.instancespage.axaml.1010_"),
            instance.Id);

        if (!string.IsNullOrWhiteSpace(newName))
            viewModel.RenameSelectedInstance(newName);
    }

    /// <summary>
    /// Opens the world management window for the selected instance.
    /// </summary>
    private async void OnWorldsInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var instance = viewModel.SelectedInstance;
        if (instance is null)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.instancespage.axaml.1012_");
            return;
        }

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var worldWindow = new WorldListWindow(instance, viewModel);
        await worldWindow.ShowDialog(owner);
    }

    /// <summary>
    /// Opens a file dialog and installs the chosen modpack archive as a new instance.
    /// </summary>
    private async void OnInstallModpackClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.instancespage.axaml.1014_"),
                AllowMultiple = false,
                FileTypeFilter = new[]
                {
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.modpack_")) { Patterns = new[] { "*.zip", "*.mrpack" } },
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.1018_")) { Patterns = new[] { "*" } }
                }
            });

        if (files.Count == 0)
            return;

        var storageFile = files[0];
        if (storageFile.Path.Scheme == "file")
        {
            var excluded = await AskOptionalFilesAsync(viewModel, storageFile.Path.LocalPath);
            await viewModel.InstallModpackAsync(storageFile.Path.LocalPath, excluded);
        }
    }

    /// <summary>
    /// Asks the user which optional files of the modpack are installed.
    /// </summary>
    /// <param name="viewModel">The view model of the instances.</param>
    /// <param name="archivePath">The path of the modpack archive.</param>
    /// <returns>
    /// The keys of the optional files the user chose not to install; the set is
    /// empty when the modpack has no optional files.
    /// </returns>
    private async Task<IReadOnlySet<string>> AskOptionalFilesAsync(
        MainWindowViewModel viewModel, string archivePath)
    {
        var optionalFiles = await viewModel.GetOptionalFilesAsync(archivePath);
        if (optionalFiles.Count == 0)
            return new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        var window = new OptionalFilesWindow(optionalFiles);
        var confirmed = await window.ShowDialog<bool>(owner);

        return confirmed
            ? window.GetExcludedFiles()
            : new HashSet<string>(StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Backs up the selected instance into the repository's backups directory.
    /// </summary>
    private async void OnBackupInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        await viewModel.BackupSelectedInstanceAsync();
    }

    /// <summary>
    /// Saves the selected instance as a shareable zip via the file save dialog.
    /// </summary>
    private async void OnExportInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var suggestedName = viewModel.SelectedInstance is { } instance
            ? $"{instance.Id}.zip"
            : "instance.zip";

        var file = await topLevel.StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("hmcl.instancespage.axaml.1020_"),
            SuggestedFileName = suggestedName,
            FileTypeChoices = new[] { new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.1022_")) { Patterns = new[] { "*.zip" } } }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        var destinationPath = file.Path.LocalPath;

        if (TopLevel.GetTopLevel(this) is not Window owner)
        {
            await viewModel.ExportSelectedInstanceAsync(destinationPath);
            return;
        }

        // The export writes a whole instance, so it runs in a dialog showing
        // its progress.
        await DialogService.RunTaskAsync(
            owner,
            Lang.Get("hmcl.instancespage.axaml.1024_"),
            Lang.Get("hmcl.instancespage.axaml.1026_"),
            async progress =>
            {
                progress.Report(0);
                await viewModel.ExportSelectedInstanceAsync(destinationPath);
                progress.Report(100);
            });
    }

    /// <summary>
    /// Opens a file dialog and restores the chosen backup zip as a new instance.
    /// </summary>
    private async void OnRestoreInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.instancespage.axaml.1028_"),
                AllowMultiple = false,
                FileTypeFilter = new[] { new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.instancespage.axaml.1030_")) { Patterns = new[] { "*.zip" } } }
            });

        if (files.Count == 0)
            return;

        var storageFile = files[0];
        if (storageFile.Path.Scheme == "file")
            await viewModel.RestoreInstanceAsync(storageFile.Path.LocalPath);
    }

    /// <summary>
    /// Opens the mod management dialog for the selected instance.
    /// </summary>
    private async void OnManageModsClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel { SelectedInstance: { } instance })
            return;

        var dialog = new ModsDialog(instance, DataContext is MainWindowViewModel vm ? vm : null);
        if (TopLevel.GetTopLevel(this) is Window owner)
            await dialog.ShowDialog(owner);
    }

    /// <summary>
    /// Opens the instance creation wizard, mirroring Java HMCL's
    /// <c>NewInstanceWizard</c>.
    /// </summary>
    private async void OnCreateInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var context = new InstanceCreationContext(viewModel);
        var wizard = new WizardWindow(Lang.Get("hmcl.instancespage.axaml.1032_"), new IWizardStep[]
        {
            new SelectGameVersionStep(context),
            new SelectLoaderStep(context),
            new ConfigureInstanceStep(context),
            new InstallInstanceStep(context)
        });

        await wizard.ShowDialog<bool>(owner);
    }

    /// <summary>
    /// Creates a game instance directly from the entered instance ID and version.
    /// </summary>
    private void OnQuickCreateInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var instanceId = InstanceIdBox.Text?.Trim();
        var version = VersionBox.Text?.Trim();
        viewModel.CreateInstance(instanceId ?? string.Empty, version ?? string.Empty);
    }

    /// <summary>
    /// Deletes the currently selected game instance.
    /// </summary>
    private async void OnDeleteInstanceClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var selected = viewModel.SelectedInstance;
        if (selected == null)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var confirmed = await DialogService.ConfirmAsync(
            owner,
            $"确定要删除实例“{selected.Id}”吗？此操作不可撤销。",
            Lang.Get("hmcl.instancespage.axaml.1034_"));

        if (!confirmed)
            return;

        viewModel.RemoveInstance(selected.Id);
    }
}
