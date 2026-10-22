using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Platform.Storage;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;

namespace HMCL.NET.Views;

/// <summary>
/// Data pack list page for the selected instance: lists the packs of the
/// datapacks directory, enables and disables them and asks the running game to
/// reload them.
/// Data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class DataPackListPage : UserControl
{
    /// <summary>
    /// Creates a new DataPackListPage.
    /// </summary>
    public DataPackListPage()
    {
        InitializeComponent();

        RefreshButton.Click += OnRefreshClick;
        AddButton.Click += OnAddClick;
        EnableButton.Click += OnEnableClick;
        DisableButton.Click += OnDisableClick;
        RemoveButton.Click += OnRemoveClick;
        ReloadButton.Click += OnReloadClick;

        // Data pack archives dropped onto the page are installed.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);

        if (DataContext is MainWindowViewModel vm)
        {
            vm.ScanDataPacks();
        }
    }

    /// <summary>
    /// Installs the data pack archives dropped onto the page.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var packs = files.Where(f => f.EndsWith(".zip", StringComparison.OrdinalIgnoreCase)).ToList();
        if (packs.Count == 0)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.datapacklistpage.axaml.868_");
            return;
        }

        foreach (var pack in packs)
            viewModel.InstallDataPack(pack);
    }

    /// <summary>
    /// Rescans the data packs directory.
    /// </summary>
    private void OnRefreshClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.ScanDataPacks();

    /// <summary>
    /// Opens a file dialog and installs the chosen data pack.
    /// </summary>
    private async void OnAddClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.datapacklistpage.axaml.datapack_add"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.datapacklistpage.axaml.datapack")) { Patterns = new[] { "*.zip" } },
                new FilePickerFileType(Lang.Get("hmcl.datapacklistpage.axaml.874_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0)
            return;

        var file = files[0];
        if (file.Path.Scheme == "file")
            viewModel.InstallDataPack(file.Path.LocalPath);
    }

    /// <summary>
    /// Enables the selected data pack.
    /// </summary>
    private void OnEnableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetDataPackEnabled(true);

    /// <summary>
    /// Disables the selected data pack.
    /// </summary>
    private void OnDisableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetDataPackEnabled(false);

    /// <summary>
    /// Deletes the selected data pack.
    /// </summary>
    private void OnRemoveClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.RemoveSelectedDataPack();

    /// <summary>
    /// Asks the running game to reload its data packs.
    /// </summary>
    private void OnReloadClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.ReloadDataPacks();
}
