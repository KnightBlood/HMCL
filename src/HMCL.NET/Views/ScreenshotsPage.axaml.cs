using System;
using System.Diagnostics;
using System.IO;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCLCore.NET.Game;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The page managing the screenshots of the selected instance, offering the
/// deletion, the copy and the use of a screenshot as the background of the
/// launcher. It mirrors Java HMCL's screenshot page.
/// </summary>
public partial class ScreenshotsPage : UserControl
{
    /// <summary>
    /// Creates a new ScreenshotsPage.
    /// </summary>
    public ScreenshotsPage()
    {
        InitializeComponent();

        RefreshButton.Click += OnRefreshClick;
        OpenDirectoryButton.Click += OnOpenDirectoryClick;
        SelectAllButton.Click += OnSelectAllClick;
        DeleteMenuItem.Click += OnDeleteClick;
        CopyMenuItem.Click += OnCopyClick;
        SetBackgroundMenuItem.Click += OnSetBackgroundClick;

        FilterBox.SelectionChanged += OnFilterChanged;
    }

    /// <inheritdoc />
    protected override void OnDataContextChanged(EventArgs e)
    {
        base.OnDataContextChanged(e);

        if (DataContext is MainWindowViewModel viewModel)
        {
            viewModel.ScanScreenshots();
            UpdateStatus(viewModel);
        }
    }

    /// <summary>
    /// Returns the screenshot chosen in the list.
    /// </summary>
    /// <returns>The chosen screenshot, or null when there is none.</returns>
    private ScreenshotInfo? GetSelectedScreenshot()
        => ScreenshotList.SelectedItem as ScreenshotInfo;

    /// <summary>
    /// Refreshes the list of the screenshots.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRefreshClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        viewModel.ScanScreenshots();
        UpdateStatus(viewModel);
    }

    /// <summary>
    /// Opens the directory holding the screenshots.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnOpenDirectoryClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        var directory = viewModel.ScreenshotDirectory;
        if (string.IsNullOrEmpty(directory) || !Directory.Exists(directory))
        {
            StatusText.Text = Lang.Get("hmcl.screenshotspage.axaml.1090_");
            return;
        }

        try
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = directory,
                UseShellExecute = true
            });
        }
        catch (Exception)
        {
            StatusText.Text = Lang.Get("hmcl.screenshotspage.axaml.1092_");
        }
    }

    /// <summary>
    /// Selects every screenshot of the list.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnSelectAllClick(object? sender, RoutedEventArgs e)
    {
        ScreenshotList.SelectAll();
        StatusText.Text = $"已选择 {ScreenshotList.SelectedItems?.Count ?? 0} 张截图";
    }

    /// <summary>
    /// Deletes the chosen screenshot.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnDeleteClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (viewModel.DeleteScreenshot(GetSelectedScreenshot()))
            UpdateStatus(viewModel);
    }

    /// <summary>
    /// Copies the chosen screenshot into the pictures folder of the user.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnCopyClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (viewModel.CopyScreenshot(GetSelectedScreenshot()))
            UpdateStatus(viewModel);
    }

    /// <summary>
    /// Uses the chosen screenshot as the background of the launcher.
    /// </summary>
    /// <param name="sender">The menu item.</param>
    /// <param name="e">The click event.</param>
    private void OnSetBackgroundClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (viewModel.SetScreenshotAsBackground(GetSelectedScreenshot()))
            UpdateStatus(viewModel);
    }

    /// <summary>
    /// Applies the date filter chosen by the user.
    /// </summary>
    /// <param name="sender">The combo box.</param>
    /// <param name="e">The selection event.</param>
    private void OnFilterChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (FilterBox.SelectedItem is not ComboBoxItem item)
            return;

        viewModel.ScreenshotFilter = item.Tag switch
        {
            "Today" => ScreenshotFilter.Today,
            "ThisWeek" => ScreenshotFilter.ThisWeek,
            "ThisMonth" => ScreenshotFilter.ThisMonth,
            _ => ScreenshotFilter.All
        };

        UpdateStatus(viewModel);
    }

    /// <summary>
    /// Shows the number of the screenshots of the list.
    /// </summary>
    /// <param name="viewModel">The view model of the launcher.</param>
    private void UpdateStatus(MainWindowViewModel viewModel)
        => StatusText.Text = $"共 {viewModel.Screenshots.Count} 张截图";
}
