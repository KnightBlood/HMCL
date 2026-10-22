using System;
using System.Globalization;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Data.Converters;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Value converter inverting a boolean binding value.
/// </summary>
public sealed class InverseBooleanConverter : IValueConverter
{
    /// <summary>
    /// Shared converter instance used in XAML.
    /// </summary>
    public static readonly InverseBooleanConverter Instance = new();

    /// <summary>
    /// Converts true to false and false to true.
    /// </summary>
    public object Convert(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is bool b && !b;

    /// <summary>
    /// Converts back with the same inversion.
    /// </summary>
    public object ConvertBack(object? value, Type targetType, object? parameter, CultureInfo culture)
        => value is bool b && !b;
}

/// <summary>
/// The settings page for editing launch options.
/// Bindings resolve against the owning <see cref="SettingsViewModel"/> data context.
/// </summary>
public partial class SettingsPage : UserControl
{
    /// <summary>
    /// Creates a new SettingsPage.
    /// </summary>
    public SettingsPage()
    {
        InitializeComponent();
        SaveButton.Click += OnSaveClick;
        DetectJavaButton.Click += OnDetectJavaClick;
        RefreshJavaButton.Click += OnRefreshJavaClick;
        UseJavaButton.Click += OnUseJavaClick;
        AddJavaButton.Click += OnAddJavaClick;
        RemoveJavaButton.Click += OnRemoveJavaClick;
        RevealJavaButton.Click += OnRevealJavaClick;
        UninstallJavaButton.Click += OnUninstallJavaClick;
        RestoreJavaButton.Click += OnRestoreJavaClick;
        DownloadJavaButton.Click += OnDownloadJavaClick;
        ExportThemeButton.Click += OnExportThemeClick;
        ImportThemeButton.Click += OnImportThemeClick;
        ForceWriteButton.Click += OnForceWriteClick;
        CreatePresetButton.Click += OnCreatePresetClick;
        RenamePresetButton.Click += OnRenamePresetClick;
        DefaultPresetButton.Click += OnDefaultPresetClick;
        DeletePresetButton.Click += OnDeletePresetClick;
        ApplyThemeButton.Click += OnApplyThemeClick;
        ManageThemeButton.Click += OnManageThemeClick;
        OpenLauncherLogButton.Click += OnOpenLauncherLogClick;
        ExportLauncherLogButton.Click += async (_, _) => await ExportLauncherLogAsync();
        OpenDocumentationButton.Click += (_, _) => OpenLink(AboutDialog.DocumentationUrl);
        OpenFeedbackButton.Click += (_, _) => OpenLink(AboutDialog.FeedbackUrl);
        OpenProjectButton.Click += (_, _) => OpenLink(AboutDialog.ProjectUrl);

        // The background image is chosen among the images of the system.
        BackgroundImageLine.FileTypes = new[]
        {
            new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1094_"))
            {
                Patterns = new[] { "*.png", "*.jpg", "*.jpeg", "*.bmp", "*.webp" }
            },
            new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1096_")) { Patterns = new[] { "*" } }
        };
    }

    /// <summary>
    /// Exports the appearance settings of the launcher into a theme pack.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnExportThemeClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var file = await topLevel.StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("hmcl.settingspage.axaml.1098_"),
            SuggestedFileName = "theme.zip",
            FileTypeChoices = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1100_")) { Patterns = new[] { "*.zip" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        viewModel.ExportThemePack(file.Path.LocalPath);
    }

    /// <summary>
    /// Backs up the settings file and overwrites it with the settings
    /// currently loaded by the launcher.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnForceWriteClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        _ = ConfirmAndForceWriteAsync(viewModel, owner);
    }

    /// <summary>
    /// Asks the user to confirm and then overwrites the settings file.
    /// </summary>
    /// <param name="viewModel">The view model of the settings.</param>
    /// <param name="owner">The window owning the dialog.</param>
    /// <returns>A task that completes when the file is overwritten.</returns>
    private async Task ConfirmAndForceWriteAsync(SettingsViewModel viewModel, Window owner)
    {
        var confirmed = await DialogService.ConfirmAsync(
            owner,
            Lang.Get("settings.file.force_write.confirm", "启动器将备份当前配置文件并用本版本加载的数据覆盖它，本版本不支持的字段可能会丢失。继续吗？"),
            Lang.Get("settings.file.force_write", "备份并覆盖"));

        if (!confirmed)
            return;

        viewModel.ForceWriteSettings();
    }

    /// <summary>
    /// Imports a theme pack and applies the appearance settings it holds.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnImportThemeClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new Avalonia.Platform.Storage.FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.settingspage.axaml.1102_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1104_")) { Patterns = new[] { "*.zip" } },
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1106_")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        viewModel.ImportThemePack(files[0].Path.LocalPath);
    }

    /// <summary>
    /// Opens the given address in the default browser.
    /// </summary>
    /// <param name="url">The address opened in the browser.</param>
    private static void OpenLink(string url)
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            // The browser cannot be opened.
        }
    }

    /// <summary>
    /// Rescans the known Java runtimes and refreshes the management list.
    /// </summary>
    private void OnRefreshJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.RescanJavaRuntimes();
    }

    /// <summary>
    /// Uses the selected runtime as the preferred Java executable.
    /// </summary>
    private void OnUseJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.UseSelectedJavaRuntime();
    }

    /// <summary>
    /// Opens a file picker and registers the chosen Java executable.
    /// </summary>
    private async void OnAddJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.settingspage.axaml.1108_"),
                AllowMultiple = false,
                FileTypeFilter = new[]
                {
                    new Avalonia.Platform.Storage.FilePickerFileType("Java")
                    {
                        Patterns = new[] { "java.exe", "javaw.exe", "java" }
                    },
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.settingspage.axaml.1110_")) { Patterns = new[] { "*" } }
                }
            });

        if (files.Count == 0)
            return;

        if (files[0].Path.Scheme == "file")
            viewModel.AddJavaRuntime(files[0].Path.LocalPath);
    }

    /// <summary>
    /// Removes the selected runtime from the management list; the runtime is
    /// disabled, so that its files are kept but it is no longer used.
    /// </summary>
    private void OnRemoveJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.DisableSelectedJavaRuntime();
    }

    /// <summary>
    /// Opens the folder of the selected runtime in the file manager of the
    /// system.
    /// </summary>
    private void OnRevealJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.RevealSelectedJavaRuntime();
    }

    /// <summary>
    /// Uninstalls the selected runtime after the user confirmed the action;
    /// the files of the runtimes downloaded by the launcher are deleted.
    /// </summary>
    private async void OnUninstallJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel)
            return;

        var runtime = viewModel.SelectedJavaRuntime;
        if (runtime is null)
        {
            viewModel.StatusMessage = Lang.Get("hmcl.settingspage.axaml.1112_");
            return;
        }

        if (TopLevel.GetTopLevel(this) is Window owner)
        {
            var confirmed = await DialogService.ConfirmAsync(
                owner,
                viewModel.IsManagedJavaRuntime(runtime)
                    ? $"确定要卸载 Java 运行时 {runtime.Version} 吗？该运行时的文件将被删除。"
                    : $"确定要从列表中移除 Java 运行时 {runtime.Version} 吗？",
                Lang.Get("hmcl.settingspage.axaml.1114_"));

            if (!confirmed)
                return;
        }

        viewModel.UninstallSelectedJavaRuntime();
    }

    /// <summary>
    /// Restores the selected disabled runtime, so that it is found again by
    /// the next scan of the Java runtimes.
    /// </summary>
    private void OnRestoreJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel { SelectedDisabledJavaRuntime: { } path } viewModel)
            viewModel.RestoreDisabledJavaRuntime(path);
    }

    /// <summary>
    /// Opens the Mojang Java runtime download dialog.
    /// </summary>
    private async void OnDownloadJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var dialog = new JavaDownloadDialog(viewModel);
        await dialog.ShowDialog(owner);
    }

    /// <summary>
    /// Rescans for installed Java runtimes and updates the Java path field.
    /// </summary>
    private void OnDetectJavaClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.DetectJava();
    }

    /// <summary>
    /// Opens the log file of the launcher in the file manager of the system.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnOpenLauncherLogClick(object? sender, RoutedEventArgs e)
    {
        var path = HMCL.NET.Util.LauncherLog.Path;
        var viewModel = DataContext as SettingsViewModel;
        if (string.IsNullOrEmpty(path) || !System.IO.File.Exists(path))
        {
            if (viewModel is not null)
                viewModel.StatusMessage = Lang.Get("settings.launcher.log.missing", "启动器日志尚未生成");
            return;
        }

        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = "explorer.exe",
                Arguments = $"/select,\"{path}\"",
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            // The file manager cannot be opened.
        }
    }

    /// <summary>
    /// Exports the log file of the launcher into a file chosen by the user.
    /// </summary>
    private async Task ExportLauncherLogAsync()
    {
        if (TopLevel.GetTopLevel(this) is not Window owner ||
            DataContext is not SettingsViewModel viewModel)
            return;

        var file = await owner.StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("settings.launcher.log.export", "导出启动器日志"),
            SuggestedFileName = "hmcl.log",
            FileTypeChoices = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType("Log") { Patterns = new[] { "*.log", "*.txt" } }
            }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        var source = HMCL.NET.Util.LauncherLog.Path;
        if (string.IsNullOrEmpty(source) || !System.IO.File.Exists(source))
        {
            viewModel.StatusMessage = Lang.Get("settings.launcher.log.missing", "启动器日志尚未生成");
            return;
        }

        try
        {
            System.IO.File.Copy(source, file.Path.LocalPath, true);
            viewModel.StatusMessage = Lang.Get("settings.launcher.log.exported", "启动器日志已导出");
        }
        catch (Exception ex) when (ex is System.IO.IOException or UnauthorizedAccessException)
        {
            viewModel.StatusMessage = $"导出启动器日志失败: {ex.Message}";
        }
    }

    /// <summary>
    /// Opens the window managing the theme packs of the launcher.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnManageThemeClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var window = new ThemePackManagerWindow(viewModel);
        window.ShowDialog(owner);
    }

    /// <summary>
    /// Applies the theme selected in the list of the themes.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnApplyThemeClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel &&
            viewModel.SelectedThemeItem is { } item)
        {
            viewModel.ApplyTheme(item.Theme);
        }
    }

    /// <summary>
    /// Creates a new preset of the game settings.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnCreatePresetClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.CreateGameSettingsPreset();
    }

    /// <summary>
    /// Asks the user for the new name of the selected preset and renames it.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnRenamePresetClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var preset = viewModel.SelectedGameSettingsPreset;
        if (preset is null)
            return;

        var name = await DialogService.InputAsync(
            owner,
            Lang.Get("GameSettingsPreset.Rename", "重命名预设"),
            Lang.Get("GameSettingsPreset.Title", "预设"),
            preset.Name);

        if (string.IsNullOrWhiteSpace(name))
            return;

        viewModel.RenameGameSettingsPreset(name);
    }

    /// <summary>
    /// Makes the selected preset the default one.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnDefaultPresetClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.SetDefaultGameSettingsPreset();
    }

    /// <summary>
    /// Asks the user to confirm and then deletes the selected preset.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnDeletePresetClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not SettingsViewModel viewModel ||
            TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var preset = viewModel.SelectedGameSettingsPreset;
        if (preset is null)
            return;

        var confirmed = await DialogService.ConfirmAsync(
            owner,
            $"确定要删除预设 {preset.Name} 吗？",
            Lang.Get("GameSettingsPreset.Delete", "删除预设"));

        if (!confirmed)
            return;

        viewModel.DeleteGameSettingsPreset();
    }

    /// <summary>
    /// Persists the currently edited settings back to the settings manager.
    /// </summary>
    private void OnSaveClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is SettingsViewModel viewModel)
            viewModel.SaveSettings();
    }
}
