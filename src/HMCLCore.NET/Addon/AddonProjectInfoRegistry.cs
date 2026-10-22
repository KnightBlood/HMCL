using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Text;

namespace HMCLCore.NET.Addon;

/// <summary>
/// The information of a project of an addon, like a mod or a modpack, holding
/// the names of the project in several languages and the identifiers of the
/// project in the remote repositories.
/// </summary>
public sealed class AddonProjectInfo
{
    /// <summary>
    /// Creates the information of a project.
    /// </summary>
    /// <param name="mcModId">The identifier of the project on MCMod.</param>
    /// <param name="curseForgeId">The identifier of the project on CurseForge.</param>
    /// <param name="modrinthId">The identifier of the project on Modrinth.</param>
    /// <param name="mainName">The name of the project in the main language.</param>
    /// <param name="subName">The name of the project in the secondary language.</param>
    /// <param name="abbrName">The abbreviation of the name of the project.</param>
    /// <param name="modIds">The identifiers of the mods of the project.</param>
    public AddonProjectInfo(
        int mcModId,
        string? curseForgeId,
        string? modrinthId,
        string? mainName,
        string? subName,
        string? abbrName,
        IReadOnlyList<string>? modIds = null)
    {
        McModId = mcModId;
        CurseForgeId = curseForgeId;
        ModrinthId = modrinthId;
        MainName = mainName;
        SubName = subName;
        AbbrName = abbrName;
        ModIds = modIds ?? Array.Empty<string>();
    }

    /// <summary>
    /// The identifier of the project on MCMod.
    /// </summary>
    public int McModId { get; }

    /// <summary>
    /// The identifier of the project on CurseForge.
    /// </summary>
    public string? CurseForgeId { get; }

    /// <summary>
    /// The identifier of the project on Modrinth.
    /// </summary>
    public string? ModrinthId { get; }

    /// <summary>
    /// The name of the project in the main language, which is the Chinese name
    /// of the projects of MCMod.
    /// </summary>
    public string? MainName { get; }

    /// <summary>
    /// The name of the project in the secondary language, which is the English
    /// name of the projects of MCMod.
    /// </summary>
    public string? SubName { get; }

    /// <summary>
    /// The abbreviation of the name of the project.
    /// </summary>
    public string? AbbrName { get; }

    /// <summary>
    /// The identifiers of the mods of the project.
    /// </summary>
    public IReadOnlyList<string> ModIds { get; }
}

/// <summary>
/// The registry of the information of the projects of the addons, like the
/// registry of the projects of Java HMCL.
/// </summary>
/// <remarks>
/// The registry is built from the tables of MCMod shipped with the launcher;
/// it holds the names of the mods and of the modpacks in Chinese, so that the
/// user recognizes the addons installed by the other launchers.
/// </remarks>
public sealed class AddonProjectInfoRegistry
{
    private readonly Dictionary<int, AddonProjectInfo> _byMcModId = new();
    private readonly Dictionary<string, AddonProjectInfo> _byModrinthId = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, AddonProjectInfo> _byCurseForgeId = new(StringComparer.OrdinalIgnoreCase);
    private readonly Dictionary<string, AddonProjectInfo> _byModId = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Creates a registry reading the given embedded resource.
    /// </summary>
    /// <param name="resourceName">The name of the embedded resource.</param>
    /// <param name="withModIds">Whether the table holds the identifiers of the mods.</param>
    private AddonProjectInfoRegistry(string resourceName, bool withModIds)
    {
        Load(resourceName, withModIds);
    }

    /// <summary>
    /// The registry of the information of the mods.
    /// </summary>
    public static AddonProjectInfoRegistry Mod { get; } =
        new("HMCLCore.NET.Resources.addon.mod.csv", true);

    /// <summary>
    /// The registry of the information of the modpacks.
    /// </summary>
    public static AddonProjectInfoRegistry Modpack { get; } =
        new("HMCLCore.NET.Resources.addon.modpack.csv", false);

    /// <summary>
    /// The number of the projects of the registry.
    /// </summary>
    public int Count => _byMcModId.Count;

    /// <summary>
    /// Returns the information of the project with the given identifier on
    /// MCMod.
    /// </summary>
    /// <param name="mcModId">The identifier of the project.</param>
    /// <returns>The information of the project, or null when it is unknown.</returns>
    public AddonProjectInfo? GetByMcModId(int mcModId)
        => _byMcModId.TryGetValue(mcModId, out var info) ? info : null;

    /// <summary>
    /// Returns the information of the project with the given identifier on
    /// Modrinth.
    /// </summary>
    /// <param name="modrinthId">The identifier of the project.</param>
    /// <returns>The information of the project, or null when it is unknown.</returns>
    public AddonProjectInfo? GetByModrinthId(string? modrinthId)
        => modrinthId is not null && _byModrinthId.TryGetValue(modrinthId, out var info) ? info : null;

    /// <summary>
    /// Returns the information of the project with the given identifier on
    /// CurseForge.
    /// </summary>
    /// <param name="curseForgeId">The identifier of the project.</param>
    /// <returns>The information of the project, or null when it is unknown.</returns>
    public AddonProjectInfo? GetByCurseForgeId(string? curseForgeId)
        => curseForgeId is not null && _byCurseForgeId.TryGetValue(curseForgeId, out var info) ? info : null;

    /// <summary>
    /// Returns the information of the project holding the given mod.
    /// </summary>
    /// <param name="modId">The identifier of the mod.</param>
    /// <returns>The information of the project, or null when it is unknown.</returns>
    public AddonProjectInfo? GetByModId(string? modId)
        => modId is not null && _byModId.TryGetValue(modId, out var info) ? info : null;

    /// <summary>
    /// Reads the table of the registry.
    /// </summary>
    /// <param name="resourceName">The name of the embedded resource.</param>
    /// <param name="withModIds">Whether the table holds the identifiers of the mods.</param>
    private void Load(string resourceName, bool withModIds)
    {
        try
        {
            using var stream = typeof(AddonProjectInfoRegistry).GetTypeInfo().Assembly
                .GetManifestResourceStream(resourceName);

            if (stream is null)
                return;

            using var reader = new StreamReader(stream, Encoding.UTF8);
            string? line;
            while ((line = reader.ReadLine()) is not null)
            {
                var trimmed = line.Trim();
                if (trimmed.Length == 0 || trimmed.StartsWith('#'))
                    continue;

                var fields = trimmed.Split(',');
                if (fields.Length < 5)
                    continue;

                if (!int.TryParse(fields[0], out var mcModId))
                    continue;

                var info = new AddonProjectInfo(
                    mcModId,
                    NullIfEmpty(fields[1]),
                    NullIfEmpty(fields[2]),
                    NullIfEmpty(fields[3]),
                    NullIfEmpty(fields[4]),
                    fields.Length > 5 ? NullIfEmpty(fields[5]) : null,
                    withModIds && fields.Length > 6
                        ? fields[6].Split(';', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
                        : Array.Empty<string>());

                _byMcModId[mcModId] = info;

                if (info.ModrinthId is not null)
                    _byModrinthId[info.ModrinthId] = info;

                if (info.CurseForgeId is not null)
                    _byCurseForgeId[info.CurseForgeId] = info;

                foreach (var modId in info.ModIds)
                    _byModId[modId] = info;
            }
        }
        catch (Exception ex) when (ex is IOException or InvalidOperationException)
        {
            // The registry stays empty when the table cannot be read.
        }
    }

    /// <summary>
    /// Returns the given field, or null when it is empty.
    /// </summary>
    /// <param name="value">The value of the field.</param>
    /// <returns>The value of the field, or null.</returns>
    private static string? NullIfEmpty(string value)
        => string.IsNullOrWhiteSpace(value) ? null : value;
}
