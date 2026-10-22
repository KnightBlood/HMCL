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
/// Shader pack list page for the selected instance: lists the packs of the
/// shaderpacks directory, enables and disables them and imports new ones.
/// Data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class ShaderListPage : UserControl
{
    /// <summary>
    /// Creates a new ShaderListPage.
    /// </summary>
    public ShaderListPage()
    {
        InitializeComponent();

        RefreshButton.Click += OnRefreshClick;
        AddButton.Click += OnAddClick;
        EnableButton.Click += OnEnableClick;
        DisableButton.Click += OnDisableClick;
        RemoveButton.Click += OnRemoveClick;

        // Shader pack archives dropped onto the page are imported.
        FileDropHelper.EnableFileDrop(this, OnFilesDropped);

        if (DataContext is MainWindowViewModel vm)
            vm.ScanShaderPacks();
    }

    /// <summary>
    /// Imports the shader pack archives dropped onto the page.
    /// </summary>
    /// <param name="files">The paths of the dropped files.</param>
    private void OnFilesDropped(IReadOnlyList<string> files)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var packs = files.Where(f => f.EndsWith(".zip", StringComparison.OrdinalIgnoreCase)).ToList();
        if (packs.Count == 0)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.shaderlistpage.axaml.1116_");
            return;
        }

        foreach (var pack in packs)
            viewModel.ImportShaderPack(pack);
    }

    /// <summary>
    /// Rescans the shader packs directory.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRefreshClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.ScanShaderPacks();

    /// <summary>
    /// Opens a file dialog and imports the chosen shader pack.
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
            Title = Lang.Get("hmcl.shaderlistpage.axaml.1118_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("hmcl.shaderlistpage.axaml.download_shader")) { Patterns = new[] { "*.zip" } },
                new FilePickerFileType(Lang.Get("hmcl.shaderlistpage.axaml.1122_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0)
            return;

        var file = files[0];
        if (file.Path.Scheme == "file")
            viewModel.ImportShaderPack(file.Path.LocalPath);
    }

    /// <summary>
    /// Enables the selected shader pack.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnEnableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetShaderPackEnabled(true);

    /// <summary>
    /// Disables the selected shader pack.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnDisableClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.SetShaderPackEnabled(false);

    /// <summary>
    /// Deletes the selected shader pack.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRemoveClick(object? sender, RoutedEventArgs e) =>
        (DataContext as MainWindowViewModel)?.RemoveSelectedShaderPack();
}
