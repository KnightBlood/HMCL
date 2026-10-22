using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Styling;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Input;
using Avalonia.Markup.Xaml;

namespace HMCL.NET;

public partial class App : Application
{
    /// <summary>
    /// Whether the launcher is quitting; the window is not hidden in the
    /// notification area while it quits.
    /// </summary>
    public static bool IsQuitting { get; private set; }

    /// <summary>
    /// Asks the launcher to quit, so that its window is not hidden in the
    /// notification area when it is closed.
    /// </summary>
    public static void RequestQuit() => IsQuitting = true;

    /// <summary>
    /// Whether the launcher hides itself in the notification area when its
    /// window is closed.
    /// </summary>
    public static bool MinimizeToTray { get; set; } = true;

    public override void Initialize()
    {
        AvaloniaXamlLoader.Load(this);

        // The colors of the panels of the launcher follow the theme, so that
        // the dark theme is not broken by the colors written in the pages of
        // the launcher.
        Resources.MergedDictionaries.Add(CreateThemeDictionaries());

        // The crashes of the launcher are reported to the user, like the crash
        // window of Java HMCL.
        AppDomain.CurrentDomain.UnhandledException += OnUnhandledException;
        TaskScheduler.UnobservedTaskException += OnUnobservedTaskException;
    }

    /// <summary>
    /// Shows the window reporting the crash of the launcher.
    /// </summary>
    /// <param name="sender">The application.</param>
    /// <param name="e">The event of the crash.</param>
    private static void OnUnhandledException(object sender, UnhandledExceptionEventArgs e)
    {
        if (e.ExceptionObject is not Exception exception)
            return;

        ShowCrashWindow(exception);
    }

    /// <summary>
    /// Shows the window reporting the crash of a task nobody watched.
    /// </summary>
    /// <param name="sender">The task scheduler.</param>
    /// <param name="e">The event of the crash.</param>
    private static void OnUnobservedTaskException(object? sender, UnobservedTaskExceptionEventArgs e)
    {
        e.SetObserved();

        ShowCrashWindow(e.Exception);
    }

    /// <summary>
    /// Shows the window reporting a crash of the launcher.
    /// </summary>
    /// <param name="exception">The exception that crashed the launcher.</param>
    private static void ShowCrashWindow(Exception exception)
    {
        try
        {
            var report = new HMCLCore.NET.Util.CrashReport(exception);

            var lifetime = Application.Current?.ApplicationLifetime;
            var owner = (lifetime as IClassicDesktopStyleApplicationLifetime)?.MainWindow;
            var viewModel = owner?.DataContext as HMCL.NET.ViewModels.MainWindowViewModel;

            var window = new HMCL.NET.Views.CrashWindow(report, viewModel is null
                ? null
                : crash => _ = viewModel.ReportCrashAsync(crash));

            if (owner is not null)
            {
                window.Show(owner);
                return;
            }

            window.Show();
        }
        catch (Exception)
        {
            // The crash window cannot be shown; the crash is logged instead.
            HMCLCore.NET.Util.Logging.Logger.Error("The launcher crashed", exception);
        }
    }

    /// <summary>
    /// Creates the colors of the panels of the launcher for the light and the
    /// dark themes.
    /// </summary>
    /// <returns>The dictionary holding the colors of both themes.</returns>
    private static ResourceDictionary CreateThemeDictionaries()
    {
        var light = new ResourceDictionary
        {
            ["HMCL.PanelBackground"] = new SolidColorBrush(Color.Parse("#FAFAFA")),
            ["HMCL.CardBackground"] = new SolidColorBrush(Color.Parse("#FFFFFF")),
            // The pages of the launcher are drawn over the wallpaper with a
            // translucent surface, so that the controls stay readable.
            ["HMCL.PageBackground"] = new SolidColorBrush(Color.Parse("#F2FAFAFA")),
            ["HMCL.SidebarBackground"] = new SolidColorBrush(Color.Parse("#B3F5F5F5")),
            ["HMCL.BorderBrush"] = new SolidColorBrush(Color.Parse("#DDDDDD")),
            ["HMCL.MutedText"] = new SolidColorBrush(Color.Parse("#666666")),
            // The bar of the title of the launcher uses the primary color of
            // the theme, like the tool bar of Java HMCL.
            ["HMCL.TitleBarBackground"] = new SolidColorBrush(Color.Parse("#2B579A")),
            ["HMCL.TitleBarForeground"] = Brushes.White,
            ["HMCL.AccentBackground"] = new SolidColorBrush(Color.Parse("#2B579A")),
            ["HMCL.AccentForeground"] = Brushes.White
        };

        var dark = new ResourceDictionary
        {
            ["HMCL.PanelBackground"] = new SolidColorBrush(Color.Parse("#2B2B2B")),
            ["HMCL.CardBackground"] = new SolidColorBrush(Color.Parse("#333333")),
            ["HMCL.PageBackground"] = new SolidColorBrush(Color.Parse("#F22B2B2B")),
            ["HMCL.SidebarBackground"] = new SolidColorBrush(Color.Parse("#B3252525")),
            ["HMCL.BorderBrush"] = new SolidColorBrush(Color.Parse("#3F3F3F")),
            ["HMCL.MutedText"] = new SolidColorBrush(Color.Parse("#AAAAAA")),
            ["HMCL.TitleBarBackground"] = new SolidColorBrush(Color.Parse("#1E3F6E")),
            ["HMCL.TitleBarForeground"] = Brushes.White,
            ["HMCL.AccentBackground"] = new SolidColorBrush(Color.Parse("#1E3F6E")),
            ["HMCL.AccentForeground"] = Brushes.White
        };

        var dictionaries = new ResourceDictionary();
        dictionaries.ThemeDictionaries[ThemeVariant.Light] = light;
        dictionaries.ThemeDictionaries[ThemeVariant.Dark] = dark;

        return dictionaries;
    }

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            desktop.MainWindow = new MainWindow();

            // The service of the online mode is stopped with the launcher, and
            // the session of the anonymous statistics ends as well.
            desktop.Exit += (_, _) =>
            {
                HMCLCore.NET.Terracotta.TerracottaBundle.Stop();

                var viewModel = (desktop.MainWindow?.DataContext as HMCL.NET.ViewModels.MainWindowViewModel);
                if (viewModel is not null)
                    _ = viewModel.EndAnalyticsSessionAsync();
            };
        }

        base.OnFrameworkInitializationCompleted();
    }

    /// <summary>
    /// Shows the main window of the launcher when its icon is clicked.
    /// </summary>
    /// <param name="sender">The icon of the notification area.</param>
    /// <param name="e">The click event.</param>
    private void OnTrayIconClicked(object? sender, EventArgs e) => ShowMainWindow();

    /// <summary>
    /// Shows the main window of the launcher.
    /// </summary>
    private void OnShowMainWindow(object? sender, EventArgs e) => ShowMainWindow();

    /// <summary>
    /// Shows the main window of the launcher.
    /// </summary>
    private void ShowMainWindow()
    {
        if (ApplicationLifetime is not IClassicDesktopStyleApplicationLifetime desktop)
            return;

        var window = desktop.MainWindow;
        if (window is null)
            return;

        if (!window.IsVisible)
            window.Show();

        window.WindowState = WindowState.Normal;
        window.Activate();
    }

    /// <summary>
    /// Closes the launcher.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnQuit(object? sender, EventArgs e)
    {
        IsQuitting = true;

        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
            desktop.Shutdown();
    }
}
