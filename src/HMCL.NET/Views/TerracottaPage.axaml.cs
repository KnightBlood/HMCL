using System;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Platform.Storage;
using HMCLCore.NET.Terracotta;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;

namespace HMCL.NET.Views;

/// <summary>
/// The page of the online mode of the launcher, showing the state of the
/// local service, the nodes of the network and the rooms the player can join.
/// It mirrors the online page of Java HMCL.
/// </summary>
public partial class TerracottaPage : UserControl
{
    /// <summary>
    /// Creates a new TerracottaPage.
    /// </summary>
    public TerracottaPage()
    {
        InitializeComponent();

        // The package of the online component can be dropped into the page, so
        // that it is installed from a third-party download channel.
        DragDrop.SetAllowDrop(this, true);
        DragDrop.AddDragOverHandler(this, OnDragOver);
        DragDrop.AddDropHandler(this, OnDrop);

        InstallLocalButton.Click += async (_, _) => await InstallLocalPackageAsync();
        ExportLogButton.Click += async (_, _) => await ExportLogsAsync();
        FeedbackButton.Click += (_, _) => OpenFeedback();
    }

    /// <summary>
    /// Opens the form used to send the feedback about the online component.
    /// </summary>
    private void OpenFeedback()
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = viewModel.TerracottaFeedbackLink,
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            // The browser cannot be opened.
        }
    }

    /// <summary>
    /// Reads the logs of the online component and writes them into a file
    /// chosen by the user.
    /// </summary>
    private async Task ExportLogsAsync()
    {
        if (DataContext is not MainWindowViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var file = await owner.StorageProvider.SaveFilePickerAsync(new FilePickerSaveOptions
        {
            Title = Lang.Get("terracotta.export_log", "导出联机组件日志"),
            SuggestedFileName = "terracotta.log",
            FileTypeChoices = new[]
            {
                new FilePickerFileType("Log") { Patterns = new[] { "*.log", "*.txt" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        await viewModel.ExportTerracottaLogsAsync(file.Path.LocalPath);
    }

    /// <summary>
    /// Asks the user for the package of the online component and installs it.
    /// </summary>
    private async Task InstallLocalPackageAsync()
    {
        if (DataContext is not MainWindowViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var files = await owner.StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("terracotta.from_local.title", "第三方下载渠道"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType("Terracotta") { Patterns = new[] { "*.tar.gz", "*.tgz" } },
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1302_", "所有文件")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        await viewModel.InstallTerracottaFromLocalAsync(files[0].Path.LocalPath);
    }

    /// <summary>
    /// Accepts the files dropped into the page when they are the package of the
    /// online component.
    /// </summary>
    /// <param name="sender">The page.</param>
    /// <param name="e">The drag event.</param>
    private void OnDragOver(object? sender, DragEventArgs e)
    {
        e.DragEffects = GetPackagePath(e.DataTransfer) is null
            ? DragDropEffects.None
            : DragDropEffects.Copy;
    }

    /// <summary>
    /// Installs the package of the online component dropped into the page.
    /// </summary>
    /// <param name="sender">The page.</param>
    /// <param name="e">The drag event.</param>
    private async void OnDrop(object? sender, DragEventArgs e)
    {
        var path = GetPackagePath(e.DataTransfer);
        if (path is null)
            return;

        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var metadata = TerracottaMetadata.Current;
        var expected = metadata?.PackageName;

        if (!string.IsNullOrEmpty(expected) &&
            !string.Equals(Path.GetFileName(path), expected, StringComparison.OrdinalIgnoreCase))
        {
            await DialogService.ShowMessageAsync(
                owner,
                string.Format(
                    Lang.Get("terracotta.from_local.file_name_mismatch",
                        "你应该下载名为 %1$s 的联机组件安装包，而不是 %2$s"),
                    expected, Path.GetFileName(path)),
                Lang.Get("terracotta.from_local.title", "第三方下载渠道"));

            return;
        }

        await viewModel.InstallTerracottaFromLocalAsync(path);
    }

    /// <summary>
    /// Returns the path of the package dropped into the page, or null when the
    /// data does not hold one.
    /// </summary>
    /// <param name="transfer">The data of the drag.</param>
    /// <returns>The path of the package, or null.</returns>
    private static string? GetPackagePath(IDataTransfer transfer)
    {
        var files = transfer.TryGetFiles();
        if (files is null || files.Length == 0)
            return null;

        var file = files.FirstOrDefault(item => item is IStorageItem { Path: { IsFile: true } });
        return file?.Path.LocalPath;
    }
}
