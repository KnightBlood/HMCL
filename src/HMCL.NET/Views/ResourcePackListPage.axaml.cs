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
/// Resource pack list page for the selected instance: lists the packs of the
/// resourcepacks directory, enables and disables them and reports compatibility
/// problems.
/// Data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class ResourcePackListPage : UserControl
{
    /// <summary>
    /// Creates a new ResourcePackListPage.
    /// </summary>
    public ResourcePackListPage()
    {
        InitializeComponent();

        RefreshButton.Click += OnRefreshClick;
        AddButton.Click += OnAddClick;
        EnableButton.Click += OnEnableClick;
        DisableButton.Click += OnDisableClick;
        RemoveButton.Click += OnRemoveClick;

        // Resource pack archives dropped onto the page are imported.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);

        if (DataContext is MainWindowViewModel vm)
        {
            vm.ScanResourcePacks();
        }
    }

    /// <summary>
    /// Imports the resource pack archives dropped onto the page.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var packs = files.Where(f => f.EndsWith(".zip", StringComparison.OrdinalIgnoreCase)).ToList();
        if (packs.Count == 0)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.resourcepacklistpage.axaml.1082_");
            return;
        }

        foreach (var pack in packs)
            viewModel.ImportResourcePack(pack);
    }

    /// <summary>
    /// Rescans the resource packs directory.
    /// </summary>
    private void OnRefreshClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.ScanResourcePacks();

    /// <summary>
    /// Opens a file dialog and imports the chosen resource pack.
    /// </summary>
    private async void OnAddClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.resourcepacklistpage.axaml.resourcepack_add"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.resourcepacklistpage.axaml.resourcepack")) { Patterns = new[] { "*.zip" } },
                new FilePickerFileType(Lang.Get("hmcl.resourcepacklistpage.axaml.1088_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0)
            return;

        var file = files[0];
        if (file.Path.Scheme == "file")
            viewModel.ImportResourcePack(file.Path.LocalPath);
    }

    /// <summary>
    /// Enables the selected resource pack.
    /// </summary>
    private void OnEnableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetResourcePackEnabled(true);

    /// <summary>
    /// Disables the selected resource pack.
    /// </summary>
    private void OnDisableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetResourcePackEnabled(false);

    /// <summary>
    /// Deletes the selected resource pack.
    /// </summary>
    private void OnRemoveClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.RemoveSelectedResourcePack();
}
