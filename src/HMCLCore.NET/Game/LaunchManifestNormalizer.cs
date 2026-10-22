using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// Repairs the launch manifest of an instance before the game is started,
/// like the launch manifest normalizer of Java HMCL.
/// </summary>
/// <remarks>
/// The repairs make the combinations of the mod loaders work: the tweaker
/// classes of the LaunchWrapper era, the transformer discovery service of
/// ModLauncher, the ignore list of BootstrapLauncher and the duplicated ASM
/// libraries of LegacyFabric.
/// </remarks>
public static class LaunchManifestNormalizer
{
    /// <summary>The main class of the vanilla game.</summary>
    public const string VanillaMain = "net.minecraft.client.main.Main";

    /// <summary>The main class of the LaunchWrapper, used by the old loaders.</summary>
    public const string LaunchWrapperMain = "net.minecraft.launchwrapper.Launch";

    /// <summary>The main class of ModLauncher, used by the Forge of 1.13 and newer.</summary>
    public const string ModLauncherMain = "cpw.mods.modlauncher.Launcher";

    /// <summary>The main class of BootstrapLauncher, used by the Forge and the NeoForge of 1.17 and newer.</summary>
    public const string BootstrapLauncherMain = "cpw.mods.bootstraplauncher.BootstrapLauncher";

    /// <summary>The main class of the Forge bootstrap of 1.17 and newer.</summary>
    public const string ForgeBootstrapMain = "net.minecraftforge.bootstrap.ForgeBootstrap";

    /// <summary>The main class of the NeoForge bootstrap of 1.20 and newer.</summary>
    public const string NeoForgeBootstrapMain = "net.neoforged.fml.startup.Client";

    /// <summary>The tweaker class of LiteLoader.</summary>
    public const string LiteLoaderTweaker = "com.mumfrey.liteloader.launch.LiteLoaderTweaker";

    /// <summary>The tweaker classes of the different versions of the Forge.</summary>
    public static readonly IReadOnlyList<string> ForgeTweakers = new List<string>
    {
        "net.minecraftforge.legacy._1_5_2.LibraryFixerTweaker",
        "cpw.mods.fml.common.launcher.FMLTweaker",
        "net.minecraftforge.fml.common.launcher.FMLTweaker"
    };

    /// <summary>The tweaker classes of OptiFine.</summary>
    public static readonly IReadOnlyList<string> OptiFineTweakers = new List<string>
    {
        "optifine.OptiFineTweaker",
        "optifine.OptiFineForgeTweaker"
    };

    /// <summary>
    /// Applies the repairs of the launch manifest of an instance.
    /// </summary>
    /// <param name="manifest">The manifest of the instance.</param>
    /// <returns>The repaired manifest.</returns>
    public static GameInstanceManifest RepairForLaunch(GameInstanceManifest manifest)
    {
        ArgumentNullException.ThrowIfNull(manifest);

        var analyzer = GameComponentAnalyzer.Analyze(manifest);
        var builder = new Builder(manifest);

        builder.SetLibraries(UniqueLibraries(manifest.Libraries));

        var mainClass = builder.MainClass;
        if (mainClass == LaunchWrapperMain)
        {
            // The LaunchWrapper era: the Forge of 1.12 and older, LiteLoader
            // and the OptiFine.
            RepairLaunchWrapper(builder, analyzer, true);
            if (builder.MainClass == ModLauncherMain)
                RepairModLauncher(builder, analyzer);
        }
        else if (mainClass == ModLauncherMain)
        {
            // The Forge of 1.13 and newer with the OptiFine.
            RepairModLauncher(builder, analyzer);
        }
        else if (mainClass == BootstrapLauncherMain)
        {
            // The Forge and the NeoForge of 1.17 and newer.
            RepairBootstrapLauncher(builder, analyzer);
        }

        if (analyzer.Has(GameComponentType.LegacyFabric))
            RepairDuplicateAsm(builder, analyzer);

        return RemoveLegacyLog4jPatch(builder.Build());
    }

    /// <summary>
    /// Repairs the tweaker classes of the LaunchWrapper.
    /// </summary>
    /// <param name="builder">The manifest being repaired.</param>
    /// <param name="analyzer">The components of the instance.</param>
    /// <param name="reorderTweakClass">
    /// Whether the tweaker classes are moved to the end of the arguments.
    /// </param>
    private static void RepairLaunchWrapper(
        Builder builder, GameComponentAnalyzer analyzer, bool reorderTweakClass)
    {
        var hasLiteLoader = analyzer.Has(GameComponentType.LiteLoader);
        var hasForge = analyzer.Has(GameComponentType.Forge);
        var hasModLauncher = HasForgeModLauncher(analyzer);

        if (hasLiteLoader && !hasModLauncher)
        {
            builder.ReplaceTweakClass(LiteLoaderTweaker, LiteLoaderTweaker, !reorderTweakClass, reorderTweakClass);
        }
        else
        {
            builder.RemoveTweakClass(LiteLoaderTweaker);
        }

        if (analyzer.Has(GameComponentType.OptiFine))
        {
            if (!hasLiteLoader && !hasForge)
            {
                // The OptiFine alone uses the plain tweaker.
                if (builder.HasTweakClass(OptiFineTweakers[1]))
                    builder.ReplaceTweakClass(OptiFineTweakers[1], OptiFineTweakers[0], !reorderTweakClass, reorderTweakClass);
            }
            else if (hasModLauncher)
            {
                // ModLauncher is preferred over the LaunchWrapper.
                builder.MainClass = ModLauncherMain;
                foreach (var tweaker in OptiFineTweakers)
                    builder.RemoveTweakClass(tweaker);
            }
            else if (builder.HasTweakClass(OptiFineTweakers[0]))
            {
                // With the Forge or the LiteLoader, the tweaker of the Forge is
                // needed.
                builder.ReplaceTweakClass(OptiFineTweakers[0], OptiFineTweakers[1], !reorderTweakClass, reorderTweakClass);
            }
        }
        else
        {
            foreach (var tweaker in OptiFineTweakers)
                builder.RemoveTweakClass(tweaker);
        }

        foreach (var tweaker in ForgeTweakers)
        {
            if (!hasForge)
            {
                builder.RemoveTweakClass(tweaker);
            }
            else if (!hasModLauncher && builder.HasTweakClass(tweaker))
            {
                builder.ReplaceTweakClass(tweaker, tweaker, !reorderTweakClass, reorderTweakClass);
            }
        }
    }

    /// <summary>
    /// Adds the transformer discovery service needed by the Forge and the
    /// OptiFine on ModLauncher.
    /// </summary>
    /// <param name="builder">The manifest being repaired.</param>
    /// <param name="analyzer">The components of the instance.</param>
    private static void RepairModLauncher(Builder builder, GameComponentAnalyzer analyzer)
    {
        if (!analyzer.Has(GameComponentType.Forge) || !analyzer.Has(GameComponentType.OptiFine))
            return;

        var servicePresent = builder.Libraries.Any(library => library.Is("org.jackhuang.hmcl", "transformer-discovery-service"));

        var optiFine = builder.Libraries.FirstOrDefault(library => library.Is("optifine", "OptiFine"));
        if (optiFine is null)
            return;

        var candidate = "-Dhmcl.transformer.candidates=${library_directory}/" + GetLibraryPath(optiFine);
        if (!builder.JvmArguments.Contains(candidate))
            builder.JvmArguments.Add(candidate);

        if (!servicePresent)
            builder.AddLibrary(new Library { Name = "org.jackhuang.hmcl:transformer-discovery-service:1.0" });
    }

    /// <summary>
    /// Repairs the ignore list of the BootstrapLauncher.
    /// </summary>
    /// <param name="builder">The manifest being repaired.</param>
    /// <param name="analyzer">The components of the instance.</param>
    private static void RepairBootstrapLauncher(Builder builder, GameComponentAnalyzer analyzer)
    {
        if (!analyzer.Has(GameComponentType.Forge) && !analyzer.Has(GameComponentType.NeoForge))
            return;

        const string prefix = "-DignoreList=";
        for (var index = 0; index < builder.JvmArguments.Count; index++)
        {
            var argument = builder.JvmArguments[index];
            if (!argument.StartsWith(prefix, StringComparison.Ordinal))
                continue;

            var values = argument[prefix.Length..];
            if (!values.Split(',').Contains("${primary_jar_name}"))
                builder.JvmArguments[index] = argument + ",${primary_jar_name}";
        }
    }

    /// <summary>
    /// Removes the ASM library duplicated by LegacyFabric.
    /// </summary>
    /// <param name="builder">The manifest being repaired.</param>
    /// <param name="analyzer">The components of the instance.</param>
    private static void RepairDuplicateAsm(Builder builder, GameComponentAnalyzer analyzer)
    {
        if (!analyzer.Has(GameComponentType.LegacyFabric))
            return;

        if (!builder.Libraries.Any(library => library.Is("org.ow2.asm", "asm")))
            return;

        builder.SetLibraries(builder.Libraries
            .Where(library => !library.Is("org.ow2.asm", "asm-all"))
            .ToList());
    }

    /// <summary>
    /// Removes the obsolete log4j patch of the launcher.
    /// </summary>
    /// <param name="manifest">The manifest of the instance.</param>
    /// <returns>The repaired manifest.</returns>
    private static GameInstanceManifest RemoveLegacyLog4jPatch(GameInstanceManifest manifest)
    {
        if (manifest.Libraries.Count == 0)
            return manifest;

        var library = manifest.Libraries[0];
        if (library.Is("org.glavo", "log4j-patch") || library.Is("org.glavo", "log4j-patch-beta9"))
        {
            return new GameInstanceManifest
            {
                Version = manifest.Version,
                LastPlayed = manifest.LastPlayed,
                MainClass = manifest.MainClass,
                JavaVersion = manifest.JavaVersion,
                Arguments = manifest.Arguments,
                Libraries = manifest.Libraries.Skip(1).ToList(),
                AssetIndex = manifest.AssetIndex,
                LoggingConfig = manifest.LoggingConfig,
                Downloads = manifest.Downloads,
                Demo = manifest.Demo,
                CustomResolution = manifest.CustomResolution,
                GameSettings = manifest.GameSettings,
                IconType = manifest.IconType,
                Patches = manifest.Patches
            };
        }

        return manifest;
    }

    /// <summary>
    /// Removes the duplicated libraries of a manifest, keeping the newer
    /// version of the libraries sharing the same identifier and the variants
    /// of the libraries which have different compatibility rules.
    /// </summary>
    /// <param name="libraries">The libraries of the manifest.</param>
    /// <returns>The libraries without duplicates.</returns>
    private static IReadOnlyList<Library> UniqueLibraries(IReadOnlyList<Library> libraries)
    {
        var result = new List<Library>();
        var indexes = new Dictionary<string, List<int>>(StringComparer.Ordinal);

        foreach (var library in libraries)
        {
            var id = GetLibraryId(library);

            if (!indexes.TryGetValue(id, out var positions))
            {
                indexes[id] = new List<int> { result.Count };
                result.Add(library);
                continue;
            }

            var duplicate = false;
            foreach (var position in positions)
            {
                var other = result[position];

                // The rules differ: both are kept, as they are the variants of
                // the platforms.
                if (!RulesEqual(library, other))
                    continue;

                var comparison = CompareVersions(library.Version, other.Version);
                if (comparison > 0)
                {
                    result[position] = library;
                }
                else if (comparison == 0 && AreEquivalent(library, other))
                {
                    // The same library: the entry with the most metadata wins.
                    if (Describe(library).Length > Describe(other).Length)
                        result[position] = library;
                }
                else
                {
                    // The same identifier and version, but different contents,
                    // for example the jar and the natives of the speech
                    // library: both are kept.
                    continue;
                }

                duplicate = true;
                break;
            }

            if (!duplicate)
            {
                positions.Add(result.Count);
                result.Add(library);
            }
        }

        return result;
    }

    /// <summary>
    /// Returns whether the compatibility rules of two libraries are equal.
    /// </summary>
    /// <param name="left">The first library.</param>
    /// <param name="right">The second library.</param>
    /// <returns>Whether the rules are equal.</returns>
    private static bool RulesEqual(Library left, Library right)
    {
        var leftRules = left.Rules ?? new List<CompatibilityRule>();
        var rightRules = right.Rules ?? new List<CompatibilityRule>();

        if (leftRules.Count != rightRules.Count)
            return false;

        for (var index = 0; index < leftRules.Count; index++)
        {
            if (leftRules[index].Action != rightRules[index].Action ||
                leftRules[index].Os != rightRules[index].Os)
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Returns whether two libraries hold the same data.
    /// </summary>
    /// <param name="left">The first library.</param>
    /// <param name="right">The second library.</param>
    /// <returns>Whether the libraries are equivalent.</returns>
    private static bool AreEquivalent(Library left, Library right)
    {
        return left.Name == right.Name &&
               left.Url == right.Url &&
               left.Classifier == right.Classifier &&
               (left.Downloads?.Artifact == null) == (right.Downloads?.Artifact == null) &&
               string.Join(';', left.Natives ?? new List<string>()) ==
               string.Join(';', right.Natives ?? new List<string>());
    }

    /// <summary>
    /// Returns the text describing a library, used to prefer the entry with
    /// the most metadata.
    /// </summary>
    /// <param name="library">The library described.</param>
    /// <returns>The description of the library.</returns>
    private static string Describe(Library library)
        => $"{library.Name}|{library.Url}|{library.Classifier}|{library.Downloads?.Artifact?.Url}|{library.Downloads?.Artifact?.Sha1}";

    /// <summary>
    /// Returns the identifier of a library, that is its group and its artifact.
    /// </summary>
    /// <param name="library">The library.</param>
    /// <returns>The identifier of the library.</returns>
    private static string GetLibraryId(Library library)
    {
        var parts = library.Name.Split(':');
        return parts.Length >= 2 ? parts[0] + ":" + parts[1] : library.Name;
    }

    /// <summary>
    /// Returns the version of a library.
    /// </summary>
    /// <param name="library">The library.</param>
    /// <returns>The version of the library.</returns>
    private static string GetLibraryVersion(Library library)
    {
        var parts = library.Name.Split(':');
        return parts.Length >= 3 ? parts[2] : string.Empty;
    }

    /// <summary>
    /// Returns the path of a library inside the directory of the libraries.
    /// </summary>
    /// <param name="library">The library.</param>
    /// <returns>The path of the library.</returns>
    private static string GetLibraryPath(Library library)
    {
        var parts = library.Name.Split(':');
        if (parts.Length < 3)
            return library.Name;

        var group = parts[0].Replace('.', '/');
        var artifact = parts[1];
        var version = parts[2];
        var classifier = library.Classifier;

        return classifier is null
            ? $"{group}/{artifact}/{version}/{artifact}-{version}.jar"
            : $"{group}/{artifact}/{version}/{artifact}-{version}-{classifier}.jar";
    }

    /// <summary>
    /// Compares two versions of a library.
    /// </summary>
    /// <param name="left">The first version.</param>
    /// <param name="right">The second version.</param>
    /// <returns>
    /// A negative number when the first version is older, a positive one when
    /// it is newer and zero when they are equal.
    /// </returns>
    private static int CompareVersions(string left, string right)
    {
        var leftParts = left.Split('.');
        var rightParts = right.Split('.');

        for (var index = 0; index < Math.Max(leftParts.Length, rightParts.Length); index++)
        {
            var leftValue = index < leftParts.Length && int.TryParse(leftParts[index], out var leftNumber)
                ? leftNumber
                : 0;
            var rightValue = index < rightParts.Length && int.TryParse(rightParts[index], out var rightNumber)
                ? rightNumber
                : 0;

            if (leftValue != rightValue)
                return leftValue.CompareTo(rightValue);
        }

        return string.Compare(left, right, StringComparison.Ordinal);
    }

    /// <summary>
    /// Returns whether the instance uses the Forge of the ModLauncher.
    /// </summary>
    /// <param name="analyzer">The components of the instance.</param>
    /// <returns>Whether the ModLauncher is used.</returns>
    private static bool HasForgeModLauncher(GameComponentAnalyzer analyzer)
    {
        var version = analyzer.GetVersion(GameComponentType.Forge);
        if (string.IsNullOrEmpty(version))
            return false;

        // The Forge of 1.13 and newer uses the ModLauncher.
        var parts = version.Split('.');
        if (parts.Length == 0 || !int.TryParse(parts[0], out var major))
            return false;

        return major >= 1 && (major > 1 || !int.TryParse(parts[1], out var minor) || minor >= 13);
    }

    /// <summary>
    /// Builds the repaired manifest of an instance.
    /// </summary>
    private sealed class Builder
    {
        private readonly GameInstanceManifest _manifest;
        private readonly List<Library> _libraries;

        /// <summary>
        /// Creates the builder of the given manifest.
        /// </summary>
        /// <param name="manifest">The manifest being repaired.</param>
        public Builder(GameInstanceManifest manifest)
        {
            _manifest = manifest;
            _libraries = manifest.Libraries.ToList();
            JvmArguments = manifest.Arguments.Jvm.ToList();
            GameArguments = manifest.Arguments.Game.ToList();
            MainClass = manifest.MainClass;
        }

        /// <summary>The libraries of the manifest.</summary>
        public List<Library> Libraries => _libraries;

        /// <summary>The JVM arguments of the manifest.</summary>
        public List<string> JvmArguments { get; }

        /// <summary>The game arguments of the manifest.</summary>
        public List<string> GameArguments { get; }

        /// <summary>The main class of the manifest.</summary>
        public string MainClass { get; set; }

        /// <summary>
        /// Replaces the libraries of the manifest.
        /// </summary>
        /// <param name="libraries">The new libraries.</param>
        public void SetLibraries(IReadOnlyList<Library> libraries)
        {
            _libraries.Clear();
            _libraries.AddRange(libraries);
        }

        /// <summary>
        /// Adds a library to the manifest.
        /// </summary>
        /// <param name="library">The added library.</param>
        public void AddLibrary(Library library) => _libraries.Add(library);

        /// <summary>
        /// Returns whether the manifest holds the given tweaker class.
        /// </summary>
        /// <param name="tweakClass">The tweaker class.</param>
        /// <returns>Whether the tweaker class is present.</returns>
        public bool HasTweakClass(string tweakClass)
            => GameArguments.Contains(tweakClass);

        /// <summary>
        /// Removes the given tweaker class from the manifest.
        /// </summary>
        /// <param name="target">The removed tweaker class.</param>
        public void RemoveTweakClass(string target) => ReplaceTweakClass(target, null, false, false);

        /// <summary>
        /// Replaces the given tweaker class of the manifest.
        /// </summary>
        /// <param name="target">The replaced tweaker class.</param>
        /// <param name="replacement">The new tweaker class, or null to remove it.</param>
        /// <param name="inPlace">
        /// Whether the tweaker class is replaced where it is, instead of being
        /// moved to the end of the arguments.
        /// </param>
        /// <param name="reserve">
        /// Whether the tweaker class is moved to the start of the arguments.
        /// </param>
        public void ReplaceTweakClass(string target, string? replacement, bool inPlace, bool reserve)
        {
            var replaced = false;
            var index = 0;
            while (index < GameArguments.Count)
            {
                if (GameArguments[index] == "--tweakClass" && index + 1 < GameArguments.Count &&
                    GameArguments[index + 1] == target)
                {
                    if (!replaced && inPlace && replacement is not null)
                    {
                        GameArguments[index + 1] = replacement;
                        replaced = true;
                        index += 2;
                        continue;
                    }

                    if (!replaced && reserve && replacement is not null)
                    {
                        GameArguments.Insert(0, replacement);
                        replaced = true;
                        index += 2;
                        continue;
                    }

                    GameArguments.RemoveAt(index + 1);
                    GameArguments.RemoveAt(index);
                    continue;
                }

                index++;
            }

            if (!replaced && replacement is not null)
                GameArguments.Add(replacement);
        }

        /// <summary>
        /// Builds the repaired manifest.
        /// </summary>
        /// <returns>The repaired manifest.</returns>
        public GameInstanceManifest Build() => new()
        {
            Version = _manifest.Version,
            LastPlayed = _manifest.LastPlayed,
            MainClass = MainClass,
            JavaVersion = _manifest.JavaVersion,
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = JvmArguments,
                Game = GameArguments
            },
            Libraries = _libraries,
            AssetIndex = _manifest.AssetIndex,
            LoggingConfig = _manifest.LoggingConfig,
            Downloads = _manifest.Downloads,
            Demo = _manifest.Demo,
            CustomResolution = _manifest.CustomResolution,
            GameSettings = _manifest.GameSettings,
            IconType = _manifest.IconType,
            Patches = _manifest.Patches
        };
    }
}
