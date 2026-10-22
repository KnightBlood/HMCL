using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// A patch that can be applied to the manifest of an instance, for example the
/// patch of a mod loader, which adds libraries, arguments and a main class to
/// the manifest of the game.
/// </summary>
public sealed class GameInstancePatch
{
    /// <summary>
    /// The priority of the patch of the game itself.
    /// </summary>
    public const int PriorityMc = 1000;

    /// <summary>
    /// The priority of the patch of a mod loader.
    /// </summary>
    public const int PriorityLoader = 2000;

    /// <summary>
    /// The identifier of the component the patch belongs to, for example
    /// <c>fabric</c> or <c>forge</c>.
    /// </summary>
    public string? Id { get; init; }

    /// <summary>
    /// The version of the component the patch belongs to.
    /// </summary>
    public string? Version { get; init; }

    /// <summary>
    /// The priority of the patch; the patches with a lower priority are applied
    /// first.
    /// </summary>
    public int? Priority { get; init; }

    /// <summary>
    /// The main class of the game, which replaces the main class of the
    /// manifest when it is not empty.
    /// </summary>
    public string? MainClass { get; init; }

    /// <summary>
    /// The libraries added by the patch.
    /// </summary>
    public List<Library>? Libraries { get; init; }

    /// <summary>
    /// The arguments added by the patch.
    /// </summary>
    public GameInstanceManifest.LaunchArguments? Arguments { get; init; }

    /// <summary>
    /// The legacy arguments of the game, which replace the arguments of the
    /// manifest when they are not null.
    /// </summary>
    public string? MinecraftArguments { get; init; }

    /// <summary>
    /// The version the instance inherits from.
    /// </summary>
    public string? InheritsFrom { get; init; }

    /// <summary>
    /// The version the client JAR of the instance belongs to.
    /// </summary>
    public string? Jar { get; init; }

    /// <summary>
    /// The asset index of the instance.
    /// </summary>
    public GameInstanceManifest.AssetIndexInfo? AssetIndex { get; init; }

    /// <summary>
    /// The name of the assets directory of the instance.
    /// </summary>
    public string? Assets { get; init; }

    /// <summary>
    /// The minimum major version of Java required by the instance.
    /// </summary>
    public int? JavaVersion { get; init; }

    /// <summary>
    /// Whether the instance is hidden in the list of the instances.
    /// </summary>
    public bool Hidden { get; init; }

    /// <summary>
    /// The type of the version the patch belongs to.
    /// </summary>
    public string? Type { get; init; }

    /// <summary>
    /// The priority of the patch; the patches without a priority are applied
    /// first.
    /// </summary>
    public int EffectivePriority => Priority ?? int.MinValue;

    /// <summary>
    /// The libraries added by the patch.
    /// </summary>
    public IReadOnlyList<Library> EffectiveLibraries
        => Libraries ?? (IReadOnlyList<Library>)Array.Empty<Library>();

    /// <summary>
    /// Applies the patch to a manifest.
    /// </summary>
    /// <param name="manifest">The manifest to apply the patch to.</param>
    /// <returns>
    /// The manifest with the libraries, the arguments and the main class of
    /// the patch applied.
    /// </returns>
    public GameInstanceManifest ApplyTo(GameInstanceManifest manifest)
    {
        ArgumentNullException.ThrowIfNull(manifest);

        // The libraries of the patch are appended to the libraries of the
        // manifest, the libraries that are already present are kept.
        var libraries = new List<Library>(manifest.Libraries);
        var names = new HashSet<string>(
            libraries.Select(library => library.Name), StringComparer.OrdinalIgnoreCase);

        foreach (var library in EffectiveLibraries)
        {
            if (!string.IsNullOrEmpty(library.Name) && names.Add(library.Name))
                libraries.Add(library);
        }

        var jvmArguments = new List<string>(manifest.Arguments.Jvm);
        if (Arguments?.Jvm != null)
            jvmArguments.AddRange(Arguments.Jvm);

        var gameArguments = new List<string>(manifest.Arguments.Game);
        if (Arguments?.Game != null)
            gameArguments.AddRange(Arguments.Game);

        var patches = new List<GameInstancePatch>(manifest.Patches) { this };

        return new GameInstanceManifest
        {
            Version = manifest.Version,
            LastPlayed = manifest.LastPlayed,
            MainClass = string.IsNullOrEmpty(MainClass) ? manifest.MainClass : MainClass,
            JavaVersion = JavaVersion is > 0 ? JavaVersion.Value : manifest.JavaVersion,
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = jvmArguments,
                Game = gameArguments
            },
            Libraries = libraries,
            AssetIndex = AssetIndex ?? manifest.AssetIndex,
            LoggingConfig = manifest.LoggingConfig,
            Downloads = manifest.Downloads,
            Demo = manifest.Demo,
            CustomResolution = manifest.CustomResolution,
            GameSettings = manifest.GameSettings,
            IconType = manifest.IconType,
            Patches = patches
        };
    }

    /// <summary>
    /// Creates the patch of a manifest, which describes the same components as
    /// the manifest itself.
    /// </summary>
    /// <param name="manifest">The manifest to create the patch of.</param>
    /// <param name="id">The identifier of the component of the patch.</param>
    /// <param name="priority">The priority of the patch.</param>
    /// <returns>The patch of the manifest.</returns>
    public static GameInstancePatch FromManifest(
        GameInstanceManifest manifest, string? id = null, int? priority = null)
    {
        ArgumentNullException.ThrowIfNull(manifest);

        return new GameInstancePatch
        {
            Id = id,
            Version = manifest.Version,
            Priority = priority,
            MainClass = manifest.MainClass,
            Libraries = new List<Library>(manifest.Libraries),
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = new List<string>(manifest.Arguments.Jvm),
                Game = new List<string>(manifest.Arguments.Game)
            },
            AssetIndex = manifest.AssetIndex,
            JavaVersion = manifest.JavaVersion
        };
    }
}
