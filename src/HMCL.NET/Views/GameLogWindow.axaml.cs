using System;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Standalone window showing live game output and a stop button,
/// mirroring Java HMCL's <c>LogWindow</c>.
/// Bindings resolve against <see cref="ViewModels.MainWindowViewModel"/>.
/// </summary>
public partial class GameLogWindow : Window
{
    /// <summary>
    /// Creates a new game log window.
    /// </summary>
    public GameLogWindow()
    {
        InitializeComponent();

        ExportLogButton.Click += OnExportLogClick;
        DumpButton.Click += OnDumpClick;
    }

    /// <summary>
    /// Writes the dump of the threads of the running game into a file chosen
    /// by the user.
    /// </summary>
    private async void OnDumpClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (StorageProvider is not { } storageProvider)
            return;

        var file = await storageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("game.dump", "生成游戏转储"),
            SuggestedFileName = $"minecraft-dump-{DateTime.Now:yyyyMMdd-HHmmss}.txt",
            FileTypeChoices = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType("Dump") { Patterns = new[] { "*.txt", "*.dump" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        await viewModel.GenerateGameDumpAsync(file.Path.LocalPath);
    }

    /// <summary>
    /// Saves the game log, the launcher log and the crash reports of the
    /// selected instance into a zip archive.
    /// </summary>
    private async void OnExportLogClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (StorageProvider is not { } storageProvider)
            return;

        var suggestedName = $"hmcl-logs-{DateTime.Now:yyyyMMdd-HHmmss}.zip";

        var file = await storageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("hmcl.gamelogwindow.axaml.924_"),
            SuggestedFileName = suggestedName,
            FileTypeChoices = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.gamelogwindow.axaml.926_")) { Patterns = new[] { "*.zip" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        viewModel.ExportGameLogs(file.Path.LocalPath);
    }
}
