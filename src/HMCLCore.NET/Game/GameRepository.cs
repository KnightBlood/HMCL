using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Game;

/// <summary>
/// Provides indexed access to local game instances.
/// Ported from Java GameRepository interface.
/// </summary>
public interface IGameRepository
{
    /// <summary>
    /// The filesystem layout used by this repository.
    /// </summary>
    GameRepositoryLayout Layout { get; }

    /// <summary>
    /// Returns the current published snapshot of the registered instance index.
    /// </summary>
    GameRepositorySnapshot Snapshot { get; }

    /// <summary>
    /// Returns whether the instance exists in the current repository index.
    /// </summary>
    bool HasInstance(GameInstanceID instanceId);

    /// <summary>
    /// Returns the indexed game instance for the given id.
    /// </summary>
    GameInstance GetInstance(GameInstanceID id);

    /// <summary>
    /// Returns the number of loaded instances.
    /// </summary>
    int InstanceCount { get; }

    /// <summary>
    /// Reloads repository state from backing storage.
    /// </summary>
    void Refresh();

    /// <summary>
    /// Creates a task that reloads repository state from backing storage.
    /// </summary>
    Task RefreshAsync();

    /// <summary>
    /// Returns the directory containing files owned by an instance.
    /// </summary>
    string GetInstanceRoot(GameInstanceID instanceId);

    /// <summary>
    /// Renames an instance.
    /// </summary>
    bool RenameInstance(GameInstanceID from, GameInstanceID to);

    /// <summary>
    /// Deletes an instance and its backing directory from the repository.
    /// </summary>
    /// <returns>Whether the instance was found and removed.</returns>
    bool DeleteInstance(GameInstanceID instanceId);
}

/// <summary>
/// Default implementation of the game repository.
/// </summary>
public sealed class DefaultGameRepository : IGameRepository
{
    private readonly GameRepositoryLayout _layout;
    private GameRepositorySnapshot _snapshot;

    public DefaultGameRepository(GameRepositoryLayout layout)
    {
        _layout = layout ?? throw new ArgumentNullException(nameof(layout));
        _snapshot = new GameRepositorySnapshot(new Dictionary<GameInstanceID, GameInstanceSnapshotEntry>());
    }

    public GameRepositoryLayout Layout => _layout;

    public GameRepositorySnapshot Snapshot => _snapshot;

    public int InstanceCount => _snapshot.InstanceCount;

    public bool HasInstance(GameInstanceID instanceId) => _snapshot.HasInstance(instanceId);

    public GameInstance GetInstance(GameInstanceID id) => _snapshot.GetInstance(id);

    public void Refresh()
    {
        // Load instances from disk
        var instances = new Dictionary<GameInstanceID, GameInstanceSnapshotEntry>();
        
        var instancesRoot = _layout.InstancesDirectory;
        if (Directory.Exists(instancesRoot))
        {
            foreach (var dir in Directory.GetDirectories(instancesRoot))
            {
                var instanceId = new GameInstanceID { Value = Path.GetFileName(dir) };
                var manifestPath = Path.Combine(dir, "instance.json");
                
                GameInstance instance;
                if (File.Exists(manifestPath))
                {
                    var json = File.ReadAllText(manifestPath);
                    var manifest = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.GameInstanceManifest);
                    instance = new GameInstance
                    {
                        Id = instanceId.Value,
                        Version = manifest?.Version ?? string.Empty,
                        Profile = instanceId.Value,
                        GameDirectory = dir,
                        Type = manifest?.JavaVersion > 0 ? "release" : "unknown",
                        LastLaunch = manifest?.LastPlayed ?? default,
                        Loader = DetectLoader(manifest?.MainClass),
                        LoaderVersion = DetectLoaderVersion(manifest),
                        IconPath = GameInstanceIcons.GetCustomIconPath(dir)
                    };
                }
                else
                {
                    instance = new GameInstance
                    {
                        Id = instanceId.Value,
                        Profile = instanceId.Value,
                        GameDirectory = dir,
                        IconPath = GameInstanceIcons.GetCustomIconPath(dir)
                    };
                }

                instances[instanceId] = new GameInstanceSnapshotEntry(instance);
            }
        }

        _snapshot = new GameRepositorySnapshot(instances);
    }

    /// <summary>
    /// Infers the mod loader from a version's main class.
    /// </summary>
    /// <param name="mainClass">The version main class.</param>
    /// <returns>Vanilla, Fabric, Quilt, Forge or NeoForge.</returns>
    private static string DetectLoader(string? mainClass)
    {
        if (string.IsNullOrEmpty(mainClass))
            return "Vanilla";

        if (mainClass.Contains("quilt", StringComparison.OrdinalIgnoreCase))
            return "Quilt";
        if (mainClass.Contains("fabric", StringComparison.OrdinalIgnoreCase) ||
            mainClass.Contains("knot", StringComparison.OrdinalIgnoreCase))
            return "Fabric";
        if (mainClass.Contains("neoforge", StringComparison.OrdinalIgnoreCase))
            return "NeoForge";
        if (mainClass.Contains("forge", StringComparison.OrdinalIgnoreCase) ||
            mainClass.Contains("bootstraplauncher", StringComparison.OrdinalIgnoreCase))
            return "Forge";

        return "Vanilla";
    }

    /// <summary>
    /// Infers the version of the mod loader from the patches of a manifest.
    /// </summary>
    /// <param name="manifest">The manifest of the instance.</param>
    /// <returns>The version of the mod loader, or an empty string when it is unknown.</returns>
    private static string DetectLoaderVersion(GameInstanceManifest? manifest)
    {
        if (manifest is null)
            return string.Empty;

        var patch = manifest.Patches.FirstOrDefault(p =>
            p.Priority == GameInstancePatch.PriorityLoader);

        return patch?.Version ?? string.Empty;
    }

    public async Task RefreshAsync()
    {
        await Task.Run(() => Refresh());
    }

    public string GetInstanceRoot(GameInstanceID instanceId) => _layout.GetInstanceRoot(instanceId);

    public bool RenameInstance(GameInstanceID from, GameInstanceID to)
    {
        if (!_snapshot.HasInstance(from))
            return false;

        var fromRoot = GetInstanceRoot(from);
        var toRoot = GetInstanceRoot(to);

        if (!Directory.Exists(fromRoot) || Directory.Exists(toRoot))
            return false;

        try
        {
            Directory.Move(fromRoot, toRoot);
            Refresh();
            return true;
        }
        catch
        {
            return false;
        }
    }

    public bool DeleteInstance(GameInstanceID instanceId)
    {
        if (!_snapshot.HasInstance(instanceId))
            return false;

        var root = GetInstanceRoot(instanceId);

        if (!Directory.Exists(root))
            return false;

        try
        {
            Directory.Delete(root, recursive: true);
            Refresh();
            return true;
        }
        catch
        {
            return false;
        }
    }

    /// <summary>
    /// Instantaneous output directories that don't belong in a backup.
    /// </summary>
    private static readonly List<string> BackupExcludedDirectories =
        new() { "logs", "crash-reports", "cache", "screenshots", "versions", "libraries", "assets", "natives" };

    /// <summary>
    /// Creates a zip archive with the essential instance files (manifest, mods,
    /// configs, saves) under backups/{instanceId} and returns its path.
    /// </summary>
    /// <param name="instanceId">The instance to back up.</param>
    /// <returns>The archive path, or null when archiving failed.</returns>
    public string? BackupInstance(GameInstanceID instanceId) =>
        CreateInstanceArchive(instanceId, backupsDirectory =>
        {
            Directory.CreateDirectory(backupsDirectory);
            return Path.Combine(backupsDirectory,
                $"{instanceId.Value}-{DateTime.Now:yyyyMMdd-HHmmss}.zip");
        });

    /// <summary>
    /// Creates a shareable instance archive at an arbitrary destination path,
    /// with the same content set as a backup archive.
    /// </summary>
    /// <param name="instanceId">The instance to export.</param>
    /// <param name="destinationPath">The target zip file path.</param>
    /// <returns>The archive path, or null when archiving failed.</returns>
    public string? ExportInstance(GameInstanceID instanceId, string destinationPath) =>
        CreateInstanceArchive(instanceId, _ => destinationPath);

    /// <summary>
    /// Shared archive builder: walks the instance directory, skipping the excluded
    /// output directories, and writes every remaining file to a zip archive at
    /// the path chosen by the caller.
    /// </summary>
    /// <param name="instanceId">The instance to archive.</param>
    /// <param name="resolvePath">Maps the backups directory to the archive path.</param>
    private string? CreateInstanceArchive(GameInstanceID instanceId, Func<string, string> resolvePath)
    {
        if (!_snapshot.HasInstance(instanceId))
            return null;

        var root = GetInstanceRoot(instanceId);
        if (!Directory.Exists(root))
            return null;

        try
        {
            var archivePath = resolvePath(Path.Combine(_layout.BaseDirectory, "backups", instanceId.Value));

            using var archive = ZipFile.Open(archivePath, ZipArchiveMode.Create);
            AddDirectoryEntries(root, root, string.Empty, archive);

            return archivePath;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Validates an instance id and produces a canonical form: stripped of
    /// path-hostile characters and any trailing backup timestamp suffix.
    /// </summary>
    /// <param name="name">The raw name, typically the backup file name.</param>
    public static string SanitizeInstanceName(string name)
    {
        foreach (var c in Path.GetInvalidFileNameChars())
            name = name.Replace(c, '_');

        // Strip the "-yyyyMMdd-HHmmss" suffix produced by BackupInstance.
        var stem = name.Length >= 17 && name.Contains('-') ?
            System.Text.RegularExpressions.Regex.Replace(name, @"-20\d{6}-\d{6}$", string.Empty) : name;

        return stem.Length == 0 ? "instance" : stem;
    }

    /// <summary>
    /// Restores a backup archive into a new instance directory derived from the
    /// archive file name, generating a numbered suffix when the directory exists.
    /// </summary>
    /// <param name="archivePath">The zipped instance archive to restore.</param>
    /// <param name="preferredName">The preferred instance id (stem of the file name).</param>
    /// <returns>The restored instance id, or null when restoration failed.</returns>
    public string? RestoreInstance(string archivePath, string preferredName)
    {
        try
        {
            var baseName = SanitizeInstanceName(preferredName);
            string instanceId = baseName;
            int ordinal = 0;
            while (Directory.Exists(GetInstanceRoot(new GameInstanceID { Value = instanceId })))
            {
                ordinal++;
                instanceId = $"{baseName}-{ordinal}";
            }

            var targetDirectory = GetInstanceRoot(new GameInstanceID { Value = instanceId });
            Directory.CreateDirectory(targetDirectory);
            ZipFile.ExtractToDirectory(archivePath, targetDirectory, overwriteFiles: true);

            Refresh();
            return instanceId;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Recursively appends all backupable files of a directory into the archive,
    /// skipping the excluded output directories.
    /// </summary>
    private void AddDirectoryEntries(string rootDirectory, string currentDirectory, string prefix, ZipArchive archive)
    {
        foreach (var directory in Directory.GetDirectories(currentDirectory))
        {
            var directoryName = Path.GetFileName(directory);
            if (BackupExcludedDirectories.Contains(directoryName, StringComparer.OrdinalIgnoreCase))
                continue;

            AddDirectoryEntries(rootDirectory, directory, $"{prefix}{directoryName}/", archive);
        }

        foreach (var file in Directory.GetFiles(currentDirectory))
        {
            var entryName = $"{prefix}{Path.GetFileName(file)}";
            archive.CreateEntryFromFile(file, entryName, CompressionLevel.Optimal);
        }
    }
}
/// <summary>
/// Repository filesystem layout.
/// </summary>
public sealed class GameRepositoryLayout
{
    /// <summary>
    /// The base directory of the repository.
    /// </summary>
    public string BaseDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The directory containing game instances.
    /// </summary>
    public string InstancesDirectory => Path.Combine(BaseDirectory, "instances");

    /// <summary>
    /// Returns the root directory for an instance.
    /// </summary>
    public string GetInstanceRoot(GameInstanceID instanceId) => Path.Combine(InstancesDirectory, instanceId.Value);
}

/// <summary>
/// Immutable snapshot of registered game instances.
/// </summary>
public sealed class GameRepositorySnapshot
{
    private readonly Dictionary<GameInstanceID, GameInstanceSnapshotEntry> _instances;

    public GameRepositorySnapshot(Dictionary<GameInstanceID, GameInstanceSnapshotEntry> instances)
    {
        _instances = instances ?? new Dictionary<GameInstanceID, GameInstanceSnapshotEntry>();
    }

    public int InstanceCount => _instances.Count;

    public bool HasInstance(GameInstanceID instanceId) => _instances.ContainsKey(instanceId);

    /// <summary>
    /// Returns every registered game instance in the snapshot.
    /// </summary>
    public IReadOnlyList<GameInstance> GetAllInstances()
    {
        var instances = new List<GameInstance>(_instances.Count);
        foreach (var entry in _instances.Values)
            instances.Add(entry.Instance);
        return instances;
    }

    public GameInstance GetInstance(GameInstanceID id)
    {
        if (!_instances.TryGetValue(id, out var entry))
            throw new NoSuchGameInstanceException(id.Value);
        return entry.Instance;
    }
}

/// <summary>
/// Entry in the game repository snapshot.
/// </summary>
public sealed class GameInstanceSnapshotEntry
{
    public GameInstance Instance { get; }
    public GameInstanceManifest Manifest { get; }

    public GameInstanceSnapshotEntry(GameInstance instance)
    {
        Instance = instance;
        Manifest = new GameInstanceManifest
        {
            Version = instance.Version,
            MainClass = "net.minecraft.client.main.Main",
            Libraries = new List<Library>()
        };
    }
}

/// <summary>
/// Exception thrown when a game instance is not found.
/// </summary>
public sealed class NoSuchGameInstanceException : Exception
{
    public NoSuchGameInstanceException(string instanceId) 
        : base($"Game instance not found: {instanceId}") { }
}