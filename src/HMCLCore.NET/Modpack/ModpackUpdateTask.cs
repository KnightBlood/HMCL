using System;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Modpack;

/// <summary>
/// Updates an instance to a newer version of the modpack it was created from.
/// The instance directory is backed up before the update; when the update fails
/// the backup is restored so the instance keeps working.
/// </summary>
public sealed class ModpackUpdateTask : TaskBase
{
    private readonly string _instanceDirectory;
    private readonly string _archivePath;
    private readonly Modpack _modpack;
    private readonly Func<int, int, Task<string?>>? _curseForgeFileResolver;

    /// <summary>
    /// Creates a new modpack update task.
    /// </summary>
    /// <param name="instanceDirectory">The instance root to update.</param>
    /// <param name="archivePath">The archive of the new modpack version.</param>
    /// <param name="modpack">The metadata of the new modpack version.</param>
    /// <param name="curseForgeFileResolver">
    /// Optional resolver mapping CurseForge (projectID, fileID) pairs to download URLs.
    /// </param>
    public ModpackUpdateTask(string instanceDirectory, string archivePath, Modpack modpack,
        Func<int, int, Task<string?>>? curseForgeFileResolver = null)
    {
        _instanceDirectory = instanceDirectory ?? throw new ArgumentNullException(nameof(instanceDirectory));
        _archivePath = archivePath ?? throw new ArgumentNullException(nameof(archivePath));
        _modpack = modpack ?? throw new ArgumentNullException(nameof(modpack));
        _curseForgeFileResolver = curseForgeFileResolver;
        Name = $"更新整合包 {modpack.Name}";
    }

    /// <summary>
    /// The directory holding the pre-update backup, or null before the task ran.
    /// </summary>
    public string? BackupDirectory { get; private set; }

    /// <summary>
    /// The computed update plan, or null before the task ran.
    /// </summary>
    public ModpackUpdatePlan? Plan { get; private set; }

    /// <summary>
    /// Applies the update: computes the differences, backs up the instance,
    /// installs the new pack content and removes the files the new version dropped.
    /// </summary>
    /// <param name="token">A cancellation token.</param>
    public override async Task Run(CancellationToken token = default)
    {
        var installed = ModpackConfiguration.Read(_instanceDirectory);
        var target = ModpackUpdatePlanner.Describe(_archivePath, _modpack);
        Plan = ModpackUpdatePlanner.Compare(installed, target);

        // Back up the instance so a failed update can be rolled back.
        BackupDirectory = CreateBackupDirectory();
        CopyDirectory(_instanceDirectory, BackupDirectory);

        try
        {
            var install = new ModpackInstallTask(_archivePath, _instanceDirectory, _modpack, _curseForgeFileResolver);
            await install.Run(token);

            RemoveObsoleteFiles(installed, target);
            OnProgressChanged(100, 100);
        }
        catch
        {
            Rollback();
            throw;
        }
    }

    /// <summary>
    /// Deletes the files that the previous modpack version contributed but the
    /// new version no longer contains.
    /// </summary>
    /// <param name="installed">The configuration of the previous version.</param>
    /// <param name="target">The description of the new version.</param>
    private void RemoveObsoleteFiles(ModpackConfiguration? installed, ModpackConfiguration target)
    {
        if (installed == null)
            return;

        var targetPaths = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var file in target.Files)
            targetPaths.Add(file.Path);

        foreach (var file in installed.Files)
        {
            if (targetPaths.Contains(file.Path))
                continue;

            var path = Path.Combine(_instanceDirectory, file.Path);
            if (File.Exists(path))
            {
                try
                {
                    File.Delete(path);
                }
                catch (IOException)
                {
                    // A locked file is kept; the update continues without it.
                }
            }
        }
    }

    /// <summary>
    /// Creates an unused backup directory next to the instance.
    /// </summary>
    private string CreateBackupDirectory()
    {
        var backupRoot = Path.Combine(_instanceDirectory, "backup");
        Directory.CreateDirectory(backupRoot);

        while (true)
        {
            var candidate = Path.Combine(backupRoot,
                $"{Path.GetFileName(_instanceDirectory.TrimEnd(Path.DirectorySeparatorChar))}-{Random.Shared.Next(10000000)}");
            if (!Directory.Exists(candidate) && !File.Exists(candidate))
                return candidate;
        }
    }

    /// <summary>
    /// Restores the instance from the backup folder.
    /// </summary>
    private void Rollback()
    {
        if (BackupDirectory == null || !Directory.Exists(BackupDirectory))
            return;

        try
        {
            if (Directory.Exists(_instanceDirectory))
                Directory.Delete(_instanceDirectory, recursive: true);

            CopyDirectory(BackupDirectory, _instanceDirectory);
        }
        catch (IOException)
        {
            // The backup is kept so the user can restore it manually.
        }
    }

    /// <summary>
    /// Recursively copies a directory.
    /// </summary>
    /// <param name="source">The source directory.</param>
    /// <param name="destination">The destination directory.</param>
    private static void CopyDirectory(string source, string destination)
    {
        Directory.CreateDirectory(destination);

        foreach (var directory in Directory.GetDirectories(source))
        {
            CopyDirectory(directory, Path.Combine(destination, Path.GetFileName(directory)));
        }

        foreach (var file in Directory.GetFiles(source))
        {
            File.Copy(file, Path.Combine(destination, Path.GetFileName(file)), overwrite: true);
        }
    }
}
