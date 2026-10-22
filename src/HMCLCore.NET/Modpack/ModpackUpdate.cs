using System;
using System.Collections.Generic;
using System.IO;
using System.IO.Compression;
using System.Security.Cryptography;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Serialization;

namespace HMCLCore.NET.Modpack;

/// <summary>
/// The persisted description of the modpack an instance was created from.
/// It is stored as "modpack.cfg" inside the instance directory and lets the
/// launcher compute the differences when a newer version of the pack is
/// installed.
/// </summary>
public sealed class ModpackConfiguration
{
    /// <summary>
    /// The file name of the configuration inside the instance directory.
    /// </summary>
    public const string FileName = "modpack.cfg";

    /// <summary>
    /// The modpack name.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// The modpack version.
    /// </summary>
    public string? Version { get; set; }

    /// <summary>
    /// The Minecraft version the modpack targets.
    /// </summary>
    public string? GameVersion { get; set; }

    /// <summary>
    /// The files the modpack contributes to the instance directory.
    /// </summary>
    public List<ModpackConfigurationFile> Files { get; set; } = new();

    /// <summary>
    /// Reads the configuration of an instance directory.
    /// </summary>
    /// <param name="instanceDirectory">The instance root.</param>
    /// <returns>The configuration, or null when the instance has none.</returns>
    public static ModpackConfiguration? Read(string instanceDirectory)
    {
        var path = Path.Combine(instanceDirectory, FileName);
        if (!File.Exists(path))
            return null;

        try
        {
            return JsonSerializer.Deserialize(File.ReadAllText(path), LauncherJsonContext.Default.ModpackConfiguration);
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Writes the configuration into an instance directory.
    /// </summary>
    /// <param name="instanceDirectory">The instance root.</param>
    public void Write(string instanceDirectory)
    {
        Directory.CreateDirectory(instanceDirectory);
        var path = Path.Combine(instanceDirectory, FileName);
        File.WriteAllText(path, JsonSerializer.Serialize(this, LauncherJsonContext.Default.ModpackConfiguration));
    }
}

/// <summary>
/// A file contributed by a modpack to the instance directory.
/// </summary>
public sealed class ModpackConfigurationFile
{
    /// <summary>
    /// The path relative to the instance directory.
    /// </summary>
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// The SHA1 hash of the file content, or null when it could not be computed.
    /// </summary>
    public string? Sha1 { get; set; }
}

/// <summary>
/// The kind of change a modpack update applies to one file.
/// </summary>
public enum ModpackChangeKind
{
    /// <summary>
    /// The file is new and will be added.
    /// </summary>
    Added = 0,

    /// <summary>
    /// The file exists in both versions but its content differs.
    /// </summary>
    Modified = 1,

    /// <summary>
    /// The file was removed by the new version of the pack.
    /// </summary>
    Removed = 2,

    /// <summary>
    /// The file is identical in both versions.
    /// </summary>
    Unchanged = 3
}

/// <summary>
/// A single file difference between the installed modpack and the new one.
/// </summary>
public sealed class ModpackChange
{
    /// <summary>
    /// The path relative to the instance directory.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The kind of change.
    /// </summary>
    public ModpackChangeKind Kind { get; init; }

    /// <summary>
    /// The hash of the currently installed file, or null when it is missing.
    /// </summary>
    public string? OldHash { get; init; }

    /// <summary>
    /// The hash of the file in the new modpack, or null when it was removed.
    /// </summary>
    public string? NewHash { get; init; }

    /// <summary>
    /// A human-readable description of the change.
    /// </summary>
    public string Description => Kind switch
    {
        ModpackChangeKind.Added => $"新增 {Path}",
        ModpackChangeKind.Modified => $"修改 {Path}",
        ModpackChangeKind.Removed => $"删除 {Path}",
        _ => $"不变 {Path}"
    };
}

/// <summary>
/// The comparison between the installed modpack and a newer version of it.
/// </summary>
public sealed class ModpackUpdatePlan
{
    /// <summary>
    /// The modpack name.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The installed modpack version, or null when the instance has no modpack.
    /// </summary>
    public string? OldVersion { get; init; }

    /// <summary>
    /// The version of the new modpack.
    /// </summary>
    public string? NewVersion { get; init; }

    /// <summary>
    /// The per-file differences.
    /// </summary>
    public IReadOnlyList<ModpackChange> Changes { get; init; } = Array.Empty<ModpackChange>();

    /// <summary>
    /// The number of files that will be added.
    /// </summary>
    public int AddedCount => Count(ModpackChangeKind.Added);

    /// <summary>
    /// The number of files whose content changed.
    /// </summary>
    public int ModifiedCount => Count(ModpackChangeKind.Modified);

    /// <summary>
    /// The number of files that will be removed.
    /// </summary>
    public int RemovedCount => Count(ModpackChangeKind.Removed);

    /// <summary>
    /// The number of files that stay unchanged.
    /// </summary>
    public int UnchangedCount => Count(ModpackChangeKind.Unchanged);

    /// <summary>
    /// Whether the update would change anything at all.
    /// </summary>
    public bool HasChanges => AddedCount > 0 || ModifiedCount > 0 || RemovedCount > 0;

    /// <summary>
    /// A human-readable summary of the plan.
    /// </summary>
    public string Summary =>
        $"{Name}: {OldVersion ?? "未安装"} → {NewVersion ?? "未知"} " +
        $"(新增 {AddedCount}, 修改 {ModifiedCount}, 删除 {RemovedCount})";

    private int Count(ModpackChangeKind kind)
    {
        var count = 0;
        foreach (var change in Changes)
        {
            if (change.Kind == kind)
                count++;
        }

        return count;
    }
}

/// <summary>
/// Compares the modpack installed in an instance with the content of a newer
/// modpack archive, producing the list of file differences.
/// </summary>
public static class ModpackUpdatePlanner
{
    /// <summary>
    /// The archive prefixes that map into the instance directory.
    /// </summary>
    private static readonly string[] OverridePrefixes =
    {
        "overrides/", "clientOverrides/", "client-overrides/"
    };

    /// <summary>
    /// Builds the description of a modpack archive without extracting it.
    /// The override entries are hashed so the update planner can detect
    /// modified files.
    /// </summary>
    /// <param name="archivePath">The modpack archive.</param>
    /// <param name="modpack">The metadata of the modpack.</param>
    /// <returns>The description of the pack contents.</returns>
    public static ModpackConfiguration Describe(string archivePath, Modpack modpack)
    {
        var configuration = new ModpackConfiguration
        {
            Name = modpack.Name,
            Version = modpack.Version,
            GameVersion = modpack.GameVersion
        };

        using (var archive = ZipFile.OpenRead(archivePath))
        {
            foreach (var entry in archive.Entries)
            {
                if (!IsOverrideEntry(entry))
                    continue;

                var relativePath = GetRelativePath(entry);
                if (string.IsNullOrEmpty(relativePath) || string.IsNullOrEmpty(entry.Name))
                    continue;

                configuration.Files.Add(new ModpackConfigurationFile
                {
                    Path = relativePath,
                    Sha1 = ComputeEntryHash(entry)
                });
            }
        }

        foreach (var transfer in modpack.Manifest?.Files ?? new List<ModpackFileTransfer>())
        {
            if (string.IsNullOrEmpty(transfer.Path))
                continue;

            configuration.Files.Add(new ModpackConfigurationFile
            {
                Path = transfer.Path.Replace('/', Path.DirectorySeparatorChar),
                Sha1 = string.IsNullOrEmpty(transfer.Sha1) ? null : transfer.Sha1
            });
        }

        return configuration;
    }

    /// <summary>
    /// Compares the installed modpack configuration with a new one.
    /// </summary>
    /// <param name="installed">The configuration of the installed modpack, or null.</param>
    /// <param name="target">The description of the new modpack.</param>
    /// <returns>The update plan listing the per-file differences.</returns>
    public static ModpackUpdatePlan Compare(ModpackConfiguration? installed, ModpackConfiguration target)
    {
        var changes = new List<ModpackChange>();
        var targetFiles = new Dictionary<string, ModpackConfigurationFile>(StringComparer.OrdinalIgnoreCase);
        foreach (var file in target.Files)
            targetFiles[file.Path] = file;

        var installedFiles = new Dictionary<string, ModpackConfigurationFile>(StringComparer.OrdinalIgnoreCase);
        if (installed != null)
        {
            foreach (var file in installed.Files)
                installedFiles[file.Path] = file;
        }

        foreach (var file in target.Files)
        {
            if (!installedFiles.TryGetValue(file.Path, out var old))
            {
                changes.Add(new ModpackChange
                {
                    Path = file.Path,
                    Kind = ModpackChangeKind.Added,
                    NewHash = file.Sha1
                });
                continue;
            }

            var same = old.Sha1 != null && file.Sha1 != null &&
                      old.Sha1.Equals(file.Sha1, StringComparison.OrdinalIgnoreCase);

            changes.Add(new ModpackChange
            {
                Path = file.Path,
                Kind = same ? ModpackChangeKind.Unchanged : ModpackChangeKind.Modified,
                OldHash = old.Sha1,
                NewHash = file.Sha1
            });
        }

        foreach (var file in installedFiles.Values)
        {
            if (!targetFiles.ContainsKey(file.Path))
            {
                changes.Add(new ModpackChange
                {
                    Path = file.Path,
                    Kind = ModpackChangeKind.Removed,
                    OldHash = file.Sha1
                });
            }
        }

        return new ModpackUpdatePlan
        {
            Name = target.Name,
            OldVersion = installed?.Version,
            NewVersion = target.Version,
            Changes = changes
        };
    }

    /// <summary>
    /// Returns whether the archive entry belongs to any of the override folders.
    /// </summary>
    /// <param name="entry">The archive entry.</param>
    internal static bool IsOverrideEntry(ZipArchiveEntry entry)
    {
        foreach (var prefix in OverridePrefixes)
        {
            if (entry.FullName.StartsWith(prefix, StringComparison.OrdinalIgnoreCase))
                return true;
        }

        return false;
    }

    /// <summary>
    /// Returns the path of an override entry relative to the instance directory.
    /// </summary>
    /// <param name="entry">The archive entry.</param>
    internal static string GetRelativePath(ZipArchiveEntry entry)
    {
        var slash = entry.FullName.IndexOf('/');
        return slash >= 0 ? entry.FullName[(slash + 1)..] : string.Empty;
    }

    /// <summary>
    /// Computes the SHA1 hash of an archive entry.
    /// </summary>
    /// <param name="entry">The archive entry.</param>
    /// <returns>The hexadecimal hash, or null when it could not be computed.</returns>
    internal static string? ComputeEntryHash(ZipArchiveEntry entry)
    {
        try
        {
            using var stream = entry.Open();
            using var sha1 = SHA1.Create();
            return Convert.ToHexString(sha1.ComputeHash(stream));
        }
        catch
        {
            return null;
        }
    }
}
