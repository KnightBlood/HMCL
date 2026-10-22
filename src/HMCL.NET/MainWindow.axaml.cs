using System;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Platform;
using Avalonia.Markup.Xaml;
using Avalonia.Media;
using HMCLCore.NET.Auth;
using HMCLCore.NET.Download;
using HMCLCore.NET.Game;
using HMCLCore.NET.Java;
using HMCLCore.NET.Launch;
using HMCLCore.NET.Setting;
using HMCLCore.NET.Util;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;

namespace HMCL.NET;

public partial class MainWindow : Window
{
    private readonly MainWindowViewModel _viewModel;

    private readonly SettingsManager _settingsManager;

    public MainWindow()
    {
        // Root directory for HMCL data files.
        var rootDirectory = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hmcl");

        // Initialize core library components.
        var accountManager = new AccountManager();
        var settingsManager = new SettingsManager(Path.Combine(rootDirectory, "settings.json"));
        var javaManager = new JavaManager();

        _settingsManager = settingsManager;

        // The configuration files of the old versions of the launcher are
        // migrated into the current settings file, when there is no settings
        // file yet.
        MigrateLegacyConfig(settingsManager, rootDirectory);

        settingsManager.Load();
        javaManager.Scan(settingsManager.Settings.DisabledJava);
        javaManager.EnsurePreferredRuntime(settingsManager.Settings.JavaPath);

        // The theme of the launcher is applied before the controls are created,
        // so that they all use the colors of the theme from the start.
        AppearanceManager.ApplyThemeVariant(settingsManager.Settings.Theme);

        InitializeComponent();

        // Load the localized resources of the language chosen by the user. The
        // resources are applied to the application, so every element bound to
        // them shows the texts of the new language.
        LanguageManager.Apply(settingsManager.Settings.Language);

        // Determine whether the April Fools' mode of the launcher is used; the
        // user can turn it off in the settings.
        AprilFools.Initialize(settingsManager.Settings.DisableAprilFools);

        // Wire the launcher pipeline used to start actual game processes.
        var launcherExecutor = new GameLauncherExecutor(
            new GameLauncher(), javaManager, rootDirectory);

        // The game directories known to the launcher; the default one keeps the
        // data inside the .hmcl folder so existing installations keep working.
        if (settingsManager.Settings.GameDirectories.Count == 0)
        {
            settingsManager.Settings.GameDirectories.Add(new GameDirectory
            {
                Id = "default",
                Name = "默认目录",
                Path = rootDirectory
            });
            settingsManager.Settings.SelectedGameDirectoryId = "default";
            settingsManager.Save();
        }

        var gameDirectoryManager = new GameDirectoryManager(
            settingsManager.Settings.GameDirectories,
            settingsManager.Settings.SelectedGameDirectoryId,
            directory => new DefaultGameRepository(
                new GameRepositoryLayout { BaseDirectory = directory.GetAbsolutePath(rootDirectory) }));

        foreach (var directory in gameDirectoryManager.Directories)
            GameDirectoryManager.EnsureLayout(directory, rootDirectory);

        // The cache of the downloaded files follows the selected game
        // directory, so that the files cached for one directory are not mixed
        // with the files of another one.
        DefaultCacheRepository.Instance.ChangeDirectory(gameDirectoryManager.Selected.GetAbsolutePath(rootDirectory));
        gameDirectoryManager.SelectionChanged += (_, _) =>
            DefaultCacheRepository.Instance.ChangeDirectory(gameDirectoryManager.Selected.GetAbsolutePath(rootDirectory));

        _viewModel = new MainWindowViewModel(
            accountManager, settingsManager, javaManager, launcherExecutor, gameDirectoryManager);

        // Only register a default offline account when persistence restored none.
        if (accountManager.Accounts.Count == 0)
        {
            var offlineAccount = new OfflineAccount("Player");
            accountManager.AddAccount(offlineAccount);
            accountManager.SelectAccount(offlineAccount);
            _viewModel.SyncAccounts();
        }

        _viewModel.LoadInstances();

        // Open a dedicated log window on each launch, like Java HMCL.
        _viewModel.GameLogWindowRequested += () =>
        {
            var logWindow = new HMCL.NET.Views.GameLogWindow { DataContext = _viewModel };
            logWindow.Show();
        };

        // The causes of a crash of the game are shown in the log window.
        _viewModel.GameCrashDetected += () =>
        {
            var logWindow = new HMCL.NET.Views.GameLogWindow { DataContext = _viewModel };
            logWindow.Show();
            logWindow.Activate();
        };

        DataContext = _viewModel;

        // Apply the appearance settings (theme, font, background) on startup and
        // whenever the user changes them in the settings page.
        ApplyAppearance(settingsManager.Settings);
        _viewModel.Settings.AppearanceChanged += () => ApplyAppearance(_settingsManager.Settings);

        // The background image can also be changed by the screenshot page.
        _viewModel.BackgroundImageChanged += () =>
            RootView.SetBackgroundImage(_settingsManager.Settings.BackgroundImagePath);

        // The launcher hides itself or closes when the game starts, and shows
        // itself again when the game ends.
        _viewModel.LauncherVisibilityRequested += ApplyLauncherVisibility;
        _viewModel.LauncherReopenRequested += ShowMainWindow;

        // The position and the size of the window are restored.
        RestoreWindowState();

        // Closing the window hides the launcher in the notification area,
        // unless the user quits it from there.
        Closing += OnClosing;

        // The state of the window is saved when the launcher loses the focus,
        // so that it is kept even when the launcher does not exit normally.
        Deactivated += (_, _) => SaveWindowState();
    }

    /// <summary>
    /// Restores the position and the size of the window saved in the settings.
    /// </summary>
    private void RestoreWindowState()
    {
        var settings = _settingsManager.Settings;

        if (settings.WindowWidth > 0)
            Width = settings.WindowWidth;

        if (settings.WindowHeight > 0)
            Height = settings.WindowHeight;

        if (settings.WindowLeft is { } left && settings.WindowTop is { } top &&
            IsOnScreen(left, top))
        {
            Position = new PixelPoint((int)left, (int)top);
        }

        if (settings.WindowMaximized)
            WindowState = WindowState.Maximized;
    }

    /// <summary>
    /// Returns whether the given position is on one of the screens of the
    /// machine.
    /// </summary>
    /// <param name="left">The position of the left edge of the window.</param>
    /// <param name="top">The position of the top edge of the window.</param>
    /// <returns>Whether the position is visible.</returns>
    private bool IsOnScreen(double left, double top)
    {
        try
        {
            var screens = Screens?.All;
            if (screens is null || screens.Count == 0)
                return true;

            foreach (var screen in screens)
            {
                if (left >= screen.WorkingArea.X - 50 &&
                    left <= screen.WorkingArea.X + screen.WorkingArea.Width - 50 &&
                    top >= screen.WorkingArea.Y - 50 &&
                    top <= screen.WorkingArea.Y + screen.WorkingArea.Height - 50)
                {
                    return true;
                }
            }

            return false;
        }
        catch (Exception)
        {
            // The screens are not known; the position is kept.
            return true;
        }
    }

    /// <summary>
    /// Saves the position and the size of the window into the settings.
    /// </summary>
    private void SaveWindowState()
    {
        try
        {
            var maximized = WindowState == WindowState.Maximized;

            _settingsManager.Update(settings =>
            {
                settings.WindowMaximized = maximized;

                if (!maximized)
                {
                    settings.WindowLeft = Position.X;
                    settings.WindowTop = Position.Y;
                    settings.WindowWidth = (int)Width;
                    settings.WindowHeight = (int)Height;
                }
            });
        }
        catch (Exception)
        {
            // The state of the window is optional.
        }
    }

    /// <summary>
    /// Hides the launcher in the notification area when its window is closed.
    /// </summary>
    /// <param name="sender">The window.</param>
    /// <param name="e">The closing event.</param>
    private void OnClosing(object? sender, WindowClosingEventArgs e)
    {
        // The state of the window is saved before the launcher hides itself.
        SaveWindowState();

        if (App.IsQuitting || !App.MinimizeToTray)
            return;

        e.Cancel = true;
        Hide();
    }

    /// <summary>
    /// Hides the launcher or closes it, as the visibility chosen in the
    /// settings says.
    /// </summary>
    /// <param name="visibility">The visibility chosen in the settings.</param>
    private void ApplyLauncherVisibility(LauncherVisibility visibility)
    {
        switch (visibility)
        {
            case LauncherVisibility.Hide:
            case LauncherVisibility.HideAndReopen:
                Hide();
                break;

            case LauncherVisibility.Close:
            case LauncherVisibility.CloseAndReopen:
                // The launcher is closed for good, so the closing handler must
                // not hide it in the notification area.
                App.RequestQuit();
                Close();
                break;
        }
    }

    /// <summary>
    /// Shows the main window of the launcher.
    /// </summary>
    private void ShowMainWindow()
    {
        if (!IsVisible)
            Show();

        WindowState = WindowState.Normal;
        Activate();
    }

    /// <summary>
    /// Migrates the configuration file of the old versions of the launcher into
    /// the current settings file.
    /// </summary>
    /// <param name="settingsManager">The manager holding the settings.</param>
    /// <param name="rootDirectory">The directory holding the files of the launcher.</param>
    private static void MigrateLegacyConfig(SettingsManager settingsManager, string rootDirectory)
    {
        try
        {
            // The current settings are never overwritten by the legacy ones.
            if (File.Exists(settingsManager.SettingsFile) ||
                LegacyConfigMigrator.IsAlreadyMigrated(rootDirectory))
            {
                return;
            }

            var migration = LegacyConfigMigrator.Migrate(rootDirectory);
            if (migration is null)
                return;

            var legacy = migration.Settings;
            settingsManager.Update(settings =>
            {
                settings.Memory = legacy.Memory;
                settings.JavaPath = legacy.JavaPath;
                settings.UseCustomJvmArgs = legacy.UseCustomJvmArgs;
                settings.CustomJvmArgs = legacy.CustomJvmArgs;
                settings.WindowWidth = legacy.WindowWidth;
                settings.WindowHeight = legacy.WindowHeight;
                settings.Language = legacy.Language;
                settings.FontFamily = legacy.FontFamily;
                settings.FontSize = legacy.FontSize;
                settings.BackgroundImagePath = legacy.BackgroundImagePath;
                settings.VersionListSource = legacy.VersionListSource;
                settings.FileDownloadSource = legacy.FileDownloadSource;
                settings.ProxyConfig = legacy.ProxyConfig;
                settings.LastUsername = legacy.LastUsername;
            });

            LegacyConfigMigrator.RecordMigration(rootDirectory, migration.Path);
            LauncherLog.Info($"已从旧版配置文件迁移设置: {migration.Path}");
        }
        catch (Exception ex)
        {
            LauncherLog.Warn($"迁移旧版配置失败: {ex.Message}");
        }
    }

    /// <summary>
    /// Applies the appearance settings to the window: the theme variant, the
    /// interface font, the custom background image and the transparency effect.
    /// </summary>
    /// <param name="settings">The launcher settings holding the appearance options.</param>
    public void ApplyAppearance(LauncherSettings settings)
    {
        if (settings == null)
            return;

        AppearanceManager.Apply(
            settings.Theme, settings.FontFamily, settings.FontSize, settings.ThemeColor,
            settings.ThemeColorStyle, settings.ThemeContrast, settings.DisableAnimations,
            settings.TextRenderingMode);

        AppearanceManager.ApplyTextRenderingMode(RootView);

        // The background of the launcher follows the kind of background chosen
        // in the settings.
        switch (settings.BackgroundType)
        {
            case HMCLCore.NET.Setting.BackgroundType.Default:
            case HMCLCore.NET.Setting.BackgroundType.Builtin:
                RootView.SetBackgroundImage(GetBuiltinWallpaper(settings), settings.BackgroundOpacity);
                break;

            case HMCLCore.NET.Setting.BackgroundType.Image:
                RootView.SetBackgroundImage(settings.BackgroundImagePath, settings.BackgroundOpacity);
                break;

            case HMCLCore.NET.Setting.BackgroundType.Network:
                RootView.SetBackgroundImage(settings.NetworkBackgroundImageUrl, settings.BackgroundOpacity);
                break;

            default:
                RootView.SetBackgroundImage((string?)null, settings.BackgroundOpacity);
                break;
        }

        if (settings.EnableTransparencyEffects)
        {
            // The window is blurred by the system, so that the content behind
            // it shows through, like the window of Java HMCL.
            Background = Avalonia.Media.Brushes.Transparent;
            TransparencyLevelHint = new[] { WindowTransparencyLevel.AcrylicBlur };
        }
        else
        {
            if (Application.Current != null &&
                Application.Current.TryFindResource("WindowBackground", null, out var resource) &&
                resource is IBrush brush)
            {
                Background = brush;
            }

            TransparencyLevelHint = Array.Empty<WindowTransparencyLevel>();
        }

        // The title bar of the launcher is drawn by the launcher itself, so the
        // client area of the window is extended over the caption of the
        // system; the transparency of the bar follows the settings.
        try
        {
            ExtendClientAreaToDecorationsHint = true;
            ExtendClientAreaTitleBarHeightHint = 40;
        }
        catch (Exception)
        {
            // The transparent title bar is not supported by every platform.
        }
    }

    /// <summary>
    /// Returns the built-in wallpaper chosen in the settings.
    /// </summary>
    /// <param name="settings">The launcher settings holding the appearance options.</param>
    /// <returns>The wallpaper, or null when none is chosen.</returns>
    private static Avalonia.Media.IImage? GetBuiltinWallpaper(LauncherSettings settings)
    {
        // The wallpaper of the theme is used when no wallpaper is chosen.
        var wallpaper = HMCLCore.NET.Setting.ThemeColorScheme.ParseWallpaperId(settings.BuiltinWallpaperId)
            ?? HMCLCore.NET.Setting.BuiltinWallpaper.Wallpaper20160225;

        // The wallpapers are stored as the resources of the launcher.
        var fileName = HMCLCore.NET.Setting.ThemeColorScheme.GetWallpaperFileName(wallpaper);

        try
        {
            using var stream = AssetLoader.Open(new Uri($"avares://HMCL.NET/Assets/wallpapers/{fileName}"));
            return new Avalonia.Media.Imaging.Bitmap(stream);
        }
        catch (Exception)
        {
            // A missing wallpaper simply leaves the background disabled.
            return null;
        }
    }
}
