using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// A component of a game version, telling its type and its version.
/// </summary>
public sealed class GameComponentMark
{
    /// <summary>
    /// Creates a mark of a component.
    /// </summary>
    /// <param name="componentType">The type of the component.</param>
    /// <param name="version">The version of the component.</param>
    public GameComponentMark(GameComponentType componentType, string version)
    {
        ComponentType = componentType;
        Version = version;
    }

    /// <summary>
    /// The type of the component.
    /// </summary>
    public GameComponentType ComponentType { get; }

    /// <summary>
    /// The version of the component.
    /// </summary>
    public string Version { get; }

    /// <summary>
    /// The name of the component shown to the user.
    /// </summary>
    public string DisplayName => $"{ComponentType.GetDisplayName()} {Version}";

    public override string ToString() => DisplayName;
}

/// <summary>
/// Analyzes the components installed in a game version.
/// </summary>
public sealed class GameComponentAnalyzer
{
    private readonly List<GameComponentMark> _marks;

    /// <summary>
    /// Creates an analyzer holding the given marks.
    /// </summary>
    /// <param name="marks">The marks of the components.</param>
    private GameComponentAnalyzer(List<GameComponentMark> marks)
    {
        _marks = marks;
    }

    /// <summary>
    /// The components found in the analyzed manifest.
    /// </summary>
    public IReadOnlyList<GameComponentMark> Marks => _marks;

    /// <summary>
    /// Analyzes the given manifest and returns the components it holds.
    /// </summary>
    /// <param name="manifest">The manifest of the game version.</param>
    /// <returns>The analyzer of the manifest.</returns>
    public static GameComponentAnalyzer Analyze(GameInstanceManifest? manifest)
    {
        var marks = new List<GameComponentMark>();
        if (manifest is null)
            return new GameComponentAnalyzer(marks);

        var libraries = new List<Library>(manifest.Libraries);
        foreach (var patch in manifest.Patches)
            libraries.AddRange(patch.EffectiveLibraries);

        foreach (var library in libraries)
        {
            var mark = AnalyzeLibrary(library);
            if (mark != null && !marks.Any(existing => existing.ComponentType == mark.ComponentType))
                marks.Add(mark);
        }

        // The patches of the manifest describe the components too, for example
        // the patch of a mod loader.
        foreach (var patch in manifest.Patches)
        {
            var type = AnalyzePatchId(patch.Id);
            if (type is null || string.IsNullOrEmpty(patch.Version))
                continue;

            if (!marks.Any(existing => existing.ComponentType == type))
                marks.Add(new GameComponentMark(type.Value, patch.Version!));
        }

        return new GameComponentAnalyzer(marks);
    }

    /// <summary>
    /// Returns whether the analyzer found the given component.
    /// </summary>
    /// <param name="type">The type of the component.</param>
    /// <returns>Whether the component was found.</returns>
    public bool Has(GameComponentType type) => GetVersion(type) != null;

    /// <summary>
    /// Returns the version of the given component, or null when it was not
    /// found.
    /// </summary>
    /// <param name="type">The type of the component.</param>
    /// <returns>The version of the component, or null.</returns>
    public string? GetVersion(GameComponentType type)
        => _marks.FirstOrDefault(mark => mark.ComponentType == type)?.Version;

    /// <summary>
    /// Analyzes a library and returns the component it belongs to.
    /// </summary>
    /// <param name="library">The library to analyze.</param>
    /// <returns>The mark of the component, or null when the library is not a component.</returns>
    private static GameComponentMark? AnalyzeLibrary(Library library)
    {
        var name = library.Name;
        if (string.IsNullOrEmpty(name))
            return null;

        var parts = name.Split(':');
        if (parts.Length < 2)
            return null;

        var group = parts[0];
        var artifact = parts[1];
        var version = parts.Length > 2 ? parts[2] : string.Empty;

        GameComponentType? type = null;

        if (group == "net.minecraftforge" && artifact == "forge")
            type = GameComponentType.Forge;
        else if (group == "net.neoforged" && artifact == "neoforge")
            type = GameComponentType.NeoForge;
        else if (group.StartsWith("com.cleanroommc", StringComparison.OrdinalIgnoreCase))
            type = GameComponentType.Cleanroom;
        else if (group == "net.fabricmc" && artifact == "fabric-loader")
            type = GameComponentType.Fabric;
        else if (group == "net.fabricmc.fabric-api" && artifact == "fabric-api")
            type = GameComponentType.FabricApi;
        else if (group == "org.quiltmc" && artifact == "quilt-loader")
            type = GameComponentType.Quilt;
        else if (group == "org.quiltmc.quilted-fabric-api" && artifact == "quilted-fabric-api")
            type = GameComponentType.QuiltApi;
        else if (group == "net.fabricmc.legacyfabric" && artifact == "legacyfabric-loader")
            type = GameComponentType.LegacyFabric;
        else if (group == "net.fabricmc.fabric-api" && artifact == "fabric-api")
            type = GameComponentType.FabricApi;
        else if (artifact.StartsWith("optifine", StringComparison.OrdinalIgnoreCase))
            type = GameComponentType.OptiFine;
        else if (artifact.StartsWith("liteloader", StringComparison.OrdinalIgnoreCase))
            type = GameComponentType.LiteLoader;

        if (type is null)
            return null;

        return new GameComponentMark(type.Value, version);
    }

    /// <summary>
    /// Returns the type of the component described by the given patch
    /// identifier.
    /// </summary>
    /// <param name="id">The identifier of the patch.</param>
    /// <returns>The type of the component, or null when the patch is unknown.</returns>
    private static GameComponentType? AnalyzePatchId(string? id)
    {
        if (string.IsNullOrEmpty(id))
            return null;

        return GameComponentTypes.ModLoaders
            .Concat(new[] { GameComponentType.FabricApi, GameComponentType.QuiltApi, GameComponentType.LegacyFabricApi })
            .FirstOrDefault(type => type.GetPatchId().Equals(id, StringComparison.OrdinalIgnoreCase));
    }
}
