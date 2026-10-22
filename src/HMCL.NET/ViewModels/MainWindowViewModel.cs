using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Globalization;
using System.ComponentModel;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Http;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Threading.Tasks;
using System.Windows.Input;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Threading;
using HMCLCore.NET.Auth;
using HMCLCore.NET.Auth.AuthlibInjector;
using HMCLCore.NET.Addon;
using HMCLCore.NET.Download;
using HMCLCore.NET.Events;
using HMCLCore.NET.Game;
using HMCLCore.NET.Java;
using HMCLCore.NET.Launch;
using HMCLCore.NET.Modpack;
using HMCLCore.NET.Setting;
using HMCLCore.NET.Tasks;
using HMCLCore.NET.Terracotta;
using HMCLCore.NET.Util;
using HMCL.NET.Services;
using HMCL.NET.Controls;
using HMCL.NET.Util;
using HMCL.NET.Views;
using HMCL.NET.Views.Dialogs;
using HMCL.NET.Views.Wizard;

namespace HMCL.NET.ViewModels;

/// <summary>
/// View model base class with property change notification.
/// </summary>
public abstract class ViewModelBase : INotifyPropertyChanged
{
    /// <summary>
    /// Raises a property change notification.
    /// </summary>
    protected bool Set<T>(ref T field, T value, [CallerMemberName] string? propertyName = null)
    {
        if (Equals(field, value)) return false;
        field = value;
        OnPropertyChanged(propertyName);
        return true;
    }

    /// <summary>
    /// Called when a property changes.
    /// </summary>
    protected virtual void OnPropertyChanged(string? propertyName = null)
    {
        PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(propertyName));
    }

    /// <summary>
    /// Property changed event.
    /// </summary>
    public event PropertyChangedEventHandler? PropertyChanged;
}

/// <summary>
/// View model for the main application window.
/// Manages accounts, game instances, and launch actions.
/// </summary>
public sealed class MainWindowViewModel : ViewModelBase
{
    private readonly AccountManager _accountManager;
    private readonly SettingsManager _settingsManager;
    private readonly JavaManager _javaManager;
    private readonly GameLauncherExecutor _launchExecutor;
    private readonly GameDirectoryManager _gameDirectoryManager;
    private string _statusMessage = Lang.Get("hmcl.mainwindowviewmodel.712_", "就绪");
    private bool _isLaunching;
    private Account? _selectedAccount;
    private GameInstance? _selectedInstance;
    private string? _pendingAssetIndex;

    /// <summary>
    /// The status message shown at the bottom of the window.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => Set(ref _statusMessage, value);
    }

    /// <summary>
    /// Whether a launch operation is in progress.
    /// </summary>
    public bool IsLaunching
    {
        get => _isLaunching;
        set => Set(ref _isLaunching, value);
    }

    /// <summary>
    /// All available accounts.
    /// </summary>
    public ObservableCollection<Account> Accounts { get; } = new();

    /// <summary>
    /// The authlib-injector servers saved by the user.
    /// </summary>
    public ObservableCollection<AuthlibInjectorServer> AuthServers { get; } = new();

    /// <summary>
    /// The authlib-injector server selected in the accounts page.
    /// </summary>
    public AuthlibInjectorServer? SelectedAuthServer
    {
        get => _selectedAuthServer;
        set => Set(ref _selectedAuthServer, value);
    }

    private AuthlibInjectorServer? _selectedAuthServer;

    /// <summary>
    /// The manager of the saved authlib-injector servers.
    /// </summary>
    public AuthlibInjectorServerManager AuthServerManager =>
        new(_settingsManager.Settings.AuthServers);

    /// <summary>
    /// Reloads the saved authlib-injector servers from the settings.
    /// </summary>
    public void SyncAuthServers()
    {
        AuthServers.Clear();
        foreach (var server in AuthServerManager.Servers)
            AuthServers.Add(server);
    }

    /// <summary>
    /// Locates the authlib-injector server at the given URL, saves it and
    /// refreshes the list of the saved servers.
    /// </summary>
    /// <param name="url">The URL of the server entered by the user.</param>
    /// <returns>Whether the server was found and saved.</returns>
    public async Task<bool> AddAuthServerAsync(string url)
    {
        if (string.IsNullOrWhiteSpace(url))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.514_"));
            return false;
        }

        try
        {
            var server = await AuthlibInjectorServer.LocateAsync(url);
            AuthServerManager.Add(server);
            _settingsManager.Save();
            SyncAuthServers();
            SelectedAuthServer = AuthServers.FirstOrDefault(item => item.Url == server.Url);
            SetStatus($"已添加认证服务器 {server.DisplayName}");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"添加认证服务器失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Removes the selected authlib-injector server from the saved servers.
    /// </summary>
    public void RemoveSelectedAuthServer()
    {
        if (SelectedAuthServer is not { } server)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.516_"));
            return;
        }

        if (AuthServerManager.Remove(server.Url))
        {
            _settingsManager.Save();
            SyncAuthServers();
            SelectedAuthServer = null;
            SetStatus($"已删除认证服务器 {server.DisplayName}");
        }
    }

    /// <summary>
    /// The repository backing the currently selected game directory.
    /// </summary>
    public DefaultGameRepository Repository => _gameDirectoryManager.Repository;

    /// <summary>
    /// The manager owning the game directories and their repositories.
    /// </summary>
    public GameDirectoryManager Directories => _gameDirectoryManager;

    /// <summary>
    /// The repository base directory holding versions, libraries and assets.
    /// </summary>
    public string BaseDirectory => Repository.Layout.BaseDirectory;

    /// <summary>
    /// The path of the Java executable used to run installer tasks. Falls back
    /// to the configured Java path when no runtime was detected.
    /// </summary>
    public string JavaExecutablePath =>
        _javaManager.PreferredRuntime?.Path ?? _settingsManager.Settings.JavaPath;

    /// <summary>
    /// Loads a version JSON from the repository by version id.
    /// </summary>
    /// <param name="versionId">The version id.</param>
    /// <returns>The JSON text, or null when the file is missing.</returns>
    public string? LoadVersionJson(string versionId)
    {
        var path = Path.Combine(Repository.Layout.BaseDirectory, "versions", versionId, $"{versionId}.json");
        return File.Exists(path) ? File.ReadAllText(path) : null;
    }

    /// <summary>
    /// All available game instances.
    /// </summary>
    public ObservableCollection<GameInstance> GameInstances { get; } = new();

    /// <summary>
    /// Instances filtered by <see cref="InstanceSearchText"/>, displayed in the list.
    /// </summary>
    public ObservableCollection<GameInstance> FilteredInstances { get; } = new();

    /// <summary>
    /// The text used to filter the instance list by id.
    /// </summary>
    public string InstanceSearchText
    {
        get => _instanceSearchText;
        set
        {
            Set(ref _instanceSearchText, value);
            ApplyInstanceFilter();
        }
    }

    private string _instanceSearchText = string.Empty;

    /// <summary>
    /// Rebuilds <see cref="FilteredInstances"/> from <see cref="GameInstances"/>.
    /// </summary>
    private void ApplyInstanceFilter()
    {
        FilteredInstances.Clear();
        foreach (var instance in GameInstances)
        {
            if (string.IsNullOrWhiteSpace(InstanceSearchText) ||
                instance.Id.Contains(InstanceSearchText, StringComparison.OrdinalIgnoreCase))
                FilteredInstances.Add(instance);
        }
    }

    /// <summary>
    /// The currently selected account.
    /// </summary>
    public Account? SelectedAccount
    {
        get => _selectedAccount;
        set
        {
            if (Set(ref _selectedAccount, value))
                OnLaunchStateChanged();
        }
    }

    /// <summary>
    /// The currently selected game instance.
    /// </summary>
    public GameInstance? SelectedInstance
    {
        get => _selectedInstance;
        set
        {
            if (Set(ref _selectedInstance, value))
            {
                OnLaunchStateChanged();
                RefreshInstanceComponents();
            }
        }
    }

    /// <summary>
    /// The components installed in the selected instance, like its mod loader.
    /// </summary>
    public ObservableCollection<GameComponentMark> InstanceComponents { get; } = new();

    /// <summary>
    /// Refreshes the list of the components installed in the selected instance.
    /// </summary>
    public void RefreshInstanceComponents()
    {
        InstanceComponents.Clear();

        if (SelectedInstance is not { } instance)
            return;

        var manifest = LoadInstanceManifest(instance);
        if (manifest is null)
            return;

        var analyzer = GameComponentAnalyzer.Analyze(manifest);
        foreach (var mark in analyzer.Marks)
            InstanceComponents.Add(mark);
    }

    /// <summary>
    /// Installs OptiFine into the launcher, so that the selected instance can
    /// be launched with it.
    /// </summary>
    /// <returns>Whether OptiFine was installed.</returns>
    public async Task<bool> InstallOptiFineAsync()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.518_"));
            return false;
        }

        var javaPath = _javaManager.PreferredRuntime?.Path;
        if (string.IsNullOrEmpty(javaPath))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.520_"));
            return false;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.522_");

        try
        {
            var progress = new Progress<string>(line => LauncherLog.Info(line));

            var versionId = await LoaderInstaller.InstallAsync(
                LoaderKind.OptiFine, Repository.Layout.BaseDirectory, javaPath!, instance.Version, progress: progress);

            if (string.IsNullOrEmpty(versionId))
            {
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.524_"));
                return false;
            }

            SetStatus($"OptiFine 已安装 ({versionId})");
            await Repository.RefreshAsync();
            LoadInstances();
            RefreshInstanceComponents();
            return true;
        }
        catch (Exception ex)
        {
            LauncherLog.Error(Lang.Get("hmcl.mainwindowviewmodel.526_"), ex);
            SetStatus($"OptiFine 安装失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Installs a mod loader into the launcher from a local installer file,
    /// without downloading anything.
    /// </summary>
    /// <param name="installerPath">The path of the installer file.</param>
    /// <returns>Whether the loader was installed.</returns>
    public async Task<bool> InstallLoaderFromFileAsync(string installerPath)
    {
        if (string.IsNullOrWhiteSpace(installerPath) || !File.Exists(installerPath))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.528_"));
            return false;
        }

        var javaPath = _javaManager.PreferredRuntime?.Path;
        if (string.IsNullOrEmpty(javaPath))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.530_"));
            return false;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.532_");

        try
        {
            var progress = new Progress<string>(line => LauncherLog.Info(line));

            var versionId = await ForgeInstaller.InstallFromFileAsync(
                Repository.Layout.BaseDirectory,
                javaPath!,
                installerPath,
                loaderVersion: null,
                kind: null,
                gameVersion: SelectedInstance?.Version,
                progress: progress);

            if (string.IsNullOrEmpty(versionId))
            {
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.534_"));
                return false;
            }

            SetStatus($"已安装 {versionId}");
            await Repository.RefreshAsync();
            LoadInstances();
            RefreshInstanceComponents();
            return true;
        }
        catch (Exception ex)
        {
            LauncherLog.Error(Lang.Get("hmcl.mainwindowviewmodel.536_"), ex);
            SetStatus($"安装器执行失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// The currently selected tab index on the instance detail page.
    /// </summary>
    public int SelectedTabIndex
    {
        get => _selectedTabIndex;
        set => Set(ref _selectedTabIndex, value);
    }

    private int _selectedTabIndex;

    /// <summary>
    /// The resource packs for the selected instance (populated by scanning the
    /// instance's resourcepacks directory).
    /// </summary>
    public IReadOnlyList<ResourcePack> ResourcePacks { get; set; } = new List<ResourcePack>();

    /// <summary>
    /// The resource pack manager of the selected instance, or null when no
    /// instance is selected.
    /// </summary>
    private ResourcePackManager? _resourcePackManager;

    /// <summary>
    /// The resource pack selected in the resource pack list.
    /// </summary>
    public ResourcePack? SelectedResourcePack
    {
        get => _selectedResourcePack;
        set => Set(ref _selectedResourcePack, value);
    }

    private ResourcePack? _selectedResourcePack;

    /// <summary>
    /// Whether the Minecraft version of the instance supports resource packs.
    /// </summary>
    public bool ResourcePacksSupported { get; set; } = true;

    /// <summary>
    /// Scans the selected instance's resource packs directory.
    /// </summary>
    public void ScanResourcePacks()
    {
        ResourcePackManager? manager = null;

        if (SelectedInstance is { } instance)
        {
            var resourcePackDir = Path.Combine(instance.GameDirectory, "resourcepacks");
            var optionsFile = Path.Combine(instance.GameDirectory, "options.txt");
            manager = new ResourcePackManager(resourcePackDir, optionsFile, instance.Version);
            manager.Scan();
        }

        _resourcePackManager = manager;
        ResourcePacks = manager?.Packs ?? (IReadOnlyList<ResourcePack>)new List<ResourcePack>();
        ResourcePacksSupported = manager == null || ResourcePackFormats.IsSupported(manager.MinecraftVersion);

        if (SelectedResourcePack != null &&
            !ResourcePacks.Any(p => ReferenceEquals(p, SelectedResourcePack)))
        {
            SelectedResourcePack = null;
        }
    }

    /// <summary>
    /// Enables or disables the selected resource pack and refreshes the list.
    /// </summary>
    /// <param name="enabled">Whether the pack should be enabled.</param>
    public void SetResourcePackEnabled(bool enabled)
    {
        if (_resourcePackManager == null || SelectedResourcePack == null)
            return;

        if (_resourcePackManager.SetEnabled(SelectedResourcePack, enabled))
        {
            SetStatus(enabled
                ? $"已启用资源包 {SelectedResourcePack.Name}"
                : $"已禁用资源包 {SelectedResourcePack.Name}");
        }

        ScanResourcePacks();
    }

    /// <summary>
    /// Deletes the selected resource pack and refreshes the list.
    /// </summary>
    public void RemoveSelectedResourcePack()
    {
        if (_resourcePackManager == null || SelectedResourcePack == null)
            return;

        var name = SelectedResourcePack.Name;
        if (_resourcePackManager.Remove(SelectedResourcePack))
        {
            SetStatus($"已删除资源包 {name}");
            SelectedResourcePack = null;
            ScanResourcePacks();
        }
        else
        {
            SetStatus($"删除资源包 {name} 失败");
        }
    }

    /// <summary>
    /// Imports a resource pack archive into the selected instance.
    /// </summary>
    /// <param name="archivePath">The archive to import.</param>
    public void ImportResourcePack(string archivePath)
    {
        if (_resourcePackManager == null)
            return;

        var pack = _resourcePackManager.Import(archivePath);
        if (pack != null)
        {
            SetStatus($"已导入资源包 {pack.Name}");
            ScanResourcePacks();
        }
        else
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.538_"));
        }
    }

    /// <summary>
    /// Imports the given mod files into the mods directory of the selected
    /// instance. Used when the user drops mod files onto the instance list.
    /// </summary>
    /// <param name="filePaths">The paths of the mod files to install.</param>
    /// <returns>The number of mods that were installed.</returns>
    public int InstallModsIntoSelectedInstance(IReadOnlyList<string> filePaths)
    {
        if (filePaths == null || filePaths.Count == 0)
            return 0;

        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.540_"));
            return 0;
        }

        var manager = new LocalAddonManager(Path.Combine(instance.GameDirectory, "mods"));

        var installed = 0;
        var failed = 0;
        foreach (var path in filePaths)
        {
            if (manager.Install(path) != null)
                installed++;
            else
                failed++;
        }

        SetStatus(failed == 0
            ? $"已向实例 {instance.Id} 导入 {installed} 个模组"
            : $"已向实例 {instance.Id} 导入 {installed} 个模组，{failed} 个文件无法导入");

        return installed;
    }

    /// <summary>
    /// The data packs for the selected instance (populated by scanning the
    /// instance's datapacks directory).
    /// </summary>
    public IReadOnlyList<DataPack> DataPacks { get; set; } = new List<DataPack>();

    /// <summary>
    /// The data pack manager of the selected instance, or null when no
    /// instance is selected.
    /// </summary>
    private DataPackManager? _dataPackManager;

    /// <summary>
    /// The data pack selected in the data pack list.
    /// </summary>
    public DataPack? SelectedDataPack
    {
        get => _selectedDataPack;
        set => Set(ref _selectedDataPack, value);
    }

    private DataPack? _selectedDataPack;

    /// <summary>
    /// Scans the selected instance's data packs directory.
    /// </summary>
    public void ScanDataPacks()
    {
        DataPackManager? manager = null;

        if (SelectedInstance is { } instance)
        {
            var dataPackDir = Path.Combine(instance.GameDirectory, "datapacks");
            manager = new DataPackManager(dataPackDir);
            manager.Scan();
        }

        _dataPackManager = manager;
        DataPacks = manager?.Packs ?? (IReadOnlyList<DataPack>)new List<DataPack>();

        if (SelectedDataPack != null && !DataPacks.Any(p => ReferenceEquals(p, SelectedDataPack)))
            SelectedDataPack = null;
    }

    /// <summary>
    /// Enables or disables the selected data pack and refreshes the list.
    /// </summary>
    /// <param name="enabled">Whether the pack should be active.</param>
    public void SetDataPackEnabled(bool enabled)
    {
        if (_dataPackManager == null || SelectedDataPack == null)
            return;

        if (_dataPackManager.SetEnabled(SelectedDataPack, enabled))
        {
            SetStatus(enabled
                ? $"已启用数据包 {SelectedDataPack.Name}"
                : $"已禁用数据包 {SelectedDataPack.Name}");
        }

        ScanDataPacks();
    }

    /// <summary>
    /// Deletes the selected data pack and refreshes the list.
    /// </summary>
    public void RemoveSelectedDataPack()
    {
        if (_dataPackManager == null || SelectedDataPack == null)
            return;

        var name = SelectedDataPack.Name;
        if (_dataPackManager.Remove(SelectedDataPack))
        {
            SetStatus($"已删除数据包 {name}");
            SelectedDataPack = null;
            ScanDataPacks();
        }
        else
        {
            SetStatus($"删除数据包 {name} 失败");
        }
    }

    /// <summary>
    /// Installs a data pack archive into the selected instance.
    /// </summary>
    /// <param name="archivePath">The archive to install.</param>
    public void InstallDataPack(string archivePath)
    {
        if (_dataPackManager == null)
            return;

        var pack = _dataPackManager.Install(archivePath);
        if (pack != null)
        {
            SetStatus($"已安装数据包 {pack.Name}");
            ScanDataPacks();
        }
        else
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.542_"));
        }
    }

    /// <summary>
    /// Asks the running game to reload its data packs.
    /// </summary>
    public void ReloadDataPacks()
    {
        if (_gameProcess is not { } process || process.HasExited)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.544_"));
            return;
        }

        if (process.SendInput("/reload"))
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.546_"));
        else
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.548_"));
    }

    /// <summary>
    /// The shader packs for the selected instance (populated by scanning the
    /// instance's shaderpacks directory).
    /// </summary>
    public ObservableCollection<DataPack> ShaderPacks { get; } = new();

    /// <summary>
    /// The shader pack selected in the shader pack list.
    /// </summary>
    public DataPack? SelectedShaderPack
    {
        get => _selectedShaderPack;
        set => Set(ref _selectedShaderPack, value);
    }

    private DataPack? _selectedShaderPack;

    /// <summary>
    /// Scans the selected instance's shader packs directory.
    /// </summary>
    public void ScanShaderPacks()
    {
        ShaderPacks.Clear();

        if (SelectedInstance == null)
            return;

        var shaderPackDir = Path.Combine(SelectedInstance.GameDirectory, "shaderpacks");
        var manager = new DataPackManager(shaderPackDir);
        manager.Scan();

        foreach (var pack in manager.Packs)
            ShaderPacks.Add(pack);
    }

    /// <summary>
    /// Enables or disables the selected shader pack.
    /// </summary>
    /// <param name="enabled">Whether the shader pack should be active.</param>
    /// <returns>Whether the shader pack was changed.</returns>
    public bool SetShaderPackEnabled(bool enabled)
    {
        if (SelectedInstance == null || SelectedShaderPack is null)
            return false;

        var shaderPackDir = Path.Combine(SelectedInstance.GameDirectory, "shaderpacks");
        var manager = new DataPackManager(shaderPackDir);
        manager.Scan();

        var pack = manager.Packs.FirstOrDefault(p =>
            string.Equals(p.Path, SelectedShaderPack.Path, StringComparison.OrdinalIgnoreCase));

        if (pack is null || !manager.SetEnabled(pack, enabled))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.550_"));
            return false;
        }

        ScanShaderPacks();
        SetStatus(enabled ? Lang.Get("hmcl.mainwindowviewmodel.552_") : Lang.Get("hmcl.mainwindowviewmodel.554_"));
        return true;
    }

    /// <summary>
    /// Imports a shader pack archive into the shader packs directory.
    /// </summary>
    /// <param name="archivePath">The path of the archive to import.</param>
    /// <returns>Whether the shader pack was imported.</returns>
    public bool ImportShaderPack(string archivePath)
    {
        if (SelectedInstance == null)
            return false;

        var shaderPackDir = Path.Combine(SelectedInstance.GameDirectory, "shaderpacks");
        var manager = new DataPackManager(shaderPackDir);

        var pack = manager.Install(archivePath);
        if (pack is null)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.556_"));
            return false;
        }

        ScanShaderPacks();
        SetStatus($"已导入光影 {pack.Name}");
        return true;
    }

    /// <summary>
    /// Deletes the selected shader pack.
    /// </summary>
    /// <returns>Whether the shader pack was deleted.</returns>
    public bool RemoveSelectedShaderPack()
    {
        if (SelectedInstance == null || SelectedShaderPack is null)
            return false;

        var shaderPackDir = Path.Combine(SelectedInstance.GameDirectory, "shaderpacks");
        var manager = new DataPackManager(shaderPackDir);
        manager.Scan();

        var pack = manager.Packs.FirstOrDefault(p =>
            string.Equals(p.Path, SelectedShaderPack.Path, StringComparison.OrdinalIgnoreCase));

        if (pack is null || !manager.Remove(pack))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.558_"));
            return false;
        }

        ScanShaderPacks();
        SetStatus(Lang.Get("hmcl.mainwindowviewmodel.560_"));
        return true;
    }

    /// <summary>
    /// The screenshot files for the selected instance (populated by scanning the
    /// instance's screenshots directory).
    /// </summary>
    public ObservableCollection<ScreenshotInfo> Screenshots { get; } = new();

    /// <summary>
    /// The date filter applied to the list of the screenshots.
    /// </summary>
    public ScreenshotFilter ScreenshotFilter
    {
        get => _screenshotFilter;
        set
        {
            if (Set(ref _screenshotFilter, value))
                ScanScreenshots();
        }
    }

    private ScreenshotFilter _screenshotFilter = ScreenshotFilter.All;

    /// <summary>
    /// Scans the selected instance's screenshots directory.
    /// </summary>
    public void ScanScreenshots()
    {
        Screenshots.Clear();

        if (SelectedInstance == null)
            return;

        var manager = new ScreenshotManager(SelectedInstance.GameDirectory);
        foreach (var screenshot in manager.GetScreenshots())
        {
            if (ScreenshotFilter != ScreenshotFilter.All && !IsInFilter(screenshot))
                continue;

            Screenshots.Add(screenshot);
        }
    }

    /// <summary>
    /// Returns whether the given screenshot is inside the current date filter.
    /// </summary>
    /// <param name="screenshot">The screenshot to check.</param>
    /// <returns>Whether the screenshot is shown.</returns>
    private bool IsInFilter(ScreenshotInfo screenshot)
    {
        var takenAt = screenshot.TakenAt.Date;
        var today = DateTime.Today;

        return ScreenshotFilter switch
        {
            ScreenshotFilter.Today => takenAt == today,
            ScreenshotFilter.ThisWeek => takenAt >= today.AddDays(-(int)today.DayOfWeek),
            ScreenshotFilter.ThisMonth => takenAt.Year == today.Year && takenAt.Month == today.Month,
            _ => true
        };
    }

    /// <summary>
    /// Deletes the given screenshot of the selected instance.
    /// </summary>
    /// <param name="screenshot">The screenshot to delete.</param>
    /// <returns>Whether the screenshot was deleted.</returns>
    public bool DeleteScreenshot(ScreenshotInfo? screenshot)
    {
        if (SelectedInstance == null || screenshot is null)
            return false;

        var manager = new ScreenshotManager(SelectedInstance.GameDirectory);
        if (!manager.Delete(screenshot.Path))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.562_"));
            return false;
        }

        ScanScreenshots();
        SetStatus($"已删除截图 {screenshot.FileName}");
        return true;
    }

    /// <summary>
    /// Copies the given screenshot into the pictures directory of the user.
    /// </summary>
    /// <param name="screenshot">The screenshot to copy.</param>
    /// <returns>Whether the screenshot was copied.</returns>
    public bool CopyScreenshot(ScreenshotInfo? screenshot)
    {
        if (SelectedInstance == null || screenshot is null)
            return false;

        var manager = new ScreenshotManager(SelectedInstance.GameDirectory);
        var target = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.MyPictures), "HMCL.NET", "Screenshots");

        var copied = manager.Copy(screenshot.Path, target);
        if (copied is null)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.564_"));
            return false;
        }

        SetStatus($"截图已复制到 {copied}");
        return true;
    }

    /// <summary>
    /// Uses the given screenshot as the background image of the launcher.
    /// </summary>
    /// <param name="screenshot">The screenshot used as the background.</param>
    /// <returns>Whether the background was changed.</returns>
    public bool SetScreenshotAsBackground(ScreenshotInfo? screenshot)
    {
        if (screenshot is null)
            return false;

        _settingsManager.Update(settings => settings.BackgroundImagePath = screenshot.Path);
        Settings.BackgroundImagePath = screenshot.Path;
        Settings.SaveSettings();
        ApplyBackgroundImage();
        SetStatus(Lang.Get("hmcl.mainwindowviewmodel.566_"));
        return true;
    }

    /// <summary>
    /// Returns the directory holding the screenshots of the selected instance.
    /// </summary>
    /// <returns>The directory of the screenshots.</returns>
    public string ScreenshotDirectory
        => SelectedInstance is { } instance
            ? new ScreenshotManager(instance.GameDirectory).ScreenshotDirectory
            : string.Empty;

    /// <summary>
    /// The blueprints of the selected instance.
    /// </summary>
    public ObservableCollection<BlueprintInfo> Blueprints { get; } = new();

    /// <summary>
    /// The blueprint selected in the blueprint list.
    /// </summary>
    public BlueprintInfo? SelectedBlueprint
    {
        get => _selectedBlueprint;
        set => Set(ref _selectedBlueprint, value);
    }

    private BlueprintInfo? _selectedBlueprint;

    /// <summary>
    /// Scans the blueprints directory of the selected instance.
    /// </summary>
    public void ScanBlueprints()
    {
        Blueprints.Clear();

        if (SelectedInstance == null)
            return;

        var manager = new BlueprintManager(SelectedInstance.GameDirectory);
        foreach (var blueprint in manager.GetBlueprints())
            Blueprints.Add(blueprint);
    }

    /// <summary>
    /// Imports a blueprint into the selected instance.
    /// </summary>
    /// <param name="sourcePath">The path of the imported file.</param>
    /// <returns>Whether the blueprint was imported.</returns>
    public bool ImportBlueprint(string sourcePath)
    {
        if (SelectedInstance == null)
            return false;

        var manager = new BlueprintManager(SelectedInstance.GameDirectory);
        var blueprint = manager.Import(sourcePath);

        if (blueprint is null)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.568_"));
            return false;
        }

        ScanBlueprints();
        SetStatus($"已导入蓝图 {blueprint.FileName}");
        return true;
    }

    /// <summary>
    /// Deletes the selected blueprint.
    /// </summary>
    /// <returns>Whether the blueprint was deleted.</returns>
    public bool RemoveSelectedBlueprint()
    {
        if (SelectedInstance == null || SelectedBlueprint == null)
            return false;

        var manager = new BlueprintManager(SelectedInstance.GameDirectory);
        if (!manager.Remove(SelectedBlueprint))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.570_"));
            return false;
        }

        ScanBlueprints();
        SetStatus(Lang.Get("hmcl.mainwindowviewmodel.572_"));
        return true;
    }

    /// <summary>
    /// Raised when the background image of the launcher changed.
    /// </summary>
    public event Action? BackgroundImageChanged;

    /// <summary>
    /// Applies the background image of the launcher.
    /// </summary>
    private void ApplyBackgroundImage() => BackgroundImageChanged?.Invoke();

    /// <summary>
    /// Whether there is a selected account and instance ready to launch.
    /// </summary>
    public bool CanLaunch => SelectedAccount != null && SelectedInstance != null;

    /// <summary>
    /// The description of the hardware of the machine, shown on the launch page.
    /// </summary>
    public string HardwareDescription
    {
        get => _hardwareDescription;
        private set => Set(ref _hardwareDescription, value);
    }

    private string _hardwareDescription = Lang.Get("hmcl.mainwindowviewmodel.574_", "正在读取硬件信息...");

    /// <summary>
    /// Reads the hardware of the machine in the background.
    /// </summary>
    /// <returns>A task that completes when the hardware is read.</returns>
    private async Task LoadHardwareInfoAsync()
    {
        try
        {
            var info = await Task.Run(HardwareInfo.Detect);
            HardwareDescription = info.Description;
        }
        catch (Exception)
        {
            HardwareDescription = Lang.Get("hmcl.mainwindowviewmodel.576_");
        }
    }

    /// <summary>
    /// The message shown by the hint pane of the launcher.
    /// </summary>
    public HintInfo? Hint    {
        get => _hint;
        private set => Set(ref _hint, value);
    }

    private HintInfo? _hint;

    /// <summary>
    /// Shows a message in the hint pane of the launcher.
    /// </summary>
    /// <param name="type">The kind of the message.</param>
    /// <param name="message">The text of the message.</param>
    public void ShowHint(HintType type, string message)
    {
        Hint = new HintInfo(type, message);
        SetStatus(message);
    }

    /// <summary>
    /// Hides the message of the hint pane.
    /// </summary>
    public void ClearHint() => Hint = null;

    /// <summary>
    /// The message shown by the toast of the launcher.
    /// </summary>
    public string? ToastMessage
    {
        get => _toastMessage;
        private set => Set(ref _toastMessage, value);
    }

    private string? _toastMessage;
    private DispatcherTimer? _toastTimer;

    /// <summary>
    /// Shows a transient message at the corner of the window.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    public void ShowToast(string message)
    {
        if (string.IsNullOrWhiteSpace(message))
            return;

        ToastMessage = message;

        _toastTimer?.Stop();
        _toastTimer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(4) };
        _toastTimer.Tick += (_, _) =>
        {
            _toastTimer.Stop();
            ToastMessage = null;
        };

        _toastTimer.Start();
    }

    /// <summary>
    /// The settings view model used by the settings page.
    /// </summary>
    public SettingsViewModel Settings { get; }

    /// <summary>
    /// The account manager exposed for account-related dialogs.
    /// </summary>
    public AccountManager AccountService => _accountManager;

    /// <summary>
    /// The settings manager exposed for read-only settings access.
    /// </summary>
    public SettingsManager SettingsService => _settingsManager;

    /// <summary>
    /// Returns the presets of the game settings of the launcher.
    /// </summary>
    /// <returns>The presets, the default one first.</returns>
    public IReadOnlyList<GameSettingsPreset> GetGameSettingsPresets()
    {
        _settingsManager.EnsureGameSettingsPresets();

        var presets = _settingsManager.Settings.GameSettingsPresets;
        var defaultId = _settingsManager.Settings.DefaultGameSettingsPresetId;

        return presets
            .OrderByDescending(preset => preset.Id == defaultId)
            .ToList();
    }

    /// <summary>
    /// The version download view model used by the download page.
    /// </summary>
    public DownloadViewModel Download { get; }

    /// <summary>
    /// The task manager tracking the running launcher tasks.
    /// </summary>
    public TaskManager TaskManager { get; } = new();

    /// <summary>
    /// The view model backing the task list page and the sidebar task indicator.
    /// </summary>
    public TaskListViewModel TaskList { get; }

    /// <summary>
    /// Command that launches the selected game instance with the selected account.
    /// </summary>
    public RelayCommand LaunchCommand { get; }

    /// <summary>
    /// Command that reloads the list of the game instances, like the refresh
    /// button of the instance page of Java HMCL.
    /// </summary>
    public RelayCommand ReloadInstancesCommand { get; }

    /// <summary>
    /// Command that exports the selected game instance as a modpack zip.
    /// </summary>
    public RelayCommand ExportModpackCommand { get; }

    /// <summary>
    /// Creates a new MainWindowViewModel.
    /// </summary>
    /// <param name="accountManager">The account manager backing the account list.</param>
    /// <param name="settingsManager">The settings manager supplying launch options.</param>
    /// <param name="javaManager">The Java manager used to resolve the runtime.</param>
    /// <param name="launchExecutor">The executor performing the actual game launch.</param>
    /// <param name="gameDirectoryManager">The manager owning the game directories and their repositories.</param>
    public MainWindowViewModel(AccountManager accountManager, SettingsManager settingsManager,
        JavaManager javaManager, GameLauncherExecutor launchExecutor, GameDirectoryManager gameDirectoryManager)
    {
        _accountManager = accountManager ?? throw new ArgumentNullException(nameof(accountManager));
        _settingsManager = settingsManager ?? throw new ArgumentNullException(nameof(settingsManager));
        _javaManager = javaManager ?? throw new ArgumentNullException(nameof(javaManager));
        _launchExecutor = launchExecutor ?? throw new ArgumentNullException(nameof(launchExecutor));
        _gameDirectoryManager = gameDirectoryManager ?? throw new ArgumentNullException(nameof(gameDirectoryManager));

        LaunchCommand = new RelayCommand(LaunchAsync, () => CanLaunch);
        ReloadInstancesCommand = new RelayCommand(LoadInstances);
        StopGameCommand = new RelayCommand(StopGame);
        DismissUpdateBubbleCommand = new RelayCommand(() => ShowUpdateBubble = false);
        OpenUpdatePageCommand = new RelayCommand(OpenUpdatePage);
        DownloadUpdateCommand = new RelayCommand(DownloadUpdateAsync);
        ExportModpackCommand = new RelayCommand(ExportModpackAsync);
        RefreshTerracottaCommand = new RelayCommand(() => _ = RefreshTerracottaAsync());
        ScanTerracottaCommand = new RelayCommand(() => _ = ScanTerracottaAsync());
        JoinTerracottaCommand = new RelayCommand(() => _ = JoinTerracottaAsync());
        InstallTerracottaCommand = new RelayCommand(() => _ = InstallTerracottaAsync());
        CopyTerracottaRoomCodeCommand = new RelayCommand(CopyTerracottaRoomCode);

        // The component of the online mode is installed in the directory of the
        // dependencies of the launcher.
        TerracottaManager.DependenciesDirectory = Repository.Layout.BaseDirectory;

        if (TerracottaMetadata.Current is { } metadata)
        {
            foreach (var link in metadata.GetLinks())
                TerracottaLinks.Add(link);
        }
        Settings = new SettingsViewModel(settingsManager, javaManager, _analytics)
        {
            BaseDirectory = Repository.Layout.BaseDirectory
        };
        Download = new DownloadViewModel(this, Path.Combine(Repository.Layout.BaseDirectory, "cache"));
        TaskList = new TaskListViewModel(TaskManager);

        // Publish the known game directories and the current selection.
        foreach (var directory in _gameDirectoryManager.Directories)
            GameDirectories.Add(directory);
        _selectedGameDirectory = _gameDirectoryManager.Selected;

        // Load accounts from manager
        foreach (var account in _accountManager.Accounts)
            Accounts.Add(account);

        if (_accountManager.HasSelectedAccount && _accountManager.SelectedAccount != null)
            SelectedAccount = _accountManager.SelectedAccount;

        // Restore persisted accounts, then keep the file in sync on every change.
        LoadPersistedAccounts();
        SyncAccounts();
        SyncAuthServers();
        _accountManager.AccountsChanged += () =>
            AccountPersistence.Save(_accountsFile, _accountManager.Accounts,
                _accountManager.SelectedAccount?.Username);

        // Apply persisted proxy settings so this session routes downloads through it.
        ProxyApplier.Apply(settingsManager.Settings.ProxyConfig);

        // The launcher writes its log next to the other data of the launcher.
        LauncherLog.Enable(Path.Combine(Repository.Layout.BaseDirectory, "hmcl.log"));

        // The font of the log window follows the settings of the user.
        LogFontFamily = settingsManager.Settings.LogFontFamily ?? string.Empty;
        LogFontSize = settingsManager.Settings.LogFontSize > 0 ? settingsManager.Settings.LogFontSize : 12;

        // The number of the parallel downloads follows the settings of the user.
        DownloadConcurrency.Apply(
            settingsManager.Settings.AutoDownloadThreads,
            settingsManager.Settings.DownloadThreads);

        // Best-effort update check drives the launch page bubble.
        _ = CheckForUpdatesAsync();

        // The announcements of the launcher are shown on the launch page.
        _ = LoadAnnouncementsAsync();

        // The anonymous usage statistics are only sent when the user enables
        // them and the launcher is configured to send them.
        InitializeAnalytics();

        // The hardware of the machine is shown on the launch page.
        _ = LoadHardwareInfoAsync();

        // First-run guidance: if no Java runtime is configured, point the user
        // to Settings before they try launching.
        if (string.IsNullOrWhiteSpace(_settingsManager.Settings.JavaPath) &&
            _javaManager.PreferredRuntime == null)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.578_");
        }
    }

    /// <summary>
    /// Path of the persisted account file inside the repository's .hmcl directory.
    /// </summary>
    private string _accountsFile = "accounts.json";

    /// <summary>
    /// Restores accounts saved by previous launcher sessions into the account manager.
    /// </summary>
    private void LoadPersistedAccounts()
    {
        try
        {
            _accountsFile = Path.Combine(Repository.Layout.BaseDirectory, ".hmcl", "accounts.json");
            var (accounts, selectedUsername) = AccountPersistence.Load(_accountsFile);

            foreach (var account in accounts)
                _accountManager.AddAccount(account);

            var selected = accounts.Find(a =>
                string.Equals(a.Username, selectedUsername, StringComparison.OrdinalIgnoreCase));
            if (selected != null)
                _accountManager.SelectAccount(selected);
        }
        catch (Exception)
        {
            // Persistence loading is best-effort; start with an empty account list on failure.
        }
    }

    /// <summary>
    /// Validates the selected account before launch, refreshing Microsoft/Yggdrasil
    /// tokens through their services when possible. Failure to refresh does not
    /// block the launch: the existing credentials are used instead.
    /// </summary>
    private async Task EnsureAccountReadyAsync()
    {
        if (SelectedAccount is not OAuthAccount account)
            return;

        // Refresh Microsoft tokens when a refresh token is available.
        if (account is HMCLCore.NET.Auth.Microsoft.MicrosoftAccount ms)
        {
            if (string.IsNullOrEmpty(ms.RefreshToken))
                return;

            var service = new MicrosoftAuthService();
            var tokens = await service.RefreshAsync(ms.RefreshToken);
            if (tokens != null && !string.IsNullOrEmpty(tokens.AccessToken))
            {
                ms.SetTokens(tokens.AccessToken, tokens.RefreshToken, ms.ClientToken);
            }

            return;
        }

        // Refresh Yggdrasil tokens against the account's own auth server.
        if (account is HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount ygg &&
            !string.IsNullOrEmpty(ygg.ServerUrl) &&
            !string.IsNullOrEmpty(ygg.YggdrasilAccessToken))
        {
            var service = new HMCLCore.NET.Auth.Yggdrasil.YggdrasilAuthService();
            var response = await service.RefreshAsync(
                ygg.ServerUrl,
                ygg.YggdrasilAccessToken,
                ygg.YggdrasilClientToken ?? string.Empty);

            if (!string.IsNullOrEmpty(response?.AccessToken))
                ygg.SetYggdrasilTokens(response.AccessToken, response.ClientToken ?? ygg.YggdrasilClientToken ?? string.Empty);
        }
    }

    /// <summary>
    /// Notifies listeners that launch availability may have changed.
    /// </summary>
    private void OnLaunchStateChanged()
    {
        OnPropertyChanged(nameof(CanLaunch));
        LaunchCommand.RaiseCanExecuteChanged();
    }

    /// <summary>
    /// Adds an account to the manager and UI.
    /// </summary>
    public void AddAccount(Account account)
    {
        _accountManager.AddAccount(account);
        Accounts.Add(account);
    }

    /// <summary>
    /// Reloads the account list from the account manager while preserving the current selection.
    /// </summary>
    public void SyncAccounts()
    {
        Accounts.Clear();
        foreach (var account in _accountManager.Accounts)
            Accounts.Add(account);

        if (_accountManager.HasSelectedAccount && _accountManager.SelectedAccount is { } selected)
            SelectedAccount = selected;
    }

    /// <summary>
    /// Sets the skin of the given account.
    /// </summary>
    /// <remarks>
    /// The skin of a Mojang account is uploaded to the profile of the player;
    /// the skin of an offline account is kept in a local file.
    /// </remarks>
    /// <param name="account">The account whose skin is changed.</param>
    /// <param name="filePath">The path of the PNG file holding the skin.</param>
    /// <returns>Whether the skin was changed.</returns>
    public async Task<bool> SetAccountSkinAsync(Account account, string filePath)
    {
        if (account is null || string.IsNullOrWhiteSpace(filePath) || !File.Exists(filePath))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.580_"));
            return false;
        }

        switch (account)
        {
            case HMCLCore.NET.Auth.Microsoft.MicrosoftAccount microsoft when !string.IsNullOrEmpty(microsoft.AccessToken):
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.582_");
                var service = new HMCLCore.NET.Auth.Textures.SkinService();
                var uploaded = await service.UploadSkinAsync(microsoft.AccessToken!, filePath);
                SetStatus(uploaded ? Lang.Get("hmcl.mainwindowviewmodel.584_") : Lang.Get("hmcl.mainwindowviewmodel.account_skin_upload_failed"));
                return uploaded;
            }

            case OfflineAccount offline:
            {
                // The skin of an offline account is stored in the launcher
                // directory, so that it survives the removal of the original
                // file.
                var directory = Path.Combine(Repository.Layout.BaseDirectory, "skins", "accounts");
                Directory.CreateDirectory(directory);

                var target = Path.Combine(directory, $"{offline.Username}.png");
                File.Copy(filePath, target, true);

                offline.SkinPath = target;
                OnPropertyChanged(nameof(Accounts));
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.588_"));
                return true;
            }

            default:
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.590_"));
                return false;
        }
    }

    /// <summary>
    /// Removes the skin of the given account.
    /// </summary>
    /// <param name="account">The account whose skin is removed.</param>
    /// <returns>Whether the skin was removed.</returns>
    public async Task<bool> ResetAccountSkinAsync(Account account)
    {
        switch (account)
        {
            case HMCLCore.NET.Auth.Microsoft.MicrosoftAccount microsoft when !string.IsNullOrEmpty(microsoft.AccessToken):
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.592_");
                var service = new HMCLCore.NET.Auth.Textures.SkinService();
                var skins = await service.GetSkinsAsync(microsoft.AccessToken!);
                var active = skins.FirstOrDefault(skin => skin.IsActive);
                if (active is null)
                {
                    SetStatus(Lang.Get("hmcl.mainwindowviewmodel.594_"));
                    return false;
                }

                var reset = await service.ResetSkinAsync(microsoft.AccessToken!, active.Id);
                SetStatus(reset ? Lang.Get("hmcl.mainwindowviewmodel.596_") : Lang.Get("hmcl.mainwindowviewmodel.598_"));
                return reset;
            }

            case OfflineAccount offline:
            {
                offline.SkinPath = null;
                OnPropertyChanged(nameof(Accounts));
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.600_"));
                return true;
            }

            default:
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.602_"));
                return false;
        }
    }

    /// <summary>
    /// Adds a game instance to the UI.
    /// </summary>
    public void AddGameInstance(GameInstance instance)
    {
        GameInstances.Add(instance);
    }

    /// <summary>
    /// Reloads the instance list from the game repository.
    /// </summary>
    public void LoadInstances()
    {
        GameInstances.Clear();
        Repository.Refresh();
        foreach (var instance in Repository.Snapshot.GetAllInstances())
            GameInstances.Add(instance);

        ApplyInstanceFilter();
    }

    /// <summary>
    /// The game directories known to the launcher.
    /// </summary>
    public ObservableCollection<GameDirectory> GameDirectories { get; } = new();

    /// <summary>
    /// The currently selected game directory.
    /// </summary>
    public GameDirectory? SelectedGameDirectory
    {
        get => _selectedGameDirectory;
        set
        {
            if (!Set(ref _selectedGameDirectory, value) || value == null)
                return;

            _gameDirectoryManager.Select(value);
            PersistGameDirectories();
            LoadInstances();
            StatusMessage = $"已切换到游戏目录 {value.DisplayName}";
        }
    }

    private GameDirectory? _selectedGameDirectory;

    /// <summary>
    /// Adds a game directory with the given path and custom name.
    /// </summary>
    /// <param name="path">The root path of the game directory.</param>
    /// <param name="name">The optional custom display name.</param>
    /// <param name="useRelativePath">
    /// Whether the path is relative to the directory of the launcher.
    /// </param>
    /// <returns>Whether the directory was added.</returns>
    public bool AddGameDirectory(string path, string? name, bool useRelativePath = false)
    {
        if (string.IsNullOrWhiteSpace(path))
            return false;

        try
        {
            var directory = _gameDirectoryManager.Add(path, name);
            directory.UseRelativePath = useRelativePath;
            HMCLCore.NET.Setting.GameDirectoryManager.EnsureLayout(directory, BaseDirectory);
            GameDirectories.Add(directory);
            PersistGameDirectories();
            StatusMessage = $"已添加游戏目录 {directory.DisplayName}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"添加游戏目录失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Updates the name and path of a game directory.
    /// </summary>
    /// <param name="directory">The game directory to update.</param>
    /// <param name="name">The new custom display name, or null to clear it.</param>
    /// <param name="path">The new root path.</param>
    /// <param name="useRelativePath">
    /// Whether the path is relative to the directory of the launcher.
    /// </param>
    /// <returns>Whether the directory was updated.</returns>
    public bool UpdateGameDirectory(GameDirectory directory, string? name, string path, bool useRelativePath = false)
    {
        if (string.IsNullOrWhiteSpace(path))
            return false;

        if (!_gameDirectoryManager.Update(directory, name, path))
            return false;

        directory.UseRelativePath = useRelativePath;
        HMCLCore.NET.Setting.GameDirectoryManager.EnsureLayout(directory, BaseDirectory);
        PersistGameDirectories();
        StatusMessage = $"已更新游戏目录 {directory.DisplayName}";
        return true;
    }

    /// <summary>
    /// Removes a game directory. The last remaining directory cannot be removed.
    /// </summary>
    /// <param name="directory">The game directory to remove.</param>
    /// <returns>Whether the directory was removed.</returns>
    public bool RemoveGameDirectory(GameDirectory directory)
    {
        if (!_gameDirectoryManager.Remove(directory))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.604_");
            return false;
        }

        GameDirectories.Remove(directory);
        _selectedGameDirectory = null;
        SelectedGameDirectory = _gameDirectoryManager.Selected;
        PersistGameDirectories();
        StatusMessage = $"已删除游戏目录 {directory.DisplayName}";
        return true;
    }

    /// <summary>
    /// Writes the game directory list and the current selection to the settings.
    /// </summary>
    private void PersistGameDirectories()
    {
        var settings = _settingsManager.Settings;
        settings.GameDirectories = GameDirectories.ToList();
        settings.SelectedGameDirectoryId = _gameDirectoryManager.Selected.Id;
        _settingsManager.Save();
    }

    /// <summary>
    /// Verifies the selected instance's files against recorded hashes and
    /// re-downloads anything missing or corrupted.
    /// </summary>
    public async Task VerifyAndRepairSelectedInstanceAsync()
    {
        if (SelectedInstance is not { } instance)
            return;

        var manifest = LoadInstanceManifest(instance);
        if (manifest == null)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.606_");
            return;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.608_");
        var report = await GameIntegrityChecker.VerifyAsync(manifest, BaseDirectory);

        if (report.IsHealthy)
        {
            StatusMessage = report.Summary;
            return;
        }

        StatusMessage = $"{report.Summary}，正在修复...";
        var repaired = await GameIntegrityChecker.RepairAsync(report);
        StatusMessage = $"{report.Summary}；已修复 {repaired}/{report.Issues.Count} 个文件";
    }

    /// <summary>
    /// Stores an image as the custom icon of the selected instance.
    /// </summary>
    /// <param name="imagePath">The image to use as the icon.</param>
    /// <returns>Whether the icon was stored.</returns>
    public bool SetSelectedInstanceIcon(string imagePath)
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.610_"));
            return false;
        }

        if (!GameInstanceIcons.SetCustomIcon(instance.GameDirectory, imagePath))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.612_"));
            return false;
        }

        LoadInstances();
        SetStatus($"已为实例 {instance.Id} 设置图标");
        return true;
    }

    /// <summary>
    /// Removes the custom icon of the selected instance.
    /// </summary>
    /// <returns>Whether an icon was removed.</returns>
    public bool RemoveSelectedInstanceIcon()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.614_"));
            return false;
        }

        if (!GameInstanceIcons.RemoveCustomIcon(instance.GameDirectory))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.616_"));
            return false;
        }

        LoadInstances();
        SetStatus($"已移除实例 {instance.Id} 的图标");
        return true;
    }

    /// <summary>
    /// Persists the current time as the instance's last-played timestamp
    /// without disturbing the launch status message.
    /// </summary>
    /// <param name="instance">The launched instance.</param>
    /// <param name="manifest">The manifest to update.</param>
    private void StampLastPlayed(GameInstance instance, GameInstanceManifest manifest)
    {
        try
        {
            manifest.LastPlayed = DateTime.UtcNow;
            var root = Repository.GetInstanceRoot(new GameInstanceID { Value = instance.Id });
            Directory.CreateDirectory(root);
            File.WriteAllText(Path.Combine(root, "instance.json"),
                LauncherJson.SerializeManifest(manifest));
            LoadInstances();
        }
        catch
        {
            // Best effort: failing to persist the timestamp must not break launching.
        }
    }

    /// <summary>
    /// Reads an instance manifest from disk without changing the UI status.
    /// </summary>
    /// <param name="instanceId">The instance identifier.</param>
    /// <returns>The manifest, or null when it cannot be read.</returns>
    public GameInstanceManifest? LoadInstanceManifest(string instanceId)
    {
        try
        {
            var root = Repository.GetInstanceRoot(new GameInstanceID { Value = instanceId });
            var path = Path.Combine(root, "instance.json");
            if (!File.Exists(path))
                return null;

            return LauncherJson.DeserializeManifest(File.ReadAllText(path));
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Writes an instance manifest and refreshes the instance list without
    /// changing the status message.
    /// </summary>
    /// <param name="instanceId">The instance identifier.</param>
    /// <param name="manifest">The manifest to persist.</param>
    /// <returns>Whether the manifest was written.</returns>
    public bool SaveInstanceManifest(string instanceId, GameInstanceManifest manifest)
    {
        try
        {
            var root = Repository.GetInstanceRoot(new GameInstanceID { Value = instanceId });
            Directory.CreateDirectory(root);
            File.WriteAllText(Path.Combine(root, "instance.json"),
                LauncherJson.SerializeManifest(manifest));
            LoadInstances();
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"保存实例清单失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Creates a new empty game instance directory with an instance manifest.
    /// </summary>
    /// <param name="instanceId">The unique identifier of the new instance.</param>
    /// <param name="version">The Minecraft version the instance targets.</param>
    /// <returns>Whether the instance was created.</returns>
    public bool CreateInstance(string instanceId, string version)
    {
        if (string.IsNullOrWhiteSpace(instanceId) || string.IsNullOrWhiteSpace(version))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.618_"));
            return false;
        }

        var manifest = new GameInstanceManifest
        {
            Version = version,
            MainClass = "net.minecraft.client.main.Main"
        };

        return SaveInstance(instanceId, manifest);
    }

    /// <summary>
    /// Persists an instance manifest to the repository and refreshes the instance list.
    /// Creates the instance directory when missing and overwrites the existing manifest.
    /// </summary>
    /// <param name="instanceId">The unique identifier of the instance.</param>
    /// <param name="manifest">The manifest describing the instance.</param>
    /// <returns>Whether the manifest was written successfully.</returns>
    public bool SaveInstance(string instanceId, GameInstanceManifest manifest)
    {
        if (string.IsNullOrWhiteSpace(instanceId))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.620_"));
            return false;
        }

        try
        {
            var root = Repository.GetInstanceRoot(new GameInstanceID { Value = instanceId });

            Directory.CreateDirectory(root);
            File.WriteAllText(Path.Combine(root, "instance.json"),
                LauncherJson.SerializeManifest(manifest));

            LoadInstances();
            SetStatus($"实例 {instanceId} ({manifest.Version}) 已创建");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"保存实例失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Loads the instance manifest from the instance directory, if it exists.
    /// </summary>
    /// <param name="instance">The instance whose manifest to load.</param>
    /// <returns>The manifest, or null when it cannot be read.</returns>
    public GameInstanceManifest? LoadInstanceManifest(GameInstance instance)
    {
        try
        {
            var manifestPath = Path.Combine(instance.GameDirectory, "instance.json");
            if (!File.Exists(manifestPath))
                return null;

            var json = File.ReadAllText(manifestPath);
            return LauncherJson.DeserializeManifest(json);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Updates the selected instance to another version of the game: the
    /// manifest of the instance is replaced by the manifest of the new version
    /// and the files it needs are downloaded.
    /// </summary>
    /// <param name="versionId">The identifier of the new version.</param>
    /// <returns>Whether the instance was updated.</returns>
    public async Task<bool> UpdateInstanceVersionAsync(string versionId)
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.622_"));
            return false;
        }

        if (string.IsNullOrWhiteSpace(versionId))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.624_"));
            return false;
        }

        StatusMessage = $"正在更新实例 {instance.Id} 到 {versionId}...";

        try
        {
            // Route requests through the mirror configured in the settings.
            DownloadSourceUtils.Apply(_settingsManager.Settings);

            // The version must be known by the remote manifest.
            var version = Download.Versions.FirstOrDefault(v =>
                string.Equals(v.Id, versionId, StringComparison.OrdinalIgnoreCase));

            if (version is null)
            {
                SetStatus($"未找到版本 {versionId}");
                return false;
            }

            var baseDirectory = Repository.Layout.BaseDirectory;
            var jsonPath = Path.Combine(baseDirectory, "cache", "vjson", $"{version.Id}.json");

            var downloadTask = new VersionJsonDownloadTask(new Uri(version.Url), jsonPath);
            await TaskManager.RunAsync(downloadTask);

            var json = File.ReadAllText(jsonPath);

            // Keep a repository-standard copy so that the loaders and the
            // inheritance resolution can find the parent version.
            var versionDirectory = Path.Combine(baseDirectory, "versions", version.Id);
            Directory.CreateDirectory(versionDirectory);
            File.WriteAllText(Path.Combine(versionDirectory, $"{version.Id}.json"), json);

            var manifest = VersionJsonParser.Parse(json, LoadVersionJson)
                          ?? throw new InvalidDataException(Lang.Get("hmcl.mainwindowviewmodel.626_"));

            // The settings of the instance are kept when it is updated.
            var previous = LoadInstanceManifest(instance);
            if (previous != null)
                manifest.GameSettings = previous.GameSettings;

            if (!SaveInstance(instance.Id, manifest))
                return false;

            // The files needed by the new version are downloaded.
            var classpath = await PrepareLaunchDependenciesAsync(manifest);
            StatusMessage = classpath.Length > 0
                ? $"实例 {instance.Id} 已更新到 {versionId}"
                : $"实例 {instance.Id} 已更新到 {versionId}";

            LoadInstances();
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"实例更新失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Deletes the given game instance from disk.
    /// </summary>
    public void RemoveInstance(string instanceId)
    {
        try
        {
            Repository.DeleteInstance(new GameInstanceID { Value = instanceId });
            if (SelectedInstance?.Id == instanceId)
                SelectedInstance = null;
            LoadInstances();
            SetStatus($"实例 {instanceId} 已删除");
        }
        catch (Exception ex)
        {
            SetStatus($"删除实例失败: {ex.Message}");
        }
    }

    /// <summary>
    /// Sets the status message.
    /// </summary>
    public void SetStatus(string message)
    {
        StatusMessage = message;
        RecordLauncherLine(message);
    }

    /// <summary>
    /// Appends a line to the launcher log, keeping the log bounded.
    /// </summary>
    /// <param name="line">The line to record.</param>
    private void RecordLauncherLine(string line)
    {
        LauncherLines.Add($"[{DateTime.Now:HH:mm:ss}] {line}");
        while (LauncherLines.Count > MaxGameLogLines)
            LauncherLines.RemoveAt(0);
    }

    /// <summary>
    /// Renames the selected instance to a sanitized, unique instance id.
    /// </summary>
    /// <param name="newName">The requested instance name.</param>
    /// <returns>Whether the rename succeeded.</returns>
    public bool RenameSelectedInstance(string newName)
    {
        if (SelectedInstance is not { } instance || string.IsNullOrWhiteSpace(newName))
            return false;

        var toId = DefaultGameRepository.SanitizeInstanceName(newName);
        if (string.IsNullOrEmpty(toId))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.628_"));
            return false;
        }

        if (toId == instance.Id)
            return true;

        if (GameInstances.Any(i => i.Id == toId))
        {
            SetStatus($"实例 {toId} 已存在");
            return false;
        }

        if (!Repository.RenameInstance(
                new GameInstanceID { Value = instance.Id },
                new GameInstanceID { Value = toId }))
        {
            SetStatus($"重命名实例 {instance.Id} 失败");
            return false;
        }

        SetStatus($"实例 {instance.Id} 已重命名为 {toId}");
        LoadInstances();

        // Keep the renamed instance selected.
        SelectedInstance = GameInstances.FirstOrDefault(i => i.Id == toId);
        return true;
    }

    /// <summary>
    /// Launches the selected game instance with the selected account.
    /// </summary>
    public async Task LaunchAsync()
    {
        if (SelectedAccount == null || SelectedInstance == null)
            return;

        IsLaunching = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.630_");

        try
        {
            var instance = SelectedInstance!;

            var config = await BuildLaunchConfigAsync(instance);
            if (config is null)
                return;

            // Run the real launch via the executor; the game process is managed by the core library.
            ManagedProcess gameProcess = await _launchExecutor.LaunchAsync(config);

            // Capture the new process and stream its output into the game log.
            GameLines.Clear();
            AttachGameProcess(gameProcess, instance.GameDirectory);
            StampLastPlayed(instance, LoadInstanceManifest(instance) ?? new GameInstanceManifest
            {
                Version = instance.Version,
                MainClass = "net.minecraft.client.main.Main"
            });

            // The debug mode of the launcher adds the diagnostic information of
            // the launch to the game log.
            if (_settingsManager.Settings.Debug)
                AppendLaunchDiagnostics(config);

            // The command of the game is always written into the launcher log.
            LauncherLog.Info($"启动命令: {_launchExecutor.BuildLaunchCommand(config)}");

            LauncherLog.Info($"游戏 {instance.Id} 已启动 (PID {gameProcess.Id})");
            StatusMessage = $"游戏 {instance.Id} 已启动 (PID {gameProcess.Id})";

            TrackUsage("game_launch");
            EventBus.Instance.Publish(new GameLaunchEvent(instance.Version, LaunchState.Running));

            // The launcher hides itself or closes when the game starts, as the
            // visibility chosen in the settings says.
            ApplyLauncherVisibility();

            // Open the dedicated log window, like Java HMCL's LogWindow; the
            // user may turn it off in the settings.
            if (config.GameSettings?.ShowLogs ?? true)
                GameLogWindowRequested?.Invoke();
        }
        catch (GameLaunchException ex)
        {
            LauncherLog.Error(Lang.Get("hmcl.mainwindowviewmodel.632_"), ex);
            StatusMessage = $"启动失败: {ex.Message}";
        }
        catch (Exception ex)
        {
            LauncherLog.Error(Lang.Get("hmcl.mainwindowviewmodel.634_"), ex);
            StatusMessage = $"启动失败: {ex.Message}";
        }
        finally
        {
            IsLaunching = false;
        }
    }

    /// <summary>
    /// Writes the diagnostic information of a launch into the game log.
    /// </summary>
    /// <param name="config">The configuration used to start the game.</param>
    private void AppendLaunchDiagnostics(LaunchConfig config)
    {
        OnGameOutputLine($"[HMCL] 工作目录: {config.WorkingDirectory}");
        OnGameOutputLine($"[HMCL] Java 运行时: {config.JavaRuntime?.Path} ({config.JavaRuntime?.Version})");
        OnGameOutputLine($"[HMCL] 内存分配: {config.Memory} MB");
        OnGameOutputLine($"[HMCL] 启动命令: {_launchExecutor.BuildLaunchCommand(config)}");
    }

    /// <summary>
    /// Raised when the launcher has to hide itself or to close after the game
    /// was started.
    /// </summary>
    public event Action<LauncherVisibility>? LauncherVisibilityRequested;

    /// <summary>
    /// Raised when the launcher has to be shown again, because the game it
    /// launched has ended.
    /// </summary>
    public event Action? LauncherReopenRequested;

    /// <summary>
    /// The visibility of the launcher chosen in the settings.
    /// </summary>
    private LauncherVisibility LauncherVisibility
        => _settingsManager.Settings.GameSettingsPreset.LauncherVisibility;

    /// <summary>
    /// Applies the visibility of the launcher chosen in the settings when the
    /// game starts.
    /// </summary>
    private void ApplyLauncherVisibility()
    {
        var visibility = LauncherVisibility;
        if (visibility == LauncherVisibility.Keep)
            return;

        LauncherVisibilityRequested?.Invoke(visibility);
    }

    /// <summary>
    /// Shows the launcher again when the game it launched has ended.
    /// </summary>
    private void ReopenLauncher()
    {
        var visibility = LauncherVisibility;
        if (visibility is not (LauncherVisibility.HideAndReopen or LauncherVisibility.CloseAndReopen))
            return;

        LauncherReopenRequested?.Invoke();
    }

    /// <summary>
    /// The latest launched game process, used for the stop-game control.
    /// </summary>
    private ManagedProcess? _gameProcess;

    /// <summary>
    /// Worlds belonging to the currently selected instance.
    /// </summary>
    public ObservableCollection<WorldInfo> Worlds { get; } = new();

    /// <summary>
    /// The currently selected world in the world list.
    /// </summary>
    public WorldInfo? SelectedWorld
    {
        get => _selectedWorld;
        set => Set(ref _selectedWorld, value);
    }

    private WorldInfo? _selectedWorld;

    /// <summary>
    /// Refreshes the world list for the currently selected instance.
    /// </summary>
    public void RefreshSelectedInstanceWorlds()
    {
        Worlds.Clear();
        SelectedWorld = null;

        var instance = SelectedInstance;
        if (instance is null)
            return;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);
        foreach (var world in manager.GetWorlds())
            Worlds.Add(world);
    }

    /// <summary>
    /// Backs up the selected world into the backups directory.
    /// </summary>
    public void BackupSelectedWorld()
    {
        var world = SelectedWorld;
        if (world is null)
            return;

        var instance = SelectedInstance;
        if (instance is null)
            return;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);

        // The world cannot be backed up while the game is using it.
        if (manager.IsWorldLocked(world.DirectoryName))
        {
            StatusMessage = Lang.Get("world.locked.failed", "世界正在被游戏使用，无法操作");
            return;
        }

        var path = manager.BackupWorld(world.DirectoryName);
        if (path != null)
        {
            StatusMessage = $"世界已备份：{Path.GetFileName(path)}";
            RefreshSelectedInstanceWorlds();
        }
        else
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.636_");
        }
    }

    /// <summary>
    /// The backups of the worlds of the selected instance.
    /// </summary>
    public ObservableCollection<WorldBackup> WorldBackups { get; } = new();

    /// <summary>
    /// Reloads the list of the backups of the worlds of the selected instance.
    /// </summary>
    public void RefreshWorldBackups()
    {
        WorldBackups.Clear();

        var instance = SelectedInstance;
        if (instance is null)
            return;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);

        foreach (var backup in manager.GetBackups())
            WorldBackups.Add(backup);
    }

    /// <summary>
    /// Deletes a backup of a world.
    /// </summary>
    /// <param name="backup">The backup deleted.</param>
    /// <returns>Whether the backup was deleted.</returns>
    public bool DeleteWorldBackup(WorldBackup backup)
    {
        if (backup is null)
            return false;

        var instance = SelectedInstance;
        if (instance is null)
            return false;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);

        if (!manager.DeleteBackup(backup.FilePath))
        {
            StatusMessage = "备份删除失败";
            return false;
        }

        RefreshWorldBackups();
        StatusMessage = $"备份已删除：{backup.FileName}";
        return true;
    }

    /// <summary>
    /// Copies the selected world into a new one.
    /// </summary>
    /// <param name="name">The name of the new world.</param>
    /// <returns>Whether the world was copied.</returns>
    public bool DuplicateSelectedWorld(string name)
    {
        var world = SelectedWorld;
        if (world is null || string.IsNullOrWhiteSpace(name))
            return false;

        // The world cannot be copied while the game is using it.
        if (world.InUse)
        {
            StatusMessage = Lang.Get("world.locked.failed", "世界正在被游戏使用，无法操作");
            return false;
        }

        var copy = WorldEditor.Duplicate(world.Path, name);
        if (copy is null)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.642_", "复制世界失败");
            return false;
        }

        StatusMessage = $"世界已复制为 {name}";
        RefreshSelectedInstanceWorlds();
        return true;
    }

    /// <summary>
    /// Deletes the selected world directory.
    /// </summary>
    public void DeleteSelectedWorld()
    {
        var world = SelectedWorld;
        if (world is null)
            return;

        var instance = SelectedInstance;
        if (instance is null)
            return;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);

        // The world cannot be deleted while the game is using it.
        if (manager.IsWorldLocked(world.DirectoryName))
        {
            StatusMessage = Lang.Get("world.locked.failed", "世界正在被游戏使用，无法操作");
            return;
        }

        if (manager.DeleteWorld(world.DirectoryName))
        {
            StatusMessage = $"世界 {world.DisplayName} 已删除";
            RefreshSelectedInstanceWorlds();
        }
        else
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.638_");
        }
    }

    /// <summary>
    /// Exports the selected world into a zip archive at the given path.
    /// </summary>
    /// <param name="destinationPath">The path of the archive to create.</param>
    /// <returns>Whether the world was exported.</returns>
    public bool ExportSelectedWorld(string destinationPath)
    {
        var world = SelectedWorld;
        if (world is null)
            return false;

        var instance = SelectedInstance;
        if (instance is null)
            return false;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);
        if (!manager.ExportWorld(world.DirectoryName, destinationPath))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.640_");
            return false;
        }

        StatusMessage = $"世界已导出：{destinationPath}";
        return true;
    }

    /// <summary>
    /// Imports a world archive into the saves directory of the selected instance.
    /// </summary>
    /// <param name="archivePath">The archive to import.</param>
    /// <returns>Whether the world was imported.</returns>
    public bool ImportWorld(string archivePath)
    {
        var instance = SelectedInstance;
        if (instance is null)
            return false;

        var backupsDir = Path.Combine(BaseDirectory, "backups", "worlds");
        var manager = new WorldManager(instance.GameDirectory, backupsDir);
        var name = manager.ImportWorld(archivePath);
        if (name != null)
        {
            StatusMessage = $"世界 {name} 已导入";
            RefreshSelectedInstanceWorlds();
            return true;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.642_");
        return false;
    }

    /// <summary>
    /// The maximum number of log lines kept in memory.
    /// </summary>
    private const int MaxGameLogLines = 2000;

    /// <summary>
    /// Game processes currently running, used by the running-games panel.
    /// </summary>
    public ObservableCollection<RunningGameInfo> RunningProcesses { get; } = new();

    /// <summary>
    /// Raised after a game process starts so the shell can open its log window,
    /// mirroring Java HMCL's separate LogWindow.
    /// </summary>
    public event Action? GameLogWindowRequested;

    /// <summary>
    /// Whether the launch page shows the "new version available" bubble.
    /// </summary>
    public bool ShowUpdateBubble
    {
        get => _showUpdateBubble;
        private set => Set(ref _showUpdateBubble, value);
    }

    private bool _showUpdateBubble;

    /// <summary>
    /// The text describing the latest available version.
    /// </summary>
    public string LatestVersionText
    {
        get => _latestVersionText;
        private set => Set(ref _latestVersionText, value);
    }

    private string _latestVersionText = string.Empty;

    /// <summary>
    /// Information about the available update, or null if no update is available.
    /// </summary>
    public UpdateInfo? LatestUpdate
    {
        get => _latestUpdate;
        private set => Set(ref _latestUpdate, value);
    }

    private UpdateInfo? _latestUpdate;

    /// <summary>
    /// Hides the update bubble.
    /// </summary>
    public RelayCommand DismissUpdateBubbleCommand { get; }

    /// <summary>
    /// Opens the releases page in the default browser.
    /// </summary>
    public RelayCommand OpenUpdatePageCommand { get; }

    /// <summary>
    /// Downloads and installs the available update, then restarts the application.
    /// </summary>
    public RelayCommand DownloadUpdateCommand { get; }

    /// <summary>
    /// The service sending the anonymous usage statistics of the launcher.
    /// </summary>
    private readonly CountlyService _analytics = new();

    /// <summary>
    /// Configures the service sending the anonymous usage statistics and
    /// starts the session of the launcher.
    /// </summary>
    private void InitializeAnalytics()
    {
        try
        {
            var settings = _settingsManager.Settings;

            // The anonymous identifier of the machine is generated once and
            // then kept in the settings.
            if (string.IsNullOrWhiteSpace(settings.AnalyticsDeviceId))
            {
                settings.AnalyticsDeviceId = Guid.NewGuid().ToString("N");
                _settingsManager.Update(s => s.AnalyticsDeviceId = settings.AnalyticsDeviceId);
            }

            _analytics.Enabled = settings.EnableAnalytics;
            _analytics.ServerUrl = settings.AnalyticsServerUrl;
            _analytics.AppKey = settings.AnalyticsAppKey;
            _analytics.DeviceId = settings.AnalyticsDeviceId;

            if (_analytics.IsConfigured)
                _ = _analytics.TrackSessionStartAsync();
        }
        catch (Exception)
        {
            // The statistics are optional and must never affect the launcher.
        }
    }

    /// <summary>
    /// Sends the use of a feature of the launcher to the statistics service.
    /// </summary>
    /// <param name="key">The name of the feature.</param>
    /// <param name="segmentation">The details of the use of the feature.</param>
    private void TrackUsage(string key, IReadOnlyDictionary<string, string>? segmentation = null)
    {
        if (!_analytics.IsConfigured)
            return;

        _ = _analytics.TrackEventAsync(key, segmentation);
    }

    /// <summary>
    /// Installs a mod loader into the launcher and creates the instance using
    /// it, like the installer page of Java HMCL.
    /// </summary>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="loaderVersion">The version of the loader.</param>
    /// <returns>The identifier of the created instance, or null on failure.</returns>
    public async Task<string?> InstallLoaderAsync(
        string gameVersion, HMCLCore.NET.Download.LoaderKind kind, string loaderVersion)
    {
        var baseDirectory = Repository.Layout.BaseDirectory;
        var javaPath = _javaManager.PreferredRuntime?.Path;
        if (string.IsNullOrEmpty(javaPath))
        {
            SetStatus("未找到 Java 运行时，请先配置 Java");
            return null;
        }

        try
        {
            // The version of the game is downloaded first, so that the loader
            // can be installed on top of it.
            var manifestPath = Path.Combine(baseDirectory, "cache", "vjson", $"{gameVersion}.json");
            if (!File.Exists(manifestPath))
            {
                var versionUrl = Download.Versions
                    .FirstOrDefault(version => string.Equals(version.Id, gameVersion, StringComparison.OrdinalIgnoreCase))?.Url;

                if (versionUrl is null)
                {
                    SetStatus($"未找到版本 {gameVersion}");
                    return null;
                }

                var downloadTask = new VersionJsonDownloadTask(new Uri(versionUrl), manifestPath);
                await TaskManager.RunAsync(downloadTask);
            }

            var versionDirectory = Path.Combine(baseDirectory, "versions", gameVersion);
            Directory.CreateDirectory(versionDirectory);
            File.Copy(manifestPath, Path.Combine(versionDirectory, $"{gameVersion}.json"), true);

            var progress = new Progress<string>(message => LauncherLog.Info(message));

            string? installed;
            switch (kind)
            {
                case HMCLCore.NET.Download.LoaderKind.Forge:
                case HMCLCore.NET.Download.LoaderKind.NeoForge:
                    installed = await HMCLCore.NET.Download.ForgeInstaller.InstallAsync(
                        kind, baseDirectory, javaPath!, gameVersion, loaderVersion, progress);
                    break;

                case HMCLCore.NET.Download.LoaderKind.OptiFine:
                case HMCLCore.NET.Download.LoaderKind.LiteLoader:
                case HMCLCore.NET.Download.LoaderKind.Cleanroom:
                    installed = await HMCLCore.NET.Download.LoaderInstaller.InstallAsync(
                        kind, baseDirectory, javaPath!, gameVersion, loaderVersion, progress);
                    break;

                default:
                    // The loaders using a meta profile are installed by merging
                    // the profile into the version of the game.
                    var vanilla = VersionJsonParser.Parse(
                        File.ReadAllText(manifestPath), LoadVersionJson);

                    if (vanilla is null)
                    {
                        SetStatus($"无法解析版本 {gameVersion}");
                        return null;
                    }

                    var merged = await HMCLCore.NET.Download.MetaLoaderInstaller.CreateManifestAsync(
                        kind, vanilla, loaderVersion);

                    if (merged is null)
                    {
                        SetStatus($"{kind} 安装失败");
                        return null;
                    }

                    installed = $"{gameVersion}-{kind.ToString().ToLowerInvariant()}-{loaderVersion}";
                    var directory = Path.Combine(baseDirectory, "versions", installed);
                    Directory.CreateDirectory(directory);
                    SaveInstance(installed, merged);
                    break;
            }

            if (string.IsNullOrEmpty(installed))
            {
                SetStatus($"{kind} 安装失败");
                return null;
            }

            // The version of the loader is saved as an instance of the launcher.
            var instanceId = $"{gameVersion}-{kind.ToString().ToLowerInvariant()}-{loaderVersion}";
            if (kind is not (HMCLCore.NET.Download.LoaderKind.Fabric
                or HMCLCore.NET.Download.LoaderKind.Quilt
                or HMCLCore.NET.Download.LoaderKind.LegacyFabric))
            {
                SaveInstance(instanceId, new GameInstanceManifest
                {
                    Version = installed,
                    MainClass = "net.minecraft.client.main.Main"
                });
            }

            LoadInstances();
            return instanceId;
        }
        catch (Exception ex)
        {
            SetStatus($"安装 {kind} 失败: {ex.Message}");
            return null;
        }
    }

    /// <summary>
    /// Sends the end of the session of the launcher to the developer, when the
    /// anonymous statistics are enabled.
    /// </summary>
    /// <returns>A task that completes when the request is sent.</returns>
    public async Task EndAnalyticsSessionAsync()
    {
        try
        {
            if (!_analytics.IsConfigured)
                return;

            var duration = (int)(DateTime.Now - _sessionStart).TotalSeconds;
            await _analytics.TrackSessionEndAsync(duration);
        }
        catch (Exception ex)
        {
            // The statistics are optional and must never affect the launcher.
            LauncherLog.Warn($"无法发送统计会话结束: {ex.Message}");
        }
    }

    /// <summary>
    /// The time the session of the launcher started at.
    /// </summary>
    private readonly DateTime _sessionStart = DateTime.Now;

    /// <summary>
    /// Sends the report of a crash of the launcher to the developer, when the
    /// user agreed to it.
    /// </summary>
    /// <param name="report">The report of the crash.</param>
    /// <returns>A task that completes when the report is sent.</returns>
    public async Task ReportCrashAsync(HMCLCore.NET.Util.CrashReport report)
    {
        try
        {
            if (!_analytics.IsConfigured)
                return;

            await _analytics.TrackCrashAsync(report);
        }
        catch (Exception ex)
        {
            // The report is optional and must never affect the launcher.
            LauncherLog.Warn($"无法发送崩溃报告: {ex.Message}");
        }
    }

    /// <summary>
    /// The announcements shown by the launcher on its main page.
    /// </summary>
    public ObservableCollection<Announcement> Announcements { get; } = new();

    /// <summary>
    /// The state of the local service of the online mode.
    /// </summary>
    public string TerracottaStatus
    {
        get => _terracottaStatus;
        private set => Set(ref _terracottaStatus, value);
    }

    private string _terracottaStatus = Lang.Get("hmcl.mainwindowviewmodel.644_", "正在检查联机组件");

    /// <summary>
    /// The description of the state of the local service.
    /// </summary>
    public string TerracottaStatusDescription
    {
        get => _terracottaStatusDescription;
        private set => Set(ref _terracottaStatusDescription, value);
    }

    private string _terracottaStatusDescription = string.Empty;

    /// <summary>
    /// The nodes of the network of the online mode.
    /// </summary>
    public ObservableCollection<string> TerracottaNodes { get; } = new();

    /// <summary>
    /// The players of the session of the online mode.
    /// </summary>
    public ObservableCollection<TerracottaProfile> TerracottaProfiles { get; } = new();

    /// <summary>
    /// The invitation code of the session hosted by the launcher.
    /// </summary>
    public string TerracottaRoomCode
    {
        get => _terracottaRoomCode;
        private set
        {
            if (Set(ref _terracottaRoomCode, value))
                OnPropertyChanged(nameof(HasTerracottaRoomCode));
        }
    }

    private string _terracottaRoomCode = string.Empty;

    /// <summary>
    /// Whether the session has an invitation code.
    /// </summary>
    public bool HasTerracottaRoomCode => !string.IsNullOrEmpty(TerracottaRoomCode);

    /// <summary>
    /// The backup address of the session joined by the launcher.
    /// </summary>
    public string TerracottaBackupAddress
    {
        get => _terracottaBackupAddress;
        private set => Set(ref _terracottaBackupAddress, value);
    }

    private string _terracottaBackupAddress = string.Empty;

    /// <summary>
    /// The difficulty of the connection estimated by the service.
    /// </summary>
    public string TerracottaDifficulty
    {
        get => _terracottaDifficulty;
        private set => Set(ref _terracottaDifficulty, value);
    }

    private string _terracottaDifficulty = string.Empty;

    /// <summary>
    /// The guide telling the user how to install the online component from a
    /// third-party download channel.
    /// </summary>
    public string TerracottaPackageGuide
    {
        get
        {
            var packageName = TerracottaMetadata.Current?.PackageName ?? string.Empty;
            return string.Format(
                CultureInfo.CurrentCulture,
                Lang.Get("terracotta.from_local.guide",
                    "请下载名为 %s 的联机组件安装包，下载完成后将文件拖入本页面即可安装。"),
                packageName);
        }
    }

    /// <summary>
    /// Reads the logs of the online component and writes them into a file
    /// chosen by the user.
    /// </summary>
    /// <param name="destinationPath">The path of the file receiving the logs.</param>
    /// <returns>Whether the logs were written.</returns>
    public async Task<bool> ExportTerracottaLogsAsync(string destinationPath)
    {
        try
        {
            var logs = await TerracottaManager.ExportLogsAsync();
            if (string.IsNullOrEmpty(logs))
            {
                StatusMessage = "无法读取联机组件的日志";
                return false;
            }

            await File.WriteAllTextAsync(destinationPath, logs);
            StatusMessage = $"联机组件日志已导出: {destinationPath}";
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            StatusMessage = $"导出联机组件日志失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// The address of the form used to send the feedback about the online
    /// component.
    /// </summary>
    public string TerracottaFeedbackLink =>
        $"https://docs.hmcl.net/multiplayer/feedback.html?version={TerracottaMetadata.Current?.LatestVersion ?? "0.0.0"}";

    /// <summary>
    /// Whether the online component is installed.
    /// </summary>
    public bool IsTerracottaInstalled
    {
        get => _isTerracottaInstalled;
        private set => Set(ref _isTerracottaInstalled, value);
    }

    private bool _isTerracottaInstalled;

    /// <summary>
    /// Whether the online component is ready to be used.
    /// </summary>
    public bool IsTerracottaReady
    {
        get => _isTerracottaReady;
        private set => Set(ref _isTerracottaReady, value);
    }

    private bool _isTerracottaReady;

    /// <summary>
    /// Copies the invitation code of the session into the clipboard.
    /// </summary>
    public RelayCommand CopyTerracottaRoomCodeCommand { get; }

    /// <summary>
    /// The name of the room the player joins.
    /// </summary>
    public string TerracottaRoomName
    {
        get => _terracottaRoomName;
        set => Set(ref _terracottaRoomName, value);
    }

    private string _terracottaRoomName = string.Empty;

    /// <summary>
    /// Refreshes the state of the local service of the online mode.
    /// </summary>
    public RelayCommand RefreshTerracottaCommand { get; }

    /// <summary>
    /// Asks the local service to look for the rooms of the network.
    /// </summary>
    public RelayCommand ScanTerracottaCommand { get; }

    /// <summary>
    /// Asks the local service to join a room.
    /// </summary>
    public RelayCommand JoinTerracottaCommand { get; }

    /// <summary>
    /// Downloads and installs the component of the online mode.
    /// </summary>
    public RelayCommand InstallTerracottaCommand { get; }

    /// <summary>
    /// The links of the online mode shown to the user.
    /// </summary>
    public ObservableCollection<TerracottaLink> TerracottaLinks { get; } = new();

    /// <summary>
    /// Checks the state of the local service of the online mode and refreshes
    /// the list of the nodes of the network.
    /// </summary>
    /// <returns>A task that completes when the state is known.</returns>
    private async Task RefreshTerracottaAsync()
    {
        try
        {
            var state = await TerracottaManager.InitializeAsync();
            ApplyTerracottaState(state);

            var nodes = await TerracottaManager.GetNodesAsync();
            TerracottaNodes.Clear();
            foreach (var node in nodes)
                TerracottaNodes.Add(node);
        }
        catch (Exception ex)
        {
            TerracottaStatus = Lang.Get("hmcl.mainwindowviewmodel.646_");
            TerracottaStatusDescription = ex.Message;
        }
    }

    /// <summary>
    /// Applies the state of the local service to the view model.
    /// </summary>
    /// <param name="state">The state of the service.</param>
    private void ApplyTerracottaState(TerracottaState state)
    {
        TerracottaStatus = state.DisplayName;
        TerracottaStatusDescription = state.Description;

        // The buttons of the page depend on the state of the service, like the
        // page of Java HMCL.
        IsTerracottaInstalled = state is not TerracottaState.Uninitialized;
        IsTerracottaReady = state is TerracottaState.Ready;

        if (state is not TerracottaState.Ready ready)
            return;

        TerracottaRoomCode = ready.RoomCode ?? string.Empty;
        TerracottaBackupAddress = ready.BackupAddress ?? string.Empty;
        TerracottaDifficulty = ready.Difficulty switch
        {
            HMCLCore.NET.Terracotta.TerracottaDifficulty.Easiest => Lang.Get("terracotta.difficulty.easiest", "网络极好，几乎已连接！"),
            HMCLCore.NET.Terracotta.TerracottaDifficulty.Simple => Lang.Get("terracotta.difficulty.simple", "网络良好，连接可能需要一些时间"),
            HMCLCore.NET.Terracotta.TerracottaDifficulty.Medium => Lang.Get("terracotta.difficulty.medium", "网络一般，已启用备用路由，连接仍可能失败"),
            HMCLCore.NET.Terracotta.TerracottaDifficulty.Tough => Lang.Get("terracotta.difficulty.tough", "网络较差，已启用备用路由，连接仍可能失败"),
            _ => Lang.Get("terracotta.difficulty.estimate_only", "成功率为估算值，仅供参考")
        };

        TerracottaProfiles.Clear();
        foreach (var profile in ready.Profiles)
            TerracottaProfiles.Add(profile);
    }

    /// <summary>
    /// Asks the local service to look for the rooms of the network.
    /// </summary>
    /// <returns>A task that completes when the request is sent.</returns>
    private async Task ScanTerracottaAsync()
    {
        var player = SelectedAccount?.Username ?? Lang.Get("hmcl.mainwindowviewmodel.terracotta_player_anonymous");
        await TerracottaManager.ScanAsync(player);
        ApplyTerracottaState(TerracottaManager.State);
    }

    /// <summary>
    /// Copies the invitation code of the session into the clipboard.
    /// </summary>
    private void CopyTerracottaRoomCode()
    {
        if (string.IsNullOrEmpty(TerracottaRoomCode))
            return;

        var owner = GetMainWindow();
        if (owner is null)
            return;

        _ = ClipboardUtils.SetTextAsync(owner, TerracottaRoomCode);
        ShowToast(Lang.Get("terracotta.status.host_ok.code.copy.toast", "邀请码已复制到剪贴板"));
    }

    /// <summary>
    /// Asks the local service to join a room.
    /// </summary>
    /// <returns>A task that completes when the request is sent.</returns>
    private async Task JoinTerracottaAsync()
    {
        if (string.IsNullOrWhiteSpace(TerracottaRoomName))
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.650_"));
            return;
        }

        var player = SelectedAccount?.Username ?? Lang.Get("hmcl.mainwindowviewmodel.terracotta_player_anonymous_");
        await TerracottaManager.JoinAsync(TerracottaRoomName, player);
        ApplyTerracottaState(TerracottaManager.State);
    }

    /// <summary>
    /// Downloads and installs the component of the online mode, then starts it.
    /// </summary>
    /// <returns>A task that completes when the component is started.</returns>
    private async Task InstallTerracottaAsync()
    {
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.654_");

        try
        {
            var state = await TerracottaManager.DownloadAsync();
            ApplyTerracottaState(state);

            SetStatus(state switch
            {
                TerracottaState.Unknown => Lang.Get("hmcl.mainwindowviewmodel.656_"),
                TerracottaState.Fatal => $"联机组件不可用: {state.Description}",
                _ => state.DisplayName
            });
        }
        catch (Exception ex)
        {
            ApplyTerracottaState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            SetStatus($"联机组件安装失败: {ex.Message}");
        }
    }

    /// <summary>
    /// Installs the online component from a package downloaded by the user and
    /// starts the service.
    /// </summary>
    /// <param name="packagePath">The path of the package.</param>
    /// <returns>Whether the component was installed.</returns>
    public async Task<bool> InstallTerracottaFromLocalAsync(string packagePath)
    {
        if (string.IsNullOrWhiteSpace(packagePath) || !File.Exists(packagePath))
        {
            SetStatus("联机组件安装包不存在");
            return false;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.654_");

        try
        {
            var state = await TerracottaManager.InstallAsync(packagePath);
            ApplyTerracottaState(state);

            SetStatus(state switch
            {
                TerracottaState.Unknown => Lang.Get("hmcl.mainwindowviewmodel.656_"),
                TerracottaState.Fatal => $"联机组件不可用: {state.Description}",
                _ => state.DisplayName
            });

            return state is not TerracottaState.Fatal;
        }
        catch (Exception ex)
        {
            ApplyTerracottaState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            SetStatus($"联机组件安装失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// The announcement currently shown on the main page.
    /// </summary>
    public Announcement? CurrentAnnouncement
    {
        get => _currentAnnouncement;
        private set
        {
            if (Set(ref _currentAnnouncement, value))
                OnPropertyChanged(nameof(HasAnnouncement));
        }
    }

    private Announcement? _currentAnnouncement;

    /// <summary>
    /// Whether an announcement is shown on the main page.
    /// </summary>
    public bool HasAnnouncement => CurrentAnnouncement is { } announcement
        && (!string.IsNullOrWhiteSpace(announcement.Title) ||
            !string.IsNullOrWhiteSpace(announcement.Content));

    /// <summary>
    /// Checks for a newer release and shows the update bubble.
    /// Best effort: failures are ignored.
    /// </summary>
    private async Task CheckForUpdatesAsync()
    {
        try
        {
            var currentVersion = System.Reflection.Assembly.GetExecutingAssembly().GetName().Version
                ?? new Version(1, 0, 0);
            var checker = new UpdateChecker();
            var includePrerelease = string.Equals(
                _settingsManager.Settings.UpdateChannel, "development", StringComparison.OrdinalIgnoreCase);
            var update = await checker.CheckForUpdateAsync(currentVersion, includePrerelease);
            if (update is null)
                return;

            LatestUpdate = update;
            LatestVersionText = $"发现新版本 {update.TagName}";
            ShowUpdateBubble = true;

            // The notes of the release are shown as an announcement; the
            // releases without notes show no announcement at all.
            if (!string.IsNullOrWhiteSpace(update.Body))
            {
                CurrentAnnouncement = AnnouncementService.CreateReleaseAnnouncement(
                    $"新版本 {update.TagName}", update.Body, update.HtmlUrl);
            }
        }
        catch
        {
            // Update checking is optional and must never affect startup.
        }
    }

    /// <summary>
    /// Reads the announcements of the launcher and shows the first one.
    /// </summary>
    /// <returns>A task that completes when the announcements are read.</returns>
    private async Task LoadAnnouncementsAsync()
    {
        try
        {
            var service = new AnnouncementService();
            var announcements = await service.GetAnnouncementsAsync();

            Announcements.Clear();
            foreach (var announcement in announcements)
                Announcements.Add(announcement);

            if (announcements.Count > 0)
            {
                // The announcements without text are not shown.
                var announcement = announcements[0];
                if (!string.IsNullOrWhiteSpace(announcement.Title) ||
                    !string.IsNullOrWhiteSpace(announcement.Content))
                {
                    CurrentAnnouncement = announcement;
                }
            }
        }
        catch
        {
            // The announcements are optional and must never affect startup.
        }
    }

    /// <summary>
    /// Opens the project's releases page in the default browser.
    /// </summary>
    private void OpenUpdatePage()
    {
        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = "https://github.com/KnightBlood/HMCL/releases",
                UseShellExecute = true
            });
        }
        catch
        {
            // Ignore browser launch failures.
        }
    }

    /// <summary>
    /// Downloads the available update and launches the installer, then exits the application.
    /// </summary>
    private async void DownloadUpdateAsync()
    {
        if (LatestUpdate is not { } update || string.IsNullOrEmpty(update.ExeDownloadUrl))
            return;

        try
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.658_");
            var checker = new UpdateChecker();
            var downloadedPath = await checker.DownloadUpdateAsync(update.ExeDownloadUrl);

            // The downloaded file is checked before it replaces the launcher.
            var verification = SelfUpdater.Verify(downloadedPath);
            if (!verification.IsValid)
            {
                StatusMessage = $"更新文件校验失败: {verification.Reason}";
                return;
            }

            LauncherLog.Info($"已下载更新 {update.TagName} (SHA-256 {verification.Sha256})");

            // The launcher is replaced by the new version, which is started
            // right after the current process exits.
            if (!await SelfUpdater.ApplyAsync(downloadedPath))
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.660_");
                return;
            }

            Environment.Exit(0);
        }
        catch (Exception ex)
        {
            StatusMessage = $"更新下载失败: {ex.Message}";
        }
    }

    /// <summary>
    /// The system memory summary shown at the bottom of the sidebar,
    /// mirroring Java HMCL's memory status bar.
    /// </summary>
    public string MemoryStatus
    {
        get
        {
            var totalGb = GC.GetGCMemoryInfo().TotalAvailableMemoryBytes / 1024d / 1024d / 1024d;
            return string.Format(Lang.Get("hmcl.memory_status", "系统内存: {0:F1} GB"), totalGb);
        }
    }

    /// <summary>
    /// The live game log lines produced by the running game process.
    /// </summary>
    public ObservableCollection<string> GameLines { get; } = new();

    /// <summary>
    /// The font family of the log window.
    /// </summary>
    public string LogFontFamily
    {
        get => _logFontFamily;
        private set => Set(ref _logFontFamily, value);
    }

    private string _logFontFamily = string.Empty;

    /// <summary>
    /// The size of the font of the log window.
    /// </summary>
    public double LogFontSize
    {
        get => _logFontSize;
        private set => Set(ref _logFontSize, value);
    }

    private double _logFontSize = 12;

    /// <summary>
    /// The launcher log lines, exported together with the game log.
    /// </summary>
    public ObservableCollection<string> LauncherLines { get; } = new();

    /// <summary>
    /// Exports the game log, the launcher log and the crash reports of the
    /// selected instance into a zip archive.
    /// </summary>
    /// <param name="destinationPath">The path of the zip archive to create.</param>
    /// <returns>Whether the export succeeded.</returns>
    public bool ExportGameLogs(string destinationPath)
    {
        var instance = SelectedInstance;
        var gameDirectory = instance?.GameDirectory ?? Repository.Layout.BaseDirectory;

        try
        {
            var written = HMCLCore.NET.Game.LogExporter.Export(
                destinationPath, gameDirectory, GameLines, LauncherLines);

            SetStatus($"已导出日志到 {destinationPath} (共 {written} 个文件)");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"导出日志失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Writes the dump of the threads of the running game into a file.
    /// </summary>
    /// <param name="destinationPath">The path of the written file.</param>
    /// <returns>Whether the dump was written.</returns>
    public async Task<bool> GenerateGameDumpAsync(string destinationPath)
    {
        var process = _gameProcess;
        if (process is null)
        {
            SetStatus("游戏未在运行，无法生成转储");
            return false;
        }

        try
        {
            var javaPath = _javaManager.PreferredRuntime?.Path;
            var written = await HMCLCore.NET.Game.GameDumpGenerator.WriteDumpToAsync(
                process.Id, javaPath, destinationPath);

            SetStatus(written ? $"已生成游戏转储: {destinationPath}" : "生成游戏转储失败");
            return written;
        }
        catch (Exception ex)
        {
            SetStatus($"生成游戏转储失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// A command terminating the currently tracked game process, if any.
    /// </summary>
    public RelayCommand StopGameCommand { get; }

    /// <summary>
    /// Subscribes the game log to the process' output streams and exit notification.
    /// When a crash report appears after an abnormal exit, its summary is logged.
    /// </summary>
    /// <param name="process">The freshly started game process.</param>
    /// <param name="gameDirectory">The working game directory to scan for crash reports.</param>
    private void AttachGameProcess(ManagedProcess process, string gameDirectory)
    {
        _gameProcess = process;

        var instance = SelectedInstance!;
        var info = new RunningGameInfo(instance, process);
        RunningProcesses.Add(info);

        process.OutputLineReceived += OnGameOutputLine;
        process.ErrorLineReceived += OnGameOutputLine;
        process.Exited += () => _ = WatchGameProcessAsync(process, info, gameDirectory);
    }

    /// <summary>
    /// Waits for the game process to exit and reports its exit code.
    /// </summary>
    /// <param name="process">The exited game process.</param>
    /// <param name="info">The information of the running game.</param>
    /// <param name="gameDirectory">The working game directory to scan for crash reports.</param>
    /// <returns>A task that completes when the process has been reported.</returns>
    private async Task WatchGameProcessAsync(ManagedProcess process, RunningGameInfo info, string gameDirectory)
    {
        var exitCode = -1;
        try
        {
            exitCode = await process.WaitForExitAsync();
        }
        catch (Exception)
        {
            // The exit code stays unknown when the process cannot be waited for.
        }

        await Dispatcher.UIThread.InvokeAsync(() =>
        {
            StatusMessage = $"游戏已退出 (PID {process.Id}, 退出码 {exitCode})";
            LauncherLog.Info($"游戏已退出 (PID {process.Id}, 退出码 {exitCode})");

            EventBus.Instance.Publish(new GameLaunchEvent(
                SelectedInstance?.Version ?? string.Empty, LaunchState.Exited));

            // The launcher shows itself again when it hides itself to let the
            // game run.
            ReopenLauncher();

            // Remove from running list once exited.
            RunningProcesses.Remove(info);
            if (ReferenceEquals(_gameProcess, process))
                _gameProcess = null;

            // Detect and summarize the newest crash report, if one was written.
            var crashReport = FindLatestCrashReport(gameDirectory);
            if (crashReport != null)
            {
                var analyzer = new HMCLCore.NET.Download.CrashReportAnalyzer();
                var summary = analyzer.Analyze(crashReport);
                OnGameOutputLine($"[HMCL] 检测到崩溃报告: {Path.GetFileName(crashReport)}");
                OnGameOutputLine($"[HMCL] 错误类型: {summary.ErrorType}, Minecraft 版本: {summary.MinecraftVersion}");
                LauncherLog.Warn($"检测到崩溃报告: {Path.GetFileName(crashReport)} ({summary.ErrorType})");
            }

            // The causes of the crash are shown to the user, like the crash
            // window of Java HMCL.
            AnalyzeCrash(gameDirectory);
        });
    }

    /// <summary>
    /// The causes of the last crash of the game, shown in the game log window.
    /// </summary>
    public ObservableCollection<string> CrashCauses { get; } = new();

    /// <summary>
    /// Whether the causes of a crash of the game are known.
    /// </summary>
    public bool HasCrashCauses
    {
        get => _hasCrashCauses;
        private set => Set(ref _hasCrashCauses, value);
    }

    private bool _hasCrashCauses;

    /// <summary>
    /// Finds the causes of the crash of the game in its logs and shows them to
    /// the user.
    /// </summary>
    /// <param name="gameDirectory">The directory the game ran in.</param>
    private void AnalyzeCrash(string gameDirectory)
    {
        CrashCauses.Clear();
        HasCrashCauses = false;

        var results = HMCLCore.NET.Game.CrashReportAnalyzer.FindCausesInLogs(gameDirectory);
        foreach (var result in results)
        {
            var text = Lang.Get(result.TextKey, Lang.Get("game.crash.reason.unknown", "无法确定崩溃原因"));
            var message = FormatCrashCause(text, result);
            CrashCauses.Add(message);
            OnGameOutputLine($"[HMCL] 崩溃原因: {message}");
        }

        if (CrashCauses.Count == 0)
            return;

        HasCrashCauses = true;

        // The user is told about the crash without opening the log window.
        ToastMessage = CrashCauses[0];
        GameCrashDetected?.Invoke();
    }

    /// <summary>
    /// Fills the placeholders of the text of a cause of a crash.
    /// </summary>
    /// <param name="text">The text of the cause.</param>
    /// <param name="result">The cause of the crash.</param>
    /// <returns>The text shown to the user.</returns>
    private static string FormatCrashCause(string text, HMCLCore.NET.Game.CrashCauseResult result)
    {
        var arguments = result.Arguments;

        var expected = arguments.TryGetValue("expected", out var javaVersion)
            ? GetJavaVersionFromClassFileVersion(
                int.TryParse(javaVersion, out var classFileVersion) ? classFileVersion : 0)
            : string.Empty;

        var id = arguments.TryGetValue("id", out var modId) ? modId : string.Empty;
        var name = arguments.TryGetValue("name", out var modName) ? modName : string.Empty;
        var file = arguments.TryGetValue("file", out var fileName) ? fileName : string.Empty;
        var reason = arguments.TryGetValue("reason", out var reasonText) ? reasonText : string.Empty;

        return text
            .Replace("%1$s", id.Length > 0 ? id : name)
            .Replace("%2$s", file.Length > 0 ? file : expected)
            .Replace("%3$s", reason)
            .Replace("%s", expected);
    }

    /// <summary>
    /// Returns the name of the Java version with the given major version.
    /// </summary>
    /// <param name="major">The major version of the Java runtime.</param>
    /// <returns>The name of the version.</returns>
    private static string GetJavaVersionFromMajorVersion(int major) => major switch
    {
        6 => "Java 6",
        7 => "Java 7",
        8 => "Java 8",
        9 => "Java 9",
        10 => "Java 10",
        11 => "Java 11",
        12 => "Java 12",
        13 => "Java 13",
        14 => "Java 14",
        15 => "Java 15",
        16 => "Java 16",
        17 => "Java 17",
        18 => "Java 18",
        19 => "Java 19",
        20 => "Java 20",
        21 => "Java 21",
        22 => "Java 22",
        23 => "Java 23",
        24 => "Java 24",
        _ => "Java " + major
    };

    /// <summary>
    /// Returns the name of the Java version of a class file version, like the
    /// class file version 61 standing for Java 17.
    /// </summary>
    /// <param name="classFileVersion">The version of the class file.</param>
    /// <returns>The name of the version.</returns>
    private static string GetJavaVersionFromClassFileVersion(int classFileVersion)
        => GetJavaVersionFromMajorVersion(classFileVersion - 44);

    /// <summary>
    /// Applies the settings of a MultiMC instance to the game settings of the
    /// installed instance.
    /// </summary>
    /// <param name="settings">The settings of the MultiMC instance.</param>
    /// <returns>The game settings of the installed instance.</returns>
    private static GameSettingsInstance ApplyMultiMcSettings(MultiMcInstanceSettings? settings)
    {
        var result = new GameSettingsInstance();
        if (settings is null)
            return result;

        // The instance always runs in its own directory.
        result.Override(GameSettings.PropertyRunningDirectory);

        if (settings.OverrideJavaLocation)
        {
            result.Override(GameSettings.PropertyJavaType);
            result.Override(GameSettings.PropertyCustomJavaPath);
            result.JavaType = JavaVersionType.Custom;
            result.CustomJavaPath = settings.JavaPath ?? string.Empty;
        }

        if (settings.OverrideMemory)
        {
            result.Override(GameSettings.PropertyAutoMemory);
            result.Override(GameSettings.PropertyPermSize);
            result.Override(GameSettings.PropertyMaxMemory);
            result.Override(GameSettings.PropertyMinMemory);
            result.AutoMemory = false;
            result.PermSize = settings.PermGen;
            if (settings.MaxMemory is { } maxMemory)
                result.MaxMemory = maxMemory;
            if (settings.MinMemory is { } minMemory)
                result.MinMemory = minMemory;
        }

        if (settings.OverrideCommands)
        {
            result.Override(GameSettings.PropertyWrapper);
            result.Override(GameSettings.PropertyPreCommand);
            result.Wrapper = settings.WrapperCommand ?? string.Empty;
            result.PreCommand = settings.PreLaunchCommand ?? string.Empty;
        }

        if (settings.OverrideJavaArgs)
        {
            result.Override(GameSettings.PropertyJvmOptions);
            result.JvmOptions = settings.JvmArgs ?? string.Empty;
        }

        if (settings.OverrideConsole)
        {
            result.Override(GameSettings.PropertyShowLogs);
            result.ShowLogs = settings.ShowConsole;
        }

        if (settings.OverrideWindow)
        {
            result.Override(GameSettings.PropertyFullscreen);
            result.Override(GameSettings.PropertyWidth);
            result.Override(GameSettings.PropertyHeight);
            result.Fullscreen = settings.Fullscreen;
            if (settings.Width is { } width)
                result.Width = width;
            if (settings.Height is { } height)
                result.Height = height;
        }

        return result;
    }

    /// <summary>
    /// Raised when the game crashed and the causes of the crash are known.
    /// </summary>
    public event Action? GameCrashDetected;

    /// <summary>
    /// Returns the newest crash report file in the instance's crash-reports directory.
    /// </summary>
    private static string? FindLatestCrashReport(string gameDirectory)
    {
        try
        {
            var directory = Path.Combine(gameDirectory, "crash-reports");
            if (!Directory.Exists(directory))
                return null;

            return new DirectoryInfo(directory)
                .GetFiles("*.txt")
                .OrderByDescending(f => f.LastWriteTime)
                .Select(f => f.FullName)
                .FirstOrDefault();
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Appends one game output line to the wrapped log buffer on the UI thread.
    /// Keeps the buffer bounded by trimming the oldest lines.
    /// </summary>
    private void OnGameOutputLine(string line)
    {
        LauncherLog.Info(line);

        Dispatcher.UIThread.Post(() =>
        {
            GameLines.Add(line);
            while (GameLines.Count > MaxGameLogLines)
                GameLines.RemoveAt(0);
        });
    }

    /// <summary>
    /// Terminates the tracked game process when it is still running.
    /// </summary>
    private void StopGame()
    {
        if (_gameProcess is { } process && !process.HasExited)
        {
            process.Kill();
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.662_");
        }
    }

    /// <summary>
    /// Backs up the selected instance into the repository's backups directory.
    /// Excludes log/cache directories; keeps mods, configs and saves.
    /// </summary>
    /// <returns>The archive path, or null when no instance was selected or backup failed.</returns>
    public async Task<string?> BackupSelectedInstanceAsync()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.664_"));
            return null;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.666_");
        var archivePath = await Task.Run(() => Repository.BackupInstance(
            new GameInstanceID { Value = instance.Id }));

        StatusMessage = archivePath != null
            ? $"已备份实例 {instance.Id}: {archivePath}"
            : $"备份实例 {instance.Id} 失败";
        return archivePath;
    }

    /// <summary>
    /// Reads the optional files of a modpack, so that the user may choose the
    /// ones which are installed.
    /// </summary>
    /// <param name="archivePath">The path of the modpack archive.</param>
    /// <returns>The optional files of the modpack.</returns>
    public async Task<IReadOnlyList<ModpackOptionalFile>> GetOptionalFilesAsync(string archivePath)
    {
        try
        {
            var modpack = await Task.Run(() => ModpackReader.Load(archivePath));
            return modpack?.OptionalFiles ?? Array.Empty<ModpackOptionalFile>();
        }
        catch (Exception ex) when (ex is IOException or InvalidDataException)
        {
            return Array.Empty<ModpackOptionalFile>();
        }
    }

    /// <summary>
    /// Installs a modpack archive as a new instance: reads metadata to derive
    /// the instance name, then extracts overrides and downloads remote files.
    /// </summary>
    /// <param name="archivePath">The zip or mrpack archive path.</param>
    /// <param name="excludedFiles">
    /// The keys of the optional files the user chose not to install.
    /// </param>
    /// <returns>The created instance id, or null when installation failed.</returns>
    public async Task<string?> InstallModpackAsync(string archivePath, IReadOnlySet<string>? excludedFiles = null)
    {
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.668_");
        var modpack = await Task.Run(() => ModpackReader.Load(archivePath));
        if (modpack == null)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.670_"));
            return null;
        }

        // Derive a unique instance id from the pack name.
        var baseName = DefaultGameRepository.SanitizeInstanceName(
            string.IsNullOrEmpty(modpack.Name) ? Path.GetFileNameWithoutExtension(archivePath) : modpack.Name!);
        var instanceId = baseName;
        var ordinal = 0;
        while (GameInstances.Any(i => i.Id == instanceId))
        {
            ordinal++;
            instanceId = $"{baseName}-{ordinal}";
        }

        var instanceRoot = Repository.GetInstanceRoot(new GameInstanceID { Value = instanceId });

        try
        {
            // CurseForge packs reference mods by ids that must be resolved through
            // the API; without a key they are counted as unresolved instead.
            Func<int, int, Task<string?>>? curseForgeResolver = null;
            if (!string.IsNullOrEmpty(_settingsManager.Settings.CurseForgeApiKey))
            {
                var curseForgeRepository = new CurseForgeRepository
                {
                    ApiKey = _settingsManager.Settings.CurseForgeApiKey
                };
                curseForgeResolver = (projectId, fileId) =>
                    curseForgeRepository.GetFileDownloadUrlAsync(projectId, fileId);
            }

            var task = new ModpackInstallTask(archivePath, instanceRoot, modpack, curseForgeResolver, excludedFiles);
            task.ProgressChanged += (_, e) =>
            {
                if (e.TotalBytes is { } total && total > 0)
                    StatusMessage = $"安装整合包: {e.DownloadedBytes}/{total}";
            };

            await TaskManager.RunAsync(task);

            // The settings of a MultiMC instance are applied to the instance
            // after the installation, like the post-install task of Java HMCL.
            var gameSettings = ApplyMultiMcSettings(modpack.MultiMcSettings);

            SaveInstance(instanceId, new GameInstanceManifest
            {
                Version = modpack.GameVersion ?? string.Empty,
                MainClass = "net.minecraft.client.main.Main",
                GameSettings = gameSettings
            });

            var summary = $"整合包 {modpack.Name} 已安装为实例 {instanceId} " +
                          $"(解压 {task.ExtractedFiles} 个文件, 下载 {task.DownloadedFiles} 个模组)";
            if (task.UnresolvedFiles > 0)
                summary += $", 未解析 {task.UnresolvedFiles} 个 CurseForge 模组 (需配置 API Key)";

            StatusMessage = summary;

            // The mods that could not be resolved are reported to the user.
            if (task.UnresolvedMods.Count > 0)
                await ReportUnresolvedModsAsync(task.UnresolvedMods);

            // The mods of the new instance are compared with the mods of the
            // pack, so that the user knows what is missing.
            var installed = Repository.Snapshot.GetAllInstances()
                .FirstOrDefault(instance => instance.Id == instanceId);

            TrackUsage("modpack_install");

            if (installed != null)
            {
                var adviser = ModAdviser.Analyze(installed, modpack);
                if (adviser.HasSuggestions)
                    ShowHint(HintType.Warning, $"整合包检查: {adviser.Summary}");
            }

            return instanceId;
        }
        catch (Exception ex)
        {
            SetStatus($"整合包安装失败: {ex.Message}");
            return null;
        }
    }

    /// <summary>
    /// Shows the mods of a modpack that could not be installed.
    /// </summary>
    /// <param name="mods">The mods that could not be resolved.</param>
    /// <returns>A task that completes when the report is closed.</returns>
    private async Task ReportUnresolvedModsAsync(IReadOnlyList<UnresolvedMod> mods)
    {
        var owner = GetMainWindow();
        if (owner is null)
            return;

        var lines = mods.Select(mod => "• " + mod.DisplayName);
        var message = $"以下 {mods.Count} 个模组无法解析，请配置 CurseForge API Key 后重新安装：\n\n" +
                      string.Join(Environment.NewLine, lines);

        await DialogService.ShowMessageAsync(owner, message, Lang.Get("hmcl.mainwindowviewmodel.672_"));
    }

    /// <summary>
    /// Returns the main window of the launcher, when it is available.
    /// </summary>
    /// <returns>The main window, or null when it is not available.</returns>
    private static Window? GetMainWindow()
    {
        var lifetime = Application.Current?.ApplicationLifetime;
        if (lifetime is null)
            return null;

        return (lifetime as IClassicDesktopStyleApplicationLifetime)?.MainWindow;
    }

    /// <summary>
    /// Updates the selected instance to a newer version of the modpack it was
    /// created from: computes the file differences, asks the user to confirm and
    /// then applies the update with an automatic rollback on failure.
    /// </summary>
    /// <param name="archivePath">The archive of the new modpack version.</param>
    /// <returns>Whether the update was applied.</returns>
    public async Task<bool> UpdateModpackAsync(string archivePath)
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.674_"));
            return false;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.676_");
        var modpack = await Task.Run(() => ModpackReader.Load(archivePath));
        if (modpack == null)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.678_"));
            return false;
        }

        var instanceRoot = Repository.GetInstanceRoot(new GameInstanceID { Value = instance.Id });
        if (!Directory.Exists(instanceRoot))
        {
            SetStatus($"实例 {instance.Id} 的目录不存在");
            return false;
        }

        // Compute the differences between the installed pack and the new one.
        var installed = ModpackConfiguration.Read(instanceRoot);
        var target = await Task.Run(() => ModpackUpdatePlanner.Describe(archivePath, modpack));
        var plan = ModpackUpdatePlanner.Compare(installed, target);

        if (!plan.HasChanges)
        {
            SetStatus($"整合包 {plan.Name} 已经是最新版本");
            return false;
        }

        // Ask the user to confirm the update.
        Window? owner = null;
        var lifetime = Application.Current?.ApplicationLifetime;
        if (lifetime is { })
        {
            owner = (lifetime as IClassicDesktopStyleApplicationLifetime)?.MainWindow;
        }

        var dialog = new ModpackUpdateDialog(plan);
        var confirmed = await dialog.ShowDialog<bool>(owner!);
        if (!confirmed)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.680_"));
            return false;
        }

        try
        {
            Func<int, int, Task<string?>>? curseForgeResolver = null;
            if (!string.IsNullOrEmpty(_settingsManager.Settings.CurseForgeApiKey))
            {
                var curseForgeRepository = new CurseForgeRepository
                {
                    ApiKey = _settingsManager.Settings.CurseForgeApiKey
                };
                curseForgeResolver = (projectId, fileId) =>
                    curseForgeRepository.GetFileDownloadUrlAsync(projectId, fileId);
            }

            var task = new ModpackUpdateTask(instanceRoot, archivePath, modpack, curseForgeResolver);
            await TaskManager.RunAsync(task);

            StatusMessage = $"整合包 {plan.Name} 已更新到 {plan.NewVersion ?? "新版本"} " +
                            $"(新增 {plan.AddedCount}, 修改 {plan.ModifiedCount}, 删除 {plan.RemovedCount})";
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"整合包更新失败，已还原实例: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Removes an account from the manager and refreshes the UI list.
    /// </summary>
    /// <param name="account">The account to remove.</param>
    public void RemoveAccount(Account account)
    {
        if (account == null)
            return;

        _accountManager.RemoveAccount(account);
        SyncAccounts();
        SetStatus($"账户 {account.Username} 已删除");
    }

    /// <summary>
    /// Exports the selected instance's shareable archive to an arbitrary destination.
    /// </summary>
    /// <param name="destinationPath">The user-chosen zip file path.</param>
    /// <returns>Whether the archive was written.</returns>
    public async Task<bool> ExportSelectedInstanceAsync(string destinationPath)
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.682_"));
            return false;
        }

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.684_");
        var archivePath = await Task.Run(() => Repository.ExportInstance(
            new GameInstanceID { Value = instance.Id }, destinationPath));

        StatusMessage = archivePath != null
            ? $"已导出实例 {instance.Id}: {archivePath}"
            : $"导出实例 {instance.Id} 失败";
        return archivePath != null;
    }

    /// <summary>
    /// Builds the configuration used to launch the given instance: the
    /// arguments of the game are generated and the files it needs are
    /// downloaded.
    /// </summary>
    /// <param name="instance">The instance to launch.</param>
    /// <returns>The configuration of the launch, or null when it cannot be built.</returns>
    private async Task<LaunchConfig?> BuildLaunchConfigAsync(GameInstance instance)
    {
        var javaRuntime = _javaManager.PreferredRuntime;
        if (javaRuntime == null)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.686_");
            return null;
        }

        // Apply the download source (official or mirror) configured in settings.
        DownloadSourceUtils.Apply(_settingsManager.Settings);

        // Verify/refresh the selected account's credentials before building the launch.
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.688_");
        await EnsureAccountReadyAsync();

        // Load the instance manifest; fall back to a stub for bare instances.
        var manifest = LoadInstanceManifest(instance) ?? new GameInstanceManifest
        {
            Version = instance.Version,
            MainClass = "net.minecraft.client.main.Main"
        };

        // The manifest is repaired before the launch, so that the combinations
        // of the mod loaders work, like the launch manifest normalizer of
        // Java HMCL.
        manifest = LaunchManifestNormalizer.RepairForLaunch(manifest.ApplyPatches());

        var settings = _settingsManager.Settings;

        // The Java runtime must be new enough for the game; the check can be
        // turned off in the settings of the game.
        var effectiveSettings = _settingsManager.GetGameSettingsPreset(manifest.GameSettings);
        if (effectiveSettings.NotCheckJvm != true &&
            manifest.JavaVersion > 0 &&
            javaRuntime.MajorVersion > 0 &&
            javaRuntime.MajorVersion < manifest.JavaVersion)
        {
            StatusMessage = Lang.Get(
                "hmcl.mainwindowviewmodel.java_version_too_low",
                $"Java 版本过低：游戏需要 Java {manifest.JavaVersion}，当前为 Java {javaRuntime.MajorVersion}");
            LauncherLog.Warn(
                $"Java 版本过低：游戏需要 Java {manifest.JavaVersion}，当前为 Java {javaRuntime.MajorVersion}");
            return null;
        }

        // Build the base launch config; argument lists are filled by the generator below.
        var config = new LaunchConfig
        {
            Instance = instance,
            Account = SelectedAccount ?? new OfflineAccount("Player"),
            JavaRuntime = javaRuntime,
            WorkingDirectory = instance.GameDirectory,
            GameDirectory = instance.GameDirectory,
            VersionName = instance.Version,
            Memory = settings.Memory,
            AssetsDirectory = Path.Combine(Repository.Layout.BaseDirectory, "assets"),
            LibrariesDirectory = Path.Combine(Repository.Layout.BaseDirectory, "libraries"),
            PrimaryJarPath = Path.Combine(
                Repository.Layout.BaseDirectory, "versions", instance.Version, instance.Version + ".jar"),
            UseCustomJvmArgs = settings.UseCustomJvmArgs,
            CustomJvmArgs = settings.CustomJvmArgs,
            WindowWidth = settings.WindowWidth,
            WindowHeight = settings.WindowHeight
        };

        StatusMessage = $"正在启动 {instance.Id}...";

        // Third-party Yggdrasil accounts need the authlib-injector agent so
        // the game redirects session authentication to the configured server.
        if (SelectedAccount is HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount yggAccount &&
            !string.IsNullOrEmpty(yggAccount.ServerUrl))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.690_");
            var injectorCache = Path.Combine(Repository.Layout.BaseDirectory, "authlib-injector");
            var launcher = new AuthlibInjectorLauncher();
            config.AuthlibInjectorJarPath = await launcher.EnsureJarAsync(injectorCache);
            config.AuthlibInjectorServerUrl = yggAccount.ServerUrl;

            // Best-effort: pre-fetch server metadata so the game skips its
            // initial metadata request on launch. Result is cached locally.
            config.AuthlibInjectorPrefetchData =
                await launcher.FetchPrefetchDataAsync(yggAccount.ServerUrl, injectorCache);
        }

        // Download missing libraries/assets/client JAR and assemble the classpath.
        var classpath = await PrepareLaunchDependenciesAsync(manifest);
        config.ClassPath = classpath;
        config.MainClass = manifest.MainClass;
        config.AssetIndex = _pendingAssetIndex;

        // Resolve effective game settings: inherited preset + instance overrides.
        config.GameSettings = new GameSettingsEffective(
            _settingsManager.GetGameSettingsPreset(manifest.GameSettings), manifest.GameSettings);

        // The native libraries of the game are taken from the custom directory
        // when the user chose one.
        config.NativesDirectory = config.GameSettings?.UseCustomNatives == true &&
                                   !string.IsNullOrEmpty(config.GameSettings.NativesDirectory)
            ? config.GameSettings.NativesDirectory!
            : Path.Combine(Repository.Layout.BaseDirectory, "natives", manifest.Version);

        // The size of the permanent generation area of the virtual machine.
        config.PermSize = config.GameSettings?.PermSize;

        // Apply process priority from effective game settings.
        config.ProcessPriority = config.GameSettings?.ProcessPriority;

        // Apply remaining game settings to launch config.
        config.Wrapper = config.GameSettings?.Wrapper;
        config.PreCommand = config.GameSettings?.PreCommand;
        config.PostCommand = config.GameSettings?.PostCommand;
        config.RunningDirectory = config.GameSettings?.RunningDirectory;
        config.QuickPlayMode = config.GameSettings?.QuickPlayMode ?? QuickPlayMode.None;
        config.QuickPlayArgument = config.GameSettings?.QuickPlayArgument;

        if (config.GameSettings?.EnvironmentVariables is { Count: > 0 } envVars)
        {
            foreach (var kvp in envVars)
                config.EnvironmentVariables[kvp.Key] = kvp.Value;
        }

        // Generate concrete JVM/game argument lists from the config + manifest.
        var generator = new LaunchArgumentGenerator();
        config.JvmArguments = generator.GenerateJvmArguments(config, manifest);
        config.GameArguments = generator.GenerateGameArguments(config, manifest);

        return config;
    }

    /// <summary>
    /// Writes the script starting the selected instance, so that the game can
    /// be started without the launcher.
    /// </summary>
    /// <returns>Whether the script was written.</returns>
    public async Task<bool> GenerateLaunchScriptAsync()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.692_"));
            return false;
        }

        try
        {
            var config = await BuildLaunchConfigAsync(instance);
            if (config is null)
                return false;

            var directory = Path.Combine(instance.GameDirectory, "launcher");
            Directory.CreateDirectory(directory);

            var scriptPath = Path.Combine(
                directory, OperatingSystem.IsWindows() ? "launch.bat" : "launch.sh");

            LaunchScriptGenerator.WriteScript(config, scriptPath);
            SetStatus($"启动脚本已生成: {scriptPath}");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"生成启动脚本失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Starts the selected instance and checks that it keeps running, so that
    /// the user can test the settings of the instance.
    /// </summary>
    /// <returns>Whether the game started correctly.</returns>
    public async Task<bool> TestLaunchAsync()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.694_"));
            return false;
        }

        try
        {
            var config = await BuildLaunchConfigAsync(instance);
            if (config is null)
                return false;

            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.696_");
            var result = await LaunchScriptGenerator.TestLaunchAsync(config);

            if (result.Succeeded)
            {
                SetStatus(Lang.Get("hmcl.mainwindowviewmodel.698_"));
                return true;
            }

            SetStatus($"启动测试失败: {result.Message}");
            if (!string.IsNullOrWhiteSpace(result.Output))
                LauncherLog.Warn(result.Output);

            return false;
        }
        catch (Exception ex)
        {
            SetStatus($"启动测试失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Launches the selected instance as a modpack zip (mods + config).
    /// </summary>
    /// <returns>Whether the modpack was exported.</returns>
    public async Task<bool> ExportModpackAsync()
    {
        if (SelectedInstance is not { } instance)
        {
            SetStatus(Lang.Get("hmcl.mainwindowviewmodel.700_"));
            return false;
        }

        var baseDir = Repository.Layout.BaseDirectory;
        var defaultName = $"modpack-{instance.GetDisplayName()}-{DateTime.Now:yyyyMMdd-HHmmss}.zip";
        var defaultPath = Path.Combine(baseDir, "modpacks", defaultName);

        // The export wizard lets the user choose the type of the pack, the
        // information shown by its manifest and the files it contains.
        Window? owner = null;
        var lifetime = Application.Current?.ApplicationLifetime;
        if (lifetime is { })
        {
            owner = (lifetime as IClassicDesktopStyleApplicationLifetime)?.MainWindow;
        }

        var context = new ModpackExportContext(instance, defaultPath);
        var wizard = new WizardWindow(Lang.Get("hmcl.mainwindowviewmodel.modpack_export"), new IWizardStep[]
        {
            new ModpackTypeSelectionStep(context),
            new ModpackInfoStep(context),
            new ModpackFileSelectionStep(context)
        });

        var result = await wizard.ShowDialog<bool>(owner!);
        if (!result)
            return false;

        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.704_");
        try
        {
            var options = context.BuildOptions();
            await Task.Run(() =>
            {
                var exporter = new ModpackExporter();
                exporter.ExportInstance(instance, context.ExportPath, options);
            });
            SetStatus($"已导出模组包: {context.ExportPath}");
            return true;
        }
        catch (Exception ex)
        {
            SetStatus($"导出模组包失败: {ex.Message}");
            return false;
        }
    }

    /// <summary>
    /// Restores a backup zip into a new instance derived from the file name
    /// and refreshes the instance list.
    /// </summary>
    /// <param name="archivePath">The path of the backed up zip.</param>
    /// <returns>Whether the instance was restored.</returns>
    public async Task<bool> RestoreInstanceAsync(string archivePath)
    {
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.706_");
        var instanceId = await Task.Run(() => Repository.RestoreInstance(
            archivePath, System.IO.Path.GetFileNameWithoutExtension(archivePath)));

        if (instanceId == null)
        {
            StatusMessage = $"还原备份失败: {archivePath}";
            return false;
        }

        LoadInstances();
        StatusMessage = $"已还原为实例 {instanceId}";
        return true;
    }

    /// <summary>
    /// Rescans installed Java runtimes and returns the resulting preferred
    /// runtime path (empty when none was found).
    /// </summary>
    public string RescanJavaRuntimes()
    {
        _javaManager.Scan();
        _javaManager.EnsurePreferredRuntime(_settingsManager.Settings.JavaPath);

        var runtime = _javaManager.PreferredRuntime;
        StatusMessage = runtime != null
            ? $"找到 Java 运行时: {runtime.Vendor} {runtime.Version}"
            : Lang.Get("hmcl.mainwindowviewmodel.708_");
        return runtime?.Path ?? string.Empty;
    }

    /// <summary>
    /// Resolves missing libraries, assets and the client JAR from the instance manifest,
    /// downloading each unresolved task sequentially, then builds the classpath.
    /// </summary>
    /// <param name="manifest">The manifest describing required dependencies.</param>
    /// <returns>The assembled classpath for the launch configuration.</returns>
    private async Task<string> PrepareLaunchDependenciesAsync(GameInstanceManifest manifest)
    {
        var baseDirectory = Repository.Layout.BaseDirectory;
        var librariesDirectory = Path.Combine(baseDirectory, "libraries");
        var versionsDirectory = Path.Combine(baseDirectory, "versions");

        var dependencyManager = new DefaultDependencyManager(
            librariesDirectory,
            Path.Combine(baseDirectory, "assets"),
            CreateDownloadProviderManager(),
            versionsDirectory,
            Path.Combine(baseDirectory, "natives"));

        // The manifest may specify the asset index used by the game arguments.
        _pendingAssetIndex = !string.IsNullOrEmpty(manifest.AssetIndex?.Id)
            ? manifest.AssetIndex.Id
            : null;

        var tasks = dependencyManager.ResolveDependencies(manifest);
        if (tasks.Count > 0)
        {
            var settings = _settingsManager.Settings;
            var concurrency = settings.AutoDownloadThreads
                ? DownloadConcurrency.EffectiveLimit
                : Math.Clamp(settings.DownloadThreads, 1, 64);

            StatusMessage = $"准备依赖: 需要 {tasks.Count} 个文件 (并发 {concurrency})";

            var completed = 0;
            var failures = new List<Exception>();
            var gate = new object();
            using var semaphore = new SemaphoreSlim(concurrency, concurrency);

            var running = tasks.Select(async task =>
            {
                await semaphore.WaitAsync();
                try
                {
                    await TaskManager.RunAsync(task);
                }
                catch (Exception ex)
                {
                    lock (gate)
                        failures.Add(ex);
                }
                finally
                {
                    semaphore.Release();
                    var done = Interlocked.Increment(ref completed);
                    StatusMessage = $"下载依赖 {done}/{tasks.Count}";
                }
            }).ToList();

            await Task.WhenAll(running);

            SetStatus(failures.Count > 0
                ? $"依赖准备完成，但有 {failures.Count} 个文件失败"
                : Lang.Get("hmcl.mainwindowviewmodel.710_"));
        }

        // Extract the native libraries now that all archives have been
        // downloaded, so that the game can load them through java.library.path.
        try
        {
            dependencyManager.ExtractNatives(manifest);
        }
        catch (Exception ex)
        {
            SetStatus($"解压原生库失败: {ex.Message}");
        }

        // Classpath = applicable libraries + the client JAR when present.
        var classpath = new LibraryBuilder().AddLibraries(manifest.Libraries).BuildClasspath(librariesDirectory);
        var clientJar = dependencyManager.GetClientJarPath(manifest.Version);
        if (File.Exists(clientJar))
        {
            classpath = string.IsNullOrEmpty(classpath)
                ? clientJar
                : classpath + Path.PathSeparator + clientJar;
        }

        return classpath;
    }

    /// <summary>
    /// Creates the download provider registry with the default Mojang provider.
    /// </summary>
    private static DownloadProviderManager CreateDownloadProviderManager()
    {
        var manager = new DownloadProviderManager();
        manager.RegisterProvider(new MojangDownloadProvider());
        return manager;
    }
}

/// <summary>
/// View model for the version download page.
/// Fetches the remote Mojang version list and installs selected versions
/// into the game repository.
/// </summary>
public sealed class DownloadViewModel : ViewModelBase
{
    private readonly MainWindowViewModel _owner;
    private readonly string _cacheDirectory;

    private string _statusMessage = Lang.Get("hmcl.mainwindowviewmodel.712_", "就绪");
    private bool _isBusy;
    private VersionMetadata? _selectedVersion;
    private string _typeFilter = Lang.Get("hmcl.mainwindowviewmodel.curse_category_0_", "全部");
    private string _searchText = string.Empty;

    /// <summary>
    /// Creates a new DownloadViewModel.
    /// </summary>
    /// <param name="owner">The owning main view model used to persist downloaded instances.</param>
    /// <param name="cacheDirectory">Directory used to cache remote metadata files.</param>
    public DownloadViewModel(MainWindowViewModel owner, string cacheDirectory)
    {
        _owner = owner ?? throw new ArgumentNullException(nameof(owner));
        _cacheDirectory = cacheDirectory ?? throw new ArgumentNullException(nameof(cacheDirectory));

        RefreshCommand = new RelayCommand(RefreshVersionsAsync, () => !IsBusy);
        DownloadCommand = new RelayCommand(DownloadVersionAsync, () => SelectedVersion != null && !IsBusy);
        DownloadVersionCommand = new RelayCommand(parameter =>
        {
            if (parameter is not VersionMetadata version)
                return;

            SelectedVersion = version;
            _ = DownloadVersionAsync();
        }, () => !IsBusy);
        SearchModsCommand = new RelayCommand(SearchModsAsync, () => !IsBusy);
        SearchModpacksCommand = new RelayCommand(SearchModpacksAsync, () => !IsBusy);
        InstallSelectedModpackCommand = new RelayCommand(InstallSelectedModpackAsync,
            () => SelectedRemoteModpack != null && !IsBusy);
        ShowModpackDescriptionCommand = new RelayCommand(ShowModpackDescription,
            () => SelectedRemoteModpack != null);
        InstallSelectedModCommand = new RelayCommand(InstallSelectedModAsync,
            () => SelectedRemoteMod != null && !IsBusy);
        LoadInstallersCommand = new RelayCommand(() => _ = LoadInstallersAsync(), () => !IsBusy);
        InstallLoaderCommand = new RelayCommand(() => _ = InstallLoaderAsync(), () => CanInstallLoader);
    }

    /// <summary>
    /// The main view model owning this download page.
    /// </summary>
    public MainWindowViewModel Owner => _owner;

    /// <summary>
    /// The raw list of versions obtained from the remote manifest.
    /// </summary>
    public ObservableCollection<VersionMetadata> Versions { get; } = new();

    /// <summary>
    /// The installers of the mod loaders available for the selected version of
    /// the game.
    /// </summary>
    public ObservableCollection<InstallerInfo> Installers { get; } = new();

    /// <summary>
    /// The installer selected in the installer list.
    /// </summary>
    public InstallerInfo? SelectedInstaller
    {
        get => _selectedInstaller;
        set
        {
            if (Set(ref _selectedInstaller, value))
                OnPropertyChanged(nameof(SelectedInstallerVersion));
        }
    }

    private InstallerInfo? _selectedInstaller;

    /// <summary>
    /// The version of the loader selected in the installer list.
    /// </summary>
    public string SelectedInstallerVersion
    {
        get => _selectedInstallerVersion;
        set
        {
            if (Set(ref _selectedInstallerVersion, value))
                OnPropertyChanged(nameof(CanInstallLoader));
        }
    }

    private string _selectedInstallerVersion = string.Empty;

    /// <summary>
    /// Whether the selected loader can be installed.
    /// </summary>
    public bool CanInstallLoader =>
        SelectedInstaller is { HasVersions: true } && !string.IsNullOrEmpty(SelectedInstallerVersion) && !IsBusy;

    /// <summary>
    /// Loads the installers available for the selected version of the game.
    /// </summary>
    /// <returns>A task that completes when the installers are loaded.</returns>
    public async Task LoadInstallersAsync()
    {
        Installers.Clear();

        var version = SelectedVersion?.Id;
        if (string.IsNullOrEmpty(version))
            return;

        IsBusy = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.686_", "正在获取安装器列表...");

        try
        {
            var installers = await InstallerCatalog.GetAsync(version!);
            foreach (var installer in installers)
                Installers.Add(installer);

            SelectedInstaller = Installers.FirstOrDefault();
            SelectedInstallerVersion = SelectedInstaller?.Versions.FirstOrDefault() ?? string.Empty;

            StatusMessage = Installers.Count > 0
                ? $"共 {Installers.Count} 种安装器可用"
                : "该版本暂无可用的安装器";
        }
        catch (Exception ex)
        {
            StatusMessage = $"获取安装器列表失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
            InstallerChanged?.Invoke();
        }
    }

    /// <summary>
    /// Raised when the list of the installers or the selected loader changed.
    /// </summary>
    public event Action? InstallerChanged;

    /// <summary>
    /// Installs the selected loader and creates the instance using it.
    /// </summary>
    /// <returns>Whether the loader was installed.</returns>
    public async Task<bool> InstallLoaderAsync()
    {
        if (SelectedInstaller is not { } installer || SelectedVersion is not { } version)
        {
            StatusMessage = "请先选择安装器与游戏版本";
            return false;
        }

        if (string.IsNullOrEmpty(SelectedInstallerVersion))
        {
            StatusMessage = "请先选择安装器的版本";
            return false;
        }

        IsBusy = true;
        StatusMessage = $"正在安装 {installer.Name} {SelectedInstallerVersion}...";

        try
        {
            var instanceId = await _owner.InstallLoaderAsync(
                version.Id, installer.Kind, SelectedInstallerVersion);

            if (instanceId is null)
            {
                StatusMessage = $"{installer.Name} 安装失败";
                return false;
            }

            StatusMessage = $"{installer.Name} 已安装，实例 {instanceId}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"{installer.Name} 安装失败: {ex.Message}";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Versions passing the current type filter, displayed in the list.
    /// </summary>
    public ObservableCollection<VersionMetadata> FilteredVersions { get; } = new();

    /// <summary>
    /// Available type filters for the version list, mirroring Java HMCL's
    /// 全部 / 正式版 / 快照版 / 旧版 categories.
    /// </summary>
    public IReadOnlyList<string> TypeFilters { get; } =
        new List<string> { Lang.Get("hmcl.mainwindowviewmodel.curse_category_0_"), Lang.Get("hmcl.mainwindowviewmodel.instance_game_release"), Lang.Get("hmcl.mainwindowviewmodel.720_"), Lang.Get("hmcl.mainwindowviewmodel.instance_game_april_fools"), Lang.Get("hmcl.mainwindowviewmodel.724_") };

    /// <summary>
    /// The currently applied type filter.
    /// </summary>
    public string TypeFilter
    {
        get => _typeFilter;
        set
        {
            _typeFilter = value;
            OnPropertyChanged();
            ApplyFilter();
        }
    }

    /// <summary>
    /// The text used to search versions by id.
    /// </summary>
    public string SearchText
    {
        get => _searchText;
        set
        {
            Set(ref _searchText, value);
            ApplyFilter();
        }
    }

    /// <summary>
    /// Mod repositories offered in the mod download panel.
    /// </summary>
    public IReadOnlyList<string> ModRepositories { get; } =
        new List<string> { "Modrinth", "CurseForge" };

    /// <summary>
    /// The repository used to search mods.
    /// </summary>
    public string SelectedModRepository
    {
        get => _selectedModRepository;
        set => Set(ref _selectedModRepository, value);
    }

    private string _selectedModRepository = "Modrinth";

    /// <summary>
    /// The mod search query.
    /// </summary>
    public string ModSearchQuery
    {
        get => _modSearchQuery;
        set => Set(ref _modSearchQuery, value);
    }

    private string _modSearchQuery = string.Empty;

    /// <summary>
    /// Status text for mod search/installation operations.
    /// </summary>
    public string ModInstallStatus
    {
        get => _modInstallStatus;
        private set => Set(ref _modInstallStatus, value);
    }

    private string _modInstallStatus = string.Empty;

    /// <summary>
    /// Remote mod search results.
    /// </summary>
    public ObservableCollection<RemoteAddon> RemoteModResults { get; } = new();

    /// <summary>
    /// The selected remote mod to install.
    /// </summary>
    public RemoteAddon? SelectedRemoteMod
    {
        get => _selectedRemoteMod;
        set => Set(ref _selectedRemoteMod, value);
    }

    private RemoteAddon? _selectedRemoteMod;

    /// <summary>
    /// Searches the selected repository for mods.
    /// </summary>
    public RelayCommand SearchModsCommand { get; }

    /// <summary>
    /// A command searching the modpacks in the remote repositories.
    /// </summary>
    public RelayCommand SearchModpacksCommand { get; }

    /// <summary>
    /// A command installing the selected modpack as a new instance.
    /// </summary>
    public RelayCommand InstallSelectedModpackCommand { get; }

    /// <summary>
    /// Installs the selected mod into the selected instance.
    /// </summary>
    public RelayCommand InstallSelectedModCommand { get; }

    /// <summary>
    /// The selected addon category: mod, resourcepack, world, datapack or shader.
    /// </summary>
    public string AddonCategory => _addonCategory;

    private string _addonCategory = "mod";

    /// <summary>
    /// The title shown on the addon download panel for the current category.
    /// </summary>
    public string AddonCategoryTitle => _addonCategory switch
    {
        "resourcepack" => Lang.Get("hmcl.mainwindowviewmodel.726_"),
        "world" => Lang.Get("hmcl.mainwindowviewmodel.728_"),
        "datapack" => Lang.Get("hmcl.mainwindowviewmodel.730_"),
        "shader" => Lang.Get("hmcl.mainwindowviewmodel.732_"),
        _ => Lang.Get("hmcl.mainwindowviewmodel.734_")
    };

    /// <summary>
    /// Switches the addon panel to another category and clears previous results.
    /// </summary>
    /// <param name="category">One of mod, resourcepack, world, datapack, shader.</param>
    public void SetAddonCategory(string category)
    {
        if (_addonCategory == category)
            return;

        _addonCategory = category;
        OnPropertyChanged(nameof(AddonCategory));
        OnPropertyChanged(nameof(AddonCategoryTitle));
        RemoteModResults.Clear();
        SelectedRemoteMod = null;
        ModInstallStatus = string.Empty;
    }

    /// <summary>
    /// The Modrinth project type matching the current addon category.
    /// </summary>
    private string ModrinthProjectType => _addonCategory switch
    {
        "resourcepack" => "resourcepack",
        "datapack" => "datapack",
        "shader" => "shader",
        "world" => "mod", // Modrinth has no world project type.
        _ => "mod"
    };

    /// <summary>
    /// The CurseForge class id matching the current addon category.
    /// </summary>
    private int CurseForgeClassId => _addonCategory switch
    {
        "resourcepack" => 12,
        "world" => 17,
        "shader" => 6552,
        "datapack" => 6945,
        _ => 6
    };

    /// <summary>
    /// The instance subdirectory receiving the installed addon.
    /// </summary>
    private string AddonDestinationDirectory => _addonCategory switch
    {
        "resourcepack" => "resourcepacks",
        "world" => "saves",
        "shader" => "shaderpacks",
        "datapack" => "datapacks",
        _ => "mods"
    };

    /// <summary>
    /// Creates the repository selected in the mod panel.
    /// </summary>
    private RemoteAddonRepositoryEx CreateModRepository() =>
        SelectedModRepository == "CurseForge"
            ? new CurseForgeRepository
            {
                ApiKey = _owner.SettingsService.Settings.CurseForgeApiKey,
                ClassId = CurseForgeClassId
            }
            : new ModrinthRepository { ProjectType = ModrinthProjectType };

    /// <summary>
    /// The modpacks found in the remote repositories.
    /// </summary>
    public ObservableCollection<RemoteAddon> RemoteModpackResults { get; } = new();

    /// <summary>
    /// The modpack selected in the list of the modpacks.
    /// </summary>
    public RemoteAddon? SelectedRemoteModpack
    {
        get => _selectedRemoteModpack;
        set
        {
            if (Set(ref _selectedRemoteModpack, value))
                InstallSelectedModpackCommand.RaiseCanExecuteChanged();
        }
    }

    private RemoteAddon? _selectedRemoteModpack;

    /// <summary>
    /// The query used to search the modpacks in the remote repositories.
    /// </summary>
    public string ModpackSearchQuery { get; set; } = string.Empty;

    /// <summary>
    /// A command showing the description of the selected modpack.
    /// </summary>
    public RelayCommand ShowModpackDescriptionCommand { get; }

    /// <summary>
    /// Raised when the description of a modpack is shown to the user.
    /// </summary>
    public event Action<RemoteAddon>? ModpackDescriptionRequested;

    /// <summary>
    /// Asks the user interface to show the description of the selected modpack.
    /// </summary>
    private void ShowModpackDescription()
    {
        if (SelectedRemoteModpack is { } modpack)
            ModpackDescriptionRequested?.Invoke(modpack);
    }

    /// <summary>
    /// Searches the modpacks in the remote repositories and fills the result
    /// list.
    /// </summary>
    private async Task SearchModpacksAsync()
    {
        if (string.IsNullOrWhiteSpace(ModpackSearchQuery))
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.738_");
            return;
        }

        // The CurseForge API needs a key, so the Modrinth repository is used
        // when no key is configured.
        RemoteAddonRepositoryEx repository = _owner.Settings.CurseForgeApiKey is { Length: > 0 } key
            ? new CurseForgeRepository { ApiKey = key }
            : new ModrinthRepository { ProjectType = "modpack" };

        IsBusy = true;
        ModInstallStatus = $"正在搜索整合包 {repository.Name}...";
        try
        {
            var results = await repository.SearchAsync(ModpackSearchQuery);
            RemoteModpackResults.Clear();
            foreach (var addon in results)
                RemoteModpackResults.Add(addon);

            ModInstallStatus = $"找到 {results.Count} 个整合包";
        }
        catch (Exception ex)
        {
            ModInstallStatus = $"搜索整合包失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Downloads the selected modpack and installs it as a new instance.
    /// </summary>
    private async Task InstallSelectedModpackAsync()
    {
        if (SelectedRemoteModpack is null)
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.742_");
            return;
        }

        RemoteAddonRepositoryEx repository = _owner.Settings.CurseForgeApiKey is { Length: > 0 } key
            ? new CurseForgeRepository { ApiKey = key }
            : new ModrinthRepository { ProjectType = "modpack" };

        IsBusy = true;
        ModInstallStatus = $"正在下载整合包 {SelectedRemoteModpack.Name}...";
        try
        {
            var resolved = await repository.ResolveFileAsync(
                SelectedRemoteModpack, string.Empty, ModLoaderType.None);

            if (resolved?.DownloadUrl is not { } downloadUrl)
            {
                ModInstallStatus = "未找到整合包的下载地址";
                return;
            }

            var directory = Path.Combine(Path.GetTempPath(), "hmcl-modpacks");
            Directory.CreateDirectory(directory);
            var archive = Path.Combine(directory, $"{resolved.Id}.mrpack");

            var task = new HMCLCore.NET.Download.GameDownloadTask(new Uri(downloadUrl), archive);
            await task.Run();

            var instanceId = await _owner.InstallModpackAsync(archive);
            ModInstallStatus = instanceId is null
                ? "整合包安装失败"
                : $"整合包已安装为实例 {instanceId}";
        }
        catch (Exception ex)
        {
            ModInstallStatus = $"安装整合包失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Searches the selected repository and fills the result list.
    /// </summary>
    private async Task SearchModsAsync()    {
        if (_owner.SelectedInstance == null)
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.736_");
            return;
        }

        if (string.IsNullOrWhiteSpace(ModSearchQuery))
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.738_");
            return;
        }

        var repository = CreateModRepository();
        if (repository is CurseForgeRepository { ApiKey: "" or null })
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.740_");
            return;
        }

        IsBusy = true;
        ModInstallStatus = $"正在搜索 {repository.Name}...";
        try
        {
            var results = await repository.SearchAsync(ModSearchQuery);
            RemoteModResults.Clear();
            foreach (var addon in results)
                RemoteModResults.Add(addon);
            ModInstallStatus = $"找到 {results.Count} 个结果";
        }
        catch (Exception ex)
        {
            ModInstallStatus = $"搜索失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Resolves the selected mod for the instance's game version and installs it.
    /// </summary>
    private async Task InstallSelectedModAsync()
    {
        var instance = _owner.SelectedInstance;
        if (instance == null || SelectedRemoteMod == null)
        {
            ModInstallStatus = Lang.Get("hmcl.mainwindowviewmodel.742_");
            return;
        }

        var repository = CreateModRepository();
        IsBusy = true;
        ModInstallStatus = $"正在安装 {SelectedRemoteMod.Name}...";
        try
        {
            var resolved = await repository.ResolveFileAsync(
                SelectedRemoteMod, instance.Version, ModLoaderType.None);
            if (resolved?.DownloadUrl == null)
            {
                ModInstallStatus = $"未找到适用于 {instance.Version} 的版本";
                return;
            }

            var modsDirectory = Path.Combine(instance.GameDirectory, AddonDestinationDirectory);
            var task = new ModInstallTask(resolved, modsDirectory, instance.Version);
            await _owner.TaskManager.RunAsync(task);
            ModInstallStatus = $"已安装 {resolved.Name} 到实例 {instance.Id}";
        }
        catch (Exception ex)
        {
            ModInstallStatus = $"安装失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// The version selected in the list.
    /// </summary>
    public VersionMetadata? SelectedVersion
    {
        get => _selectedVersion;
        set
        {
            if (Set(ref _selectedVersion, value))
            {
                DownloadCommand.RaiseCanExecuteChanged();

                // The installers of the loaders are listed for the selected
                // version of the game.
                _ = LoadInstallersAsync();
            }
        }
    }


    /// <summary>
    /// The status message shown on the download page.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => Set(ref _statusMessage, value);
    }

    /// <summary>
    /// Whether a remote operation (refresh or download) is running.
    /// </summary>
    public bool IsBusy
    {
        get => _isBusy;
        set
        {
            if (Set(ref _isBusy, value))
            {
                RefreshCommand.RaiseCanExecuteChanged();
                DownloadCommand.RaiseCanExecuteChanged();
            }
        }
    }

    /// <summary>
    /// The state of the list of the versions, shown by the loading pane of the
    /// page.
    /// </summary>
    public LoadingState LoadState
    {
        get => _loadState;
        private set => Set(ref _loadState, value);
    }

    private LoadingState _loadState = LoadingState.Loading;

    /// <summary>
    /// The message shown by the loading pane when the content cannot be
    /// loaded.
    /// </summary>
    public string? LoadError
    {
        get => _loadError;
        private set => Set(ref _loadError, value);
    }

    private string? _loadError;

    /// <summary>
    /// Command that refreshes the version list from Mojang.
    /// </summary>
    public RelayCommand RefreshCommand { get; }

    /// <summary>
    /// Command that installs the version given as the parameter of the
    /// command, like the button of the versions of Java HMCL.
    /// </summary>
    public RelayCommand DownloadVersionCommand { get; }

    /// <summary>
    /// Command that loads the installers of the selected version of the game.
    /// </summary>
    public RelayCommand LoadInstallersCommand { get; }

    /// <summary>
    /// Command that installs the selected loader and creates the instance
    /// using it.
    /// </summary>
    public RelayCommand InstallLoaderCommand { get; }

    /// <summary>
    /// Command that downloads and installs the selected version.
    /// </summary>
    public RelayCommand DownloadCommand { get; }

    /// <summary>
    /// Loads the cached remote manifest if it exists, otherwise fetches a fresh one.
    /// </summary>
    public async Task RefreshVersionsAsync()
    {
        IsBusy = true;
        LoadState = LoadingState.Loading;
        LoadError = null;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.744_");

        try
        {
            // Route requests through the mirror configured in the settings.
            DownloadSourceUtils.Apply(_owner.SettingsService.Settings);

            var manifestPath = Path.Combine(_cacheDirectory, "version_manifest.json");

            var fetchTask = new VersionRemoteFetchTask(manifestPath);
            fetchTask.ProgressChanged += (_, _) => { };
            await _owner.TaskManager.RunAsync(fetchTask);

            Versions.Clear();
            foreach (var version in fetchTask.Versions
                         .OrderByDescending(v => v, VersionMetadataComparer.Instance))
                Versions.Add(version);

            ApplyFilter();
            LoadState = LoadingState.Loaded;
            StatusMessage = $"已获取 {Versions.Count} 个版本";
        }
        catch (Exception ex)
        {
            LoadState = LoadingState.Failed;
            LoadError = $"获取版本列表失败: {ex.Message}";
            StatusMessage = LoadError;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Downloads the version JSON for the selected version and installs it as an instance.
    /// </summary>
    public async Task DownloadVersionAsync()
    {
        if (SelectedVersion is not { } version)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.746_");
            return;
        }

        IsBusy = true;
        StatusMessage = $"正在下载版本 {version.Id}...";

        try
        {
            // Route requests through the mirror configured in the settings.
            DownloadSourceUtils.Apply(_owner.SettingsService.Settings);

            var jsonPath = Path.Combine(_cacheDirectory, "vjson", $"{version.Id}.json");

            var downloadTask = new VersionJsonDownloadTask(new Uri(version.Url), jsonPath);
            await _owner.TaskManager.RunAsync(downloadTask);

            var json = File.ReadAllText(jsonPath);

            // Keep a repository-standard copy so loader installers and
            // inheritance resolution can find the parent version.
            var versionDirectory = Path.Combine(_owner.BaseDirectory, "versions", version.Id);
            Directory.CreateDirectory(versionDirectory);
            File.WriteAllText(Path.Combine(versionDirectory, $"{version.Id}.json"), json);

            var manifest = VersionJsonParser.Parse(json, _owner.LoadVersionJson);

            if (manifest == null)
                throw new InvalidDataException(Lang.Get("hmcl.mainwindowviewmodel.748_"));

            _owner.SaveInstance(version.Id, manifest);
            StatusMessage = $"版本 {version.Id} 已安装";
        }
        catch (Exception ex)
        {
            StatusMessage = $"版本安装失败: {ex.Message}";
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Rebuilds the filtered view from the raw version list.
    /// </summary>
    private void ApplyFilter()
    {
        FilteredVersions.Clear();

        // Type categories mirror Java HMCL: 全部 / 正式版 / 快照版 / 愚人节 / 旧版.
        bool MatchesType(VersionMetadata v)
        {
            if (TypeFilter == Lang.Get("hmcl.mainwindowviewmodel.instance_game_release_"))
                return v.IsRelease;

            if (TypeFilter == Lang.Get("hmcl.mainwindowviewmodel.752_"))
                return v.IsSnapshot;

            if (TypeFilter == Lang.Get("hmcl.mainwindowviewmodel.instance_game_april_fools_"))
                return v.IsAprilFools;

            if (TypeFilter == Lang.Get("hmcl.mainwindowviewmodel.756_"))
                return v.IsOldBeta || v.IsOldAlpha;

            return true;
        }

        foreach (var version in Versions)
        {
            if (!MatchesType(version))
                continue;

            if (!string.IsNullOrWhiteSpace(SearchText) &&
                !version.Id.Contains(SearchText, StringComparison.OrdinalIgnoreCase))
                continue;

            FilteredVersions.Add(version);
        }
    }
}

/// <summary>
/// View model for the login/authentication screen.
/// </summary>
public sealed class LoginViewModel : ViewModelBase
{
    private readonly AccountManager _accountManager;

    private string _username = string.Empty;
    private string _password = string.Empty;
    private string _serverUrl = string.Empty;
    private string _authorizationCode = string.Empty;
    private string _statusMessage = string.Empty;
    private bool _isLoggingIn;

    /// <summary>
    /// Raised after an account has been authenticated and registered.
    /// Carries the newly created account.
    /// </summary>
    public event Action<Account>? LoginSucceeded;

    /// <summary>
    /// The text describing the logged-in account for display purposes.
    /// </summary>
    public string LoginTitle => "Login";

    /// <summary>
    /// The account registered by the latest successful login, if it exists.
    /// </summary>
    public Account? LastLoginAccount { get; private set; }

    /// <summary>
    /// The username entered by the user.
    /// </summary>
    public string Username
    {
        get => _username;
        set => Set(ref _username, value);
    }

    /// <summary>
    /// The password entered by the user.
    /// </summary>
    public string Password
    {
        get => _password;
        set => Set(ref _password, value);
    }

    /// <summary>
    /// The server URL for Yggdrasil/AuthlibInjector authentication.
    /// </summary>
    public string ServerUrl
    {
        get => _serverUrl;
        set => Set(ref _serverUrl, value);
    }

    /// <summary>
    /// The pasted Microsoft OAuth authorization code or callback URL.
    /// </summary>
    public string AuthorizationCode
    {
        get => _authorizationCode;
        set => Set(ref _authorizationCode, value);
    }

    /// <summary>
    /// The current status message.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => Set(ref _statusMessage, value);
    }

    /// <summary>
    /// Whether a login operation is in progress.
    /// </summary>
    public bool IsLoggingIn
    {
        get => _isLoggingIn;
        set => Set(ref _isLoggingIn, value);
    }

    /// <summary>
    /// Creates a new LoginViewModel.
    /// </summary>
    public LoginViewModel(AccountManager accountManager)
    {
        _accountManager = accountManager ?? throw new ArgumentNullException(nameof(accountManager));
    }

    /// <summary>
    /// Logs in with an offline account.
    /// </summary>
    public void LoginOffline()
    {
        if (string.IsNullOrWhiteSpace(Username))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.758_");
            return;
        }

        IsLoggingIn = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.760_");

        try
        {
            var account = new HMCLCore.NET.Auth.OfflineAccount(Username);
            _accountManager.AddAccount(account);
            _accountManager.SelectAccount(account);
            LastLoginAccount = account;
            StatusMessage = $"离线账户 {Username} 已创建";
            LoginSucceeded?.Invoke(account);
        }
        catch (Exception ex)
        {
            StatusMessage = $"创建失败: {ex.Message}";
        }
        finally
        {
            IsLoggingIn = false;
        }
    }

    /// <summary>
    /// Logs in with a Microsoft account using the OAuth code the user
    /// pastes from the browser callback. Covers the full login chain:
    /// code → tokens → Xbox Live → XSTS → Minecraft session → profile.
    /// </summary>
    public async Task LoginMicrosoftAsync()
    {
        if (string.IsNullOrWhiteSpace(AuthorizationCode))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.762_");
            return;
        }

        IsLoggingIn = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.764_");

        try
        {
            var service = new HMCLCore.NET.Auth.Microsoft.MicrosoftLoginService();
            var session = await service.LoginWithCodeAsync(ParseAuthorizationCode(AuthorizationCode));

            if (session == null)
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.766_");
                return;
            }

            var account = new HMCLCore.NET.Auth.Microsoft.MicrosoftAccount(session.ProfileName);
            account.SetTokens(session.MicrosoftAccessToken,
                session.MicrosoftRefreshToken, null);
            account.SetXboxTokens(session.XblToken, session.XstsToken);
            account.SetMinecraftSession(session.MinecraftAccessToken,
                session.ProfileUuid, session.ProfileName);

            _accountManager.AddAccount(account);
            _accountManager.SelectAccount(account);
            LastLoginAccount = account;
            StatusMessage = $"Microsoft 账户 {session.ProfileName} 已登录";
            LoginSucceeded?.Invoke(account);
        }
        catch (Exception ex)
        {
            StatusMessage = $"登录失败: {ex.Message}";
        }
        finally
        {
            IsLoggingIn = false;
        }
    }

    /// <summary>
    /// Opens the Microsoft authorization page in the default browser.
    /// </summary>
    public void OpenAuthorizationPage()
    {
        try
        {
            var url = new HMCLCore.NET.Auth.Microsoft.MicrosoftLoginService()
                .GetAuthorizationUrl(HMCLCore.NET.Auth.OAuth.GenerateState());

            var psi = new System.Diagnostics.ProcessStartInfo
            {
                FileName = url,
                UseShellExecute = true
            };
            System.Diagnostics.Process.Start(psi);
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.768_");
        }
        catch (Exception ex)
        {
            StatusMessage = $"打开浏览器失败: {ex.Message}";
        }
    }

    /// <summary>
    /// Extracts the authorization code from a pasted value which may be either
    /// a bare code or the full redirect URL containing the code parameter.
    /// </summary>
    private static string ParseAuthorizationCode(string input)
    {
        var trimmed = input.Trim();
        var marker = trimmed.IndexOf("code=", StringComparison.OrdinalIgnoreCase);
        if (marker < 0)
            return trimmed;

        var start = marker + "code=".Length;
        var end = trimmed.IndexOf('&', start);
        return end < 0 ? trimmed[start..] : trimmed[start..end];
    }

    /// <summary>
    /// Logs in with a Yggdrasil account.
    /// </summary>
    public async Task LoginYggdrasilAsync()
    {
        if (string.IsNullOrWhiteSpace(Username) || string.IsNullOrWhiteSpace(Password) || string.IsNullOrWhiteSpace(ServerUrl))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.770_");
            return;
        }

        IsLoggingIn = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.772_");

        try
        {
            var service = new HMCLCore.NET.Auth.Yggdrasil.YggdrasilAuthService();
            var response = await service.AuthenticateAsync(ServerUrl, Username, Password);

            if (response == null)
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.774_");
                return;
            }

            _accessToken = response.AccessToken;
            _clientToken = response.ClientToken;

            // The accounts owning several characters let the user choose the
            // character played by the launcher.
            AvailableProfiles.Clear();
            foreach (var profile in response.AvailableProfiles)
                AvailableProfiles.Add(profile);

            if (AvailableProfiles.Count > 1 && response.SelectedProfile == null)
            {
                SelectedProfile = AvailableProfiles[0];
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.776_");
                HasCharacterSelection = true;
                return;
            }

            var selected = response.SelectedProfile ?? AvailableProfiles.FirstOrDefault();
            if (selected != null)
                CompleteYggdrasilLogin(Username, ServerUrl, response.AccessToken, response.ClientToken, selected);
            else
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.778_");
        }
        catch (Exception ex)
        {
            StatusMessage = $"验证失败: {ex.Message}";
        }
        finally
        {
            IsLoggingIn = false;
        }
    }

    /// <summary>
    /// The characters of the account, shown when the account owns several of
    /// them.
    /// </summary>
    public ObservableCollection<HMCLCore.NET.Auth.YggdrasilProfile> AvailableProfiles { get; } = new();

    /// <summary>
    /// The character chosen by the user.
    /// </summary>
    public HMCLCore.NET.Auth.YggdrasilProfile? SelectedProfile
    {
        get => _selectedProfile;
        set => Set(ref _selectedProfile, value);
    }

    private HMCLCore.NET.Auth.YggdrasilProfile? _selectedProfile;

    /// <summary>
    /// Whether the user has to choose the character played by the launcher.
    /// </summary>
    public bool HasCharacterSelection
    {
        get => _hasCharacterSelection;
        set => Set(ref _hasCharacterSelection, value);
    }

    private bool _hasCharacterSelection;

    /// <summary>
    /// Uses the chosen character of the account and finishes the login.
    /// </summary>
    public async Task SelectCharacterAsync()
    {
        if (SelectedProfile is null)
            return;

        IsLoggingIn = true;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.780_");

        try
        {
            var service = new HMCLCore.NET.Auth.Yggdrasil.YggdrasilAuthService();
            var response = await service.RefreshAsync(ServerUrl, _accessToken ?? string.Empty, _clientToken ?? string.Empty);

            if (response == null || response.AccessToken == null)
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.782_");
                return;
            }

            CompleteYggdrasilLogin(Username, ServerUrl, response.AccessToken, response.ClientToken, SelectedProfile);
        }
        catch (Exception ex)
        {
            StatusMessage = $"选择角色失败: {ex.Message}";
        }
        finally
        {
            IsLoggingIn = false;
        }
    }

    private string? _accessToken;
    private string? _clientToken;

    /// <summary>
    /// Adds the account of the Yggdrasil login to the launcher.
    /// </summary>
    /// <param name="username">The name of the account.</param>
    /// <param name="serverUrl">The address of the authentication server.</param>
    /// <param name="accessToken">The access token of the account.</param>
    /// <param name="clientToken">The client token of the account.</param>
    /// <param name="profile">The character played by the launcher.</param>
    private void CompleteYggdrasilLogin(
        string username, string serverUrl, string accessToken, string? clientToken,
        HMCLCore.NET.Auth.YggdrasilProfile profile)
    {
        _accessToken = accessToken;
        _clientToken = clientToken;

        var account = new HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount(username, serverUrl);
        account.SetYggdrasilTokens(accessToken, clientToken ?? string.Empty);
        account.SetProfile(profile.Id, profile.Name);
        _accountManager.AddAccount(account);
        _accountManager.SelectAccount(account);
        LastLoginAccount = account;

        HasCharacterSelection = false;
        StatusMessage = $"Yggdrasil 账户 {username} 已登录";
        EventBus.Instance.Publish(new AccountEvent(
            account.AccountType, account.Username, AccountEventType.LoggedIn));
        LoginSucceeded?.Invoke(account);
    }
}

/// <summary>
/// View model for the settings screen.
/// </summary>
public sealed class SettingsViewModel : ViewModelBase
{
    private readonly SettingsManager _settingsManager;

    /// <summary>
    /// The Java manager used by the runtime auto-detection button.
    /// </summary>
    private readonly JavaManager? _javaManager;

    private int _memory;
    private string _javaPath = string.Empty;
    private bool _useCustomJvmArgs;
    private string _customJvmArgs = string.Empty;
private int _windowWidth;
    private int _windowHeight;
    private DownloadSourceOption _versionListSource;
    private DownloadSourceOption _fileDownloadSource;
    private bool _autoDownloadThreads = true;
    private int _downloadThreads = 8;

    /// <summary>
    /// The download source names offered in the settings UI.
    /// </summary>
    public IReadOnlyList<DownloadSourceOption> DownloadSources { get; } = new List<DownloadSourceOption>
    {
        new(DownloadSource.Default, Lang.Get("hmcl.mainwindowviewmodel.784_")),
        new(DownloadSource.Official, Lang.Get("hmcl.mainwindowviewmodel.786_")),
        new(DownloadSource.Mirror, Lang.Get("hmcl.mainwindowviewmodel.788_"))
    };

    /// <summary>
    /// The Java runtime selection types offered in the game settings UI.
    /// </summary>
    public IReadOnlyList<JavaVersionType> JavaVersionTypes { get; } =
        new List<JavaVersionType>
        {
            JavaVersionType.Auto,
            JavaVersionType.Version,
            JavaVersionType.Detected,
            JavaVersionType.Custom
        };

    /// <summary>
    /// The proxy types offered in the settings UI. The socks protocols are
    /// applied to the download clients of the launcher; the legacy web
    /// request stack of .NET only supports the http and https protocols.
    /// </summary>
    public IReadOnlyList<string> ProxyTypes { get; } = new List<string> { "http", "https", "socks5", "socks4" };

    /// <summary>
    /// The process priorities offered in the game settings UI.
    /// </summary>
    public IReadOnlyList<HMCLCore.NET.Setting.ProcessPriority> ProcessPriorities { get; } =
        new List<HMCLCore.NET.Setting.ProcessPriority>
        {
            HMCLCore.NET.Setting.ProcessPriority.High,
            HMCLCore.NET.Setting.ProcessPriority.AboveNormal,
            HMCLCore.NET.Setting.ProcessPriority.Normal,
            HMCLCore.NET.Setting.ProcessPriority.BelowNormal,
            HMCLCore.NET.Setting.ProcessPriority.Low
        };

    /// <summary>
    /// The theme names offered in the settings UI.
    /// </summary>
    public IReadOnlyList<string> Themes { get; } = new List<string>(AppearanceManager.Themes);

    /// <summary>
    /// The languages offered in the settings UI.
    /// </summary>
    public IReadOnlyList<LanguageInfo> Languages { get; } = LanguageManager.AvailableLanguages;

    /// <summary>
    /// The language selected in the settings UI.
    /// </summary>
    public LanguageInfo? SelectedLanguage
    {
        get => _selectedLanguage;
        set
        {
            if (value is not null && Set(ref _selectedLanguage, value))
                Language = value.Id;
        }
    }

    private LanguageInfo? _selectedLanguage;

    /// <summary>
    /// The identifier of the language used by the user interface.
    /// </summary>
    /// <remarks>
    /// Changing the language applies the localized resources immediately, so
    /// the user interface is translated without restarting the launcher.
    /// </remarks>
    public string Language
    {
        get => _language;
        set
        {
            var resolved = LanguageManager.Resolve(value);
            if (!Set(ref _language, resolved))
                return;

            var language = Languages.FirstOrDefault(item => item.Id == resolved);
            if (language is not null)
                Set(ref _selectedLanguage, language);

            if (!string.Equals(resolved, LanguageManager.CurrentLanguage, StringComparison.Ordinal))
                LanguageManager.Apply(resolved);
        }
    }

    private string _language = LanguageManager.ResolveSystemLanguage();

    /// <summary>
    /// The update channels offered in the settings UI.
    /// </summary>
    public IReadOnlyList<string> UpdateChannels { get; } = new List<string> { "stable", "development" };

    /// <summary>
    /// The update channel used to look for new versions.
    /// </summary>
    public string UpdateChannel
    {
        get => _updateChannel;
        set => Set(ref _updateChannel, value);
    }

    private string _updateChannel = "stable";

    /// <summary>
    /// Whether the April Fools' mode of the launcher is disabled.
    /// </summary>
    public bool DisableAprilFools
    {
        get => _disableAprilFools;
        set => Set(ref _disableAprilFools, value);
    }

    private bool _disableAprilFools;

    /// <summary>
    /// Whether the launcher shows the diagnostic information of the game
    /// processes it starts.
    /// </summary>
    public bool Debug
    {
        get => _debug;
        set => Set(ref _debug, value);
    }

    private bool _debug;

    /// <summary>
    /// Whether the anonymous usage statistics are sent to the developer of the
    /// launcher.
    /// </summary>
    public bool EnableAnalytics
    {
        get => _enableAnalytics;
        set => Set(ref _enableAnalytics, value);
    }

    private bool _enableAnalytics;

    /// <summary>
    /// The address of the server receiving the anonymous usage statistics.
    /// </summary>
    public string AnalyticsServerUrl
    {
        get => _analyticsServerUrl;
        set => Set(ref _analyticsServerUrl, value);
    }

    private string _analyticsServerUrl = string.Empty;

    /// <summary>
    /// The key of the application on the server receiving the anonymous usage
    /// statistics.
    /// </summary>
    public string AnalyticsAppKey
    {
        get => _analyticsAppKey;
        set => Set(ref _analyticsAppKey, value);
    }

    private string _analyticsAppKey = string.Empty;

    private string _selectedTheme = AppearanceManager.SystemTheme;

    /// <summary>
    /// The selected UI theme.
    /// </summary>
    public string SelectedTheme
    {
        get => _selectedTheme;
        set
        {
            if (Set(ref _selectedTheme, value))
                ApplyTheme(value);
        }
    }

    /// <summary>
    /// The interface font family, empty when the system font is used.
    /// </summary>
    public string FontFamily
    {
        get => _fontFamily;
        set => Set(ref _fontFamily, value);
    }

    private string _fontFamily = string.Empty;

    /// <summary>
    /// The base interface font size in points.
    /// </summary>
    public double FontSize
    {
        get => _fontSize;
        set => Set(ref _fontSize, value);
    }

    private double _fontSize = 13;

    /// <summary>
    /// The custom theme color of the launcher, empty when the color of the
    /// system is used.
    /// </summary>
    public string ThemeColor
    {
        get => _themeColor;
        set => Set(ref _themeColor, value);
    }

    private string _themeColor = string.Empty;

    /// <summary>
    /// The kind of the background of the launcher.
    /// </summary>
    public BackgroundType BackgroundType
    {
        get => _backgroundType;
        set
        {
            if (Set(ref _backgroundType, value))
                RaiseAppearanceChanged();
        }
    }

    private BackgroundType _backgroundType = BackgroundType.None;

    /// <summary>
    /// The kinds of the background offered in the settings UI.
    /// </summary>
    public IReadOnlyList<NamedOption<BackgroundType>> BackgroundTypes { get; } = new List<NamedOption<BackgroundType>>
    {
        new(BackgroundType.Default, Lang.Get("launcher.background.default", "默认壁纸")),
        new(BackgroundType.None, Lang.Get("launcher.background.none", "无")),
        new(BackgroundType.Image, Lang.Get("launcher.background.image", "本地图片")),
        new(BackgroundType.Network, Lang.Get("launcher.background.network", "网络图片")),
        new(BackgroundType.Builtin, Lang.Get("launcher.background.builtin", "内置壁纸")),
        new(BackgroundType.Color, Lang.Get("launcher.background.color", "纯色"))
    };

    /// <summary>
    /// The built-in wallpapers offered in the settings UI.
    /// </summary>
    public IReadOnlyList<NamedOption<string>> BuiltinWallpapers { get; } = new List<NamedOption<string>>
    {
        new(string.Empty, Lang.Get("launcher.background.none", "无")),
        new(HMCLCore.NET.Setting.ThemeColorScheme.GetWallpaperId(HMCLCore.NET.Setting.BuiltinWallpaper.Wallpaper20210826),
            Lang.Get("launcher.wallpaper.2021_08_26", "2021-08-26")),
        new(HMCLCore.NET.Setting.ThemeColorScheme.GetWallpaperId(HMCLCore.NET.Setting.BuiltinWallpaper.Wallpaper20160225),
            Lang.Get("launcher.wallpaper.2016_02_25", "2016-02-25")),
        new(HMCLCore.NET.Setting.ThemeColorScheme.GetWallpaperId(HMCLCore.NET.Setting.BuiltinWallpaper.Wallpaper20150622),
            Lang.Get("launcher.wallpaper.2015_06_22", "2015-06-22"))
    };

    /// <summary>
    /// The identifier of the built-in wallpaper used as the background of the
    /// launcher.
    /// </summary>
    public string BuiltinWallpaperId
    {
        get => _builtinWallpaperId;
        set
        {
            if (Set(ref _builtinWallpaperId, value))
                RaiseAppearanceChanged();
        }
    }

    private string _builtinWallpaperId = string.Empty;

    /// <summary>
    /// The built-in wallpaper chosen in the settings UI.
    /// </summary>
    public NamedOption<string>? SelectedBuiltinWallpaper
    {
        get => _selectedBuiltinWallpaper;
        set
        {
            if (value is null)
                return;

            if (Set(ref _selectedBuiltinWallpaper, value))
            {
                BuiltinWallpaperId = value.Value;
                RaiseAppearanceChanged();
            }
        }
    }

    private NamedOption<string>? _selectedBuiltinWallpaper;

    /// <summary>
    /// The color styles offered in the settings UI.
    /// </summary>
    public IReadOnlyList<NamedOption<HMCLCore.NET.Setting.ThemeColorStyle>> ThemeColorStyles { get; } =
        new List<NamedOption<HMCLCore.NET.Setting.ThemeColorStyle>>
        {
            new(HMCLCore.NET.Setting.ThemeColorStyle.Default, Lang.Get("settings.launcher.theme_color_style.default", "默认")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.Fidelity, Lang.Get("settings.launcher.theme_color_style.fidelity", "忠实")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.TonalSpot, Lang.Get("settings.launcher.theme_color_style.tonal_spot", "柔和")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.Vibrant, Lang.Get("settings.launcher.theme_color_style.vibrant", "鲜艳")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.Neutral, Lang.Get("settings.launcher.theme_color_style.neutral", "中性")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.FruitSalad, Lang.Get("settings.launcher.theme_color_style.fruit_salad", "水果沙拉")),
            new(HMCLCore.NET.Setting.ThemeColorStyle.Rainbow, Lang.Get("settings.launcher.theme_color_style.rainbow", "彩虹"))
        };

    /// <summary>
    /// The color style used to derive the colors of the launcher.
    /// </summary>
    public NamedOption<HMCLCore.NET.Setting.ThemeColorStyle>? SelectedThemeColorStyle
    {
        get => _selectedThemeColorStyle;
        set
        {
            if (value is null)
                return;

            if (Set(ref _selectedThemeColorStyle, value))
            {
                ThemeColorStyle = value.Value;
                RaiseAppearanceChanged();
            }
        }
    }

    private NamedOption<HMCLCore.NET.Setting.ThemeColorStyle>? _selectedThemeColorStyle;

    /// <summary>
    /// The color style used to derive the colors of the launcher.
    /// </summary>
    public HMCLCore.NET.Setting.ThemeColorStyle ThemeColorStyle
    {
        get => _themeColorStyle;
        set
        {
            if (Set(ref _themeColorStyle, value))
                RaiseAppearanceChanged();
        }
    }

    private HMCLCore.NET.Setting.ThemeColorStyle _themeColorStyle = HMCLCore.NET.Setting.ThemeColorStyle.Default;

    /// <summary>
    /// The contrast levels offered in the settings UI.
    /// </summary>
    public IReadOnlyList<NamedOption<HMCLCore.NET.Setting.ThemeContrast>> ThemeContrasts { get; } =
        new List<NamedOption<HMCLCore.NET.Setting.ThemeContrast>>
        {
            new(HMCLCore.NET.Setting.ThemeContrast.Default, Lang.Get("settings.launcher.contrast.default", "默认")),
            new(HMCLCore.NET.Setting.ThemeContrast.Low, Lang.Get("settings.launcher.contrast.low", "低")),
            new(HMCLCore.NET.Setting.ThemeContrast.Medium, Lang.Get("settings.launcher.contrast.medium", "中")),
            new(HMCLCore.NET.Setting.ThemeContrast.High, Lang.Get("settings.launcher.contrast.high", "高"))
        };

    /// <summary>
    /// The contrast level of the colors of the launcher.
    /// </summary>
    public NamedOption<HMCLCore.NET.Setting.ThemeContrast>? SelectedThemeContrast
    {
        get => _selectedThemeContrast;
        set
        {
            if (value is null)
                return;

            if (Set(ref _selectedThemeContrast, value))
            {
                ThemeContrast = value.Value;
                RaiseAppearanceChanged();
            }
        }
    }

    private NamedOption<HMCLCore.NET.Setting.ThemeContrast>? _selectedThemeContrast;

    /// <summary>
    /// The contrast level of the colors of the launcher.
    /// </summary>
    public HMCLCore.NET.Setting.ThemeContrast ThemeContrast
    {
        get => _themeContrast;
        set
        {
            if (Set(ref _themeContrast, value))
                RaiseAppearanceChanged();
        }
    }

    private HMCLCore.NET.Setting.ThemeContrast _themeContrast = HMCLCore.NET.Setting.ThemeContrast.Default;

    /// <summary>
    /// Whether the animations of the user interface are disabled.
    /// </summary>
    public bool DisableAnimations
    {
        get => _disableAnimations;
        set
        {
            if (Set(ref _disableAnimations, value))
                RaiseAppearanceChanged();
        }
    }

    private bool _disableAnimations;

    /// <summary>
    /// The ways the text of the user interface can be rendered.
    /// </summary>
    public IReadOnlyList<NamedOption<string>> TextRenderingModes { get; } = new List<NamedOption<string>>
    {
        new("default", Lang.Get("settings.launcher.text_rendering.default", "默认")),
        new("antialias", Lang.Get("settings.launcher.text_rendering.antialias", "抗锯齿")),
        new("subpixel", Lang.Get("settings.launcher.text_rendering.subpixel", "次像素")),
        new("alias", Lang.Get("settings.launcher.text_rendering.alias", "不抗锯齿"))
    };

    /// <summary>
    /// The way the text of the user interface is rendered.
    /// </summary>
    public NamedOption<string>? SelectedTextRenderingMode
    {
        get => _selectedTextRenderingMode;
        set
        {
            if (value is null)
                return;

            if (Set(ref _selectedTextRenderingMode, value))
            {
                TextRenderingMode = value.Value;
                RaiseAppearanceChanged();
            }
        }
    }

    private NamedOption<string>? _selectedTextRenderingMode;

    /// <summary>
    /// The way the text of the user interface is rendered.
    /// </summary>
    public string TextRenderingMode
    {
        get => _textRenderingMode;
        set
        {
            if (Set(ref _textRenderingMode, value))
                RaiseAppearanceChanged();
        }
    }

    private string _textRenderingMode = "default";

    /// <summary>
    /// The kind of the background of the launcher.
    /// </summary>
    public NamedOption<BackgroundType>? SelectedBackgroundType
    {
        get => _selectedBackgroundType;
        set
        {
            if (value is null)
                return;

            if (Set(ref _selectedBackgroundType, value))
            {
                BackgroundType = value.Value;
                RaiseAppearanceChanged();
            }
        }
    }

    private NamedOption<BackgroundType>? _selectedBackgroundType;

    /// <summary>
    /// The opacity of the background image of the launcher, between 0 and 1.
    /// </summary>
    public double BackgroundOpacity
    {
        get => _backgroundOpacity;
        set => Set(ref _backgroundOpacity, value);
    }

    private double _backgroundOpacity = 0.5;

    /// <summary>
    /// The address of the background image downloaded from the network.
    /// </summary>
    public string NetworkBackgroundImageUrl
    {
        get => _networkBackgroundImageUrl;
        set => Set(ref _networkBackgroundImageUrl, value);
    }

    private string _networkBackgroundImageUrl = string.Empty;

    /// <summary>
    /// Whether the title bar of the launcher is transparent.
    /// </summary>
    public bool TitleBarTransparent
    {
        get => _titleBarTransparent;
        set => Set(ref _titleBarTransparent, value);
    }

    private bool _titleBarTransparent = true;

    /// <summary>
    /// The custom theme color of the launcher, empty when the color of the
    /// system is used.
    /// </summary>
    public string BackgroundImagePath
    {
        get => _backgroundImagePath;
        set => Set(ref _backgroundImagePath, value);
    }

    private string _backgroundImagePath = string.Empty;

    /// <summary>
    /// Whether the launcher window uses transparency effects when supported.
    /// </summary>
    public bool EnableTransparencyEffects
    {
        get => _enableTransparencyEffects;
        set => Set(ref _enableTransparencyEffects, value);
    }

    private bool _enableTransparencyEffects;

    /// <summary>
    /// Exports the appearance settings of the launcher into a theme pack.
    /// </summary>
    /// <param name="destinationPath">The path of the theme pack to write.</param>
    /// <returns>Whether the theme pack was written.</returns>
    public bool ExportThemePack(string destinationPath)
    {
        try
        {
            ThemePackManager.Export(_settingsManager.Settings, destinationPath);
            StatusMessage = $"主题已导出: {destinationPath}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"导出主题失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Imports a theme pack and applies the appearance settings it holds.
    /// </summary>
    /// <param name="archivePath">The path of the theme pack to read.</param>
    /// <returns>Whether the theme was imported.</returns>
    public bool ImportThemePack(string archivePath)
    {
        try
        {
            var directory = Path.Combine(_settingsManager.SettingsFile is { Length: > 0 } file
                ? Path.GetDirectoryName(file) ?? string.Empty
                : string.Empty, "themes");

            var theme = ThemePackManager.Import(archivePath, directory);
            if (theme is null)
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.790_");
                return false;
            }

            _settingsManager.Update(settings =>
            {
                if (!string.IsNullOrEmpty(theme.Theme))
                    settings.Theme = theme.Theme;
                if (!string.IsNullOrEmpty(theme.FontFamily))
                    settings.FontFamily = theme.FontFamily;
                if (theme.FontSize is { } fontSize)
                    settings.FontSize = fontSize;
                if (theme.EnableTransparencyEffects is { } transparency)
                    settings.EnableTransparencyEffects = transparency;
                if (!string.IsNullOrEmpty(theme.BackgroundImagePath))
                    settings.BackgroundImagePath = theme.BackgroundImagePath;

                if (!string.IsNullOrEmpty(theme.Color))
                    settings.ThemeColor = theme.Color;
                if (!string.IsNullOrEmpty(theme.ColorStyle) &&
                    Enum.TryParse<HMCLCore.NET.Setting.ThemeColorStyle>(theme.ColorStyle, true, out var colorStyle))
                {
                    settings.ThemeColorStyle = colorStyle;
                }
                if (!string.IsNullOrEmpty(theme.Contrast) &&
                    Enum.TryParse<HMCLCore.NET.Setting.ThemeContrast>(theme.Contrast, true, out var contrast))
                {
                    settings.ThemeContrast = contrast;
                }
                if (!string.IsNullOrEmpty(theme.BuiltinWallpaperId))
                {
                    settings.BuiltinWallpaperId = theme.BuiltinWallpaperId;
                    settings.BackgroundType = HMCLCore.NET.Setting.BackgroundType.Builtin;
                }
                if (theme.BackgroundOpacity is { } opacity)
                    settings.BackgroundOpacity = opacity;
                if (theme.TitleBarTransparent is { } titleBarTransparent)
                    settings.TitleBarTransparent = titleBarTransparent;
            });

            LoadSettings();
            SaveSettings();
            RaiseAppearanceChanged();
            StatusMessage = $"主题已导入: {theme.Name}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"导入主题失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Notifies the launcher that the appearance settings changed.
    /// </summary>
    private void RaiseAppearanceChanged() => AppearanceChanged?.Invoke();

    /// <summary>
    /// Raised when the appearance settings changed and should be applied to
    /// the running window.
    /// </summary>
    public event Action? AppearanceChanged;

    /// <summary>
    /// Applies the theme to the running application.
    /// </summary>
    /// <param name="theme">The theme name.</param>
    private void ApplyTheme(string theme)
    {
        try
        {
            var app = Avalonia.Application.Current;
            if (app == null) return;
            app.RequestedThemeVariant = AppearanceManager.GetThemeVariant(theme);
        }
        catch
        {
            // Best-effort theming
        }
    }

    /// <summary>
    /// The graphics backends offered in the game settings UI.
    /// </summary>
    public IReadOnlyList<GraphicsBackend> GraphicsBackends { get; } =
        new List<GraphicsBackend>
        {
            GraphicsBackend.Auto,
            GraphicsBackend.OpenGL,
            GraphicsBackend.Vulkan,
            GraphicsBackend.Software
        };

    /// <summary>
    /// The launcher visibility behaviors offered in the game settings UI.
    /// </summary>
    public IReadOnlyList<LauncherVisibility> LauncherVisibilities { get; } =
        new List<LauncherVisibility>
        {
            LauncherVisibility.Hide,
            LauncherVisibility.Close,
            LauncherVisibility.Keep,
            LauncherVisibility.HideAndReopen,
            LauncherVisibility.CloseAndReopen
        };

    /// <summary>
    /// The quick play modes offered in the game settings UI.
    /// </summary>
    public IReadOnlyList<QuickPlayMode> QuickPlayModes { get; } =
        new List<QuickPlayMode>
        {
            QuickPlayMode.None,
            QuickPlayMode.Singleplayer,
            QuickPlayMode.Multiplayer,
            QuickPlayMode.Realms
        };

    /// <summary>
    /// The base directory used for downloaded Java runtimes and game data.
    /// </summary>
    public string BaseDirectory { get; set; } = string.Empty;

    /// <summary>
    /// Detected Java runtimes shown in the Java management tab.
    /// </summary>
    public ObservableCollection<JavaRuntime> JavaRuntimes { get; } = new();

    /// <summary>
    /// The selected Java runtime in the management list.
    /// </summary>
    public JavaRuntime? SelectedJavaRuntime { get; set; }

    /// <summary>
    /// Rebuilds the Java runtime list from the manager.
    /// </summary>
    public void RefreshJavaRuntimes()
    {
        JavaRuntimes.Clear();
        if (_javaManager == null)
            return;

        foreach (var runtime in _javaManager.KnownRuntimes)
            JavaRuntimes.Add(runtime);
    }

    /// <summary>
    /// Registers a detected Java executable and refreshes the list.
    /// </summary>
    /// <param name="javaPath">The java/javaw executable path.</param>
    /// <returns>Whether the runtime was detected and registered.</returns>
    public bool AddJavaRuntime(string javaPath)
    {
        if (_javaManager == null)
            return false;

        var runtime = _javaManager.RegisterRuntime(javaPath);
        if (runtime == null)
            return false;

        RefreshJavaRuntimes();
        SelectedJavaRuntime = runtime;
        return true;
    }

    /// <summary>
    /// Uses the selected runtime as the preferred Java path.
    /// </summary>
    public void UseSelectedJavaRuntime()
    {
        if (SelectedJavaRuntime is { } runtime)
            JavaPath = runtime.Path;
    }

    /// <summary>
    /// Removes the selected runtime from the manager.
    /// </summary>
    public void RemoveSelectedJavaRuntime()
    {
        if (_javaManager == null || SelectedJavaRuntime is not { } runtime)
            return;

        _javaManager.RemoveRuntime(runtime);
        RefreshJavaRuntimes();
        SelectedJavaRuntime = null;
    }

    /// <summary>
    /// The Java runtimes disabled by the user, shown in the Java management
    /// tab; they are ignored by the launcher until they are restored.
    /// </summary>
    public ObservableCollection<string> DisabledJavaRuntimes { get; } = new();

    /// <summary>
    /// The disabled Java runtime selected in the management list.
    /// </summary>
    public string? SelectedDisabledJavaRuntime { get; set; }

    /// <summary>
    /// The message shown to the user after an operation of the Java
    /// management page.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => Set(ref _statusMessage, value);
    }

    private string _statusMessage = string.Empty;

    /// <summary>
    /// Rescans the system for the Java runtimes, keeping the runtimes
    /// disabled by the user hidden.
    /// </summary>
    public void RescanJavaRuntimes()
    {
        if (_javaManager == null)
            return;

        _javaManager.Scan(_settingsManager.Settings.DisabledJava);
        _javaManager.EnsurePreferredRuntime(_settingsManager.Settings.JavaPath);
        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();
    }

    /// <summary>
    /// Refreshes the list of the disabled Java runtimes from the settings.
    /// </summary>
    public void RefreshDisabledJavaRuntimes()
    {
        DisabledJavaRuntimes.Clear();
        foreach (var path in _settingsManager.Settings.DisabledJava)
            DisabledJavaRuntimes.Add(path);
    }

    /// <summary>
    /// Returns whether the runtime was downloaded by the launcher; only such
    /// runtimes can be uninstalled.
    /// </summary>
    /// <param name="runtime">The runtime to check.</param>
    /// <returns>Whether the runtime is managed by the launcher.</returns>
    public bool IsManagedJavaRuntime(JavaRuntime runtime)
    {
        if (string.IsNullOrEmpty(BaseDirectory))
            return false;

        var managedDirectory = Path.GetFullPath(Path.Combine(BaseDirectory, "java"));
        var runtimePath = Path.GetFullPath(runtime.Path);

        return runtimePath.StartsWith(managedDirectory + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>
    /// Disables the selected runtime, so that the launcher ignores it.
    /// </summary>
    public void DisableSelectedJavaRuntime()
    {
        if (_javaManager == null || SelectedJavaRuntime is not { } runtime)
            return;

        var disabled = _settingsManager.Settings.DisabledJava;
        if (!disabled.Any(path => string.Equals(path, runtime.Path, StringComparison.OrdinalIgnoreCase)))
            disabled.Add(runtime.Path);

        _settingsManager.Save();
        _javaManager.RemoveRuntime(runtime);

        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();
        SelectedJavaRuntime = null;
        StatusMessage = $"已禁用 Java 运行时 {runtime.Version}";
    }

    /// <summary>
    /// Restores a disabled runtime, so that it is found again by the next
    /// scan of the Java runtimes.
    /// </summary>
    /// <param name="path">The path of the disabled runtime.</param>
    public void RestoreDisabledJavaRuntime(string path)
    {
        var disabled = _settingsManager.Settings.DisabledJava;
        disabled.RemoveAll(item => string.Equals(item, path, StringComparison.OrdinalIgnoreCase));
        _settingsManager.Save();

        if (_javaManager != null)
        {
            _javaManager.Scan(disabled);
            _javaManager.EnsurePreferredRuntime(_settingsManager.Settings.JavaPath);
        }

        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();
        SelectedDisabledJavaRuntime = null;
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.792_");
    }

    /// <summary>
    /// Uninstalls the selected runtime; the files downloaded by the launcher
    /// are deleted, while the runtimes installed by the user are only removed
    /// from the list.
    /// </summary>
    /// <returns>Whether the runtime was uninstalled.</returns>
    public bool UninstallSelectedJavaRuntime()
    {
        if (_javaManager == null || SelectedJavaRuntime is not { } runtime)
            return false;

        if (IsManagedJavaRuntime(runtime))
        {
            var directory = GetManagedRuntimeDirectory(runtime);
            if (directory is null || !Directory.Exists(directory))
            {
                StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.794_");
                return false;
            }

            try
            {
                Directory.Delete(directory, true);
            }
            catch (Exception ex)
            {
                StatusMessage = $"卸载 Java 运行时失败: {ex.Message}";
                return false;
            }
        }

        var disabled = _settingsManager.Settings.DisabledJava;
        if (!disabled.Any(path => string.Equals(path, runtime.Path, StringComparison.OrdinalIgnoreCase)))
            disabled.Add(runtime.Path);

        _settingsManager.Save();
        _javaManager.RemoveRuntime(runtime);

        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();
        SelectedJavaRuntime = null;
        StatusMessage = $"已卸载 Java 运行时 {runtime.Version}";
        return true;
    }

    /// <summary>
    /// Opens the folder of the selected runtime in the file manager of the
    /// system.
    /// </summary>
    public void RevealSelectedJavaRuntime()
    {
        if (SelectedJavaRuntime is not { } runtime)
            return;

        var directory = Path.GetDirectoryName(runtime.Path);
        if (string.IsNullOrEmpty(directory) || !Directory.Exists(directory))
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.796_");
            return;
        }

        try
        {
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
            {
                FileName = directory,
                UseShellExecute = true
            });
        }
        catch (Exception ex)
        {
            StatusMessage = $"无法打开文件夹: {ex.Message}";
        }
    }

    /// <summary>
    /// Returns the directory of a runtime downloaded by the launcher.
    /// </summary>
    /// <param name="runtime">The runtime to locate.</param>
    /// <returns>The directory of the runtime, or <c>null</c> when it cannot be
    /// determined.</returns>
    private string? GetManagedRuntimeDirectory(JavaRuntime runtime)
    {
        if (string.IsNullOrEmpty(BaseDirectory))
            return null;

        var javaDirectory = Path.GetFullPath(Path.Combine(BaseDirectory, "java"));
        var runtimePath = Path.GetFullPath(runtime.Path);

        if (!runtimePath.StartsWith(javaDirectory + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
            return null;

        var relative = runtimePath.Substring(javaDirectory.Length + 1);
        var separator = relative.IndexOf(Path.DirectorySeparatorChar);
        var name = separator < 0 ? relative : relative.Substring(0, separator);

        return string.IsNullOrEmpty(name) ? null : Path.Combine(javaDirectory, name);
    }

    /// <summary>
    /// The source used to download the version lists of the game.
    /// </summary>
    public DownloadSourceOption VersionListSource
    {
        get => _versionListSource;
        set
        {
            if (value is null)
                return;

            if (Set(ref _versionListSource, value))
                ApplyDownloadSources();
        }
    }

    /// <summary>
    /// The source used to download the files of the game.
    /// </summary>
    public DownloadSourceOption FileDownloadSource
    {
        get => _fileDownloadSource;
        set
        {
            if (value is null)
                return;

            if (Set(ref _fileDownloadSource, value))
                ApplyDownloadSources();
        }
    }

    /// <summary>
    /// Applies the download sources chosen by the user to the downloads of
    /// the launcher.
    /// </summary>
    private void ApplyDownloadSources()
    {
        if (_versionListSource is null || _fileDownloadSource is null)
            return;

        MirrorSource.SetSources(_versionListSource.Value, _fileDownloadSource.Value);
    }

    /// <summary>
    /// Returns the option matching the given source.
    /// </summary>
    /// <param name="source">The source to look for.</param>
    /// <returns>The option of the source.</returns>
    private DownloadSourceOption GetDownloadSourceOption(DownloadSource source)
    {
        foreach (var option in DownloadSources)
        {
            if (option.Value == source)
                return option;
        }

        return DownloadSources[0];
    }

    /// <summary>
    /// Whether the download concurrency is chosen automatically.
    /// </summary>
    public bool AutoDownloadThreads
    {
        get => _autoDownloadThreads;
        set => Set(ref _autoDownloadThreads, value);
    }

    /// <summary>
    /// The number of parallel downloads used when automatic selection is off.
    /// </summary>
    public int DownloadThreads
    {
        get => _downloadThreads;
        set => Set(ref _downloadThreads, value);
    }

    /// <summary>
    /// The settings-model mirror of the CurseForge API key.
    /// </summary>
    private string _curseForgeApiKey = string.Empty;

    /// <summary>
    /// How to select Java runtime for game launches.
    /// </summary>
    public JavaVersionType GameJavaType
    {
        get => _gameJavaType;
        set => Set(ref _gameJavaType, value);
    }

    /// <summary>
    /// Target Java major version when Java type is Version.
    /// </summary>
    public string GameCustomJavaVersion
    {
        get => _gameCustomJavaVersion;
        set => Set(ref _gameCustomJavaVersion, value);
    }

    /// <summary>
    /// Custom Java executable path when Java type is Custom.
    /// </summary>
    public string GameCustomJavaPath
    {
        get => _gameCustomJavaPath;
        set => Set(ref _gameCustomJavaPath, value);
    }

    /// <summary>
    /// Additional JVM options for game launches.
    /// </summary>
    public string GameJvmOptions
    {
        get => _gameJvmOptions;
        set => Set(ref _gameJvmOptions, value);
    }

    /// <summary>
    /// Automatically allocate game memory.
    /// </summary>
    public bool GameAutoMemory
    {
        get => _gameAutoMemory;
        set => Set(ref _gameAutoMemory, value);
    }

    /// <summary>
    /// Minimum game memory in MiB.
    /// </summary>
    public int? GameMinMemory
    {
        get => _gameMinMemory;
        set => Set(ref _gameMinMemory, value);
    }

    /// <summary>
    /// Maximum game memory in MiB.
    /// </summary>
    public int? GameMaxMemory
    {
        get => _gameMaxMemory;
        set => Set(ref _gameMaxMemory, value);
    }

    /// <summary>
    /// Custom game arguments appended after the standard ones.
    /// </summary>
    public string GameGameArguments
    {
        get => _gameGameArguments;
        set => Set(ref _gameGameArguments, value);
    }

    /// <summary>
    /// Whether to not use javaw on Windows.
    /// </summary>
    public bool GameNotUseJavaw
    {
        get => _gameNotUseJavaw;
        set => Set(ref _gameNotUseJavaw, value);
    }

    /// <summary>
    /// Game window width.
    /// </summary>
    public int GameWindowWidth
    {
        get => _gameWindowWidth;
        set => Set(ref _gameWindowWidth, value);
    }

    /// <summary>
    /// Game window height.
    /// </summary>
    public int GameWindowHeight
    {
        get => _gameWindowHeight;
        set => Set(ref _gameWindowHeight, value);
    }

    /// <summary>
    /// Start in fullscreen mode.
    /// </summary>
    public bool GameFullscreen
    {
        get => _gameFullscreen;
        set => Set(ref _gameFullscreen, value);
    }

    /// <summary>
    /// Process priority for the game.
    /// </summary>
    public HMCLCore.NET.Setting.ProcessPriority GameProcessPriority
    {
        get => _gameProcessPriority;
        set => Set(ref _gameProcessPriority, value);
    }

    /// <summary>
    /// Launcher visibility behavior after game starts.
    /// </summary>
    public LauncherVisibility GameLauncherVisibility
    {
        get => _gameLauncherVisibility;
        set => Set(ref _gameLauncherVisibility, value);
    }

    /// <summary>
    /// Graphics backend preference.
    /// </summary>
    public GraphicsBackend GameGraphicsBackend
    {
        get => _gameGraphicsBackend;
        set => Set(ref _gameGraphicsBackend, value);
    }

    /// <summary>
    /// The wrapper command executed before the game.
    /// </summary>
    public string? GameWrapper
    {
        get => _gameWrapper;
        set => Set(ref _gameWrapper, value);
    }

    private string? _gameWrapper;

    /// <summary>
    /// The pre-launch command executed before the game starts.
    /// </summary>
    public string? GamePreCommand
    {
        get => _gamePreCommand;
        set => Set(ref _gamePreCommand, value);
    }

    private string? _gamePreCommand;

    /// <summary>
    /// The post-launch command executed after the game starts.
    /// </summary>
    public string? GamePostCommand
    {
        get => _gamePostCommand;
        set => Set(ref _gamePostCommand, value);
    }

    private string? _gamePostCommand;

    /// <summary>
    /// The running directory override for the game process.
    /// </summary>
    public string? GameRunningDirectory
    {
        get => _gameRunningDirectory;
        set => Set(ref _gameRunningDirectory, value);
    }

    private string? _gameRunningDirectory;

    /// <summary>
    /// The quick play mode used when launching the game.
    /// </summary>
    public QuickPlayMode GameQuickPlayMode
    {
        get => _gameQuickPlayMode;
        set => Set(ref _gameQuickPlayMode, value);
    }

    private QuickPlayMode _gameQuickPlayMode = QuickPlayMode.None;

    /// <summary>
    /// The quick play argument: a save name, server address or Realms world id.
    /// </summary>
    public string? GameQuickPlayArgument
    {
        get => _gameQuickPlayArgument;
        set => Set(ref _gameQuickPlayArgument, value);
    }

    private string? _gameQuickPlayArgument;

    /// <summary>
    /// Custom environment variables as a multi-line "KEY=VALUE" text.
    /// </summary>
    public string? GameEnvironmentVariables
    {
        get => _gameEnvironmentVariables;
        set => Set(ref _gameEnvironmentVariables, value);
    }

    private string? _gameEnvironmentVariables;

    /// <summary>
    /// Backing field for <see cref="GameJavaType"/>.
    /// </summary>
    private JavaVersionType _gameJavaType = JavaVersionType.Auto;

    /// <summary>
    /// Backing field for <see cref="GameCustomJavaVersion"/>.
    /// </summary>
    private string _gameCustomJavaVersion = string.Empty;

    /// <summary>
    /// Backing field for <see cref="GameCustomJavaPath"/>.
    /// </summary>
    private string _gameCustomJavaPath = string.Empty;

    /// <summary>
    /// Backing field for <see cref="GameJvmOptions"/>.
    /// </summary>
    private string _gameJvmOptions = string.Empty;

    /// <summary>
    /// Backing field for <see cref="GameAutoMemory"/>.
    /// </summary>
    private bool _gameAutoMemory = true;

    /// <summary>
    /// Backing field for <see cref="GameMinMemory"/>.
    /// </summary>
    private int? _gameMinMemory;

    /// <summary>
    /// Backing field for <see cref="GameMaxMemory"/>.
    /// </summary>
    private int? _gameMaxMemory;

    /// <summary>
    /// Backing field for <see cref="GameGameArguments"/>.
    /// </summary>
    private string _gameGameArguments = string.Empty;

    /// <summary>
    /// Backing field for <see cref="GameNotUseJavaw"/>.
    /// </summary>
    private bool _gameNotUseJavaw;

    /// <summary>
    /// Backing field for <see cref="GameWindowWidth"/>.
    /// </summary>
    private int _gameWindowWidth = 854;

    /// <summary>
    /// Backing field for <see cref="GameWindowHeight"/>.
    /// </summary>
    private int _gameWindowHeight = 480;

    /// <summary>
    /// Backing field for <see cref="GameFullscreen"/>.
    /// </summary>
    private bool _gameFullscreen;

    /// <summary>
    /// Backing field for <see cref="GameProcessPriority"/>.
    /// </summary>
    private HMCLCore.NET.Setting.ProcessPriority _gameProcessPriority = HMCLCore.NET.Setting.ProcessPriority.Normal;

    /// <summary>
    /// Backing field for <see cref="GameLauncherVisibility"/>.
    /// </summary>
    private LauncherVisibility _gameLauncherVisibility = LauncherVisibility.Hide;

    /// <summary>
    /// Backing field for <see cref="GameGraphicsBackend"/>.
    /// </summary>
    private GraphicsBackend _gameGraphicsBackend = GraphicsBackend.Auto;

    /// <summary>
    /// Whether downloads use the configured proxy.
    /// </summary>
    private bool _proxyEnabled;

    /// <summary>
    /// The proxy type (http, socks4, socks5).
    /// </summary>
    private string _proxyType = "http";

    /// <summary>
    /// The proxy host.
    /// </summary>
    private string _proxyHost = string.Empty;

    /// <summary>
    /// The proxy port.
    /// </summary>
    private int _proxyPort = 8080;

    /// <summary>
    /// The proxy username.
    /// </summary>
    private string? _proxyUsername;

    /// <summary>
    /// The proxy password.
    /// </summary>
    private string? _proxyPassword;

    /// <summary>
    /// The last proxy connectivity test result message.
    /// </summary>
    private string _proxyTestStatus = string.Empty;

    /// <summary>
    /// Whether a proxy connectivity test is in progress.
    /// </summary>
    private bool _isTestingProxy;

    /// <summary>
    /// The CurseForge API key used by the mod manager.
    /// </summary>
    public string CurseForgeApiKey
    {
        get => _curseForgeApiKey;
        set => Set(ref _curseForgeApiKey, value);
    }

    /// <summary>
    /// Whether downloads use the configured proxy.
    /// </summary>
    public bool ProxyEnabled
    {
        get => _proxyEnabled;
        set => Set(ref _proxyEnabled, value);
    }

    /// <summary>
    /// The proxy type (http, socks4, socks5).
    /// </summary>
    public string ProxyType
    {
        get => _proxyType;
        set => Set(ref _proxyType, value);
    }

    /// <summary>
    /// The proxy host.
    /// </summary>
    public string ProxyHost
    {
        get => _proxyHost;
        set => Set(ref _proxyHost, value);
    }

    /// <summary>
    /// The proxy port.
    /// </summary>
    public int ProxyPort
    {
        get => _proxyPort;
        set => Set(ref _proxyPort, value);
    }

    /// <summary>
    /// The proxy username.
    /// </summary>
    public string? ProxyUsername
    {
        get => _proxyUsername;
        set => Set(ref _proxyUsername, value);
    }

    /// <summary>
    /// The proxy password.
    /// </summary>
    public string? ProxyPassword
    {
        get => _proxyPassword;
        set => Set(ref _proxyPassword, value);
    }

    /// <summary>
    /// The last proxy connectivity test result message.
    /// </summary>
    public string ProxyTestStatus
    {
        get => _proxyTestStatus;
        private set => Set(ref _proxyTestStatus, value);
    }

    /// <summary>
    /// Whether a proxy connectivity test is in progress.
    /// </summary>
    public bool IsTestingProxy
    {
        get => _isTestingProxy;
        private set => Set(ref _isTestingProxy, value);
    }

    /// <summary>
    /// Tests whether the configured proxy can reach a public endpoint.
    /// </summary>
    public RelayCommand TestProxyCommand { get; }

    /// <summary>
    /// Memory allocation in MB.
    /// </summary>
    public int Memory
    {
        get => _memory;
        set => Set(ref _memory, value);
    }

    /// <summary>
    /// Java executable path.
    /// </summary>
    public string JavaPath
    {
        get => _javaPath;
        set
        {
            Set(ref _javaPath, value);
            OnPropertyChanged(nameof(JavaPathMissing));
        }
    }

    /// <summary>
    /// Whether no Java path is configured, used to show a guidance hint in the settings UI.
    /// </summary>
    public bool JavaPathMissing => string.IsNullOrWhiteSpace(JavaPath);

    /// <summary>
    /// Whether to use custom JVM arguments.
    /// </summary>
    public bool UseCustomJvmArgs
    {
        get => _useCustomJvmArgs;
        set => Set(ref _useCustomJvmArgs, value);
    }

    /// <summary>
    /// Custom JVM arguments string.
    /// </summary>
    public string CustomJvmArgs
    {
        get => _customJvmArgs;
        set => Set(ref _customJvmArgs, value);
    }

    /// <summary>
    /// Window width.
    /// </summary>
    public int WindowWidth
    {
        get => _windowWidth;
        set => Set(ref _windowWidth, value);
    }

    /// <summary>
    /// Window height.
    /// </summary>
    public int WindowHeight
    {
        get => _windowHeight;
        set => Set(ref _windowHeight, value);
    }

    /// <summary>
    /// The display text describing the result of the last Java detection pass.
    /// </summary>
    private string _javaRuntimeInfo = string.Empty;

    /// <summary>
    /// The display text describing the result of the last Java detection pass.
    /// </summary>
    public string JavaRuntimeInfo
    {
        get => _javaRuntimeInfo;
        set => Set(ref _javaRuntimeInfo, value);
    }

    /// <summary>
    /// Creates a new SettingsViewModel.
    /// </summary>
    /// <param name="settingsManager">The settings manager backing the edited values.</param>
    /// <param name="javaManager">The Java manager used by the auto-detection button.</param>
    /// <param name="analytics">
    /// The service sending the anonymous usage statistics; the settings of the
    /// statistics are applied to it when they are saved.
    /// </param>
    public SettingsViewModel(
        SettingsManager settingsManager,
        JavaManager? javaManager = null,
        CountlyService? analytics = null)
    {
        _settingsManager = settingsManager ?? throw new ArgumentNullException(nameof(settingsManager));
        _javaManager = javaManager;
        _analytics = analytics;
        _versionListSource = DownloadSources[0];
        _fileDownloadSource = DownloadSources[0];
        TestProxyCommand = new RelayCommand(TestProxyAsync);
        LoadSettings();
        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();
    }

    /// <summary>
    /// The service sending the anonymous usage statistics.
    /// </summary>
    private readonly CountlyService? _analytics;

    /// <summary>
    /// The themes shipped with the launcher, shown in the appearance settings.
    /// </summary>
    public IReadOnlyList<ThemeItem> BuiltinThemes { get; } = GetBuiltinThemes();

    /// <summary>
    /// Reads the themes shipped with the launcher.
    /// </summary>
    /// <returns>The themes.</returns>
    private static IReadOnlyList<ThemeItem> GetBuiltinThemes()
    {
        var items = new List<ThemeItem>();

        foreach (var pack in ThemePackManager.GetBuiltinPacks())
        {
            foreach (var theme in pack.Themes)
            {
                var name = theme.Name?.GetDisplayText(null);
                var description = theme.Description?.GetDisplayText(null);

                items.Add(new ThemeItem(
                    theme,
                    string.IsNullOrEmpty(name) ? theme.Id : name,
                    description ?? string.Empty));
            }
        }

        return items;
    }

    /// <summary>
    /// Applies one of the themes of the launcher to the appearance settings.
    /// </summary>
    /// <param name="theme">The theme applied.</param>
    public void ApplyTheme(ThemePackTheme theme)
    {
        if (theme is null)
            return;

        if (!string.IsNullOrEmpty(theme.Color))
            ThemeColor = theme.Color;

        if (!string.IsNullOrEmpty(theme.Brightness))
        {
            SelectedTheme = string.Equals(theme.Brightness, "dark", StringComparison.OrdinalIgnoreCase)
                ? AppearanceManager.DarkTheme
                : AppearanceManager.LightTheme;
        }

        if (!string.IsNullOrEmpty(theme.ColorStyle) &&
            Enum.TryParse<HMCLCore.NET.Setting.ThemeColorStyle>(theme.ColorStyle, true, out var colorStyle))
        {
            SelectedThemeColorStyle = ThemeColorStyles.FirstOrDefault(option => option.Value == colorStyle)
                                     ?? SelectedThemeColorStyle;
        }

        if (!string.IsNullOrEmpty(theme.Contrast) &&
            Enum.TryParse<HMCLCore.NET.Setting.ThemeContrast>(theme.Contrast, true, out var contrast))
        {
            SelectedThemeContrast = ThemeContrasts.FirstOrDefault(option => option.Value == contrast)
                                   ?? SelectedThemeContrast;
        }

        if (theme.Background is { } background)
        {
            switch (background.Type)
            {
                case "builtin":
                    SelectedBackgroundType = BackgroundTypes.FirstOrDefault(option =>
                        option.Value == HMCLCore.NET.Setting.BackgroundType.Builtin);
                    SelectedBuiltinWallpaper = BuiltinWallpapers.FirstOrDefault(option =>
                        string.Equals(option.Value, background.Id, StringComparison.OrdinalIgnoreCase));
                    break;

                case "image":
                    SelectedBackgroundType = BackgroundTypes.FirstOrDefault(option =>
                        option.Value == HMCLCore.NET.Setting.BackgroundType.Image);
                    break;

                case "network":
                    SelectedBackgroundType = BackgroundTypes.FirstOrDefault(option =>
                        option.Value == HMCLCore.NET.Setting.BackgroundType.Network);
                    if (!string.IsNullOrEmpty(background.Id))
                        NetworkBackgroundImageUrl = background.Id;
                    break;

                case "none":
                    SelectedBackgroundType = BackgroundTypes.FirstOrDefault(option =>
                        option.Value == HMCLCore.NET.Setting.BackgroundType.None);
                    break;
            }

            if (background.Opacity is { } opacity)
                BackgroundOpacity = opacity;
        }

        if (theme.TitleBar?.Transparent is { } transparent)
            TitleBarTransparent = transparent;

        if (theme.WindowTransparent is { } windowTransparent)
            EnableTransparencyEffects = windowTransparent;
    }

    /// <summary>
    /// Whether the window showing the log of the game is opened when the game
    /// starts.
    /// </summary>
    public bool GameShowLogs
    {
        get => _gameShowLogs;
        set => Set(ref _gameShowLogs, value);
    }

    private bool _gameShowLogs = true;

    /// <summary>
    /// The font family of the log window.
    /// </summary>
    public string LogFontFamily
    {
        get => _logFontFamily;
        set => Set(ref _logFontFamily, value);
    }

    private string _logFontFamily = string.Empty;

    /// <summary>
    /// The size of the font of the log window.
    /// </summary>
    public double LogFontSize
    {
        get => _logFontSize;
        set => Set(ref _logFontSize, value);
    }

    private double _logFontSize = 12;

    /// <summary>
    /// Whether the JVM options of the version are ignored.
    /// </summary>
    public bool GameNoJvmOptions
    {
        get => _gameNoJvmOptions;
        set => Set(ref _gameNoJvmOptions, value);
    }

    private bool _gameNoJvmOptions;

    /// <summary>
    /// Whether the optimized JVM options of the launcher are ignored.
    /// </summary>
    public bool GameNoOptimizingJvmOptions
    {
        get => _gameNoOptimizingJvmOptions;
        set => Set(ref _gameNoOptimizingJvmOptions, value);
    }

    private bool _gameNoOptimizingJvmOptions;

    /// <summary>
    /// Whether the compatibility of the Java runtime is checked.
    /// </summary>
    public bool GameNotCheckJvm
    {
        get => _gameNotCheckJvm;
        set => Set(ref _gameNotCheckJvm, value);
    }

    private bool _gameNotCheckJvm;

    /// <summary>
    /// Whether the files of the game are checked before the launch.
    /// </summary>
    public bool GameNotCheckGame
    {
        get => _gameNotCheckGame;
        set => Set(ref _gameNotCheckGame, value);
    }

    private bool _gameNotCheckGame;

    /// <summary>
    /// The size of the permanent generation area of the Java virtual machine,
    /// in MiB.
    /// </summary>
    public int? GamePermSize
    {
        get => _gamePermSize;
        set => Set(ref _gamePermSize, value);
    }

    private int? _gamePermSize;

    /// <summary>
    /// Whether a custom directory holding the native libraries is used.
    /// </summary>
    public bool GameUseCustomNatives
    {
        get => _gameUseCustomNatives;
        set => Set(ref _gameUseCustomNatives, value);
    }

    private bool _gameUseCustomNatives;

    /// <summary>
    /// The custom directory holding the native libraries of the game.
    /// </summary>
    public string GameNativesDirectory
    {
        get => _gameNativesDirectory;
        set => Set(ref _gameNativesDirectory, value);
    }

    private string _gameNativesDirectory = string.Empty;


    /// <summary>
    /// The theme packs installed in the launcher.
    /// </summary>
    public ObservableCollection<InstalledThemePack> ThemePacks { get; } = new();

    /// <summary>
    /// The directory holding the installed theme packs.
    /// </summary>
    private string ThemePacksDirectory => Path.Combine(BaseDirectory ?? string.Empty, "themes");

    /// <summary>
    /// Reloads the list of the installed theme packs.
    /// </summary>
    public void LoadThemePacks()
    {
        ThemePacks.Clear();

        foreach (var pack in ThemePackManager.ListInstalled(ThemePacksDirectory))
            ThemePacks.Add(pack);
    }

    /// <summary>
    /// Installs a theme pack from an archive chosen by the user.
    /// </summary>
    /// <param name="archivePath">The path of the archive holding the theme pack.</param>
    /// <returns>Whether the theme pack was installed.</returns>
    public bool InstallThemePack(string archivePath)
    {
        var pack = ThemePackManager.Install(archivePath, ThemePacksDirectory);
        if (pack is null)
        {
            StatusMessage = Lang.Get("theme_pack.import.failed", "主题包导入失败");
            return false;
        }

        LoadThemePacks();
        StatusMessage = $"主题包已导入：{pack.Name}";
        return true;
    }

    /// <summary>
    /// Removes an installed theme pack from the launcher.
    /// </summary>
    /// <param name="pack">The theme pack removed.</param>
    /// <returns>Whether the theme pack was removed.</returns>
    public bool DeleteThemePack(InstalledThemePack pack)
    {
        if (!ThemePackManager.Uninstall(pack))
        {
            StatusMessage = Lang.Get("theme_pack.delete.failed", "主题包删除失败");
            return false;
        }

        LoadThemePacks();
        StatusMessage = $"主题包已删除：{pack.Name}";
        return true;
    }

    /// <summary>
    /// Applies one of the themes of an installed theme pack.
    /// </summary>
    /// <param name="pack">The theme pack holding the theme.</param>
    /// <param name="theme">The theme applied.</param>
    public void ApplyThemePackTheme(InstalledThemePack pack, ThemePackTheme theme)
    {
        if (pack is null || theme is null)
            return;

        ApplyTheme(theme);
        StatusMessage = $"主题已应用：{pack.Name}";
    }

    /// <summary>
    /// The theme selected in the list of the themes.
    /// </summary>
    public ThemeItem? SelectedThemeItem
    {
        get => _selectedThemeItem;
        set => Set(ref _selectedThemeItem, value);
    }

    private ThemeItem? _selectedThemeItem;

    /// <summary>
    /// The game settings presets of the launcher.
    /// </summary>
    public ObservableCollection<GameSettingsPreset> GameSettingsPresets { get; } = new();

    /// <summary>
    /// The preset selected in the preset list.
    /// </summary>
    public GameSettingsPreset? SelectedGameSettingsPreset
    {
        get => _selectedGameSettingsPreset;
        set
        {
            if (Set(ref _selectedGameSettingsPreset, value) && value is not null)
            {
                // The values of the selected preset are shown in the settings
                // page, so that they can be edited.
                LoadPresetValues(value);
            }
        }
    }

    private GameSettingsPreset? _selectedGameSettingsPreset;

    /// <summary>
    /// Reloads the list of the presets from the settings.
    /// </summary>
    public void LoadGameSettingsPresets()
    {
        _settingsManager.EnsureGameSettingsPresets();

        var selectedId = SelectedGameSettingsPreset?.Id;
        var defaultId = _settingsManager.Settings.DefaultGameSettingsPresetId;

        GameSettingsPresets.Clear();
        foreach (var preset in _settingsManager.Settings.GameSettingsPresets)
            GameSettingsPresets.Add(preset);

        SelectedGameSettingsPreset =
            GameSettingsPresets.FirstOrDefault(p => p.Id == selectedId) ??
            GameSettingsPresets.FirstOrDefault(p => p.Id == defaultId) ??
            GameSettingsPresets.FirstOrDefault();
    }

    /// <summary>
    /// Shows the values of a preset in the settings page.
    /// </summary>
    /// <param name="preset">The preset whose values are shown.</param>
    private void LoadPresetValues(GameSettingsPreset preset)
    {
        GameJavaType = preset.JavaType;
        GameCustomJavaVersion = preset.CustomJavaVersion;
        GameCustomJavaPath = preset.CustomJavaPath;
        GameJvmOptions = preset.JvmOptions;
        GameAutoMemory = preset.AutoMemory;
        GameMinMemory = preset.MinMemory;
        GameMaxMemory = preset.MaxMemory;
        GameGameArguments = preset.GameArguments;
        GameNotUseJavaw = preset.NotUseJavaw;
        GameWindowWidth = (int)preset.Width;
        GameWindowHeight = (int)preset.Height;
        GameFullscreen = preset.Fullscreen;
        GameProcessPriority = preset.ProcessPriority;
        GameLauncherVisibility = preset.LauncherVisibility;
        GameGraphicsBackend = preset.GraphicsBackend;
        GameWrapper = preset.Wrapper;
        GamePreCommand = preset.PreCommand;
        GamePostCommand = preset.PostCommand;
        GameRunningDirectory = preset.RunningDirectory;
        GameQuickPlayMode = preset.QuickPlayMode;
        GameQuickPlayArgument = preset.QuickPlayArgument;
        GameEnvironmentVariables = EnvironmentVariableText.Format(preset.EnvironmentVariables);
        GameShowLogs = preset.ShowLogs;
        GameNoJvmOptions = preset.NoJvmOptions;
        GameNoOptimizingJvmOptions = preset.NoOptimizingJvmOptions;
        GameNotCheckJvm = preset.NotCheckJvm;
        GameNotCheckGame = preset.NotCheckGame;
        GamePermSize = preset.PermSize;
        GameUseCustomNatives = preset.UseCustomNatives;
        GameNativesDirectory = preset.NativesDirectory;
    }

    /// <summary>
    /// Writes the values shown in the settings page into a preset.
    /// </summary>
    /// <param name="preset">The preset the values are written into.</param>
    private void ApplyPresetValues(GameSettingsPreset preset)
    {
        preset.JavaType = GameJavaType;
        preset.CustomJavaVersion = GameCustomJavaVersion;
        preset.CustomJavaPath = GameCustomJavaPath;
        preset.JvmOptions = GameJvmOptions;
        preset.AutoMemory = GameAutoMemory;
        preset.MinMemory = GameMinMemory;
        preset.MaxMemory = GameMaxMemory;
        preset.GameArguments = GameGameArguments;
        preset.NotUseJavaw = GameNotUseJavaw;
        preset.Width = GameWindowWidth;
        preset.Height = GameWindowHeight;
        preset.Fullscreen = GameFullscreen;
        preset.ProcessPriority = GameProcessPriority;
        preset.LauncherVisibility = GameLauncherVisibility;
        preset.GraphicsBackend = GameGraphicsBackend;
        preset.Wrapper = GameWrapper ?? string.Empty;
        preset.PreCommand = GamePreCommand ?? string.Empty;
        preset.PostCommand = GamePostCommand ?? string.Empty;
        preset.RunningDirectory = GameRunningDirectory ?? string.Empty;
        preset.QuickPlayMode = GameQuickPlayMode;
        preset.QuickPlayArgument = GameQuickPlayArgument ?? string.Empty;
        preset.EnvironmentVariables = EnvironmentVariableText.Parse(GameEnvironmentVariables);
        preset.ShowLogs = GameShowLogs;
        preset.NoJvmOptions = GameNoJvmOptions;
        preset.NoOptimizingJvmOptions = GameNoOptimizingJvmOptions;
        preset.NotCheckJvm = GameNotCheckJvm;
        preset.NotCheckGame = GameNotCheckGame;
        preset.PermSize = GamePermSize;
        preset.UseCustomNatives = GameUseCustomNatives;
        preset.NativesDirectory = GameNativesDirectory ?? string.Empty;
    }

    /// <summary>
    /// Creates a new preset with the settings of the selected one.
    /// </summary>
    /// <returns>Whether the preset was created.</returns>
    public bool CreateGameSettingsPreset()
    {
        try
        {
            var source = SelectedGameSettingsPreset ?? _settingsManager.GetGameSettingsPreset(null);
            var preset = new GameSettingsPreset
            {
                Id = Guid.NewGuid().ToString("N"),
                Name = UniquePresetName("预设 " + (GameSettingsPresets.Count + 1))
            };

            GameSettings.CopyValues(source, preset);

            _settingsManager.Update(settings => settings.GameSettingsPresets.Add(preset));
            LoadGameSettingsPresets();
            SelectedGameSettingsPreset = GameSettingsPresets.FirstOrDefault(p => p.Id == preset.Id);
            StatusMessage = $"已创建预设 {preset.Name}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"创建预设失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Renames the selected preset.
    /// </summary>
    /// <param name="name">The new name of the preset.</param>
    /// <returns>Whether the preset was renamed.</returns>
    public bool RenameGameSettingsPreset(string name)
    {
        if (SelectedGameSettingsPreset is not { } preset || string.IsNullOrWhiteSpace(name))
            return false;

        try
        {
            preset.Name = name.Trim();
            _settingsManager.Update(settings =>
            {
                var index = settings.GameSettingsPresets.FindIndex(p => p.Id == preset.Id);
                if (index >= 0)
                    settings.GameSettingsPresets[index] = preset;
                else
                    settings.GameSettingsPresets.Add(preset);
            });

            LoadGameSettingsPresets();
            StatusMessage = $"预设已重命名为 {preset.Name}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"重命名预设失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Deletes the selected preset.
    /// </summary>
    /// <returns>Whether the preset was deleted.</returns>
    public bool DeleteGameSettingsPreset()
    {
        if (SelectedGameSettingsPreset is not { } preset)
            return false;

        if (GameSettingsPresets.Count <= 1)
        {
            StatusMessage = "至少需要保留一个预设";
            return false;
        }

        try
        {
            _settingsManager.Update(settings =>
            {
                settings.GameSettingsPresets.RemoveAll(p => p.Id == preset.Id);

                if (settings.DefaultGameSettingsPresetId == preset.Id)
                    settings.DefaultGameSettingsPresetId = string.Empty;
            });

            LoadGameSettingsPresets();
            StatusMessage = $"预设 {preset.Name} 已删除";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"删除预设失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Makes the selected preset the default one used by the instances that do
    /// not name a preset.
    /// </summary>
    /// <returns>Whether the preset was made the default one.</returns>
    public bool SetDefaultGameSettingsPreset()
    {
        if (SelectedGameSettingsPreset is not { } preset)
            return false;

        try
        {
            _settingsManager.Update(settings => settings.DefaultGameSettingsPresetId = preset.Id);
            LoadGameSettingsPresets();
            StatusMessage = $"预设 {preset.Name} 已设为默认";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"设置默认预设失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Returns a name that is not used by another preset yet.
    /// </summary>
    /// <param name="name">The wanted name of the preset.</param>
    /// <returns>The name of the preset.</returns>
    private string UniquePresetName(string name)
    {
        var names = GameSettingsPresets.Select(preset => preset.Name).ToHashSet(StringComparer.OrdinalIgnoreCase);
        if (!names.Contains(name))
            return name;

        for (var index = 2; ; index++)
        {
            var candidate = $"{name} {index}";
            if (!names.Contains(candidate))
                return candidate;
        }
    }

    /// <summary>
    /// Whether the settings file was written by a newer version of the
    /// launcher, so that it can be backed up and overwritten.
    /// </summary>
    public bool SettingsFileReadOnly => _settingsManager.IsReadOnly;

    /// <summary>
    /// Backs up the settings file and overwrites it with the settings
    /// currently loaded by this version of the launcher.
    /// </summary>
    /// <returns>Whether the file was overwritten.</returns>
    public bool ForceWriteSettings()
    {
        var backup = _settingsManager.ForceWrite();
        if (backup is null)
        {
            StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.658_", "覆盖设置文件失败");
            return false;
        }

        OnPropertyChanged(nameof(SettingsFileReadOnly));
        StatusMessage = Lang.Get("hmcl.mainwindowviewmodel.660_", "设置文件已备份并覆盖");
        return true;
    }

    /// <summary>
    /// Rescans the system for Java runtimes and picks the newest one,
    /// storing its path in the JavaPath field.
    /// </summary>
    public void DetectJava()
    {
        if (_javaManager == null)
            return;

        _javaManager.Scan(_settingsManager.Settings.DisabledJava);
        _javaManager.EnsurePreferredRuntime(JavaPath);
        RefreshJavaRuntimes();
        RefreshDisabledJavaRuntimes();

        var runtime = _javaManager.PreferredRuntime;
        if (runtime != null)
        {
            JavaPath = runtime.Path;
            JavaRuntimeInfo = $"已选择: {runtime.Vendor} {runtime.Version} ({runtime.Architecture})";
        }
        else
        {
            JavaRuntimeInfo = Lang.Get("hmcl.mainwindowviewmodel.798_");
        }
    }

    /// <summary>
    /// Loads settings from the manager into the view model.
    /// </summary>
    public void LoadSettings()
    {
        var settings = _settingsManager.Settings;
        Memory = settings.Memory;
        JavaPath = settings.JavaPath;
        UseCustomJvmArgs = settings.UseCustomJvmArgs;
        CustomJvmArgs = settings.CustomJvmArgs;
        WindowWidth = settings.WindowWidth;
        WindowHeight = settings.WindowHeight;
        VersionListSource = GetDownloadSourceOption(settings.VersionListSource);
        FileDownloadSource = GetDownloadSourceOption(settings.FileDownloadSource);

        // The download provider of the old versions of the launcher is
        // migrated to the new download sources.
        if (settings.DownloadProvider.Contains("BMCLAPI", StringComparison.OrdinalIgnoreCase) &&
            settings.VersionListSource == DownloadSource.Default &&
            settings.FileDownloadSource == DownloadSource.Default)
        {
            VersionListSource = GetDownloadSourceOption(DownloadSource.Mirror);
            FileDownloadSource = GetDownloadSourceOption(DownloadSource.Mirror);
        }

        ApplyDownloadSources();
        AutoDownloadThreads = settings.AutoDownloadThreads;
        DownloadThreads = settings.DownloadThreads;
        CurseForgeApiKey = settings.CurseForgeApiKey;

        // Global game settings preset
        GameJavaType = settings.GameSettingsPreset.JavaType;
        GameCustomJavaVersion = settings.GameSettingsPreset.CustomJavaVersion;
        GameCustomJavaPath = settings.GameSettingsPreset.CustomJavaPath;
        GameJvmOptions = settings.GameSettingsPreset.JvmOptions;
        GameAutoMemory = settings.GameSettingsPreset.AutoMemory;
        GameMinMemory = settings.GameSettingsPreset.MinMemory;
        GameMaxMemory = settings.GameSettingsPreset.MaxMemory;
        GameGameArguments = settings.GameSettingsPreset.GameArguments;
        GameNotUseJavaw = settings.GameSettingsPreset.NotUseJavaw;
        GameWindowWidth = (int)settings.GameSettingsPreset.Width;
        GameWindowHeight = (int)settings.GameSettingsPreset.Height;
        GameFullscreen = settings.GameSettingsPreset.Fullscreen;
        GameProcessPriority = settings.GameSettingsPreset.ProcessPriority;
        GameLauncherVisibility = settings.GameSettingsPreset.LauncherVisibility;
        GameGraphicsBackend = settings.GameSettingsPreset.GraphicsBackend;
        GameWrapper = settings.GameSettingsPreset.Wrapper;
        GamePreCommand = settings.GameSettingsPreset.PreCommand;
        GamePostCommand = settings.GameSettingsPreset.PostCommand;
        GameRunningDirectory = settings.GameSettingsPreset.RunningDirectory;
        GameQuickPlayMode = settings.GameSettingsPreset.QuickPlayMode;
        GameQuickPlayArgument = settings.GameSettingsPreset.QuickPlayArgument;
        GameEnvironmentVariables = EnvironmentVariableText.Format(settings.GameSettingsPreset.EnvironmentVariables);
        GameShowLogs = settings.GameSettingsPreset.ShowLogs;
        LogFontFamily = settings.LogFontFamily ?? string.Empty;
        LogFontSize = settings.LogFontSize;
        GameNoJvmOptions = settings.GameSettingsPreset.NoJvmOptions;
        GameNoOptimizingJvmOptions = settings.GameSettingsPreset.NoOptimizingJvmOptions;
        GameNotCheckJvm = settings.GameSettingsPreset.NotCheckJvm;
        GameNotCheckGame = settings.GameSettingsPreset.NotCheckGame;
        GamePermSize = settings.GameSettingsPreset.PermSize;
        GameUseCustomNatives = settings.GameSettingsPreset.UseCustomNatives;
        GameNativesDirectory = settings.GameSettingsPreset.NativesDirectory;

        // The presets of the game settings are listed in the settings page.
        LoadGameSettingsPresets();

        var proxy = settings.ProxyConfig ?? new HMCLCore.NET.Setting.ProxyConfig();
        ProxyEnabled = proxy.Enabled;
        ProxyType = proxy.Type;
        ProxyHost = proxy.Host;
        ProxyPort = proxy.Port;
        ProxyUsername = proxy.Username;
        ProxyPassword = proxy.Password;

        // Appearance
        SelectedTheme = AppearanceManager.GetThemeName(settings.Theme);
        FontFamily = settings.FontFamily ?? string.Empty;
        FontSize = settings.FontSize;
        BackgroundImagePath = settings.BackgroundImagePath ?? string.Empty;
        ThemeColor = settings.ThemeColor;
        BackgroundType = settings.BackgroundType;
        BackgroundOpacity = settings.BackgroundOpacity;
        NetworkBackgroundImageUrl = settings.NetworkBackgroundImageUrl;
        TitleBarTransparent = settings.TitleBarTransparent;
        EnableTransparencyEffects = settings.EnableTransparencyEffects;

        BuiltinWallpaperId = settings.BuiltinWallpaperId ?? string.Empty;
        ThemeColorStyle = settings.ThemeColorStyle;
        ThemeContrast = settings.ThemeContrast;
        DisableAnimations = settings.DisableAnimations;
        TextRenderingMode = string.IsNullOrEmpty(settings.TextRenderingMode)
            ? "default"
            : settings.TextRenderingMode;

        SelectedThemeColorStyle = ThemeColorStyles.FirstOrDefault(option => option.Value == ThemeColorStyle)
                                 ?? ThemeColorStyles[0];
        SelectedThemeContrast = ThemeContrasts.FirstOrDefault(option => option.Value == ThemeContrast)
                                ?? ThemeContrasts[0];
        SelectedTextRenderingMode = TextRenderingModes.FirstOrDefault(option =>
                                        string.Equals(option.Value, TextRenderingMode, StringComparison.OrdinalIgnoreCase))
                                    ?? TextRenderingModes[0];
        SelectedBuiltinWallpaper = BuiltinWallpapers.FirstOrDefault(option =>
                                    string.Equals(option.Value, BuiltinWallpaperId, StringComparison.OrdinalIgnoreCase))
                                ?? BuiltinWallpapers[0];

        // Language
        Language = settings.Language;

        // Miscellaneous
        UpdateChannel = settings.UpdateChannel;
        DisableAprilFools = settings.DisableAprilFools;
        EnableAnalytics = settings.EnableAnalytics;
        AnalyticsServerUrl = settings.AnalyticsServerUrl ?? string.Empty;
        AnalyticsAppKey = settings.AnalyticsAppKey ?? string.Empty;
        Debug = settings.Debug;
    }

    /// <summary>
    /// Saves settings from the view model to the manager and applies the Java
    /// path and proxy to the managers immediately.
    /// </summary>
    public void SaveSettings()
    {
        _settingsManager.Update(s =>
        {
            s.Memory = Memory;
            s.JavaPath = JavaPath;
            s.UseCustomJvmArgs = UseCustomJvmArgs;
            s.CustomJvmArgs = CustomJvmArgs;
            s.WindowWidth = WindowWidth;
            s.WindowHeight = WindowHeight;
            s.VersionListSource = VersionListSource.Value;
            s.FileDownloadSource = FileDownloadSource.Value;
            s.DownloadProvider = FileDownloadSource.Value == DownloadSource.Mirror
                ? "BMCLAPI"
                : "Mojang Official";
            s.AutoDownloadThreads = AutoDownloadThreads;
            s.DownloadThreads = DownloadThreads;
            DownloadConcurrency.Apply(AutoDownloadThreads, DownloadThreads);
            s.CurseForgeApiKey = CurseForgeApiKey;

            // The values shown in the settings page belong to the selected
            // preset; the default preset is kept in sync with the single
            // preset of the settings.
            var preset = SelectedGameSettingsPreset ?? _settingsManager.GetGameSettingsPreset(null);
            ApplyPresetValues(preset);

            s.GameSettingsPresets = GameSettingsPresets.ToList();
            _settingsManager.SyncDefaultGameSettingsPreset();

            s.ProxyConfig = new HMCLCore.NET.Setting.ProxyConfig
            {
                Enabled = ProxyEnabled,
                Type = ProxyType,
                Host = ProxyHost,
                Port = ProxyPort,
                Username = ProxyUsername,
                Password = ProxyPassword
            };

            s.Theme = AppearanceManager.GetThemeId(SelectedTheme);
            s.FontFamily = string.IsNullOrWhiteSpace(FontFamily) ? null : FontFamily;
            s.FontSize = FontSize;
            s.BackgroundImagePath = string.IsNullOrWhiteSpace(BackgroundImagePath) ? null : BackgroundImagePath;
            s.ThemeColor = ThemeColor;
            s.BackgroundType = BackgroundType;
            s.BackgroundOpacity = BackgroundOpacity;
            s.NetworkBackgroundImageUrl = NetworkBackgroundImageUrl;
            s.TitleBarTransparent = TitleBarTransparent;
            s.EnableTransparencyEffects = EnableTransparencyEffects;

            s.BuiltinWallpaperId = BuiltinWallpaperId ?? string.Empty;
            s.LogFontFamily = LogFontFamily ?? string.Empty;
            s.LogFontSize = LogFontSize;
            s.ThemeColorStyle = ThemeColorStyle;
            s.ThemeContrast = ThemeContrast;
            s.DisableAnimations = DisableAnimations;
            s.TextRenderingMode = TextRenderingMode ?? "default";

            s.Language = Language;

            s.UpdateChannel = UpdateChannel;
            s.DisableAprilFools = DisableAprilFools;
            s.EnableAnalytics = EnableAnalytics;
            s.AnalyticsServerUrl = AnalyticsServerUrl ?? string.Empty;
            s.AnalyticsAppKey = AnalyticsAppKey ?? string.Empty;
            s.Debug = Debug;
        });

        // Make the new Java path effective for future launches.
        _javaManager?.EnsurePreferredRuntime(JavaPath);

        // The settings of the anonymous usage statistics are applied to the
        // service sending them, so that the new values are used right away.
        if (_analytics is not null)
        {
            var settings = _settingsManager.Settings;
            _analytics.Enabled = settings.EnableAnalytics;
            _analytics.ServerUrl = settings.AnalyticsServerUrl;
            _analytics.AppKey = settings.AnalyticsAppKey;
            _analytics.DeviceId = settings.AnalyticsDeviceId;
        }

        // Apply the proxy globally so every downstream HttpClient uses it.
        ProxyApplier.Apply(ProxyEnabled ? new HMCLCore.NET.Setting.ProxyConfig
        {
            Enabled = ProxyEnabled,
            Type = ProxyType,
            Host = ProxyHost,
            Port = ProxyPort,
            Username = ProxyUsername,
            Password = ProxyPassword
        } : null);

        // Let the window pick up the new theme, font and background.
        AppearanceChanged?.Invoke();
    }

    /// <summary>
    /// Tests whether the configured proxy can reach a public endpoint.
    /// Updates <see cref="ProxyTestStatus"/> with the result.
    /// </summary>
    private async void TestProxyAsync()
    {
        if (!ProxyEnabled || string.IsNullOrWhiteSpace(ProxyHost))
        {
            ProxyTestStatus = Lang.Get("hmcl.mainwindowviewmodel.800_");
            return;
        }

        IsTestingProxy = true;
        ProxyTestStatus = Lang.Get("hmcl.mainwindowviewmodel.802_");

        try
        {
            var handler = new HttpClientHandler
            {
                Proxy = new WebProxy($"{ProxyType.ToLowerInvariant()}://{ProxyHost}:{ProxyPort}")
                {
                    UseDefaultCredentials = false,
                    Credentials = string.IsNullOrEmpty(ProxyUsername)
                        ? null
                        : new NetworkCredential(ProxyUsername, ProxyPassword ?? string.Empty)
                },
                UseProxy = true
            };

            using var client = new HttpClient(handler) { Timeout = TimeSpan.FromSeconds(10) };
            using var response = await client.GetAsync("https://example.com");
            ProxyTestStatus = $"连通正常 (HTTP {(int)response.StatusCode})";
        }
        catch (Exception ex)
        {
            ProxyTestStatus = $"连通失败: {ex.Message}";
        }
        finally
        {
            IsTestingProxy = false;
        }
    }
}

/// <summary>
/// Information about a running game process, for the running-games panel.
/// </summary>
public sealed class RunningGameInfo
{
    /// <summary>
    /// The instance the process belongs to.
    /// </summary>
    public GameInstance Instance { get; }

    /// <summary>
    /// The managed process.
    /// </summary>
    public ManagedProcess Process { get; }

    /// <summary>
    /// Creates a new RunningGameInfo.
    /// </summary>
    /// <param name="instance">The instance the process belongs to.</param>
    /// <param name="process">The managed process.</param>
    public RunningGameInfo(GameInstance instance, ManagedProcess process)
    {
        Instance = instance ?? throw new ArgumentNullException(nameof(instance));
        Process = process ?? throw new ArgumentNullException(nameof(process));
        StopCommand = new RelayCommand(Stop);
    }

    /// <summary>
    /// Command terminating this running game process.
    /// </summary>
    public ICommand StopCommand { get; }

    /// <summary>
    /// A short display text for this running game.
    /// </summary>
    public string DisplayText => $"{Instance.Id} (PID {Process.Id})";

    /// <summary>
    /// Terminates this running game process.
    /// </summary>
    public void Stop()
    {
        Process.Kill();
    }
}
