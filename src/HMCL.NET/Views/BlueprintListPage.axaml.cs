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
/// The page listing the blueprints of the selected instance, offering the
/// import and the deletion of them. It mirrors Java HMCL's blueprint page.
/// </summary>
public partial class BlueprintListPage : UserControl
{
    /// <summary>
    /// Creates a new BlueprintListPage.
    /// </summary>
    public BlueprintListPage()
    {
        InitializeComponent();

        RefreshButton.Click += OnRefreshClick;
        AddButton.Click += OnAddClick;
        RemoveButton.Click += OnRemoveClick;

        // The blueprint files dropped onto the page are imported.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);

        if (DataContext is MainWindowViewModel viewModel)
            viewModel.ScanBlueprints();
    }

    /// <summary>
    /// Imports the blueprint files dropped onto the page.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        foreach (var file in files)
            viewModel.ImportBlueprint(file);

        UpdateStatus(viewModel);
    }

    /// <summary>
    /// Rescans the blueprints directory.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRefreshClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        viewModel.ScanBlueprints();
        UpdateStatus(viewModel);
    }

    /// <summary>
    /// Opens a file dialog and imports the chosen blueprint.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnAddClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.blueprintlistpage.axaml.862_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.blueprintlistpage.axaml.864_")) { Patterns = new[] { "*.schem", "*.nbt", "*.litematic" } },
                new FilePickerFileType(Lang.Get("hmcl.blueprintlistpage.axaml.866_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0)
            return;

        var file = files[0];
        if (file.Path.Scheme == "file")
        {
            viewModel.ImportBlueprint(file.Path.LocalPath);
            UpdateStatus(viewModel);
        }
    }

    /// <summary>
    /// Deletes the selected blueprint.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRemoveClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (viewModel.RemoveSelectedBlueprint())
            UpdateStatus(viewModel);
    }

    /// <summary>
    /// Shows the number of the blueprints of the list.
    /// </summary>
    /// <param name="viewModel">The view model of the launcher.</param>
    private void UpdateStatus(MainWindowViewModel viewModel)
        => StatusText.Text = $"共 {viewModel.Blueprints.Count} 个蓝图";
}
