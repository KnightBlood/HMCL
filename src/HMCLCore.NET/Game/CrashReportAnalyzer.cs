using System;
using System.Collections.Generic;
using System.IO;
using System.Text.RegularExpressions;

namespace HMCLCore.NET.Game;

/// <summary>
/// The causes of a crash of the game detected by the crash report analyzer,
/// like the rules of the crash report analyzer of Java HMCL.
/// </summary>
public enum CrashCause
{
    /// <summary>The Java runtime used by the game is too old.</summary>
    TooOldJava,

    /// <summary>The game needs a newer Java runtime.</summary>
    NeedJdk11,

    /// <summary>The Java runtime used by the game is too new.</summary>
    JavaVersionTooHigh,

    /// <summary>The game cannot run on Java 9 or newer.</summary>
    Jdk9,

    /// <summary>The memory of the game exceeds the limit of a 32-bit runtime.</summary>
    Jvm32Bit,

    /// <summary>The computer ran out of memory.</summary>
    OutOfMemory,

    /// <summary>Too much memory was allocated for a small page file.</summary>
    MemoryExceeded,

    /// <summary>The graphics driver does not support OpenGL.</summary>
    OpenGlNotSupported,

    /// <summary>Something went wrong with the graphics driver.</summary>
    GraphicsDriver,

    /// <summary>Some mods, shaders or resource packs broke the graphics.</summary>
    GlOperationFailure,

    /// <summary>The resolution of the resource pack is too high.</summary>
    ResolutionTooHigh,

    /// <summary>The dependencies of the mods cannot be resolved.</summary>
    ModResolution,

    /// <summary>Forge could not resolve the dependencies of the mods.</summary>
    ForgeModResolution,

    /// <summary>Forge found duplicated mods.</summary>
    ForgeFoundDuplicateMods,

    /// <summary>Two mods conflict with each other.</summary>
    ModResolutionConflict,

    /// <summary>A mod requires another mod which is not installed.</summary>
    ModResolutionMissing,

    /// <summary>A mod is not compatible with the version of the game.</summary>
    ModResolutionMissingMinecraft,

    /// <summary>The versions of the mods cannot be resolved.</summary>
    ModResolutionCollection,

    /// <summary>Two mods with the same name are installed.</summary>
    DuplicatedMod,

    /// <summary>A class or a method of the game is missing.</summary>
    NoSuchMethodError,

    /// <summary>A class of the game is missing.</summary>
    NoClassDefFoundError,

    /// <summary>A mod tried to access a class it may not access.</summary>
    IllegalAccessError,

    /// <summary>The mixin of a mod could not be applied.</summary>
    MixinApplyModFailed,

    /// <summary>Some mods failed to inject their code.</summary>
    ModMixinFailure,

    /// <summary>The game failed to create the instance of a mod.</summary>
    BootstrapFailed,

    /// <summary>A mod crashed while the game was loading.</summary>
    LoadingCrashedForge,

    /// <summary>A mod crashed while the game was loading.</summary>
    LoadingCrashedFabric,

    /// <summary>A mod could not read its configuration file.</summary>
    Config,

    /// <summary>An entity of the world crashed the game.</summary>
    Entity,

    /// <summary>A block of the world crashed the game.</summary>
    Block,

    /// <summary>A library of the game is missing.</summary>
    UnsatisfiedLinkError,

    /// <summary>A file the game needs already exists.</summary>
    FileAlreadyExists,

    /// <summary>The files of the game were modified.</summary>
    FileChanged,

    /// <summary>The crash was triggered by the user.</summary>
    DebugCrash,

    /// <summary>The files of a mod were extracted.</summary>
    ModFilesAreDecompressed,

    /// <summary>The installation of Forge or NeoForge is incomplete.</summary>
    IncompleteForgeInstallation,

    /// <summary>OptiFine and a mod adding shaders are installed at the same time.</summary>
    ShadersMod,

    /// <summary>The name of a mod file is not valid.</summary>
    ModName,

    /// <summary>The version of Forge is not compatible with the Java runtime.</summary>
    Modlauncher8,

    /// <summary>Forge provided the details of the crash.</summary>
    ForgeError,

    /// <summary>The configuration files of the mods cannot be read.</summary>
    NightConfigFixes,

    /// <summary>OptiFine is not compatible with the current installation of Forge.</summary>
    OptifineIsNotCompatibleWithForge,

    /// <summary>The version of Forge or OptiFine is not compatible with the Java runtime.</summary>
    MacJdk8u261,

    /// <summary>The MixinBootstrap of the mods is missing.</summary>
    InstallMixinBootstrap,

    /// <summary>Forge is installed twice.</summary>
    ForgeRepeatInstallation,

    /// <summary>OptiFine is installed twice.</summary>
    OptifineRepeatInstallation,

    /// <summary>The game cannot run on an OpenJ9 runtime.</summary>
    OpenJ9,

    /// <summary>Too many mods are installed, so the limit of the identifiers is exceeded.</summary>
    TooManyMods
}

/// <summary>
/// One cause of a crash found by the crash report analyzer.
/// </summary>
public sealed class CrashCauseResult
{
    /// <summary>
    /// Creates the result.
    /// </summary>
    /// <param name="cause">The cause of the crash.</param>
    /// <param name="arguments">The details of the cause, keyed by their name.</param>
    public CrashCauseResult(CrashCause cause, IReadOnlyDictionary<string, string> arguments)
    {
        Cause = cause;
        Arguments = arguments;
    }

    /// <summary>
    /// The cause of the crash.
    /// </summary>
    public CrashCause Cause { get; }

    /// <summary>
    /// The details of the cause, keyed by their name.
    /// </summary>
    public IReadOnlyDictionary<string, string> Arguments { get; }

    /// <summary>
    /// The key of the localized text of the cause.
    /// </summary>
    public string TextKey => "game.crash.reason." + GetTextSuffix(Cause);

    /// <summary>
    /// Returns the suffix of the key of the localized text of a cause.
    /// </summary>
    /// <param name="cause">The cause of the crash.</param>
    /// <returns>The suffix of the key.</returns>
    private static string GetTextSuffix(CrashCause cause) => cause switch
    {
        CrashCause.TooOldJava => "too_old_java",
        CrashCause.NeedJdk11 => "need_jdk11",
        CrashCause.JavaVersionTooHigh => "java_version_is_too_high",
        CrashCause.Jdk9 => "jdk_9",
        CrashCause.Jvm32Bit => "jvm_32bit",
        CrashCause.OutOfMemory => "out_of_memory",
        CrashCause.MemoryExceeded => "memory_exceeded",
        CrashCause.OpenGlNotSupported => "opengl_not_supported",
        CrashCause.GraphicsDriver => "graphics_driver",
        CrashCause.GlOperationFailure => "gl_operation_failure",
        CrashCause.ResolutionTooHigh => "resolution_too_high",
        CrashCause.ModResolution => "mod_resolution",
        CrashCause.ForgeModResolution => "forgemod_resolution",
        CrashCause.ForgeFoundDuplicateMods => "forge_found_duplicate_mods",
        CrashCause.ModResolutionConflict => "mod_resolution_conflict",
        CrashCause.ModResolutionMissing => "mod_resolution_missing",
        CrashCause.ModResolutionMissingMinecraft => "mod_resolution_missing_minecraft",
        CrashCause.ModResolutionCollection => "mod_resolution_collection",
        CrashCause.DuplicatedMod => "duplicated_mod",
        CrashCause.NoSuchMethodError => "no_such_method_error",
        CrashCause.NoClassDefFoundError => "no_class_def_found_error",
        CrashCause.IllegalAccessError => "illegal_access_error",
        CrashCause.MixinApplyModFailed => "mixin_apply_mod_failed",
        CrashCause.ModMixinFailure => "modmixin_failure",
        CrashCause.BootstrapFailed => "bootstrap_failed",
        CrashCause.LoadingCrashedForge => "loading_crashed_forge",
        CrashCause.LoadingCrashedFabric => "loading_crashed_fabric",
        CrashCause.Config => "config",
        CrashCause.Entity => "entity",
        CrashCause.Block => "block",
        CrashCause.UnsatisfiedLinkError => "unsatisfied_link_error",
        CrashCause.FileAlreadyExists => "file_already_exists",
        CrashCause.FileChanged => "file_changed",
        CrashCause.DebugCrash => "debug_crash",
        CrashCause.ModFilesAreDecompressed => "mod_files_are_decompressed",
        CrashCause.IncompleteForgeInstallation => "incomplete_forge_installation",
        CrashCause.ShadersMod => "shaders_mod",
        CrashCause.ModName => "mod_name",
        CrashCause.Modlauncher8 => "modlauncher_8",
        CrashCause.ForgeError => "forge_error",
        CrashCause.NightConfigFixes => "night_config_fixes",
        CrashCause.OptifineIsNotCompatibleWithForge => "optifine_is_not_compatible_with_forge",
        CrashCause.MacJdk8u261 => "mac_jdk_8u261",
        CrashCause.InstallMixinBootstrap => "install_mixinbootstrap",
        CrashCause.ForgeRepeatInstallation => "forge_repeat_installation",
        CrashCause.OptifineRepeatInstallation => "optifine_repeat_installation",
        CrashCause.OpenJ9 => "openj9",
        CrashCause.TooManyMods => "too_many_mods_lead_to_exceeding_the_id_limit",
        _ => "unknown"
    };
}

/// <summary>
/// Finds the causes of a crash of the game in its logs, like the crash report
/// analyzer of Java HMCL.
/// </summary>
public static class CrashReportAnalyzer
{
    /// <summary>
    /// The rules matching the causes of the crashes.
    /// </summary>
    private static readonly (CrashCause Cause, Regex Pattern, string[] Groups)[] Rules = CreateRules();

    /// <summary>
    /// Creates the rules matching the causes of the crashes; a rule with an
    /// invalid pattern is left out.
    /// </summary>
    /// <returns>The rules.</returns>
    private static (CrashCause Cause, Regex Pattern, string[] Groups)[] CreateRules()
    {
        var rules = new List<(CrashCause Cause, Regex Pattern, string[] Groups)>();

        foreach (var rule in DefineRules())
        {
            try
            {
                rules.Add((rule.Cause, new Regex(rule.Pattern), rule.Groups));
            }
            catch (ArgumentException)
            {
                // A rule with an invalid pattern is left out.
            }
        }

        return rules.ToArray();
    }

    /// <summary>
    /// The patterns of the rules matching the causes of the crashes.
    /// </summary>
    private static (CrashCause Cause, string Pattern, string[] Groups)[] DefineRules() => new[]
    {
        (CrashCause.OpenJ9, "Open J9 is not supported|OpenJ9 is incompatible|\\.J9VMInternals\\.", Array.Empty<string>()),
        (CrashCause.NeedJdk11, "no such method: sun\\.misc\\.Unsafe\\.defineAnonymousClass|UnsupportedClassVersionError: icyllis/modernui/forge/MixinConnector has been compiled by a more recent version|could not be set\\. Level is not supported by the active JRE or ASM version", Array.Empty<string>()),
        (CrashCause.TooOldJava, "java\\.lang\\.UnsupportedClassVersionError: (.*?) version (?<expected>\\d+)\\.0", new[] { "expected" }),
        (CrashCause.Jvm32Bit, "Could not reserve enough space for (.*?)KB object heap|The specified size exceeds the maximum representable size|Invalid maximum heap size", Array.Empty<string>()),
        (CrashCause.GlOperationFailure, "1282: Invalid operation|Maybe try a lower resolution resourcepack\\?", Array.Empty<string>()),
        (CrashCause.OpenGlNotSupported, "The driver does not appear to support OpenGL", Array.Empty<string>()),
        (CrashCause.GraphicsDriver, "Pixel format not accelerated|GLX: Failed to create context|net\\.minecraftforge\\.fml\\.client\\.SplashProgress|org\\.lwjgl\\.LWJGLException|EXCEPTION_ACCESS_VIOLATION", Array.Empty<string>()),
        (CrashCause.OutOfMemory, "java\\.lang\\.OutOfMemoryError|The system is out of physical RAM or swap space|Out of Memory Error|Error occurred during initialization of VM|Too small maximum heap", Array.Empty<string>()),
        (CrashCause.MemoryExceeded, "There is insufficient memory for the Java Runtime Environment to continue", Array.Empty<string>()),
        (CrashCause.ResolutionTooHigh, "Maybe try a (lower resolution|lowerresolution) (resourcepack|texturepack)\\?", Array.Empty<string>()),
        (CrashCause.Jdk9, "java\\.lang\\.ClassCastException: (java\\.base/jdk|class jdk)", Array.Empty<string>()),
        (CrashCause.MacJdk8u261, "NSWindow drag regions should only be invalidated on the Main Thread", Array.Empty<string>()),
        (CrashCause.FileChanged, "java\\.lang\\.SecurityException: SHA1 digest error for (?<file>.*)|signer information does not match signer information of other classes in the same package", new[] { "file" }),
        (CrashCause.NoSuchMethodError, "java\\.lang\\.NoSuchMethodError: (?<class>.*)", new[] { "class" }),
        (CrashCause.NoClassDefFoundError, "java\\.lang\\.NoClassDefFoundError: (?<class>.*)", new[] { "class" }),
        (CrashCause.IllegalAccessError, "java\\.lang\\.IllegalAccessError: tried to access class (.*?) from class (?<class>.*?)", new[] { "class" }),
        (CrashCause.DuplicatedMod, "Found a duplicate mod (?<name>.*) at (?<path>.*)", new[] { "name", "path" }),
        (CrashCause.ModResolution, "ModResolutionException: (?<reason>(.*)[\\n\\r]*( - (.*)[\\n\\r]*)+)", new[] { "reason" }),
        (CrashCause.ForgeModResolution, "Missing or unsupported mandatory dependencies:(?<reason>(.*)[\\n\\r]*(\\t(.*)[\\n\\r]*)+)", new[] { "reason" }),
        (CrashCause.ForgeFoundDuplicateMods, "Found duplicate mods:(?<reason>(.*)\\R*(\\t(.*)\\R*)+)", new[] { "reason" }),
        (CrashCause.ModResolutionConflict, "ModResolutionException: Found conflicting mods: (?<sourcemod>.*) conflicts with (?<destmod>.*)", new[] { "sourcemod", "destmod" }),
        (CrashCause.ModResolutionMissing, "ModResolutionException: Could not find required mod: (?<sourcemod>.*) requires (?<destmod>.*)", new[] { "sourcemod", "destmod" }),
        (CrashCause.ModResolutionMissingMinecraft, "ModResolutionException: Could not find required mod: (?<mod>.*) requires \\{minecraft @ (?<version>.*)\\}", new[] { "mod", "version" }),
        (CrashCause.ModResolutionCollection, "ModResolutionException: Could not resolve valid mod collection \\(at: (?<sourcemod>.*) requires (?<destmod>.*)\\)", new[] { "sourcemod", "destmod" }),
        (CrashCause.FileAlreadyExists, "java\\.nio\\.file\\.FileAlreadyExistsException: (?<file>.*)", new[] { "file" }),
        (CrashCause.LoadingCrashedForge, "LoaderExceptionModCrash: Caught exception from (?<name>.*?) \\((?<id>.*)\\)", new[] { "name", "id" }),
        (CrashCause.BootstrapFailed, "Failed to create mod instance\\. ModID: (?<id>.*?),", new[] { "id" }),
        (CrashCause.LoadingCrashedFabric, "Could not execute entrypoint stage '(.*?)' due to errors, provided by '(?<id>.*)'!", new[] { "id" }),
        (CrashCause.InstallMixinBootstrap, "java\\.lang\\.ClassNotFoundException: org\\.spongepowered\\.asm\\.launch\\.MixinTweaker", Array.Empty<string>()),
        (CrashCause.Config, "Failed loading config file (?<file>.*?) of type (.*?) for modid (?<id>.*)", new[] { "id", "file" }),
        (CrashCause.Entity, "Entity Type: (?<type>.*)[\\w\\W\\n\\r]*?Entity's Exact location: (?<location>.*)", new[] { "type", "location" }),
        (CrashCause.Block, "Block: (?<type>.*)[\\w\\W\\n\\r]*?Block location: (?<location>.*)", new[] { "type", "location" }),
        (CrashCause.UnsatisfiedLinkError, "java\\.lang\\.UnsatisfiedLinkError: Failed to locate library: (?<name>.*)", new[] { "name" }),
        (CrashCause.OptifineIsNotCompatibleWithForge, "java\\.lang\\.NoSuchMethodError: 'java\\.lang\\.Class sun\\.misc\\.Unsafe\\.defineAnonymousClass|java\\.lang\\.NoSuchMethodError: 'void net\\.minecraft\\.client\\.renderer\\.texture\\.SpriteContents", Array.Empty<string>()),
        (CrashCause.ModFilesAreDecompressed, "The directories below appear to be extracted jar files\\. Fix this before you continue|Extracted mod jars found, loading will NOT continue", Array.Empty<string>()),
        (CrashCause.TooManyMods, "maximum id range exceeded", Array.Empty<string>()),
        (CrashCause.ModMixinFailure, "MixinApplyError|Mixin prepare failed |Mixin apply failed |mixin\\.injection\\.throwables\\.|\\.mixins\\.json\\] FAILED during \\)", Array.Empty<string>()),
        (CrashCause.MixinApplyModFailed, "Mixin apply for mod (?<id>.*) failed", new[] { "id" }),
        (CrashCause.ForgeError, "An exception was thrown, the game will display an error screen and halt\\.\\R*(?<reason>.*\\R*(\\s*at .*\\R)+)", new[] { "reason" }),
        (CrashCause.ForgeRepeatInstallation, "MultipleArgumentsForOptionException: Found multiple arguments for option (.*?), but you asked for only one", Array.Empty<string>()),
        (CrashCause.OptifineRepeatInstallation, "ResolutionException: Module optifine reads another module named optifine", Array.Empty<string>()),
        (CrashCause.JavaVersionTooHigh, "Unable to make protected final java\\.lang\\.Class java\\.lang\\.ClassLoader\\.defineClass|java\\.lang\\.NoSuchFieldException: ucp|Unsupported class file major version|because module java\\.base does not export|java\\.lang\\.ClassNotFoundException: jdk\\.nashorn\\.api\\.scripting\\.NashornScriptEngineFactory|java\\.lang\\.ClassNotFoundException: java\\.lang\\.invoke\\.LambdaMetafactory", Array.Empty<string>()),
        (CrashCause.ModName, "Invalid module name: '' is not a Java identifier", Array.Empty<string>()),
        (CrashCause.IncompleteForgeInstallation, "Invalid paths argument, contained no existing paths|Failed to find Minecraft resource version|Cannot find launch target fmlclient, unable to launch|Could not find net/minecraft/client/Minecraft\\.class in classloader SecureModuleClassLoader", Array.Empty<string>()),
        (CrashCause.NightConfigFixes, "com\\.electronwill\\.nightconfig\\.core\\.io\\.ParsingException: Not enough data available", Array.Empty<string>()),
        (CrashCause.ShadersMod, "java\\.lang\\.RuntimeException: Shaders Mod detected\\. Please remove it, OptiFine has built-in support for shaders\\.", Array.Empty<string>())
    };

    /// <summary>
    /// Finds the causes of the crash in the given log.
    /// </summary>
    /// <param name="log">The log of the game.</param>
    /// <returns>The causes found, without duplicates.</returns>
    public static IReadOnlyList<CrashCauseResult> FindCauses(string? log)
    {
        var results = new List<CrashCauseResult>();
        if (string.IsNullOrEmpty(log))
            return results;

        foreach (var (cause, pattern, groups) in Rules)
        {
            var match = pattern.Match(log);
            if (!match.Success)
                continue;

            var arguments = new Dictionary<string, string>(StringComparer.Ordinal);
            foreach (var group in groups)
            {
                if (match.Groups[group].Success)
                    arguments[group] = match.Groups[group].Value.Trim();
            }

            results.Add(new CrashCauseResult(cause, arguments));
        }

        return results;
    }

    /// <summary>
    /// Finds the causes of the crash in the log of the game.
    /// </summary>
    /// <param name="gameDirectory">The directory the game runs in.</param>
    /// <returns>The causes found.</returns>
    public static IReadOnlyList<CrashCauseResult> FindCausesInLogs(string gameDirectory)
    {
        var results = new List<CrashCauseResult>();

        var latestLog = Path.Combine(gameDirectory, "logs", "latest.log");
        if (File.Exists(latestLog))
        {
            try
            {
                results.AddRange(FindCauses(File.ReadAllText(latestLog)));
            }
            catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
            {
                // The log cannot be read; the other logs are used instead.
            }
        }

        var crashReports = Path.Combine(gameDirectory, "crash-reports");
        if (Directory.Exists(crashReports))
        {
            var newest = Directory.GetFiles(crashReports, "*.txt")
                .OrderByDescending(File.GetLastWriteTimeUtc)
                .FirstOrDefault();

            if (newest is not null)
            {
                try
                {
                    results.AddRange(FindCauses(File.ReadAllText(newest)));
                }
                catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
                {
                    // The crash report cannot be read.
                }
            }
        }

        return results;
    }
}
