using System;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The download page: a category list on the left and the selected category's
/// content on the right, mirroring Java HMCL's download pages.
/// Bindings resolve against <see cref="DownloadViewModel"/>.
/// </summary>
public partial class DownloadPage : UserControl
{
    /// <summary>
    /// Creates a new DownloadPage and wires the category entries.
    /// </summary>
    public DownloadPage()
    {
        InitializeComponent();

        GameCategoryButton.Click += (_, _) => ShowPanel(GamePanel);
        ModpackCategoryButton.Click += (_, _) => ShowPanel(ModpackPanel);
        ModCategoryButton.Click += (_, _) => ShowAddonPanel("mod");
        ResourcePackCategoryButton.Click += (_, _) => ShowAddonPanel("resourcepack");
        WorldCategoryButton.Click += (_, _) => ShowAddonPanel("world");
        DataPackCategoryButton.Click += (_, _) => ShowAddonPanel("datapack");
        ShaderCategoryButton.Click += (_, _) => ShowAddonPanel("shader");

        InstallLocalModpackButton.Click += OnInstallLocalModpackClick;

        // The description of the selected modpack is shown in its own window.
        if (DataContext is DownloadViewModel viewModel)
        {
            viewModel.ModpackDescriptionRequested += OnModpackDescriptionRequested;
            DataContextChanged += (_, _) =>
            {
                if (DataContext is DownloadViewModel current)
                    current.ModpackDescriptionRequested += OnModpackDescriptionRequested;
            };
        }

        // The loading pane is shown while the versions are fetched, and offers
        // to try again when they cannot be fetched.
        VersionLoadingPane.RetryRequested += (_, _) =>
        {
            if (DataContext is DownloadViewModel viewModel)
                viewModel.RefreshCommand.Execute(null);
        };

        // Game versions are the default category.
        ShowPanel(GamePanel);
    }

    /// <summary>
    /// Shows the description of the modpack chosen by the user.
    /// </summary>
    /// <param name="modpack">The modpack whose description is shown.</param>
    private void OnModpackDescriptionRequested(HMCLCore.NET.Addon.RemoteAddon modpack)
    {
        if (TopLevel.GetTopLevel(this) is not Window owner)
            return;

        var window = new ModpackDescriptionWindow(
            Lang.Get("modpack.description", "整合包说明"), modpack.Description);

        window.ShowDialog(owner);
    }

    /// <summary>
    /// Shows the given category panel and hides the others.
    /// </summary>
    private void ShowPanel(Control panel)
    {
        GamePanel.IsVisible = ReferenceEquals(panel, GamePanel);
        ModpackPanel.IsVisible = ReferenceEquals(panel, ModpackPanel);
        ModPanel.IsVisible = ReferenceEquals(panel, ModPanel);
        PlaceholderText.IsVisible = false;
    }

    /// <summary>
    /// Shows the addon panel configured for the given category.
    /// </summary>
    private void ShowAddonPanel(string category)
    {
        if (DataContext is DownloadViewModel viewModel)
            viewModel.SetAddonCategory(category);

        ShowPanel(ModPanel);
    }

    /// <summary>
    /// Opens a file picker and installs the chosen archive as a new instance.
    /// </summary>
    private async void OnInstallLocalModpackClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not DownloadViewModel download || TopLevel.GetTopLevel(this) is not { } topLevel)
            return;

        var files = await topLevel.StorageProvider.OpenFilePickerAsync(
            new Avalonia.Platform.Storage.FilePickerOpenOptions
            {
                Title = Lang.Get("hmcl.downloadpage.axaml.904_"),
                AllowMultiple = false,
                FileTypeFilter = new[]
                {
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.downloadpage.axaml.modpack")) { Patterns = new[] { "*.zip", "*.mrpack" } },
                    new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.downloadpage.axaml.908_")) { Patterns = new[] { "*" } }
                }
            });

        if (files.Count == 0)
            return;

        if (files[0].Path.Scheme == "file")
        {
            var excluded = await AskOptionalFilesAsync(download.Owner, files[0].Path.LocalPath);
            await download.Owner.InstallModpackAsync(files[0].Path.LocalPath, excluded);
        }
    }

    /// <summary>
    /// Asks the user which optional files of the modpack are installed.
    /// </summary>
    /// <param name="viewModel">The view model of the launcher.</param>
    /// <param name="archivePath">The path of the modpack archive.</param>
    /// <returns>
    /// The keys of the optional files the user chose not to install; the set is
    /// empty when the modpack has no optional files.
    /// </returns>
    private async Task<IReadOnlySet<string>> AskOptionalFilesAsync(
        MainWindowViewModel viewModel, string archivePath)
    {
        var optionalFiles = await viewModel.GetOptionalFilesAsync(archivePath);
        if (optionalFiles.Count == 0)
            return new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        if (TopLevel.GetTopLevel(this) is not Window owner)
            return new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        var window = new OptionalFilesWindow(optionalFiles);
        var confirmed = await window.ShowDialog<bool>(owner);

        return confirmed
            ? window.GetExcludedFiles()
            : new HashSet<string>(StringComparer.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Opens the mods dialog for the instance selected in the main view model.
    /// </summary>
    private async void OnOpenModsManagerClick(object? sender, RoutedEventArgs e)
    {
        if (DataContext is not DownloadViewModel download ||
            download.Owner.SelectedInstance is not { } instance)
            return;

        var dialog = new ModsDialog(instance, download.Owner);
        if (TopLevel.GetTopLevel(this) is Window owner)
            await dialog.ShowDialog(owner);
    }
}
