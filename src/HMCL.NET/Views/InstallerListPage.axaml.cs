using System;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The page listing the components installed in the selected instance and
/// offering the offline installation of a mod loader from a local installer
/// file. It mirrors Java HMCL's installer list page.
/// </summary>
public partial class InstallerListPage : UserControl
{
    /// <summary>
    /// Creates a new InstallerListPage.
    /// </summary>
    public InstallerListPage()
    {
        InitializeComponent();

        ComponentList.ItemsSource = (DataContext as MainWindowViewModel)?.InstanceComponents;

        InstallFromFileButton.Click += OnInstallFromFileClick;
        InstallOptiFineButton.Click += OnInstallOptiFineClick;
        RefreshButton.Click += OnRefreshClick;
    }

    /// <inheritdoc />
    protected override void OnDataContextChanged(EventArgs e)
    {
        base.OnDataContextChanged(e);

        if (DataContext is MainWindowViewModel viewModel)
            ComponentList.ItemsSource = viewModel.InstanceComponents;
    }

    /// <summary>
    /// Asks the user for an installer file and installs the loader it holds.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnInstallFromFileClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        if (TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(new Avalonia.Platform.Storage.FilePickerOpenOptions
        {
            Title = Lang.Get("hmcl.installerlistpage.axaml.980_"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.installerlistpage.axaml.982_")) { Patterns = new[] { "*.jar" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        StatusText.Text = Lang.Get("hmcl.installerlistpage.axaml.984_");
        await viewModel.InstallLoaderFromFileAsync(files[0].Path.LocalPath);
        StatusText.Text = viewModel.StatusMessage;
    }

    /// <summary>
    /// Installs OptiFine into the launcher.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private async void OnInstallOptiFineClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        StatusText.Text = Lang.Get("hmcl.installerlistpage.axaml.986_");
        await viewModel.InstallOptiFineAsync();
        StatusText.Text = viewModel.StatusMessage;
    }

    /// <summary>
    /// Refreshes the list of the installed components.
    /// </summary>
    /// <param name="sender">The button.</param>
    /// <param name="e">The click event.</param>
    private void OnRefreshClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not MainWindowViewModel viewModel)
            return;

        viewModel.RefreshInstanceComponents();
        StatusText.Text = $"共 {viewModel.InstanceComponents.Count} 个组件";
    }
}
