using System;
using System.Collections.ObjectModel;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Interactivity;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Java;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// A Java runtime offered by one of the runtime sources of the launcher.
/// </summary>
public sealed class JavaRuntimeItem
{
    /// <summary>
    /// The label shown in the list of the available runtimes.
    /// </summary>
    public string DisplayName { get; init; } = string.Empty;

    /// <summary>
    /// The version of the runtime.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The runtime published by Mojang, or null when the runtime comes from
    /// the Disco API.
    /// </summary>
    public MojangJavaRuntime? MojangRuntime { get; init; }

    /// <summary>
    /// The runtime published through the Disco API, or null when the runtime
    /// comes from Mojang.
    /// </summary>
    public DiscoJavaRuntime? DiscoRuntime { get; init; }

    /// <summary>
    /// The directory name the runtime is installed into.
    /// </summary>
    public string DirectoryName { get; init; } = string.Empty;
}

/// <summary>
/// A source the launcher can download Java runtimes from.
/// </summary>
public sealed class JavaRuntimeSource
{
    /// <summary>
    /// The label shown in the list of the sources.
    /// </summary>
    public string DisplayName { get; init; } = string.Empty;

    /// <summary>
    /// Whether the source is the Disco API of foojay.io.
    /// </summary>
    public bool IsDisco { get; init; }
}

/// <summary>
/// Dialog listing the Java runtimes of Mojang and of the Disco API and
/// installing the selected one into the <c>java</c> directory of the launcher.
/// </summary>
public sealed class JavaDownloadDialog : Window
{
    private readonly SettingsViewModel _settings;
    private readonly ObservableCollection<JavaRuntimeItem> _runtimes = new();
    private readonly ListBox _runtimeList = new();
    private readonly ComboBox _sourceBox = new();
    private readonly ComboBox _distributionBox = new();
    private readonly TextBlock _statusText = new()
    {
        FontSize = 12,
        Foreground = Brushes.Gray,
        TextWrapping = TextWrapping.Wrap
    };
    private readonly Button _installButton = new() { Content = Lang.Get("hmcl.javadownloaddialog.1036_") };

    /// <summary>
    /// Creates the Java download dialog.
    /// </summary>
    /// <param name="settings">The settings view model providing the base
    /// directory of the launcher and registering the installed runtimes.</param>
    public JavaDownloadDialog(SettingsViewModel settings)
    {
        _settings = settings ?? throw new ArgumentNullException(nameof(settings));

        Title = Lang.Get("hmcl.javadownloaddialog.1038_");
        Width = 560;
        Height = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _sourceBox.ItemsSource = new[]
        {
            new JavaRuntimeSource { DisplayName = Lang.Get("hmcl.javadownloaddialog.1040_") },
            new JavaRuntimeSource { DisplayName = "Disco (foojay.io)" }
        };
        _sourceBox.SelectedIndex = 0;
        _sourceBox.HorizontalAlignment = HorizontalAlignment.Stretch;
        _sourceBox.SelectionChanged += OnSourceChanged;

        _distributionBox.ItemsSource = new[]
        {
            DiscoJavaDistribution.Temurin,
            DiscoJavaDistribution.Liberica,
            DiscoJavaDistribution.Zulu,
            DiscoJavaDistribution.Graalvm,
            DiscoJavaDistribution.Semeru,
            DiscoJavaDistribution.Corretto
        };
        _distributionBox.SelectedIndex = 0;
        _distributionBox.HorizontalAlignment = HorizontalAlignment.Stretch;
        _distributionBox.IsVisible = false;
        _distributionBox.SelectionChanged += OnDistributionChanged;

        _runtimeList.ItemsSource = _runtimes;
        _runtimeList.HorizontalAlignment = HorizontalAlignment.Stretch;
        _runtimeList.ItemTemplate = new FuncDataTemplate<JavaRuntimeItem>((item, _) =>
            item == null
                ? new TextBlock()
                : new TextBlock
                {
                    Text = item.DisplayName,
                    VerticalAlignment = VerticalAlignment.Center
                });

        _installButton.HorizontalAlignment = HorizontalAlignment.Right;
        _installButton.Padding = new Thickness(16, 6);
        _installButton.Click += OnInstallClick;

        var sourcePanel = new StackPanel
        {
            Spacing = 8,
            Orientation = Orientation.Horizontal
        };
        sourcePanel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.javadownloaddialog.modpack_origin"),
            VerticalAlignment = VerticalAlignment.Center
        });
        sourcePanel.Children.Add(_sourceBox);
        sourcePanel.Children.Add(_distributionBox);

        Content = new Grid
        {
            Margin = new Thickness(16),
            RowDefinitions = new RowDefinitions("Auto,Auto,*,Auto,Auto"),
            Children =
            {
                new TextBlock
                {
                    Text = Lang.Get("hmcl.javadownloaddialog.1044_"),
                    FontWeight = FontWeight.Bold,
                    Margin = new Thickness(0, 0, 0, 8)
                },
                sourcePanel,
                _runtimeList,
                _statusText,
                _installButton
            }
        };
        this.EnableEscapeToClose();

        Grid.SetRow(sourcePanel, 1);
        Grid.SetRow(_runtimeList, 2);
        Grid.SetRow(_statusText, 3);
        Grid.SetRow(_installButton, 4);

        Opened += async (_, _) => await LoadRuntimesAsync();
    }

    /// <summary>
    /// Returns the distribution selected in the distribution list.
    /// </summary>
    private DiscoJavaDistribution SelectedDistribution =>
        _distributionBox.SelectedItem is DiscoJavaDistribution distribution
            ? distribution
            : DiscoJavaDistribution.Temurin;

    /// <summary>
    /// Reloads the runtime list when the selected source changes.
    /// </summary>
    /// <param name="sender">The source list.</param>
    /// <param name="e">The selection event.</param>
    private async void OnSourceChanged(object? sender, SelectionChangedEventArgs e)
    {
        _distributionBox.IsVisible = _sourceBox.SelectedItem is JavaRuntimeSource { IsDisco: true };
        await LoadRuntimesAsync();
    }

    /// <summary>
    /// Reloads the runtime list when the selected distribution changes.
    /// </summary>
    /// <param name="sender">The distribution list.</param>
    /// <param name="e">The selection event.</param>
    private async void OnDistributionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (_sourceBox.SelectedItem is JavaRuntimeSource { IsDisco: true })
            await LoadRuntimesAsync();
    }

    /// <summary>
    /// Loads the runtime list of the selected source.
    /// </summary>
    private async System.Threading.Tasks.Task LoadRuntimesAsync()
    {
        _statusText.Text = Lang.Get("hmcl.javadownloaddialog.1046_");
        _runtimes.Clear();

        try
        {
            if (_sourceBox.SelectedItem is JavaRuntimeSource { IsDisco: true })
            {
                var distribution = SelectedDistribution;
                var runtimes = await DiscoJavaRuntimeDownloader.GetAvailableRuntimesAsync(distribution);
                foreach (var runtime in runtimes)
                {
                    _runtimes.Add(new JavaRuntimeItem
                    {
                        DisplayName = runtime.DisplayName,
                        Version = runtime.DistributionVersion,
                        DiscoRuntime = runtime,
                        DirectoryName = $"{distribution.ToString().ToLowerInvariant()}-{runtime.DistributionVersion}"
                    });
                }

                _statusText.Text = runtimes.Count > 0
                    ? $"共 {runtimes.Count} 个可用运行时（{DiscoJavaDistributions.GetDisplayName(distribution)}）"
                    : Lang.Get("hmcl.javadownloaddialog.1048_");
            }
            else
            {
                var runtimes = await JavaRuntimeDownloader.GetAvailableRuntimesAsync();
                foreach (var runtime in runtimes)
                {
                    _runtimes.Add(new JavaRuntimeItem
                    {
                        DisplayName = runtime.DisplayName,
                        Version = runtime.Version,
                        MojangRuntime = runtime,
                        DirectoryName = $"{runtime.Component}-{runtime.Version}"
                    });
                }

                _statusText.Text = runtimes.Count > 0
                    ? $"共 {runtimes.Count} 个可用运行时"
                    : Lang.Get("hmcl.javadownloaddialog.1050_");
            }
        }
        catch (Exception ex)
        {
            _statusText.Text = $"获取失败: {ex.Message}";
        }
    }

    /// <summary>
    /// Downloads and registers the selected runtime.
    /// </summary>
    /// <param name="sender">The install button.</param>
    /// <param name="e">The click event.</param>
    private async void OnInstallClick(object? sender, RoutedEventArgs e)
    {
        if (_runtimeList.SelectedItem is not JavaRuntimeItem item)
        {
            _statusText.Text = Lang.Get("hmcl.javadownloaddialog.1052_");
            return;
        }

        _installButton.IsEnabled = false;
        try
        {
            var destination = Path.Combine(
                string.IsNullOrEmpty(_settings.BaseDirectory)
                    ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".hmcl")
                    : _settings.BaseDirectory,
                "java",
                item.DirectoryName);

            Directory.CreateDirectory(destination);

            var progress = new Progress<JavaDownloadProgress>(p => _statusText.Text = p.TotalBytes > 0
                ? $"下载中 {p.CompletedBytes / 1024 / 1024}/{p.TotalBytes / 1024 / 1024} MB（{p.CurrentFile}）"
                : $"({p.CompletedFiles}/{p.TotalFiles}) {p.CurrentFile}");

            string? executable;
            if (item.DiscoRuntime is { } disco)
            {
                executable = await DiscoJavaRuntimeDownloader.InstallAsync(disco, destination, progress);
            }
            else if (item.MojangRuntime is { } mojang)
            {
                executable = await JavaRuntimeDownloader.InstallAsync(mojang, destination, progress);
            }
            else
            {
                _statusText.Text = Lang.Get("hmcl.javadownloaddialog.1054_");
                return;
            }

            if (string.IsNullOrEmpty(executable))
            {
                _statusText.Text = Lang.Get("hmcl.javadownloaddialog.1056_");
                return;
            }

            if (!_settings.AddJavaRuntime(executable))
            {
                _statusText.Text = Lang.Get("hmcl.javadownloaddialog.1058_");
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
}
