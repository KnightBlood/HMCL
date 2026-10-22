using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.Util;
using HMCL.NET.Views.Dialogs;
using HMCL.NET.ViewModels;
using HMCLCore.NET.Game;

namespace HMCL.NET.Views;

/// <summary>
/// Modal dialog for managing the mods of one game instance: the installed
/// list (enable/disable/delete) plus a Modrinth search-and-install panel.
/// Bindings resolve against a <see cref="ModsViewModel"/>.
/// </summary>
public partial class ModsDialog : Window
{
    private readonly ModsViewModel _viewModel;

    /// <summary>
    /// Creates a preview ModsDialog for the XAML loader; usage should pass an instance.
    /// </summary>
    public ModsDialog() : this(new GameInstance { Id = "preview" })
    {
    }

    /// <summary>
    /// Creates the mod manager dialog for a game instance.
    /// </summary>
    /// <param name="instance">The instance whose mods are managed.</param>
    /// <param name="mainViewModel">The launcher view model providing settings access; may be null in previews.</param>
    public ModsDialog(GameInstance instance, MainWindowViewModel? mainViewModel = null)
    {
        _viewModel = new ModsViewModel(instance, mainViewModel);
        DataContext = _viewModel;
        InitializeComponent();

        ToggleModButton.Click += OnToggleModClick;
        DeleteModButton.Click += OnDeleteModClick;
        RefreshModsButton.Click += OnRefreshModsClick;
        CheckUpdatesButton.Click += OnCheckUpdatesClick;
        UpdateSelectedButton.Click += OnUpdateSelectedClick;
        UpdateAllButton.Click += OnUpdateAllClick;
        SearchModsButton.Click += OnSearchModsClick;
        InstallModButton.Click += OnInstallModClick;
        ExportUpdateListButton.Click += async (_, _) => await ExportUpdateListAsync();
        ShowChangelogButton.Click += (_, _) => ShowChangelog();

        // Mod files dropped onto the dialog are imported into the instance.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);

        // The escape key closes the dialog.
        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Imports the mod files dropped onto the dialog.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        var mods = files.Where(f => f.EndsWith(".jar", StringComparison.OrdinalIgnoreCase)).ToList();
        if (mods.Count == 0)
        {
            _viewModel.StatusMessage = Lang.Get("hmcl.modsdialog.axaml.1070_");
            return;
        }

        _viewModel.InstallLocalMods(mods);
    }

    /// <summary>
    /// Toggles the enabled state of the selected installed mod.
    /// </summary>
    private void OnToggleModClick(object? sender, RoutedEventArgs e) =>
        _viewModel.ToggleSelectedMod();

    /// <summary>
    /// Deletes the selected installed mod.
    /// </summary>
    private void OnDeleteModClick(object? sender, RoutedEventArgs e) =>
        _viewModel.DeleteSelectedMod();

    /// <summary>
    /// Rescans the instance's mods directory.
    /// </summary>
    private void OnRefreshModsClick(object? sender, RoutedEventArgs e) =>
        _viewModel.ReloadInstalled();

    /// <summary>
    /// Checks the installed mods for available updates.
    /// </summary>
    private async void OnCheckUpdatesClick(object? sender, RoutedEventArgs e) =>
        await _viewModel.CheckForUpdatesAsync();

    /// <summary>
    /// Installs the selected available mod update.
    /// </summary>
    private async void OnUpdateSelectedClick(object? sender, RoutedEventArgs e) =>
        await _viewModel.ApplySelectedUpdateAsync();

    /// <summary>
    /// Installs every available mod update.
    /// </summary>
    private async void OnUpdateAllClick(object? sender, RoutedEventArgs e) =>
        await _viewModel.ApplyAllUpdatesAsync();

    /// <summary>
    /// Writes the list of the available updates into a file chosen by the
    /// user.
    /// </summary>
    private async Task ExportUpdateListAsync()
    {
        var file = await StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("button.export", "导出"),
            SuggestedFileName = "mod-updates.txt",
            FileTypeChoices = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType("Text") { Patterns = new[] { "*.txt" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        _viewModel.ExportUpdateList(file.Path.LocalPath);
    }

    /// <summary>
    /// Shows the changelog of the selected update.
    /// </summary>
    private void ShowChangelog()
    {
        if (_viewModel.SelectedUpdate is not { } update)
            return;

        var window = new MessageDialog(
            Lang.Get("addon.changelog", "更新日志") + " - " + update.TargetVersion,
            ModsViewModel.GetChangelog(update));

        window.ShowDialog(this);
    }

    /// <summary>
    /// Searches the remote repository for the entered query.
    /// </summary>
    private async void OnSearchModsClick(object? sender, RoutedEventArgs e) =>
        await _viewModel.SearchRemoteModsAsync();

    /// <summary>
    /// Installs the selected remote mod into the instance.
    /// </summary>
    private async void OnInstallModClick(object? sender, RoutedEventArgs e) =>
        await _viewModel.InstallRemoteModAsync();
}
