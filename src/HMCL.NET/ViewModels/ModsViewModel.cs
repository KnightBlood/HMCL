using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using HMCLCore.NET.Addon;
using HMCLCore.NET.Game;
using HMCL.NET.Util;

namespace HMCL.NET.ViewModels;

/// <summary>
/// View model driving the mod management dialog: scans the instance's mods
/// directory, searches remote repositories (Modrinth by default) and installs
/// or toggles mods.
/// </summary>
public sealed class ModsViewModel : ViewModelBase
{
    private readonly GameInstance _instance;
    private readonly string _modsDirectory;
    private readonly LocalAddonManager _localManager;
    private readonly IReadOnlyList<RemoteAddonRepositoryEx> _repositories;

    private string _searchQuery = string.Empty;
    private string _selectedRepository = "Modrinth";
    private string _statusMessage = string.Empty;
    private bool _isBusy;

    /// <summary>
    /// Creates a new ModsViewModel for the given game instance.
    /// </summary>
    /// <param name="instance">The instance whose mods directory is managed.</param>
    /// <param name="mainViewModel">The launcher view model providing settings access; may be null in previews.</param>
    public ModsViewModel(GameInstance instance, MainWindowViewModel? mainViewModel = null)
    {
        _instance = instance ?? throw new ArgumentNullException(nameof(instance));
        _modsDirectory = Path.Combine(instance.GameDirectory, "mods");
        _localManager = new LocalAddonManager(_modsDirectory);

        var modrinth = new ModrinthRepository();
        var curseForge = new CurseForgeRepository();
        if (mainViewModel != null)
            curseForge.ApiKey = mainViewModel.SettingsService.Settings.CurseForgeApiKey;

        // Modrinth is the default; CurseForge requires a configured API key.
        _repositories = new List<RemoteAddonRepositoryEx> { modrinth, curseForge };

        InstalledMods = new ObservableCollection<LocalMod>();
        RemoteMods = new ObservableCollection<RemoteAddon>();
        AvailableUpdates = new ObservableCollection<AddonUpdate>();
        ReloadInstalled();
    }

    /// <summary>
    /// The repository names offered in the dialog.
    /// </summary>
    public IReadOnlyList<string> RepositoryNames { get; } = new List<string> { "Modrinth", "CurseForge" };

    /// <summary>
    /// The currently selected remote repository name.
    /// </summary>
    public string SelectedRepository
    {
        get => _selectedRepository;
        set => Set(ref _selectedRepository, value);
    }

    /// <summary>
    /// The repository backing the currently selected name.
    /// </summary>
    private RemoteAddonRepositoryEx ActiveRepository
    {
        get
        {
            var repository = _repositories.FirstOrDefault(r => r.Name == SelectedRepository);
            return repository ?? _repositories[0];
        }
    }

    /// <summary>
    /// The instance the dialog belongs to.
    /// </summary>
    public GameInstance Instance => _instance;

    /// <summary>
    /// The mods installed in the instance's mods directory.
    /// </summary>
    public ObservableCollection<LocalMod> InstalledMods { get; }

    /// <summary>
    /// The remote addon search results.
    /// </summary>
    public ObservableCollection<RemoteAddon> RemoteMods { get; }

    /// <summary>
    /// The updates available for the installed mods.
    /// </summary>
    public ObservableCollection<AddonUpdate> AvailableUpdates { get; }

    /// <summary>
    /// The selected available update.
    /// </summary>
    public AddonUpdate? SelectedUpdate { get; set; }

    /// <summary>
    /// The selected installed mod for enable/disable/delete actions.
    /// </summary>
    public LocalMod? SelectedInstalledMod { get; set; }

    /// <summary>
    /// The selected remote addon for installation.
    /// </summary>
    public RemoteAddon? SelectedRemoteMod { get; set; }

    /// <summary>
    /// The current search query for the remote repository.
    /// </summary>
    public string SearchQuery
    {
        get => _searchQuery;
        set => Set(ref _searchQuery, value);
    }

    /// <summary>
    /// The status message shown at the bottom of the dialog.
    /// </summary>
    public string StatusMessage
    {
        get => _statusMessage;
        set => Set(ref _statusMessage, value);
    }

    /// <summary>
    /// Whether a repository or install operation is running.
    /// </summary>
    public bool IsBusy
    {
        get => _isBusy;
        set => Set(ref _isBusy, value);
    }

    /// <summary>
    /// Rescans the instance's mods directory and refreshes the installed list.
    /// </summary>
    public void ReloadInstalled()
    {
        _localManager.Scan();
        InstalledMods.Clear();
        foreach (var mod in _localManager.Mods)
            InstalledMods.Add(mod);
    }

    /// <summary>
    /// Searches the remote repository for the entered query and refreshes the remote list.
    /// </summary>
    /// <returns>Whether new results were loaded.</returns>
    public async Task<bool> SearchRemoteModsAsync()
    {
        if (string.IsNullOrWhiteSpace(SearchQuery))
            return false;

        var repository = ActiveRepository;
        if (repository is CurseForgeRepository { ApiKey: "" or null } && SelectedRepository == "CurseForge")
        {
            StatusMessage = Lang.Get("hmcl.modsviewmodel.804_");
            return false;
        }

        IsBusy = true;
        StatusMessage = $"正在搜索 {repository.Name}...";
        try
        {
            RemoteMods.Clear();
            var results = await repository.SearchAsync(SearchQuery);
            foreach (var addon in results)
                RemoteMods.Add(addon);
            StatusMessage = $"共找到 {results.Count} 个结果";
            return results.Count > 0;
        }
        catch (Exception ex)
        {
            StatusMessage = $"搜索失败: {ex.Message}";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Resolves and installs the selected remote addon into the mods directory,
    /// then refreshes the installed list.
    /// </summary>
    /// <returns>Whether the mod was installed.</returns>
    public async Task<bool> InstallRemoteModAsync()
    {
        if (SelectedRemoteMod == null)
            return false;

        var repository = ActiveRepository;
        IsBusy = true;
        StatusMessage = $"正在安装 {SelectedRemoteMod.Name}...";
        try
        {
            // Resolve a concrete file matching the instance's game version.
            var resolved = await repository.ResolveFileAsync(
                SelectedRemoteMod, _instance.Version, ModLoaderType.None);
            if (resolved?.DownloadUrl == null)
            {
                StatusMessage = $"未找到与 {_instance.Version} 兼容的版本";
                return false;
            }

            var task = new ModInstallTask(resolved, _modsDirectory, _instance.Version);
            task.ProgressChanged += (_, e) =>
            {
                if (e.TotalBytes is { } total && total > 0)
                    StatusMessage = $"下载中: {resolved.Name} ({e.DownloadedBytes / 1024}/{total / 1024} KB)";
            };
            await task.Run();

            ReloadInstalled();
            StatusMessage = $"已安装 {resolved.Name}";
            return true;
        }
        catch (Exception ex)
        {
            StatusMessage = $"安装失败: {ex.Message}";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Checks the installed mods for newer versions in the selected repository.
    /// Mods that cannot be matched to a remote project are skipped silently.
    /// </summary>
    /// <returns>The number of available updates.</returns>
    public async Task<int> CheckForUpdatesAsync()
    {
        var repository = ActiveRepository;
        if (repository is CurseForgeRepository { ApiKey: "" or null } && SelectedRepository == "CurseForge")
        {
            StatusMessage = Lang.Get("hmcl.modsviewmodel.806_");
            return 0;
        }

        IsBusy = true;
        StatusMessage = Lang.Get("hmcl.modsviewmodel.808_");
        AvailableUpdates.Clear();
        try
        {
            var checker = new AddonUpdateChecker(repository, _instance.Version);
            var updates = await checker.CheckAsync(InstalledMods.ToList());
            foreach (var update in updates)
                AvailableUpdates.Add(update);

            UpdateUpdateList();

            StatusMessage = updates.Count > 0
                ? $"发现 {updates.Count} 个可用更新"
                : Lang.Get("hmcl.modsviewmodel.810_");
            return updates.Count;
        }
        catch (Exception ex)
        {
            StatusMessage = $"检查更新失败: {ex.Message}";
            return 0;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Whether the list shows the installed mods instead of the available
    /// updates, like the preview of the updates of Java HMCL.
    /// </summary>
    public bool PreviewUpdates
    {
        get => _previewUpdates;
        set
        {
            if (Set(ref _previewUpdates, value))
                UpdateUpdateList();
        }
    }

    private bool _previewUpdates;

    /// <summary>
    /// The mods shown in the list; they are the installed ones while the
    /// preview is shown, and the available updates otherwise.
    /// </summary>
    public ObservableCollection<object> UpdateList { get; } = new();

    /// <summary>
    /// Refreshes the list of the updates shown to the user.
    /// </summary>
    private void UpdateUpdateList()
    {
        UpdateList.Clear();

        if (PreviewUpdates)
        {
            foreach (var mod in InstalledMods)
                UpdateList.Add(mod);
        }
        else
        {
            foreach (var update in AvailableUpdates)
                UpdateList.Add(update);
        }
    }

    /// <summary>
    /// Writes the list of the available updates into a file.
    /// </summary>
    /// <param name="destinationPath">The path of the written file.</param>
    /// <returns>Whether the list was written.</returns>
    public bool ExportUpdateList(string destinationPath)
    {
        try
        {
            var lines = AvailableUpdates.Select(update =>
                $"{update.LocalMod.Name}\t{update.CurrentVersion} → {update.TargetVersion}");

            var builder = new System.Text.StringBuilder();
            builder.AppendLine(string.Join('\t', new[] { "模组", "当前版本", "目标版本" }));
            foreach (var line in lines)
                builder.AppendLine(line);

            File.WriteAllText(destinationPath, builder.ToString());
            StatusMessage = $"已导出更新列表: {destinationPath}";
            return true;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            StatusMessage = $"导出更新列表失败: {ex.Message}";
            return false;
        }
    }

    /// <summary>
    /// Returns the changelog of an update, like the changelog of the updates of
    /// Java HMCL.
    /// </summary>
    /// <param name="update">The update whose changelog is shown.</param>
    /// <returns>The text of the changelog.</returns>
    public static string GetChangelog(AddonUpdate update)
    {
        var description = update.Remote?.Description;
        if (string.IsNullOrWhiteSpace(description))
            return Lang.Get("addon.changelog.empty", "该版本没有更新日志。");

        return Views.ModpackDescriptionWindow.StripHtml(description);
    }

    /// <summary>
    /// Installs a single available update, replacing the outdated mod file.
    /// </summary>
    /// <param name="update">The update to install.</param>
    /// <returns>Whether the update was installed.</returns>
    public async Task<bool> ApplyUpdateAsync(AddonUpdate update)
    {
        IsBusy = true;
        StatusMessage = $"正在更新 {update.LocalMod.Name}...";
        try
        {
            var checker = new AddonUpdateChecker(ActiveRepository, _instance.Version);
            var installed = await checker.ApplyUpdateAsync(update, _modsDirectory);
            if (installed)
            {
                AvailableUpdates.Remove(update);
                ReloadInstalled();
                StatusMessage = $"已更新 {update.LocalMod.Name} 到 {update.TargetVersion}";
            }
            else
            {
                StatusMessage = $"更新 {update.LocalMod.Name} 失败";
            }

            return installed;
        }
        catch (Exception ex)
        {
            StatusMessage = $"更新失败: {ex.Message}";
            return false;
        }
        finally
        {
            IsBusy = false;
        }
    }

    /// <summary>
    /// Installs the selected available update.
    /// </summary>
    /// <returns>Whether the update was installed.</returns>
    public async Task<bool> ApplySelectedUpdateAsync()
    {
        if (SelectedUpdate == null)
            return false;

        return await ApplyUpdateAsync(SelectedUpdate);
    }

    /// <summary>
    /// Installs every available update.
    /// </summary>
    /// <returns>The number of successfully installed updates.</returns>
    public async Task<int> ApplyAllUpdatesAsync()
    {
        var pending = AvailableUpdates.ToList();
        var installed = 0;
        foreach (var update in pending)
        {
            if (await ApplyUpdateAsync(update))
                installed++;
        }

        return installed;
    }

    /// <summary>
    /// Toggles the enabled state of the selected installed mod.
    /// </summary>
    /// <returns>Whether the mod was toggled.</returns>
    public bool ToggleSelectedMod()
    {
        if (SelectedInstalledMod == null)
            return false;

        var changed = _localManager.ToggleEnabled(SelectedInstalledMod);
        ReloadInstalled();
        StatusMessage = changed != null
            ? $"已{(SelectedInstalledMod.Enabled ? "启用" : Lang.Get("hmcl.modsviewmodel.mods_disable"))} {SelectedInstalledMod.Name}"
            : Lang.Get("hmcl.modsviewmodel.814_");
        return changed != null;
    }

    /// <summary>
    /// Deletes the selected installed mod file.
    /// </summary>
    /// <returns>Whether the mod was deleted.</returns>
    public bool DeleteSelectedMod()
    {
        if (SelectedInstalledMod == null)
            return false;

        var name = SelectedInstalledMod.Name;
        var deleted = _localManager.DeleteMod(SelectedInstalledMod);
        ReloadInstalled();
        StatusMessage = deleted ? $"已删除 {name}" : Lang.Get("hmcl.modsviewmodel.816_");
        return deleted;
    }

    /// <summary>
    /// Installs the given mod files into the instance's mods directory. Used
    /// when the user drops mod files onto the mod manager.
    /// </summary>
    /// <param name="filePaths">The paths of the mod files to install.</param>
    /// <returns>The number of mods that were installed.</returns>
    public int InstallLocalMods(IReadOnlyList<string> filePaths)
    {
        if (filePaths == null || filePaths.Count == 0)
            return 0;

        var installed = 0;
        var failed = 0;

        foreach (var path in filePaths)
        {
            if (_localManager.Install(path) != null)
                installed++;
            else
                failed++;
        }

        ReloadInstalled();

        StatusMessage = failed == 0
            ? $"已导入 {installed} 个模组"
            : $"已导入 {installed} 个模组，{failed} 个文件无法导入";

        return installed;
    }
}
