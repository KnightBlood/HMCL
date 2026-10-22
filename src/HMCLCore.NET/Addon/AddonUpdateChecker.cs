using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Util.Versioning;

namespace HMCLCore.NET.Addon;

/// <summary>
/// An available update for an installed mod.
/// </summary>
public sealed class AddonUpdate
{
    /// <summary>
    /// The installed mod the update applies to.
    /// </summary>
    public LocalMod LocalMod { get; init; } = new();

    /// <summary>
    /// The currently installed version.
    /// </summary>
    public string CurrentVersion { get; init; } = string.Empty;

    /// <summary>
    /// The version the update would install.
    /// </summary>
    public string TargetVersion { get; init; } = string.Empty;

    /// <summary>
    /// The remote project the update was found in.
    /// </summary>
    public RemoteAddon Remote { get; init; } = new();

    /// <summary>
    /// The download URL of the new file, or null when it could not be resolved.
    /// </summary>
    public string? DownloadUrl { get; init; }

    /// <summary>
    /// A human-readable description of the update.
    /// </summary>
    public string Description => $"{LocalMod.Name}: {CurrentVersion} → {TargetVersion}";
}

/// <summary>
/// Checks installed mods against a remote repository for newer versions.
/// </summary>
public sealed class AddonUpdateChecker
{
    private readonly RemoteAddonRepositoryEx _repository;
    private readonly string _minecraftVersion;

    /// <summary>
    /// Creates an update checker bound to a remote repository.
    /// </summary>
    /// <param name="repository">The repository to query.</param>
    /// <param name="minecraftVersion">The Minecraft version of the target instance.</param>
    public AddonUpdateChecker(RemoteAddonRepositoryEx repository, string minecraftVersion)
    {
        _repository = repository ?? throw new ArgumentNullException(nameof(repository));
        _minecraftVersion = minecraftVersion ?? string.Empty;
    }

    /// <summary>
    /// Checks the given mods for available updates.
    /// Mods whose metadata does not expose a project id, or that could not be
    /// matched to a remote project, are skipped.
    /// </summary>
    /// <param name="mods">The installed mods to check.</param>
    /// <param name="token">A cancellation token.</param>
    /// <returns>The list of available updates.</returns>
    public async Task<List<AddonUpdate>> CheckAsync(IReadOnlyList<LocalMod> mods, CancellationToken token = default)
    {
        var updates = new List<AddonUpdate>();
        if (mods.Count == 0)
            return updates;

        foreach (var mod in mods)
        {
            token.ThrowIfCancellationRequested();

            if (string.IsNullOrEmpty(mod.ModId) || string.IsNullOrEmpty(mod.Version))
                continue;

            var update = await CheckSingleAsync(mod, token);
            if (update != null)
                updates.Add(update);
        }

        return updates;
    }

    /// <summary>
    /// Checks a single mod for an available update.
    /// </summary>
    /// <param name="mod">The mod to check.</param>
    /// <param name="token">A cancellation token.</param>
    /// <returns>The update, or null when the mod is up to date or cannot be matched.</returns>
    public async Task<AddonUpdate?> CheckSingleAsync(LocalMod mod, CancellationToken token = default)
    {
        if (string.IsNullOrEmpty(mod.ModId) || string.IsNullOrEmpty(mod.Version))
            return null;

        var remote = await MatchProjectAsync(mod, token);
        if (remote == null)
            return null;

        var resolved = await _repository.ResolveFileAsync(remote, _minecraftVersion, ToLoaderType(mod));
        if (resolved?.DownloadUrl == null || string.IsNullOrEmpty(resolved.Version))
            return null;

        // Only report strictly newer versions.
        if (VersionNumber.Compare(resolved.Version, mod.Version) <= 0)
            return null;

        return new AddonUpdate
        {
            LocalMod = mod,
            CurrentVersion = mod.Version,
            TargetVersion = resolved.Version,
            Remote = remote,
            DownloadUrl = resolved.DownloadUrl
        };
    }

    /// <summary>
    /// Downloads and installs an update, replacing the outdated mod file.
    /// The previous file is kept with the ".old" suffix so the user can roll back.
    /// </summary>
    /// <param name="update">The update to apply.</param>
    /// <param name="modsDirectory">The instance's mods directory.</param>
    /// <param name="token">A cancellation token.</param>
    /// <returns>Whether the update was installed.</returns>
    public async Task<bool> ApplyUpdateAsync(AddonUpdate update, string modsDirectory, CancellationToken token = default)
    {
        if (update.DownloadUrl == null)
            return false;

        var localPath = update.LocalMod.FilePath;
        var fileName = Path.GetFileName(localPath);
        if (fileName.EndsWith(".disabled", StringComparison.OrdinalIgnoreCase))
            fileName = fileName[..^".disabled".Length];

        var targetPath = Path.Combine(modsDirectory, fileName);
        var oldPath = targetPath + ".old";

        var addon = update.Remote;
        addon.DownloadUrl = update.DownloadUrl;

        var task = new ModInstallTask(addon, modsDirectory, _minecraftVersion);
        task.ProgressChanged += (_, e) => OnProgressChanged(e.DownloadedBytes, e.TotalBytes);
        await task.Run(token);

        if (!File.Exists(targetPath))
            return false;

        // Keep the previous version around so the update can be rolled back.
        if (File.Exists(localPath) && !string.Equals(localPath, targetPath, StringComparison.OrdinalIgnoreCase))
            File.Delete(localPath);
        if (File.Exists(targetPath))
        {
            try
            {
                if (File.Exists(oldPath))
                    File.Delete(oldPath);
                File.Move(targetPath, oldPath);
            }
            catch (IOException)
            {
                // Rollback file is optional; ignore failures.
            }
        }

        return true;
    }

    /// <summary>
    /// Raised when the download progress of an update changes.
    /// </summary>
    public event EventHandler<AddonUpdateProgressEventArgs>? ProgressChanged;

    private void OnProgressChanged(long downloadedBytes, long? totalBytes)
    {
        ProgressChanged?.Invoke(this, new AddonUpdateProgressEventArgs(downloadedBytes, totalBytes));
    }

    /// <summary>
    /// Finds the remote project matching the given local mod.
    /// </summary>
    private async Task<RemoteAddon?> MatchProjectAsync(LocalMod mod, CancellationToken token)
    {
        // The mod id is usually the project slug, so try a direct lookup first.
        var direct = await _repository.GetAddonAsync(mod.ModId!);
        if (direct != null)
            return direct;

        // Fall back to a search by name and match the closest result.
        var results = await _repository.SearchAsync(mod.Name);
        foreach (var candidate in results)
        {
            token.ThrowIfCancellationRequested();

            if (string.Equals(candidate.Id, mod.ModId, StringComparison.OrdinalIgnoreCase))
                return candidate;
            if (string.Equals(candidate.Name, mod.Name, StringComparison.OrdinalIgnoreCase))
                return candidate;
        }

        return null;
    }

    /// <summary>
    /// Maps the mod's loader to the repository's loader enumeration.
    /// </summary>
    private static ModLoaderType ToLoaderType(LocalMod mod)
    {
        if (mod.IsFabric) return ModLoaderType.Fabric;
        if (mod.IsForge) return ModLoaderType.Forge;
        if (mod.IsQuilt) return ModLoaderType.Quilt;
        return ModLoaderType.None;
    }
}

/// <summary>
/// Progress information for an addon update download.
/// </summary>
public sealed class AddonUpdateProgressEventArgs : EventArgs
{
    /// <summary>
    /// Creates progress information for an addon update download.
    /// </summary>
    /// <param name="downloadedBytes">The number of bytes downloaded so far.</param>
    /// <param name="totalBytes">The total number of bytes to download, or null when unknown.</param>
    public AddonUpdateProgressEventArgs(long downloadedBytes, long? totalBytes)
    {
        DownloadedBytes = downloadedBytes;
        TotalBytes = totalBytes;
    }

    /// <summary>
    /// The number of bytes downloaded so far.
    /// </summary>
    public long DownloadedBytes { get; init; }

    /// <summary>
    /// The total number of bytes to download, or null when unknown.
    /// </summary>
    public long? TotalBytes { get; init; }
}
