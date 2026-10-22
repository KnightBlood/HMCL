using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using HMCLCore.NET.Game;
using ModpackModel = HMCLCore.NET.Modpack.Modpack;
using ModpackModModel = HMCLCore.NET.Modpack.ModpackMod;

namespace HMCLCore.NET.Addon;

/// <summary>
/// A mod of a modpack compared with the mods of an instance.
/// </summary>
public sealed class ModSuggestion
{
    /// <summary>
    /// Creates a suggestion.
    /// </summary>
    /// <param name="name">The name of the mod.</param>
    /// <param name="reason">The reason the mod is suggested.</param>
    public ModSuggestion(string name, string reason)
    {
        Name = name;
        Reason = reason;
    }

    /// <summary>
    /// The name of the mod.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The reason the mod is suggested.
    /// </summary>
    public string Reason { get; }

    public override string ToString() => $"{Name} ({Reason})";
}

/// <summary>
/// Compares the mods of a modpack with the mods of an instance, so that the
/// launcher can tell the user which mods are missing and which ones are not
/// part of the pack. It mirrors the mod adviser of Java HMCL.
/// </summary>
public sealed class ModAdviser
{
    private ModAdviser(
        List<ModSuggestion> missingMods,
        List<ModSuggestion> extraMods,
        List<string> optionalFiles)
    {
        MissingMods = missingMods;
        ExtraMods = extraMods;
        OptionalFiles = optionalFiles;
    }

    /// <summary>
    /// The mods of the modpack that are not installed in the instance.
    /// </summary>
    public IReadOnlyList<ModSuggestion> MissingMods { get; }

    /// <summary>
    /// The mods of the instance that are not part of the modpack.
    /// </summary>
    public IReadOnlyList<ModSuggestion> ExtraMods { get; }

    /// <summary>
    /// The optional files of the instance, for example the disabled mods.
    /// </summary>
    public IReadOnlyList<string> OptionalFiles { get; }

    /// <summary>
    /// Whether the adviser found something to tell the user.
    /// </summary>
    public bool HasSuggestions =>
        MissingMods.Count > 0 || ExtraMods.Count > 0 || OptionalFiles.Count > 0;

    /// <summary>
    /// The summary of the advice shown to the user.
    /// </summary>
    public string Summary
    {
        get
        {
            var parts = new List<string>();

            if (MissingMods.Count > 0)
                parts.Add($"缺少 {MissingMods.Count} 个模组");
            if (ExtraMods.Count > 0)
                parts.Add($"多出 {ExtraMods.Count} 个模组");
            if (OptionalFiles.Count > 0)
                parts.Add($"包含 {OptionalFiles.Count} 个可选文件");

            return parts.Count > 0 ? string.Join("，", parts) : "没有需要处理的内容";
        }
    }

    /// <summary>
    /// Compares the mods of the given modpack with the mods of the given
    /// instance.
    /// </summary>
    /// <param name="instance">The instance holding the mods.</param>
    /// <param name="modpack">The modpack holding the expected mods.</param>
    /// <returns>The advice about the mods of the instance.</returns>
    public static ModAdviser Analyze(GameInstance instance, ModpackModel? modpack)
    {
        var missing = new List<ModSuggestion>();
        var extra = new List<ModSuggestion>();
        var optional = new List<string>();

        var modsDirectory = Path.Combine(instance.GameDirectory, "mods");
        var manager = new LocalAddonManager(modsDirectory);
        manager.Scan();

        var installed = manager.Mods.ToList();

        foreach (var mod in installed.Where(mod => !mod.Enabled))
            optional.Add(Path.GetFileName(mod.FilePath));

        var expected = modpack?.Manifest?.Mods ?? new List<ModpackModModel>();
        if (expected.Count == 0)
        {
            // The pack does not list its mods; only the optional files are
            // reported.
            return new ModAdviser(missing, extra, optional);
        }

        var installedNames = new HashSet<string>(
            installed.Select(mod => Normalize(mod.Name)), StringComparer.OrdinalIgnoreCase);

        foreach (var mod in expected)
        {
            var name = mod.Name;
            if (string.IsNullOrEmpty(name))
            {
                missing.Add(new ModSuggestion($"{mod.ProjectId}:{mod.FileId}", "整合包中的模组"));
                continue;
            }

            if (!installedNames.Contains(Normalize(name)))
                missing.Add(new ModSuggestion(name, "整合包中包含但实例中缺失"));
        }

        var expectedNames = new HashSet<string>(
            expected.Where(mod => !string.IsNullOrEmpty(mod.Name))
                .Select(mod => Normalize(mod.Name!)), StringComparer.OrdinalIgnoreCase);

        foreach (var mod in installed)
        {
            if (expectedNames.Contains(Normalize(mod.Name)))
                continue;

            extra.Add(new ModSuggestion(mod.Name, "实例中存在但整合包中未包含"));
        }

        return new ModAdviser(missing, extra, optional);
    }

    /// <summary>
    /// Normalizes the name of a mod, so that the names of the mods of a pack
    /// and of an instance can be compared.
    /// </summary>
    /// <param name="name">The name of the mod.</param>
    /// <returns>The normalized name.</returns>
    private static string Normalize(string name)
        => new string(name.Where(char.IsLetterOrDigit).ToArray()).ToLowerInvariant();
}
