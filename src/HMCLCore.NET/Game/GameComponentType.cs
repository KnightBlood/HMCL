using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// The components of a game version that the launcher is able to install.
/// </summary>
public enum GameComponentType
{
    /// <summary>
    /// The game itself.
    /// </summary>
    Game,

    /// <summary>
    /// The Forge mod loader.
    /// </summary>
    Forge,

    /// <summary>
    /// The NeoForge mod loader.
    /// </summary>
    NeoForge,

    /// <summary>
    /// The Cleanroom mod loader.
    /// </summary>
    Cleanroom,

    /// <summary>
    /// The Fabric mod loader.
    /// </summary>
    Fabric,

    /// <summary>
    /// The Fabric API.
    /// </summary>
    FabricApi,

    /// <summary>
    /// The Quilt mod loader.
    /// </summary>
    Quilt,

    /// <summary>
    /// The Quilt API.
    /// </summary>
    QuiltApi,

    /// <summary>
    /// The LegacyFabric mod loader.
    /// </summary>
    LegacyFabric,

    /// <summary>
    /// The LegacyFabric API.
    /// </summary>
    LegacyFabricApi,

    /// <summary>
    /// The LiteLoader mod loader.
    /// </summary>
    LiteLoader,

    /// <summary>
    /// The OptiFine mod loader.
    /// </summary>
    OptiFine
}

/// <summary>
/// Helpers working with the components of a game version.
/// </summary>
public static class GameComponentTypes
{
    /// <summary>
    /// The components that are mod loaders.
    /// </summary>
    public static readonly IReadOnlyList<GameComponentType> ModLoaders = new List<GameComponentType>
    {
        GameComponentType.Forge,
        GameComponentType.NeoForge,
        GameComponentType.Cleanroom,
        GameComponentType.Fabric,
        GameComponentType.Quilt,
        GameComponentType.LegacyFabric,
        GameComponentType.LiteLoader,
        GameComponentType.OptiFine
    };

    /// <summary>
    /// Returns the identifier of the patch of the component, used by the
    /// version manifests.
    /// </summary>
    /// <param name="type">The type of the component.</param>
    /// <returns>The identifier of the patch of the component.</returns>
    public static string GetPatchId(this GameComponentType type) => type switch
    {
        GameComponentType.Game => "game",
        GameComponentType.Forge => "forge",
        GameComponentType.NeoForge => "neoforge",
        GameComponentType.Cleanroom => "cleanroom",
        GameComponentType.Fabric => "fabric",
        GameComponentType.FabricApi => "fabric-api",
        GameComponentType.Quilt => "quilt",
        GameComponentType.QuiltApi => "quilt-api",
        GameComponentType.LegacyFabric => "legacy-fabric",
        GameComponentType.LegacyFabricApi => "legacy-fabric-api",
        GameComponentType.LiteLoader => "liteloader",
        GameComponentType.OptiFine => "optifine",
        _ => type.ToString().ToLowerInvariant()
    };

    /// <summary>
    /// Returns the name of the component shown to the user.
    /// </summary>
    /// <param name="type">The type of the component.</param>
    /// <returns>The name of the component.</returns>
    public static string GetDisplayName(this GameComponentType type) => type switch
    {
        GameComponentType.Game => "游戏",
        GameComponentType.Forge => "Forge",
        GameComponentType.NeoForge => "NeoForge",
        GameComponentType.Cleanroom => "Cleanroom",
        GameComponentType.Fabric => "Fabric",
        GameComponentType.FabricApi => "Fabric API",
        GameComponentType.Quilt => "Quilt",
        GameComponentType.QuiltApi => "Quilt API",
        GameComponentType.LegacyFabric => "LegacyFabric",
        GameComponentType.LegacyFabricApi => "LegacyFabric API",
        GameComponentType.LiteLoader => "LiteLoader",
        GameComponentType.OptiFine => "OptiFine",
        _ => type.ToString()
    };

    /// <summary>
    /// Returns whether the component is a mod loader.
    /// </summary>
    /// <param name="type">The type of the component.</param>
    /// <returns>Whether the component is a mod loader.</returns>
    public static bool IsModLoader(this GameComponentType type) => ModLoaders.Contains(type);
}
