using System;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Threading;
using HMCLCore.NET.Download;
using HMCLCore.NET.Game;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Dialog installing a mod loader into an instance. Fabric/Quilt are merged
/// from their meta profiles; Forge/NeoForge run their official installers.
/// </summary>
public sealed class InstallLoaderDialog : Window
{
    private readonly MainWindowViewModel _viewModel;
    private readonly GameInstance _instance;

    private readonly ComboBox _loaderBox = new();
    private readonly TextBox _versionBox = new();
    private readonly TextBlock _statusText = new()
    {
        FontSize = 12,
        Foreground = Brushes.Gray,
        TextWrapping = TextWrapping.Wrap
    };
    private readonly Button _installButton = new() { Content = Lang.Get("hmcl.installloaderdialog.button_install") };

    /// <summary>
    /// Creates the loader installation dialog.
    /// </summary>
    /// <param name="instance">The target instance.</param>
    /// <param name="viewModel">The main view model used to read/write the manifest.</param>
    public InstallLoaderDialog(GameInstance instance, MainWindowViewModel viewModel)
    {
        _instance = instance ?? throw new ArgumentNullException(nameof(instance));
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

        Title = $"安装加载器 - {instance.Id}";
        Width = 440;
        Height = 300;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _loaderBox.ItemsSource = new[] { "Fabric", "Quilt", "LegacyFabric", "Forge", "NeoForge" };
        _loaderBox.SelectedIndex = 0;
        _loaderBox.HorizontalAlignment = HorizontalAlignment.Stretch;

        _versionBox.PlaceholderText = Lang.Get("hmcl.installloaderdialog.960_");
        _versionBox.HorizontalAlignment = HorizontalAlignment.Stretch;

        _installButton.HorizontalAlignment = HorizontalAlignment.Right;
        _installButton.Padding = new Thickness(16, 6);
        _installButton.Click += OnInstallClick;

        Content = new StackPanel
        {
            Margin = new Thickness(16),
            Spacing = 10,
            Children =
            {
                new TextBlock { Text = Lang.Get("hmcl.installloaderdialog.962_"), FontWeight = FontWeight.Bold },
                _loaderBox,
                new TextBlock { Text = Lang.Get("hmcl.installloaderdialog.964_"), FontWeight = FontWeight.Bold },
                _versionBox,
                new TextBlock
                {
                    Text = $"游戏版本: {instance.Version}",
                    FontSize = 12,
                    Foreground = Brushes.Gray
                },
                _statusText,
                _installButton
            }
        };
        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Installs the selected loader and updates the instance manifest.
    /// </summary>
    private async void OnInstallClick(object? sender, RoutedEventArgs e)
    {
        _installButton.IsEnabled = false;
        _statusText.Text = Lang.Get("hmcl.installloaderdialog.966_");

        try
        {
            var kind = _loaderBox.SelectedIndex switch
            {
                1 => LoaderKind.Quilt,
                2 => LoaderKind.LegacyFabric,
                3 => LoaderKind.Forge,
                4 => LoaderKind.NeoForge,
                _ => LoaderKind.Fabric
            };

            if (kind is LoaderKind.Forge or LoaderKind.NeoForge)
            {
                await InstallViaInstallerAsync(kind);
                return;
            }

            // Fabric/Quilt: merge the meta profile into the instance manifest.
            var manifest = _viewModel.LoadInstanceManifest(_instance.Id);
            if (manifest == null)
            {
                _statusText.Text = Lang.Get("hmcl.installloaderdialog.968_");
                return;
            }

            var merged = await MetaLoaderInstaller.CreateManifestAsync(
                kind, manifest, _versionBox.Text?.Trim());

            if (merged == null)
            {
                _statusText.Text = $"该游戏版本没有可用的 {kind} 加载器";
                return;
            }

            if (!_viewModel.SaveInstanceManifest(_instance.Id, merged))
            {
                _statusText.Text = Lang.Get("hmcl.installloaderdialog.970_");
                return;
            }

            Close(true);
        }
        catch (Exception ex)
        {
            _statusText.Text = $"安装失败: {ex.Message}";
        }
        finally
        {
            _installButton.IsEnabled = true;
        }
    }

    /// <summary>
    /// Runs the Forge/NeoForge installer and adopts the produced version JSON.
    /// </summary>
    private async Task InstallViaInstallerAsync(LoaderKind kind)
    {
        var version = _versionBox.Text?.Trim();
        if (kind == LoaderKind.NeoForge && string.IsNullOrWhiteSpace(version))
        {
            _statusText.Text = Lang.Get("hmcl.installloaderdialog.972_");
            return;
        }

        var javaPath = _viewModel.Settings.JavaPath;
        if (string.IsNullOrWhiteSpace(javaPath))
            javaPath = "java";

        var progress = new Progress<string>(line =>
            _statusText.Text = line.Length > 200 ? line[..200] : line);

        var installedVersionId = await ForgeInstaller.InstallAsync(
            kind,
            _viewModel.BaseDirectory,
            javaPath,
            _instance.Version,
            version,
            progress);

        if (string.IsNullOrEmpty(installedVersionId))
        {
            _statusText.Text = Lang.Get("hmcl.installloaderdialog.974_");
            return;
        }

        var versionJson = _viewModel.LoadVersionJson(installedVersionId);
        var merged = versionJson == null
            ? null
            : VersionJsonParser.Parse(versionJson, _viewModel.LoadVersionJson);

        if (merged == null)
        {
            _statusText.Text = Lang.Get("hmcl.installloaderdialog.976_");
            return;
        }

        if (!_viewModel.SaveInstanceManifest(_instance.Id, merged))
        {
            _statusText.Text = Lang.Get("hmcl.installloaderdialog.978_");
            return;
        }

        Close(true);
    }
}
