using System;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using HMCLCore.NET.Game;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Lists, backs up, exports and deletes Minecraft worlds in an instance's <c>saves/</c>
/// directory. Mirrors Java HMCL's world management dialog.
/// </summary>
public sealed partial class WorldListWindow : Window
{
    private readonly MainWindowViewModel _viewModel = null!;

    /// <summary>
    /// Parameterless constructor required by the Avalonia XAML loader.
    /// </summary>
    public WorldListWindow()
    {
        InitializeComponent();

        // The escape key closes the window.
        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Creates the world management window for an instance.
    /// </summary>
    /// <param name="instance">The instance whose worlds are managed.</param>
    /// <param name="viewModel">The launcher view model owning the world list.</param>
    public WorldListWindow(GameInstance instance, MainWindowViewModel viewModel)
        : this()
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

        _viewModel.RefreshSelectedInstanceWorlds();
        UpdateStatus();

        RefreshButton.Click += (_, _) =>
        {
            _viewModel.RefreshSelectedInstanceWorlds();
            UpdateStatus();
        };

        BackupButton.Click += (_, _) =>
        {
            _viewModel.BackupSelectedWorld();
            _viewModel.RefreshSelectedInstanceWorlds();
            UpdateStatus();
        };

        ExportButton.Click += async (_, _) => await ExportWorldAsync();
        ImportButton.Click += async (_, _) => await ImportWorldAsync();
        DeleteButton.Click += async (_, _) => await DeleteWorldAsync();

        BackupWorldMenuItem.Click += (_, _) =>
        {
            _viewModel.BackupSelectedWorld();
            _viewModel.RefreshSelectedInstanceWorlds();
            UpdateStatus();
        };

        ExportWorldMenuItem.Click += async (_, _) => await ExportWorldAsync();
        DeleteWorldMenuItem.Click += async (_, _) => await DeleteWorldAsync();
        InfoButton.Click += (_, _) => ShowWorldInfo();
        WorldInfoMenuItem.Click += (_, _) => ShowWorldInfo();

        DuplicateButton.Click += async (_, _) => await DuplicateWorldAsync();
        DuplicateWorldMenuItem.Click += async (_, _) => await DuplicateWorldAsync();
        BackupListButton.Click += (_, _) => ShowBackups();

        WorldList.SelectionChanged += (_, _) =>
        {
            if (WorldList.SelectedItem is WorldInfo w)
                _viewModel.SelectedWorld = w;
            UpdateStatus();
        };
    }

    /// <summary>
    /// Shows the information about the selected world.
    /// </summary>
    private void ShowWorldInfo()
    {
        var world = _viewModel.SelectedWorld;
        if (world is null)
            return;

        var dialog = new WorldInfoDialog(world.Path, world.DisplayName);
        if (TopLevel.GetTopLevel(this) is Window owner)
            dialog.ShowDialog(owner);
    }

    /// <summary>
    /// Shows the backups of the worlds of the instance.
    /// </summary>
    private void ShowBackups()
    {
        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var window = new WorldBackupWindow(_viewModel);
        window.ShowDialog(owner);
    }

    /// <summary>
    /// Copies the selected world into a new one named by the user.
    /// </summary>
    private async Task DuplicateWorldAsync()
    {
        if (_viewModel.SelectedWorld is not { } world)
            return;

        var name = await DialogService.InputAsync(
            this,
            Lang.Get("world.duplicate.prompt", "请输入复制后的世界名称"),
            Lang.Get("world.duplicate", "复制世界"),
            $"{world.DisplayName} 副本");

        if (string.IsNullOrWhiteSpace(name))
            return;

        _viewModel.DuplicateSelectedWorld(name);
        UpdateStatus();
    }

    /// <summary>
    /// Exports the selected world into a zip archive chosen by the user.
    /// </summary>
    private async Task ExportWorldAsync()
    {
        if (_viewModel.SelectedWorld == null)
            return;

        var file = await StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = Lang.Get("hmcl.worldlistwindow.axaml.1294_"),
            SuggestedFileName = $"{_viewModel.SelectedWorld.DirectoryName}.zip",
            FileTypeChoices = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1296_")) { Patterns = new[] { "*.zip" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        _viewModel.ExportSelectedWorld(file.Path.LocalPath);
        UpdateStatus();
    }

    /// <summary>
    /// Imports a world archive chosen by the user.
    /// </summary>
    private async Task ImportWorldAsync()
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.worldlistwindow.axaml.1298_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1300_")) { Patterns = new[] { "*.zip" } },
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1302_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0)
            return;

        var file = files[0];
        if (file.Path.Scheme == "file")
            _viewModel.ImportWorld(file.Path.LocalPath);

        UpdateStatus();
    }

    /// <summary>
    /// Deletes the selected world after the user confirmed the action.
    /// </summary>
    private async Task DeleteWorldAsync()
    {
        if (_viewModel.SelectedWorld is not { } world)
            return;

        var confirmed = await DialogService.ConfirmAsync(
            this,
            $"确定要删除世界“{world.DisplayName}”吗？此操作不可撤销。",
            Lang.Get("hmcl.worldlistwindow.axaml.1304_"));

        if (!confirmed)
            return;

        _viewModel.DeleteSelectedWorld();
        _viewModel.RefreshSelectedInstanceWorlds();
        UpdateStatus();
    }

    /// <summary>
    /// Updates the status line below the world list.
    /// </summary>
    private void UpdateStatus()
    {
        StatusText.Text = $"共 {_viewModel.Worlds.Count} 个世界";
    }
}
