using System;
using System.IO;
using Avalonia.Controls;
using Avalonia.Input;
using HMCL.NET.ViewModels;

namespace HMCL.NET.Views;

/// <summary>
/// The main application view hosting the sidebar navigation and page content,
/// mirroring the Java HMCL root page layout.
/// The data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class MainView : UserControl
{
    /// <summary>
    /// Creates a new MainView, wires the sidebar entries and opens the launch page.
    /// </summary>
    public MainView()
    {
        InitializeComponent();

        HomeButton.Click += (_, _) => ShowLaunchPage();
        AccountsButton.Click += (_, _) => ShowAccountsPage();
        MissingAccountButton.Click += (_, _) => ShowAccountsPage();
        InstanceItemButton.Click += (_, _) => ShowInstancesPage();
        MissingInstanceButton.Click += (_, _) => ShowInstancesPage();
        InstancesButton.Click += (_, _) => ShowInstancesPage();
        InstanceDetailsButton.Click += (_, _) => NavigateInstanceDetails();
        DownloadButton.Click += (_, _) => ShowDownloadPage();
        TasksButton.Click += (_, _) => ShowTaskListPage();
        TerracottaButton.Click += (_, _) => ShowTerracottaPage();
        SettingsButton.Click += (_, _) => ShowSettingsPage();
        FeedbackButton.Click += (_, _) => ShowFeedbackPage();
        AboutButton.Click += (_, _) => ShowAboutDialog();

        // The bar of the title offers the way back to the launch page and the
        // help of the launcher, like the tool bar of Java HMCL.
        BackButton.Click += (_, _) => ShowLaunchPage();
        HelpButton.Click += (_, _) => OpenHelp();

        // The client area of the window is drawn over the caption of the
        // system, so the window is moved by the bar of the title.
        TitleBar.PointerPressed += OnTitleBarPointerPressed;

        // The launch page is the root content, like Java HMCL's MainPage.
        ShowLaunchPage();
    }

    /// <summary>
    /// Moves the window when the bar of the title is pressed, because the
    /// client area of the window covers the caption of the system.
    /// </summary>
    /// <param name="sender">The bar of the title.</param>
    /// <param name="e">The event of the press.</param>
    private void OnTitleBarPointerPressed(object? sender, PointerPressedEventArgs e)
    {
        // The buttons of the bar of the title handle their own clicks.
        if (e.Source is Button || e.GetCurrentPoint(TitleBar).Properties.IsLeftButtonPressed != true)
            return;

        if (TopLevel.GetTopLevel(this) is Window window)
            window.BeginMoveDrag(e);
    }

    /// <summary>
    /// Opens the help of the launcher in the browser of the system.
    /// </summary>
    private void OpenHelp()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = "https://docs.hmcl.net/",
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            // The browser of the system is not available; the help of the
            // launcher is simply not opened.
        }
    }

    /// <summary>
    /// Shows the page holding the feedback of the launcher.
    /// </summary>
    private void ShowFeedbackPage() =>
        PageHost.Content = new SettingsPage { DataContext = (DataContext as MainWindowViewModel)?.Settings };

    /// <summary>
    /// Shows the launch (root) page.
    /// </summary>
    private void ShowLaunchPage() =>
        PageHost.Content = new LaunchPage { DataContext = DataContext };

    /// <summary>
    /// Shows the account management page.
    /// </summary>
    private void ShowAccountsPage() =>
        PageHost.Content = new AccountsPage { DataContext = DataContext };

    /// <summary>
    /// Shows the instance management page.
    /// </summary>
    private void ShowInstancesPage() =>
        PageHost.Content = new InstancesPage { DataContext = DataContext };

    /// <summary>
    /// Shows the version download page.
    /// </summary>
    private void ShowDownloadPage() =>
        PageHost.Content = new DownloadPage { DataContext = (DataContext as MainWindowViewModel)?.Download };

    /// <summary>
    /// Shows the launcher settings page.
    /// </summary>
    private void ShowSettingsPage() =>
        PageHost.Content = new SettingsPage { DataContext = (DataContext as MainWindowViewModel)?.Settings };

    /// <summary>
    /// Shows the task list page.
    /// </summary>
    private void ShowTaskListPage() =>
        PageHost.Content = new TaskListPage { DataContext = (DataContext as MainWindowViewModel)?.TaskList };

    /// <summary>
    /// Shows the online mode page.
    /// </summary>
    private void ShowTerracottaPage() =>
        PageHost.Content = new TerracottaPage { DataContext = DataContext };

    /// <summary>
    /// Shows the dialog holding the information about the launcher.
    /// </summary>
    private void ShowAboutDialog()
    {
        var dialog = new AboutDialog();
        if (TopLevel.GetTopLevel(this) is Window owner)
            dialog.ShowDialog(owner);
    }

    /// <summary>
    /// Shows the instance detail page.
    /// </summary>
    public void NavigateInstanceDetails() =>
        PageHost.Content = new GameInstancePage { DataContext = DataContext };

    /// <summary>
    /// Shows the resource pack list page.
    /// </summary>
    public void NavigateResourcePack() =>
        PageHost.Content = new ResourcePackListPage { DataContext = DataContext };

    /// <summary>
    /// Shows the data pack list page.
    /// </summary>
    public void NavigateDataPack() =>
        PageHost.Content = new DataPackListPage { DataContext = DataContext };

    /// <summary>
    /// Shows or hides the custom background image.
    /// </summary>
    /// <param name="imagePath">
    /// The path of the image to display, or null to hide the background.
    /// </param>
    /// <param name="opacity">The opacity of the background image.</param>
    public void SetBackgroundImage(string? imagePath, double opacity = 1.0)
    {
        if (string.IsNullOrWhiteSpace(imagePath) || !File.Exists(imagePath))
        {
            SetBackgroundImage((Avalonia.Media.IImage?)null, opacity);
            return;
        }

        try
        {
            SetBackgroundImage(new Avalonia.Media.Imaging.Bitmap(imagePath), opacity);
        }
        catch (Exception)
        {
            // An unreadable image simply leaves the background disabled.
            SetBackgroundImage((Avalonia.Media.IImage?)null, opacity);
        }
    }

    /// <summary>
    /// Shows or hides the background image.
    /// </summary>
    /// <param name="image">
    /// The image to display, or null to hide the background.
    /// </param>
    /// <param name="opacity">The opacity of the background image.</param>
    public void SetBackgroundImage(Avalonia.Media.IImage? image, double opacity = 1.0)
    {
        if (image is null)
        {
            BackgroundImage.Source = null;
            BackgroundImage.IsVisible = false;
            return;
        }

        BackgroundImage.Source = image;
        BackgroundImage.Opacity = Math.Clamp(opacity, 0, 1);
        BackgroundImage.IsVisible = true;
    }

    /// <inheritdoc />
    protected override void OnDataContextChanged(EventArgs e)
    {
        base.OnDataContextChanged(e);

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.PropertyChanged += OnViewModelPropertyChanged;
            ShowToast(viewModel.ToastMessage);
        }
    }

    /// <summary>
    /// Shows the transient messages of the launcher in the toast.
    /// </summary>
    /// <param name="sender">The view model.</param>
    /// <param name="e">The property change event.</param>
    private void OnViewModelPropertyChanged(object? sender, System.ComponentModel.PropertyChangedEventArgs e)
    {
        if (e.PropertyName == nameof(MainWindowViewModel.ToastMessage) &&
            sender is MainWindowViewModel viewModel)
        {
            ShowToast(viewModel.ToastMessage);
            return;
        }

        if (e.PropertyName == nameof(MainWindowViewModel.Settings))
            UpdateTitleBarBackground();
    }

    /// <summary>
    /// Updates the background of the bar of the title, so that it can be
    /// transparent, like the title bar of Java HMCL.
    /// </summary>
    private void UpdateTitleBarBackground()
    {
        var transparent = (DataContext as MainWindowViewModel)?.Settings?.TitleBarTransparent ?? false;

        TitleBar.Background = transparent
            ? new Avalonia.Media.SolidColorBrush(Avalonia.Media.Color.Parse("#802B579A"))
            : new Avalonia.Media.SolidColorBrush(Avalonia.Media.Color.Parse("#2B579A"));
    }

    /// <summary>
    /// Shows a transient message at the corner of the window.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    public void ShowToast(string? message)
    {
        if (string.IsNullOrEmpty(message))
        {
            ToastBorder.IsVisible = false;
            return;
        }

        ToastText.Text = message;
        ToastBorder.IsVisible = true;
    }
}
