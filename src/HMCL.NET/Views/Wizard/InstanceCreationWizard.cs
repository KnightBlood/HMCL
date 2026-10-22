using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Download;
using HMCLCore.NET.Game;
using HMCLCore.NET.Util;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;

namespace HMCL.NET.Views.Wizard;

/// <summary>
/// The state shared by the steps of the instance creation wizard.
/// </summary>
public sealed class InstanceCreationContext
{
    /// <summary>
    /// Creates the context for the given launcher view model.
    /// </summary>
    /// <param name="viewModel">The launcher view model.</param>
    public InstanceCreationContext(MainWindowViewModel viewModel)
    {
        ViewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));
        Memory = viewModel.SettingsService.Settings.Memory;
    }

    /// <summary>
    /// The launcher view model owning the repository and the task manager.
    /// </summary>
    public MainWindowViewModel ViewModel { get; }

    /// <summary>
    /// The versions offered by the wizard.
    /// </summary>
    public ObservableCollection<VersionMetadata> Versions { get; } = new();

    /// <summary>
    /// The version selected by the user.
    /// </summary>
    public VersionMetadata? SelectedVersion { get; set; }

    /// <summary>
    /// The selected mod loader: 无, Forge, NeoForge, Fabric or Quilt.
    /// </summary>
    public string LoaderKind { get; set; } = Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none");

    /// <summary>
    /// The version of the selected mod loader.
    /// </summary>
    public string LoaderVersion { get; set; } = string.Empty;

    /// <summary>
    /// The name of the new instance.
    /// </summary>
    public string InstanceName { get; set; } = string.Empty;

    /// <summary>
    /// The memory allocated to the instance in megabytes.
    /// </summary>
    public int Memory { get; set; } = 2048;

    /// <summary>
    /// Additional JVM arguments of the instance.
    /// </summary>
    public string JvmOptions { get; set; } = string.Empty;

    /// <summary>
    /// The status text shown during the installation.
    /// </summary>
    public string Status { get; set; } = string.Empty;

    /// <summary>
    /// The progress of the installation, between 0 and 100.
    /// </summary>
    public double Progress { get; set; }

    /// <summary>
    /// The mod loader kinds offered by the wizard.
    /// </summary>
    public IReadOnlyList<string> LoaderKinds { get; } =
        new List<string> { Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none_"), "Forge", "NeoForge", "Fabric", "Quilt", "LegacyFabric", "OptiFine", "LiteLoader", "Cleanroom" };
}

/// <summary>
/// The first step of the wizard: picking the Minecraft version to install.
/// </summary>
public sealed class SelectGameVersionStep : IWizardStep, IWizardStepNotifier, IWizardRefreshable
{
    private readonly InstanceCreationContext _context;
    private readonly ListBox _versionList = new();
    private readonly TextBox _searchBox = new() { PlaceholderText=Lang.Get("hmcl.instancecreationwizard.1136_"), Width = 200 };
    private readonly TextBlock _statusText = new() { Foreground = new SolidColorBrush(Color.Parse("#888")) };

    /// <summary>
    /// Creates the version selection step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public SelectGameVersionStep(InstanceCreationContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _searchBox.TextChanged += (_, _) => ApplyFilter();

        _versionList.ItemsSource = _context.Versions;
        _versionList.SelectionChanged += (_, _) =>
        {
            _context.SelectedVersion = _versionList.SelectedItem as VersionMetadata;
            if (_context.SelectedVersion != null && string.IsNullOrEmpty(_context.InstanceName))
                _context.InstanceName = _context.SelectedVersion.Id;

            CanGoNextChanged?.Invoke(this, EventArgs.Empty);
        };

        var refreshButton = new Button { Content = Lang.Get("hmcl.instancecreationwizard.1138_"), Padding = new Thickness(12, 6) };
        refreshButton.Click += async (_, _) => await LoadVersionsAsync();

        var toolbar = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        toolbar.Children.Add(_searchBox);
        toolbar.Children.Add(refreshButton);

        var panel = new StackPanel { Spacing = 8 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.instancecreationwizard.1140_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });
        panel.Children.Add(toolbar);
        panel.Children.Add(_versionList);
        panel.Children.Add(_statusText);

        _versionList.MinHeight = 260;
        Content = panel;
    }

    /// <summary>
    /// Raised when the ability to continue changed.
    /// </summary>
    public event EventHandler? CanGoNextChanged;

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.instancecreationwizard.1142_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// Whether a version is selected.
    /// </summary>
    public bool CanGoNext => _context.SelectedVersion != null;

    /// <summary>
    /// The list of the versions can always be fetched again.
    /// </summary>
    public bool CanRefresh => true;

    /// <summary>
    /// Fetches the list of the versions again.
    /// </summary>
    /// <returns>A task that completes when the list is fetched.</returns>
    public async Task RefreshAsync()
    {
        await LoadVersionsAsync();
    }

    /// <summary>
    /// Loads the version list when it is still empty.
    /// </summary>
    /// <returns>A task that completes when the versions are loaded.</returns>
    public async Task OnEnterAsync()
    {
        if (_context.Versions.Count == 0)
            await LoadVersionsAsync();
    }

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Fetches the remote version manifest and fills the list.
    /// </summary>
    private async Task LoadVersionsAsync()
    {
        var viewModel = _context.ViewModel;
        _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1144_");

        try
        {
            DownloadSourceUtils.Apply(viewModel.SettingsService.Settings);

            var manifestPath = Path.Combine(viewModel.BaseDirectory, "cache", "version_manifest.json");
            var fetchTask = new VersionRemoteFetchTask(manifestPath);
            await viewModel.TaskManager.RunAsync(fetchTask);

            _context.Versions.Clear();
            foreach (var version in fetchTask.Versions
                         .OrderByDescending(v => v, VersionMetadataComparer.Instance))
                _context.Versions.Add(version);

            ApplyFilter();
            _statusText.Text = $"共 {_context.Versions.Count} 个版本";
        }
        catch (Exception ex)
        {
            _statusText.Text = $"获取版本列表失败: {ex.Message}";
        }
    }

    /// <summary>
    /// Hides the versions that do not match the search text.
    /// </summary>
    private void ApplyFilter()
    {
        var keyword = _searchBox.Text?.Trim() ?? string.Empty;
        var selected = _context.SelectedVersion;

        _versionList.ItemsSource = _context.Versions
            .Where(v => keyword.Length == 0 || v.Id.Contains(keyword, StringComparison.OrdinalIgnoreCase))
            .OrderByDescending(v => v.IsRelease)
            .ThenByDescending(v => v.ReleaseTime)
            .ToList();

        if (selected != null)
            _versionList.SelectedItem = _context.Versions.FirstOrDefault(v => v.Id == selected.Id);
    }
}

/// <summary>
/// The second step of the wizard: picking an optional mod loader.
/// </summary>
public sealed class SelectLoaderStep : IWizardStep
{
    private readonly InstanceCreationContext _context;
    private readonly ComboBox _loaderBox = new() { Width = 200 };
    private readonly TextBox _versionBox = new() { PlaceholderText=Lang.Get("hmcl.instancecreationwizard.1146_"), Width = 200 };
    private readonly TextBlock _statusText = new() { Foreground = new SolidColorBrush(Color.Parse("#888")) };

    /// <summary>
    /// Creates the loader selection step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public SelectLoaderStep(InstanceCreationContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _loaderBox.ItemsSource = _context.LoaderKinds;
        _loaderBox.SelectedItem = _context.LoaderKind;
        _loaderBox.SelectionChanged += async (_, _) =>
        {
            if (_loaderBox.SelectedItem is string kind)
            {
                _context.LoaderKind = kind;
                _context.LoaderVersion = string.Empty;
                _versionBox.Text = string.Empty;
                await ResolveLatestLoaderVersionAsync();
            }
        };

        _versionBox.TextChanged += (_, _) => _context.LoaderVersion = _versionBox.Text ?? string.Empty;

        var latestButton = new Button { Content = Lang.Get("hmcl.instancecreationwizard.1148_"), Padding = new Thickness(12, 6) };
        latestButton.Click += async (_, _) => await ResolveLatestLoaderVersionAsync();

        var loaderPanel = new StackPanel { Orientation = Orientation.Horizontal, Spacing = 8 };
        loaderPanel.Children.Add(_loaderBox);
        loaderPanel.Children.Add(_versionBox);
        loaderPanel.Children.Add(latestButton);

        var panel = new StackPanel { Spacing = 8 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.instancecreationwizard.1150_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });
        panel.Children.Add(loaderPanel);
        panel.Children.Add(_statusText);

        Content = panel;
    }

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.instancecreationwizard.1152_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// The loader step is always skippable.
    /// </summary>
    public bool CanGoNext => true;

    /// <summary>
    /// Refreshes the loader version when the step is shown.
    /// </summary>
    /// <returns>A task that completes when the version is resolved.</returns>
    public async Task OnEnterAsync()
    {
        if (_context.LoaderKind != Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none__") && string.IsNullOrEmpty(_context.LoaderVersion))
            await ResolveLatestLoaderVersionAsync();
    }

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Resolves the newest published version of the selected loader.
    /// </summary>
    private async Task ResolveLatestLoaderVersionAsync()
    {
        if (_context.LoaderKind == Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none___") || _context.SelectedVersion == null)
            return;

        var kind = ToLoaderKind(_context.LoaderKind);
        if (kind == null)
            return;

        _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1158_");

        try
        {
            string? version;
            if (kind == LoaderKind.Forge)
            {
                version = await ForgeInstaller.GetForgeVersionAsync(_context.SelectedVersion.Id);
            }
            else if (kind == LoaderKind.NeoForge)
            {
                version = await ForgeInstaller.GetNeoForgeVersionAsync(_context.SelectedVersion.Id);
            }
            else
            {
                var versions = await MetaLoaderInstaller.GetLoaderVersionsAsync(kind.Value, _context.SelectedVersion.Id);
                version = versions.FirstOrDefault(v => v.Stable)?.Version ?? versions.FirstOrDefault()?.Version;
            }

            if (string.IsNullOrEmpty(version))
            {
                _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1160_");
                return;
            }

            _context.LoaderVersion = version;
            _versionBox.Text = version;
            _statusText.Text = $"最新版本: {version}";
        }
        catch (Exception ex)
        {
            _statusText.Text = $"获取加载器版本失败: {ex.Message}";
        }
    }

    /// <summary>
    /// Converts a loader name into the matching loader kind.
    /// </summary>
    /// <param name="name">The loader name shown in the UI.</param>
    /// <returns>The loader kind, or null when the name is unknown.</returns>
    internal static LoaderKind? ToLoaderKind(string name) => name switch
    {
        "Forge" => LoaderKind.Forge,
        "NeoForge" => LoaderKind.NeoForge,
        "Fabric" => LoaderKind.Fabric,
        "Quilt" => LoaderKind.Quilt,
        "LegacyFabric" => LoaderKind.LegacyFabric,
        "OptiFine" => LoaderKind.OptiFine,
        "LiteLoader" => LoaderKind.LiteLoader,
        "Cleanroom" => LoaderKind.Cleanroom,
        _ => null
    };
}

/// <summary>
/// The third step of the wizard: naming the instance and tuning its settings.
/// </summary>
public sealed class ConfigureInstanceStep : IWizardStep, IWizardStepNotifier
{
    private readonly InstanceCreationContext _context;
    private readonly TextBox _nameBox = new() { PlaceholderText=Lang.Get("hmcl.instancecreationwizard.1162_"), Width = 260 };
    private readonly NumericUpDown _memoryBox = new()
    {
        Minimum = 512,
        Maximum = 65536,
        Increment = 512,
        FormatString = "0",
        Width = 140
    };
    private readonly TextBox _jvmBox = new() { PlaceholderText=Lang.Get("hmcl.instancecreationwizard.1164_"), Width = 320 };

    /// <summary>
    /// Creates the instance configuration step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public ConfigureInstanceStep(InstanceCreationContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _nameBox.Text = _context.InstanceName;
        _nameBox.TextChanged += (_, _) =>
        {
            _context.InstanceName = _nameBox.Text ?? string.Empty;
            CanGoNextChanged?.Invoke(this, EventArgs.Empty);
        };

        _memoryBox.Value = _context.Memory;
        _memoryBox.ValueChanged += (_, _) =>
        {
            if (_memoryBox.Value is { } value)
                _context.Memory = (int)value;
        };

        _jvmBox.TextChanged += (_, _) => _context.JvmOptions = _jvmBox.Text ?? string.Empty;

        var panel = new StackPanel { Spacing = 8 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.instancecreationwizard.1166_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });

        panel.Children.Add(new TextBlock { Text = Lang.Get("hmcl.instancecreationwizard.1168_"), FontWeight = FontWeight.SemiBold });
        panel.Children.Add(_nameBox);

        panel.Children.Add(new TextBlock { Text = Lang.Get("hmcl.instancecreationwizard.1170_"), FontWeight = FontWeight.SemiBold });
        panel.Children.Add(_memoryBox);

        panel.Children.Add(new TextBlock { Text = Lang.Get("hmcl.instancecreationwizard.1172_"), FontWeight = FontWeight.SemiBold });
        panel.Children.Add(_jvmBox);

        Content = panel;
    }

    /// <summary>
    /// Raised when the ability to continue changed.
    /// </summary>
    public event EventHandler? CanGoNextChanged;

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.instancecreationwizard.1174_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// Whether the instance name is filled in.
    /// </summary>
    public bool CanGoNext => !string.IsNullOrWhiteSpace(_context.InstanceName);

    /// <summary>
    /// Fills the instance name from the selected version when it is empty.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnEnterAsync()
    {
        if (string.IsNullOrWhiteSpace(_context.InstanceName) && _context.SelectedVersion != null)
        {
            _context.InstanceName = _context.SelectedVersion.Id;
            _nameBox.Text = _context.InstanceName;
        }

        return Task.CompletedTask;
    }

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;
}

/// <summary>
/// The last step of the wizard: showing the summary and running the installation.
/// </summary>
public sealed class InstallInstanceStep : IWizardStep, IWizardTaskStep
{
    private readonly InstanceCreationContext _context;
    private readonly TextBlock _summaryText = new() { TextWrapping = TextWrapping.Wrap };
    private readonly TextBlock _statusText = new() { Foreground = new SolidColorBrush(Color.Parse("#666")) };
    private readonly ProgressBar _progressBar = new() { Minimum = 0, Maximum = 100, Height = 6 };
    private readonly Button _installButton = new()
    {
        Content = Lang.Get("hmcl.instancecreationwizard.1176_"),
        Padding = new Thickness(20, 8),
        Background = new SolidColorBrush(Color.Parse("#2B579A")),
        Foreground = Brushes.White
    };

    /// <summary>
    /// Creates the installation step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public InstallInstanceStep(InstanceCreationContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _installButton.Click += async (_, _) => await InstallAsync();

        var panel = new StackPanel { Spacing = 8 };
        panel.Children.Add(_summaryText);
        panel.Children.Add(_installButton);
        panel.Children.Add(_progressBar);
        panel.Children.Add(_statusText);

        _panel = panel;
        Content = panel;
    }

    private readonly StackPanel _panel;

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.instancecreationwizard.1178_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// The installation is started from the step itself, so the wizard cannot
    /// be completed with the navigation buttons.
    /// </summary>
    public bool CanGoNext => false;

    /// <summary>
    /// The title of the dialog showing the progress of the installation.
    /// </summary>
    public string TaskTitle => Lang.Get("hmcl.instancecreationwizard.1176_");

    /// <summary>
    /// The message shown in the dialog while the instance is installed.
    /// </summary>
    public string TaskMessage => Lang.Get("hmcl.instancecreationwizard.1182_");

    /// <summary>
    /// Installs the instance, reporting the progress to the given progress.
    /// </summary>
    /// <param name="progress">Receives the progress of the installation.</param>
    /// <returns>A task that completes when the instance is installed.</returns>
    public async Task RunTaskAsync(IProgress<double> progress)
    {
        await InstallAsync(progress);
    }

    /// <summary>
    /// Refreshes the summary when the step is shown.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnEnterAsync()
    {
        _summaryText.Text =
            $"实例名称: {_context.InstanceName}\n" +
            $"游戏版本: {_context.SelectedVersion?.Id}\n" +
            $"模组加载器: {(_context.LoaderKind == "无" ? Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none____") : $"{_context.LoaderKind} {_context.LoaderVersion}")}\n" +
            $"内存分配: {_context.Memory} MB\n" +
            $"安装目录: {Path.Combine(_context.ViewModel.BaseDirectory, "instances", _context.InstanceName)}";

        return Task.CompletedTask;
    }

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Downloads the game version, installs the optional loader and creates the
    /// instance, then downloads the game files.
    /// </summary>
    /// <param name="progress">Receives the progress of the installation.</param>
    private async Task InstallAsync(IProgress<double>? progress = null)
    {
        if (_context.SelectedVersion == null)
            return;

        _installButton.IsEnabled = false;
        var viewModel = _context.ViewModel;

        void Report(double value) => progress?.Report(value);

        try
        {
            DownloadSourceUtils.Apply(viewModel.SettingsService.Settings);

            _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1182_");
            _progressBar.Value = 5;
            Report(5);

            var version = _context.SelectedVersion;
            var jsonPath = Path.Combine(viewModel.BaseDirectory, "cache", "vjson", $"{version.Id}.json");
            await viewModel.TaskManager.RunAsync(new VersionJsonDownloadTask(new Uri(version.Url), jsonPath));

            var json = File.ReadAllText(jsonPath);

            var versionDirectory = Path.Combine(viewModel.BaseDirectory, "versions", version.Id);
            Directory.CreateDirectory(versionDirectory);
            File.WriteAllText(Path.Combine(versionDirectory, $"{version.Id}.json"), json);

            var manifest = VersionJsonParser.Parse(json, viewModel.LoadVersionJson);
            if (manifest == null)
                throw new InvalidDataException(Lang.Get("hmcl.instancecreationwizard.1184_"));

            _progressBar.Value = 20;
            Report(20);

            if (_context.LoaderKind != Lang.Get("hmcl.instancecreationwizard.settings_game_quick_play_none_____"))
            {
                _statusText.Text = $"正在安装 {_context.LoaderKind}...";
                manifest = await InstallLoaderAsync(manifest) ?? manifest;
                _progressBar.Value = 40;
                Report(40);
            }

            _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1188_");
            if (!viewModel.SaveInstance(_context.InstanceName, manifest))
                throw new InvalidOperationException(Lang.Get("hmcl.instancecreationwizard.1190_"));

            _progressBar.Value = 50;
            Report(50);
            _statusText.Text = Lang.Get("hmcl.instancecreationwizard.1192_");

            var report = await GameIntegrityChecker.VerifyAsync(manifest, viewModel.BaseDirectory);
            await GameIntegrityChecker.RepairAsync(report);

            _progressBar.Value = 100;
            Report(100);
            _statusText.Text = report.MissingFiles == 0 && report.CorruptFiles == 0
                ? Lang.Get("hmcl.instancecreationwizard.1194_")
                : $"安装完成，仍有 {report.MissingFiles + report.CorruptFiles} 个文件未能下载，请稍后在实例管理中重试修复。";

            viewModel.LoadInstances();

            if (TopLevel.GetTopLevel(_panel) is Window owner)
                owner.Close(true);
        }
        catch (Exception ex)
        {
            _statusText.Text = $"安装失败: {ex.Message}";
            _installButton.IsEnabled = true;
        }
    }

    /// <summary>
    /// Installs the selected mod loader and returns the resulting manifest.
    /// </summary>
    /// <param name="vanilla">The manifest of the vanilla version.</param>
    /// <returns>The merged manifest, or null when the loader could not be installed.</returns>
    private async Task<GameInstanceManifest?> InstallLoaderAsync(GameInstanceManifest vanilla)
    {
        var kind = SelectLoaderStep.ToLoaderKind(_context.LoaderKind);
        if (kind == null)
            return null;

        var viewModel = _context.ViewModel;

        if (kind is LoaderKind.Forge or LoaderKind.NeoForge)
        {
            var installed = await ForgeInstaller.InstallAsync(
                kind.Value,
                viewModel.BaseDirectory,
                viewModel.JavaExecutablePath,
                vanilla.Version,
                string.IsNullOrWhiteSpace(_context.LoaderVersion) ? null : _context.LoaderVersion,
                new Progress<string>(message =>
                    Avalonia.Threading.Dispatcher.UIThread.Post(() => _statusText.Text = message)));

            if (string.IsNullOrEmpty(installed))
                return null;

            var json = viewModel.LoadVersionJson(installed);
            if (json == null)
                return null;

            var versionDirectory = Path.Combine(viewModel.BaseDirectory, "versions", installed);
            Directory.CreateDirectory(versionDirectory);
            File.WriteAllText(Path.Combine(versionDirectory, $"{installed}.json"), json);

            return VersionJsonParser.Parse(json, viewModel.LoadVersionJson);
        }

        if (kind is LoaderKind.OptiFine or LoaderKind.LiteLoader or LoaderKind.Cleanroom)
        {
            var installed = await LoaderInstaller.InstallAsync(
                kind.Value,
                viewModel.BaseDirectory,
                viewModel.JavaExecutablePath,
                vanilla.Version,
                string.IsNullOrWhiteSpace(_context.LoaderVersion) ? null : _context.LoaderVersion,
                new Progress<string>(message =>
                    Avalonia.Threading.Dispatcher.UIThread.Post(() => _statusText.Text = message)));

            if (string.IsNullOrEmpty(installed))
                return null;

            var json = viewModel.LoadVersionJson(installed);
            if (json == null)
                return null;

            var versionDirectory = Path.Combine(viewModel.BaseDirectory, "versions", installed);
            Directory.CreateDirectory(versionDirectory);
            File.WriteAllText(Path.Combine(versionDirectory, $"{installed}.json"), json);

            return VersionJsonParser.Parse(json, viewModel.LoadVersionJson);
        }

        return await MetaLoaderInstaller.CreateManifestAsync(
            kind.Value, vanilla,
            string.IsNullOrWhiteSpace(_context.LoaderVersion) ? null : _context.LoaderVersion);
    }
}
