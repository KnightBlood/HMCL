//! Reading the errors a version of the game ran into.
//!
//! The game writes its crash reports and the messages of the mod loaders to the
//! log; the messages tell the player how to fix the crash. The rules of this
//! module are the messages the launcher knows, as the version of the game the
//! launcher was written for knows them.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

/// A message the launcher reads as an error of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rule {
    /// The game runs on OpenJ9, which the game does not support.
    OpenJ9,
    /// The game needs the JDK 11 or a newer version of Java.
    NeedJdk11,
    /// The game was compiled by a newer version of Java than the one it runs
    /// on.
    TooOldJava,
    /// The launcher gave the game more memory than the 32-bit Java runtime can
    /// use.
    Jvm32Bit,
    /// The game or a mod did an operation OpenGL does not allow.
    GlOperationFailure,
    /// The graphics driver does not support OpenGL.
    OpenGlNotSupported,
    /// The graphics driver of the player cannot run the game.
    GraphicsDriver,
    /// macOS could not find the display the game draws on.
    MacosFailedToFindServicePortForDisplay,
    /// The game ran out of memory.
    OutOfMemory,
    /// The system ran out of memory.
    MemoryExceeded,
    /// The resolution of the game or of a resource pack is too high.
    ResolutionTooHigh,
    /// The game runs on Java 9 or a newer version, which it cannot run on.
    Jdk9,
    /// macOS and the JDK 8u261 cannot run the game together.
    MacJdk8u261,
    /// A file of the game changed, so its digest does not match.
    FileChanged,
    /// A mod loader or a core mod asks for a method that is not there.
    NoSuchMethodError,
    /// A mod loader or a core mod asks for a class that is not there.
    NoClassDefFoundError,
    /// A mod loader or a core mod asks for a class it may not read.
    IllegalAccessError,
    /// The same mod was installed twice.
    DuplicatedMod,
    /// The mods of a version cannot be resolved.
    ModResolution,
    /// The mods of a version need mods that are not installed.
    ForgeModResolution,
    /// The same mod was installed twice, as Forge tells it.
    ForgeFoundDuplicateMods,
    /// Two mods cannot be installed together.
    ModResolutionConflict,
    /// A mod needs a mod that is not installed.
    ModResolutionMissing,
    /// A mod needs a version of the game that is not installed.
    ModResolutionMissingMinecraft,
    /// The mods of a version cannot be collected.
    ModResolutionCollection,
    /// A file the game writes is already there.
    FileAlreadyExists,
    /// A mod crashed while the game loaded.
    LoadingCrashedForge,
    /// A mod could not be created.
    BootstrapFailed,
    /// A mod crashed while the game loaded, as Fabric tells it.
    LoadingCrashedFabric,
    /// The game runs on a Fabric that is too old for the mods.
    FabricVersion0_12,
    /// The game runs on a ModLauncher that is too old for the version of Java.
    Modlauncher8,
    /// The player asked the game to crash.
    DebugCrash,
    /// The game could not read the configuration of a mod.
    Config,
    /// Fabric found warnings about the mods of the version.
    FabricWarnings,
    /// The game crashed while it ticked an entity.
    Entity,
    /// The game crashed while it drew a block.
    Block,
    /// The game could not load a native library.
    UnsatisfiedLinkError,
    /// OptiFine and Forge are not compatible.
    OptifineIsNotCompatibleWithForge,
    /// A mod was extracted from its jar file.
    ModFilesAreDecompressed,
    /// OptiFine causes the world to fail to load.
    OptifineCausesTheWorldToFailToLoad,
    /// The mods of the version use more identifiers than the game allows.
    TooManyModsLeadToExceedingTheIdLimit,
    /// A mixin of a mod failed.
    ModMixinFailure,
    /// A mixin of a mod could not be applied.
    MixinApplyModFailed,
    /// Forge caught an exception of the game.
    ForgeError,
    /// The mod loader tells the mods of the version.
    ModResolution0,
    /// Forge was installed more than once.
    ForgeRepeatInstallation,
    /// OptiFine was installed more than once.
    OptifineRepeatInstallation,
    /// The version of Java is too high for the game.
    JavaVersionIsTooHigh,
    /// MixinBootstrap is not installed.
    InstallMixinbootstrap,
    /// A mod jar file has a name that cannot be a module name.
    ModName,
    /// Forge was not installed completely.
    IncompleteForgeInstallation,
    /// Night Config Fixes could not read a file.
    NightConfigFixes,
    /// The Shaders Mod is installed together with OptiFine.
    ShadersMod,
    /// A mod of Forge is not compatible with OptiFine.
    ModForestOptifine,
    /// Performant is not compatible with OptiFine.
    PerformantForestOptifine,
    /// Twilight Forest is not compatible with OptiFine.
    TwilightForestOptifine,
    /// Jade is not compatible with OptiFine.
    JadeForestOptifine,
    /// NeoForge is not compatible with OptiFine.
    NeoforgeForestOptifine,
    /// RivaTuner Statistics Server is not compatible with Sodium.
    RtssForestSodium,
}

/// The pattern and the groups of a rule.
struct RuleDefinition {
    /// The rule.
    rule: Rule,
    /// The pattern of the message, written as the Java version of the launcher
    /// writes it.
    pattern: &'static str,
    /// The names of the groups of the pattern that the launcher reads.
    groups: &'static [&'static str],
}

/// The rules of the launcher.
const RULES: &[RuleDefinition] = &[
    RuleDefinition {
        rule: Rule::OpenJ9,
        pattern: "(Open J9 is not supported|OpenJ9 is incompatible|\\.J9VMInternals\\.)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::NeedJdk11,
        pattern: "(no such method: sun\\.misc\\.Unsafe\\.defineAnonymousClass\\(Class,byte\\[\\],Object\\[\\]\\)Class/invokeVirtual|java\\.lang\\.UnsupportedClassVersionError: icyllis/modernui/forge/MixinConnector has been compiled by a more recent version of the Java Runtime \\(class file version 55\\.0\\), this version of the Java Runtime only recognizes class file versions up to 52\\.0|java\\.lang\\.IllegalArgumentException: The requested compatibility level JAVA_11 could not be set\\. Level is not supported by the active JRE or ASM version)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::TooOldJava,
        pattern: "java\\.lang\\.UnsupportedClassVersionError: (.*?) version (?<expected>\\d+)\\.0",
        groups: &["expected"],
    },
    RuleDefinition {
        rule: Rule::Jvm32Bit,
        pattern: "(Could not reserve enough space for (.*?)KB object heap|The specified size exceeds the maximum representable size|Invalid maximum heap size)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::GlOperationFailure,
        pattern: "(1282: Invalid operation|Maybe try a lower resolution resourcepack\\?)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::OpenGlNotSupported,
        pattern: "The driver does not appear to support OpenGL",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::GraphicsDriver,
        pattern: "(Pixel format not accelerated|GLX: Failed to create context: GLXBadFBConfig|Couldn't set pixel format|net\\.minecraftforge\\.fml\\.client\\.SplashProgress|org\\.lwjgl\\.LWJGLException|EXCEPTION_ACCESS_VIOLATION(.|\\n|\\r)+# C {2}\\[(ig|atio|nvoglv))",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::MacosFailedToFindServicePortForDisplay,
        pattern: "java\\.lang\\.IllegalStateException: GLFW error before init: \\[0x10008\\]Cocoa: Failed to find service port for display",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::OutOfMemory,
        pattern: "(java\\.lang\\.OutOfMemoryError|The system is out of physical RAM or swap space|Out of Memory Error|Error occurred during initialization of VM\\RToo small maximum heap)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::MemoryExceeded,
        pattern: "There is insufficient memory for the Java Runtime Environment to continue",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ResolutionTooHigh,
        pattern: "Maybe try a (lower resolution|lowerresolution) (resourcepack|texturepack)\\?",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::Jdk9,
        pattern: "java\\.lang\\.ClassCastException: (java\\.base/jdk|class jdk)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::MacJdk8u261,
        pattern: "Terminating app due to uncaught exception 'NSInternalInconsistencyException', reason: 'NSWindow drag regions should only be invalidated on the Main Thread!'",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::FileChanged,
        pattern: "java\\.lang\\.SecurityException: SHA1 digest error for (?<file>.*)|signer information does not match signer information of other classes in the same package",
        groups: &["file"],
    },
    RuleDefinition {
        rule: Rule::NoSuchMethodError,
        pattern: "java\\.lang\\.NoSuchMethodError: (?<class>.*?)",
        groups: &["class"],
    },
    RuleDefinition {
        rule: Rule::NoClassDefFoundError,
        pattern: "java\\.lang\\.NoClassDefFoundError: (?<class>.*)",
        groups: &["class"],
    },
    RuleDefinition {
        rule: Rule::IllegalAccessError,
        pattern: "java\\.lang\\.IllegalAccessError: tried to access class (.*?) from class (?<class>.*?)",
        groups: &["class"],
    },
    RuleDefinition {
        rule: Rule::DuplicatedMod,
        pattern: "Found a duplicate mod (?<name>.*) at (?<path>.*)",
        groups: &["name", "path"],
    },
    RuleDefinition {
        rule: Rule::ModResolution,
        pattern: "ModResolutionException: (?<reason>(.*)[\\n\\r]*( - (.*)[\\n\\r]*)+)",
        groups: &["reason"],
    },
    RuleDefinition {
        rule: Rule::ForgeModResolution,
        pattern: "Missing or unsupported mandatory dependencies:(?<reason>(.*)[\\n\\r]*(\t(.*)[\\n\\r]*)+)",
        groups: &["reason"],
    },
    RuleDefinition {
        rule: Rule::ForgeFoundDuplicateMods,
        pattern: "Found duplicate mods:(?<reason>(.*)\\R*(\t(.*)\\R*)+)",
        groups: &["reason"],
    },
    RuleDefinition {
        rule: Rule::ModResolutionConflict,
        pattern: "ModResolutionException: Found conflicting mods: (?<sourcemod>.*) conflicts with (?<destmod>.*)",
        groups: &["sourcemod", "destmod"],
    },
    RuleDefinition {
        rule: Rule::ModResolutionMissing,
        pattern: "ModResolutionException: Could not find required mod: (?<sourcemod>.*) requires (?<destmod>.*)",
        groups: &["sourcemod", "destmod"],
    },
    RuleDefinition {
        rule: Rule::ModResolutionMissingMinecraft,
        pattern: "ModResolutionException: Could not find required mod: (?<mod>.*) requires \\{minecraft @ (?<version>.*)}",
        groups: &["mod", "version"],
    },
    RuleDefinition {
        rule: Rule::ModResolutionCollection,
        pattern: "ModResolutionException: Could not resolve valid mod collection \\(at: (?<sourcemod>.*) requires (?<destmod>.*)\\)",
        groups: &["sourcemod", "destmod"],
    },
    RuleDefinition {
        rule: Rule::FileAlreadyExists,
        pattern: "java\\.nio\\.file\\.FileAlreadyExistsException: (?<file>.*)",
        groups: &["file"],
    },
    RuleDefinition {
        rule: Rule::LoadingCrashedForge,
        pattern: "LoaderExceptionModCrash: Caught exception from (?<name>.*?) \\((?<id>.*)\\)",
        groups: &["name", "id"],
    },
    RuleDefinition {
        rule: Rule::BootstrapFailed,
        pattern: "Failed to create mod instance\\. ModID: (?<id>.*?),",
        groups: &["id"],
    },
    RuleDefinition {
        rule: Rule::LoadingCrashedFabric,
        pattern: "Could not execute entrypoint stage '(.*?)' due to errors, provided by '(?<id>.*)'!",
        groups: &["id"],
    },
    RuleDefinition {
        rule: Rule::FabricVersion0_12,
        pattern: "java\\.lang\\.NoClassDefFoundError: org/spongepowered/asm/mixin/transformer/FabricMixinTransformerProxy",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::Modlauncher8,
        pattern: "java\\.lang\\.NoSuchMethodError: ('void sun\\.security\\.util\\.ManifestEntryVerifier\\.<init>\\(java\\.util\\.jar\\.Manifest\\)'|sun\\.security\\.util\\.ManifestEntryVerifier\\.<init>\\(Ljava/util/jar/Manifest;\\)V)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::DebugCrash,
        pattern: "Manually triggered debug crash",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::Config,
        pattern: "Failed loading config file (?<file>.*?) of type (.*?) for modid (?<id>.*)",
        groups: &["id", "file"],
    },
    RuleDefinition {
        rule: Rule::FabricWarnings,
        pattern: "(Warnings were found!|Incompatible mod set!|Incompatible mods found!)(.*?)[\\n\\r]+(?<reason>[^\\[]+)\\[",
        groups: &["reason"],
    },
    RuleDefinition {
        rule: Rule::Entity,
        pattern: "Entity Type: (?<type>.*)[\\w\\W\\n\\r]*?Entity's Exact location: (?<location>.*)",
        groups: &["type", "location"],
    },
    RuleDefinition {
        rule: Rule::Block,
        pattern: "Block: (?<type>.*)[\\w\\W\\n\\r]*?Block location: (?<location>.*)",
        groups: &["type", "location"],
    },
    RuleDefinition {
        rule: Rule::UnsatisfiedLinkError,
        pattern: "java\\.lang\\.UnsatisfiedLinkError: Failed to locate library: (?<name>.*)",
        groups: &["name"],
    },
    RuleDefinition {
        rule: Rule::OptifineIsNotCompatibleWithForge,
        pattern: "(java\\.lang\\.NoSuchMethodError: 'java\\.lang\\.Class sun\\.misc\\.Unsafe\\.defineAnonymousClass\\(java\\.lang\\.Class, byte\\[\\], java\\.lang\\.Object\\[\\]\\)'|java\\.lang\\.NoSuchMethodError: 'void net\\.minecraft\\.client\\.renderer\\.texture\\.SpriteContents\\.\\<init\\>\\(net\\.minecraft\\.resources\\.ResourceLocation, |java\\.lang\\.NoSuchMethodError: 'void net\\.minecraftforge\\.client\\.gui\\.overlay\\.ForgeGui\\.renderSelectedItemName\\(net\\.minecraft\\.client\\.gui\\.GuiGraphics, int\\)'|java\\.lang\\.NoSuchMethodError: 'java\\.lang\\.String com\\.mojang\\.blaze3d\\.systems\\.RenderSystem\\.getBackendDescription\\(\\)'|java\\.lang\\.NoSuchMethodError: 'net\\.minecraft\\.network\\.chat\\.FormattedText net\\.minecraft\\.client\\.gui\\.Font\\.ellipsize\\(net\\.minecraft\\.network\\.chat\\.FormattedText, int\\)'|java\\.lang\\.NoSuchMethodError: 'void net\\.minecraft\\.server\\.level\\.DistanceManager\\.(.*?)\\(net\\.minecraft\\.server\\.level\\.TicketType, net\\.minecraft\\.world\\.level\\.ChunkPos, int, java\\.lang\\.Object, boolean\\)'|java\\.lang\\.NoSuchMethodError: 'void net\\.minecraft\\.client\\.renderer\\.block\\.model\\.BakedQuad\\.\\<init\\>\\(int\\[\\], int, net\\.minecraft\\.core\\.Direction, net\\.minecraft\\.client\\.renderer\\.texture\\.TextureAtlasSprite, boolean, boolean\\)'|TRANSFORMER/net\\.optifine/net\\.optifine\\.reflect\\.Reflector\\.\\<clinit\\>\\(Reflector\\.java)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ModFilesAreDecompressed,
        pattern: "(The directories below appear to be extracted jar files\\. Fix this before you continue|Extracted mod jars found, loading will NOT continue)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::OptifineCausesTheWorldToFailToLoad,
        pattern: "java\\.lang\\.NoSuchMethodError: net\\.minecraft\\.world\\.server\\.ChunkManager$ProxyTicketManager\\.shouldForceTicks\\(J\\)Z",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::TooManyModsLeadToExceedingTheIdLimit,
        pattern: "maximum id range exceeded",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ModMixinFailure,
        pattern: "(MixinApplyError|Mixin prepare failed |Mixin apply failed |mixin\\.injection\\.throwables\\.|\\.mixins\\.json\\] FAILED during \\))",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::MixinApplyModFailed,
        pattern: "Mixin apply for mod (?<id>.*) failed",
        groups: &["id"],
    },
    RuleDefinition {
        rule: Rule::ForgeError,
        pattern: "An exception was thrown, the game will display an error screen and halt\\.\\R*(?<reason>.*\\R*(\\s*at .*\\R)+)",
        groups: &["reason"],
    },
    RuleDefinition {
        rule: Rule::ModResolution0,
        pattern: "(\tMod File:|-- MOD |\tFailure message:)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ForgeRepeatInstallation,
        pattern: "MultipleArgumentsForOptionException: Found multiple arguments for option (.*?), but you asked for only one",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::OptifineRepeatInstallation,
        pattern: "ResolutionException: Module optifine reads another module named optifine",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::JavaVersionIsTooHigh,
        pattern: "(Unable to make protected final java\\.lang\\.Class java\\.lang\\.ClassLoader\\.defineClass|java\\.lang\\.NoSuchFieldException: ucp|Unsupported class file major version|because module java\\.base does not export|java\\.lang\\.ClassNotFoundException: jdk\\.nashorn\\.api\\.scripting\\.NashornScriptEngineFactory|java\\.lang\\.ClassNotFoundException: java\\.lang\\.invoke\\.LambdaMetafactory|Exception in thread \"main\" java\\.lang\\.NullPointerException: Cannot read the array length because \"urls\" is null)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::InstallMixinbootstrap,
        pattern: "java\\.lang\\.ClassNotFoundException: org\\.spongepowered\\.asm\\.launch\\.MixinTweaker",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ModName,
        pattern: "Invalid module name: '' is not a Java identifier",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::IncompleteForgeInstallation,
        pattern: "(java\\.io\\.UncheckedIOException: java\\.io\\.IOException: Invalid paths argument, contained no existing paths: \\[(.*?)(forge-(.*?)-client\\.jar|fmlcore-(.*?)\\.jar)\\]|Failed to find Minecraft resource version (.*?) at (.*?)forge-(.*?)-client\\.jar|Cannot find launch target fmlclient, unable to launch|java\\.lang\\.IllegalStateException: Could not find net/minecraft/client/Minecraft\\.class in classloader SecureModuleClassLoader)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::NightConfigFixes,
        pattern: "com\\.electronwill\\.nightconfig\\.core\\.io\\.ParsingException: Not enough data available",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ShadersMod,
        pattern: "java\\.lang\\.RuntimeException: Shaders Mod detected\\. Please remove it, OptiFine has built-in support for shaders\\.",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::ModForestOptifine,
        pattern: "Error occurred applying transform of coremod META-INF/asm/multipart\\.js function render",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::PerformantForestOptifine,
        pattern: "org\\.spongepowered\\.asm\\.mixin\\.injection\\.throwables\\.InjectionError: Critical injection failure: Redirector OnisOnLadder\\(Lnet/minecraft/block/BlockState;Lnet/minecraft/world/World;Lnet/minecraft/util/math/BlockPos;Lnet/minecraft/entity/LivingEntity;\\)Z in performant\\.mixins\\.json:entity\\.LivingEntityMixin failed injection check, \\(0/1\\) succeeded\\. Scanned 1 target\\(s\\)\\. Using refmap performant\\.refmap\\.json",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::TwilightForestOptifine,
        pattern: "java\\.lang\\.IllegalArgumentException: (.*) outside of image bounds (.*)",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::JadeForestOptifine,
        pattern: "Critical injection failure: LVT in net/minecraft/client/renderer/GameRenderer::m_109093_\\(FJZ\\)V has incompatible changes at opcode 760 in callback jade\\.mixins\\.json:GameRendererMixin-\\>@Inject::jade\\$runTick\\(FJZLorg/spongepowered/asm/mixin/injection/callback/CallbackInfo;IILcom/mojang/blaze3d/platform/Window;Lorg/joml/Matrix4f;Lcom/mojang/blaze3d/vertex/PoseStack;Lnet/minecraft/client/gui/GuiGraphics;\\)V\\.",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::NeoforgeForestOptifine,
        pattern: "cpw\\.mods\\.modlauncher\\.InvalidLauncherSetupException: Invalid Services found OptiFine",
        groups: &[],
    },
    RuleDefinition {
        rule: Rule::RtssForestSodium,
        pattern: "RivaTuner Statistics Server \\(RTSS\\) is not compatible with Sodium",
        groups: &[],
    },
];

/// A message of the log the launcher reads, with the groups of the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashRuleMatch {
    /// The rule that matched the log.
    pub rule: Rule,
    /// The values of the groups the rule reads, by the names of the groups.
    pub groups: BTreeMap<String, String>,
}

impl CrashRuleMatch {
    /// Returns the value of a group of the message.
    pub fn group(&self, name: &str) -> Option<&str> {
        self.groups.get(name).map(String::as_str)
    }
}

/// The patterns of the rules, in the order of the rules.
fn patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        RULES
            .iter()
            .map(|definition| {
                Regex::new(&rust_pattern(definition.pattern))
                    .expect("the pattern of a crash rule is valid")
            })
            .collect()
    })
}

/// Returns the pattern of a rule in the language of the regular expressions of
/// Rust.
///
/// The patterns are written as the Java version of the launcher writes them;
/// the groups and the line breaks are written the way the Java regular
/// expressions are, which the regular expressions of Rust write differently.
fn rust_pattern(pattern: &str) -> String {
    pattern
        .replace("(?<", "(?P<")
        .replace("\\R", "(?:\\r\\n|\\r|\\n)")
}

/// Returns the messages of the log the launcher reads.
///
/// The messages are returned in the order of the rules of the launcher; the
/// log is read once, so a rule that matches more than once returns the first
/// match only.
pub fn analyze(log: &str) -> Vec<CrashRuleMatch> {
    patterns()
        .iter()
        .zip(RULES)
        .filter_map(|(pattern, definition)| {
            let captures = pattern.captures(log)?;
            let groups = definition
                .groups
                .iter()
                .filter_map(|name| {
                    captures
                        .name(name)
                        .map(|value| ((*name).to_string(), value.as_str().to_string()))
                })
                .collect();

            Some(CrashRuleMatch {
                rule: definition.rule,
                groups,
            })
        })
        .collect()
}

/// Returns the crash report of a log, if the log tells where the game saved it.
///
/// The game prints the place of the crash report as the game stops; the
/// launcher reads the report to tell the player what went wrong. The report is
/// an error when the log names a file the launcher cannot read.
pub fn find_crash_report(log: &str) -> io::Result<Option<String>> {
    let Some(captures) = crash_report_location().captures(log) else {
        return Ok(None);
    };

    let location = captures
        .name("location")
        .expect("the pattern of the place of a crash report reads the place")
        .as_str();
    std::fs::read_to_string(Path::new(location)).map(Some)
}

/// Returns the pattern of the place the game saved its crash report in.
fn crash_report_location() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(&rust_pattern(
            "#@!@# Game crashed! Crash report saved to: #@!@# (?<location>.*)",
        ))
        .expect("the pattern of the place of a crash report is valid")
    })
}

/// Returns the part of a log that is the crash report of the game.
///
/// The launcher asks the player for the crash report, so it reads the report
/// from the log the game wrote; a log that holds no report returns nothing.
pub fn extract_crash_report(raw_log: &str) -> Option<&str> {
    let begin = raw_log.rfind("---- Minecraft Crash Report ----")?;
    let end = raw_log.rfind("#@!@# Game crashed! Crash report saved to")?;
    if begin >= end {
        return None;
    }

    Some(&raw_log[begin..end])
}

/// The pattern of the stack of a crash report.
fn crash_report_stack() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(&rust_pattern(
            "Description: (.*?)[\\n\\r]+(?<stacktrace>[\\w\\W\\n\\r]+)A detailed walkthrough of the error",
        ))
        .expect("the pattern of the stack of a crash report is valid")
    })
}

/// The pattern of a line of the stack of a crash report.
fn stack_line() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(&rust_pattern("at (?<method>.*?)\\((?<sourcefile>.*?)\\)"))
            .expect("the pattern of a line of the stack is valid")
    })
}

/// The pattern of the modules of a line of the stack of a crash report.
fn stack_modules() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(&rust_pattern("\\{(?<tokens>.*)}"))
            .expect("the pattern of the modules of a line of the stack is valid")
    })
}

/// The parts of the names of the packages that do not name a mod.
fn package_blacklist() -> &'static HashSet<&'static str> {
    static KEYWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    KEYWORDS.get_or_init(|| {
        [
            // Minecraft
            "net",
            "minecraft",
            "item",
            "setup",
            "block",
            "assist",
            "optifine",
            "player",
            "unimi",
            "fastutil",
            "tileentity",
            "events",
            "common",
            "blockentity",
            "client",
            "entity",
            "mojang",
            "main",
            "gui",
            "world",
            "server",
            "dedicated",
            "map",
            "dsi",
            "renderer",
            "chunk",
            "model",
            "loading",
            "color",
            "pipeline",
            "inventory",
            "launcher",
            "physics",
            "particle",
            "gen",
            "registry",
            "worldgen",
            "texture",
            "biomes",
            "biome",
            "monster",
            "passive",
            "ai",
            "integrated",
            "tile",
            "state",
            "play",
            "override",
            "transformers",
            "structure",
            "nbt",
            "pathfinding",
            "audio",
            "entities",
            "items",
            "renderers",
            "storage",
            "universal",
            "oshi",
            "platform",
            // Java
            "java",
            "lang",
            "util",
            "nio",
            "io",
            "sun",
            "reflect",
            "zip",
            "jar",
            "jdk",
            "nashorn",
            "scripts",
            "runtime",
            "internal",
            // The parts of the names of the mods
            "mods",
            "mod",
            "impl",
            "org",
            "com",
            "cn",
            "cc",
            "jp",
            "core",
            "config",
            "registries",
            "lib",
            "ruby",
            "mc",
            "codec",
            "recipe",
            "channel",
            "embedded",
            "done",
            "netty",
            "network",
            "load",
            "github",
            "handler",
            "content",
            "feature",
            "file",
            "machine",
            "shader",
            "general",
            "helper",
            "init",
            "library",
            "api",
            "integration",
            "engine",
            "preload",
            "preinit",
            "hellominecraft",
            "jackhuang",
            // Forge
            "fml",
            "minecraftforge",
            "forge",
            "cpw",
            "modlauncher",
            "launchwrapper",
            "objectweb",
            "asm",
            "event",
            "eventhandler",
            "handshake",
            "modapi",
            "kcauldron",
            // Fabric
            "fabricmc",
            "loader",
            "game",
            "knot",
            "launch",
            "mixin",
        ]
        .into_iter()
        .collect()
    })
}

/// Returns the names of the packages of a crash report that may name a mod.
///
/// The names of the packages of the game, of Java and of the mod loaders do not
/// name a mod; the launcher offers the other names to the player, so the player
/// may find the mod that crashed the game.
pub fn find_keywords_from_crash_report(crash_report: &str) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    let Some(captures) = crash_report_stack().captures(crash_report) else {
        return result;
    };
    let stacktrace = captures
        .name("stacktrace")
        .expect("the pattern of the stack of a crash report reads the stack")
        .as_str();

    for line in stacktrace.split('\n') {
        let Some(line_captures) = stack_line().captures(line) else {
            continue;
        };
        let method = line_captures
            .name("method")
            .expect("the pattern of a line of the stack reads the method")
            .as_str();
        let parts: Vec<&str> = method.split('.').collect();
        for part in parts.iter().take(parts.len().saturating_sub(2)) {
            if !package_blacklist().contains(part) {
                result.insert((*part).to_string());
            }
        }

        let Some(modules) = stack_modules().captures(line) else {
            continue;
        };
        let modules = modules
            .name("tokens")
            .expect("the pattern of the modules reads the modules")
            .as_str();
        for module in modules.split(',') {
            let parts: Vec<&str> = module.split(':').collect();
            if parts.len() >= 2 && parts[0] == "xf" && !package_blacklist().contains(parts[1]) {
                result.insert(parts[1].to_string());
            }
        }
    }

    result
}

/// Returns the version of Java a class file version belongs to.
///
/// The class files of Java 1.1 have the version 45; the class files of the
/// versions since Java 1.2 have the version of the Java release plus 44, so the
/// launcher reads the version of Java the game was compiled by.
pub fn java_version_from_major_version(major_version: i32) -> i32 {
    if major_version >= 46 {
        major_version - 44
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reads a log the Java version of the launcher tests with.
    fn log(path: &str) -> Option<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../HMCLCore/src/test/resources")
            .join(path);
        std::fs::read_to_string(path).ok()
    }

    /// Returns the result of a rule, if the launcher read the rule.
    fn find_result(results: &[CrashRuleMatch], rule: Rule) -> Option<&CrashRuleMatch> {
        results.iter().find(|result| result.rule == rule)
    }

    /// Reads a log and returns the result of a rule.
    fn analyze_rule(path: &str, rule: Rule) -> Option<CrashRuleMatch> {
        let log = log(path)?;
        let results = analyze(&log);
        find_result(&results, rule).cloned()
    }

    #[test]
    fn the_rules_are_read() {
        let log =
            analyze_rule("logs/java9.txt", Rule::Jdk9).expect("the log of Java 9 should be read");
        assert_eq!(log.rule, Rule::Jdk9);

        assert!(analyze_rule(
            "crash-report/rtss_forest_sodium.txt",
            Rule::RtssForestSodium
        )
        .is_some());
        assert!(analyze_rule("logs/jvm_32bit.txt", Rule::Jvm32Bit).is_some());
        assert!(analyze_rule("logs/jvm_32bit2.txt", Rule::Jvm32Bit).is_some());
        assert!(analyze_rule("crash-report/graphics_driver.txt", Rule::GraphicsDriver).is_some());
        assert!(analyze_rule("logs/graphics_driver.txt", Rule::GraphicsDriver).is_some());
        assert!(analyze_rule("crash-report/splashscreen.txt", Rule::GraphicsDriver).is_some());
        assert!(analyze_rule("logs/openj9.txt", Rule::OpenJ9).is_some());
        assert!(analyze_rule("logs/mod_name.txt", Rule::ModName).is_some());
        assert!(analyze_rule(
            "logs/macos_failed_to_find_service_port_for_display.txt",
            Rule::MacosFailedToFindServicePortForDisplay
        )
        .is_some());
    }

    #[test]
    fn the_versions_of_java_are_read() {
        let result = analyze_rule("logs/too_old_java.txt", Rule::TooOldJava)
            .expect("the log should be read");
        assert_eq!(result.group("expected"), Some("60"));

        let result = analyze_rule("crash-report/too_old_java.txt", Rule::TooOldJava)
            .expect("the log should be read");
        assert_eq!(result.group("expected"), Some("52"));

        let result = analyze_rule("crash-report/too_old_java2.txt", Rule::TooOldJava)
            .expect("the log should be read");
        assert_eq!(result.group("expected"), Some("52"));

        assert!(analyze_rule(
            "logs/java_version_is_too_high.txt",
            Rule::JavaVersionIsTooHigh
        )
        .is_some());
        assert!(analyze_rule("crash-report/need_jdk11.txt", Rule::NeedJdk11).is_some());
        assert!(analyze_rule("crash-report/need_jdk112.txt", Rule::NeedJdk11).is_some());
        assert!(analyze_rule("crash-report/need_jdk113.txt", Rule::NeedJdk11).is_some());
    }

    #[test]
    fn the_files_of_the_game_are_read() {
        let result = analyze_rule("crash-report/security.txt", Rule::FileChanged)
            .expect("the log should be read");
        assert_eq!(
            result.group("file"),
            Some("assets/minecraft/texts/splashes.txt")
        );

        assert!(analyze_rule(
            "crash-report/no_class_def_found_error.txt",
            Rule::NoClassDefFoundError
        )
        .is_some());
        assert!(analyze_rule(
            "crash-report/no_class_def_found_error2.txt",
            Rule::NoClassDefFoundError
        )
        .is_some());
        assert!(analyze_rule(
            "crash-report/file_already_exists.txt",
            Rule::FileAlreadyExists
        )
        .is_some());
    }

    #[test]
    fn the_mods_of_the_game_are_read() {
        let result = analyze_rule("logs/mod_resolution.txt", Rule::ModResolution)
            .expect("the log should be read");
        assert!(result
            .group("reason")
            .is_some_and(|reason| reason.contains("Mod test depends on mod")));

        assert!(analyze_rule("logs/forgemod_resolution.txt", Rule::ForgeModResolution).is_some());
        assert!(analyze_rule(
            "logs/forge_found_duplicate_mods.txt",
            Rule::ForgeFoundDuplicateMods
        )
        .is_some());

        let result = analyze_rule(
            "logs/mod_resolution_collection.txt",
            Rule::ModResolutionCollection,
        )
        .expect("the log should be read");
        assert_eq!(result.group("sourcemod"), Some("tabtps-fabric"));
        assert_eq!(result.group("destmod"), Some("{fabricloader @ [>=0.11.1]}"));

        let result = analyze_rule("logs/fabric-mod-conflict.txt", Rule::ModResolutionConflict)
            .expect("the log should be read");
        assert_eq!(result.group("sourcemod"), Some("phosphor"));
        assert_eq!(result.group("destmod"), Some("{starlight @ [*]}"));

        let result = analyze_rule("logs/fabric-mod-missing.txt", Rule::ModResolutionMissing)
            .expect("the log should be read");
        assert_eq!(result.group("sourcemod"), Some("pca"));
        assert_eq!(result.group("destmod"), Some("{fabric @ [>=0.39.2]}"));

        let result = analyze_rule(
            "logs/fabric-minecraft.txt",
            Rule::ModResolutionMissingMinecraft,
        )
        .expect("the log should be read");
        assert_eq!(result.group("mod"), Some("fabric"));
        assert_eq!(result.group("version"), Some("[~1.16.2-alpha.20.28.a]"));

        assert!(analyze_rule("logs/fabric-version-0.12.txt", Rule::FabricVersion0_12).is_some());
        assert!(analyze_rule("crash-report/mod_resolution0.txt", Rule::ModResolution0).is_some());
        assert!(analyze_rule(
            "logs/install_mixinbootstrap.txt",
            Rule::InstallMixinbootstrap
        )
        .is_some());
        assert!(analyze_rule("logs/shaders_mod.txt", Rule::ShadersMod).is_some());
        assert!(analyze_rule(
            "logs/forge_repeat_installation.txt",
            Rule::ForgeRepeatInstallation
        )
        .is_some());
        assert!(analyze_rule(
            "logs/forge_repeat_installation2.txt",
            Rule::ForgeRepeatInstallation
        )
        .is_some());
        assert!(analyze_rule(
            "logs/optifine_repeat_installation.txt",
            Rule::OptifineRepeatInstallation
        )
        .is_some());
    }

    #[test]
    fn the_crash_of_a_mod_is_read() {
        let result = analyze_rule(
            "crash-report/loader_exception_mod_crash.txt",
            Rule::LoadingCrashedForge,
        )
        .expect("the log should be read");
        assert_eq!(result.group("name"), Some("Better PvP"));
        assert_eq!(result.group("id"), Some("xaerobetterpvp"));

        let result = analyze_rule(
            "crash-report/loading_error_fabric.txt",
            Rule::LoadingCrashedFabric,
        )
        .expect("the log should be read");
        assert_eq!(result.group("id"), Some("test"));

        let result = analyze_rule("logs/bootstrap.txt", Rule::BootstrapFailed)
            .expect("the log should be read");
        assert_eq!(result.group("id"), Some("prefab"));

        let result = analyze_rule("logs/mixin_apply_mod_failed.txt", Rule::MixinApplyModFailed)
            .expect("the log should be read");
        assert_eq!(result.group("id"), Some("enhancedblockentities"));

        let result = analyze_rule(
            "logs/unsatisfied_link_error.txt",
            Rule::UnsatisfiedLinkError,
        )
        .expect("the log should be read");
        assert_eq!(result.group("name"), Some("lwjgl.dll"));

        assert!(analyze_rule(
            "crash-report/mod/twilightforest_optifine_incompatibility.txt",
            Rule::TwilightForestOptifine
        )
        .is_some());
        assert!(analyze_rule(
            "crash-report/mod/performant_optifine_incompatibility.txt",
            Rule::PerformantForestOptifine
        )
        .is_some());
        assert!(analyze_rule(
            "crash-report/mod/neoforgeforest_optifine_incompatibility.txt",
            Rule::NeoforgeForestOptifine
        )
        .is_some());
        assert!(analyze_rule(
            "logs/optifine_is_not_compatible_with_forge.txt",
            Rule::OptifineIsNotCompatibleWithForge
        )
        .is_some());
    }

    #[test]
    fn the_configuration_of_a_mod_is_read() {
        let result =
            analyze_rule("crash-report/config.txt", Rule::Config).expect("the log should be read");
        assert_eq!(result.group("id"), Some("jumbofurnace"));
        assert_eq!(result.group("file"), Some("jumbofurnace-server.toml"));

        assert!(analyze_rule(
            "crash-report/night_config_fixes.txt",
            Rule::NightConfigFixes
        )
        .is_some());
    }

    #[test]
    fn the_warnings_of_fabric_are_read() {
        let result = analyze_rule("logs/fabric_warnings.txt", Rule::FabricWarnings)
            .expect("the log should be read");
        assert!(result
            .group("reason")
            .is_some_and(|reason| reason.contains("Conflicting versions found")));

        assert!(analyze_rule("logs/fabric_warnings2.txt", Rule::FabricWarnings).is_some());
        assert!(analyze_rule("logs/fabric_warnings3.txt", Rule::FabricWarnings).is_some());
    }

    #[test]
    fn the_memory_errors_are_read() {
        assert!(analyze_rule("crash-report/out_of_memory.txt", Rule::OutOfMemory).is_some());
        assert!(analyze_rule("logs/out_of_memory.txt", Rule::OutOfMemory).is_some());
        assert!(analyze_rule("logs/out_of_memory2.txt", Rule::OutOfMemory).is_some());
        assert!(analyze_rule("logs/memory_exceeded.txt", Rule::MemoryExceeded).is_some());
    }

    #[test]
    fn the_forge_error_is_read() {
        let result = analyze_rule("crash-report/forge_error.txt", Rule::ForgeError)
            .expect("the log should be read");
        assert!(result
            .group("reason")
            .is_some_and(|reason| reason.contains("MissingModsException")));
    }

    #[test]
    fn the_entity_and_the_block_of_a_crash_are_read() {
        let result = analyze_rule("crash-report/mod/customnpc.txt", Rule::Entity)
            .expect("the log should be read");
        assert_eq!(
            result.group("type"),
            Some("customnpcs.CustomNpc (noppes.npcs.entity.EntityCustomNpc)")
        );
        assert_eq!(result.group("location"), Some("99942.59, 4.00, 100000.98"));

        let result = analyze_rule("crash-report/mod/tconstruct.txt", Rule::Block)
            .expect("the log should be read");
        assert_eq!(
            result.group("type"),
            Some("Block{tconstruct:seared_drain}[active=true,facing=north]")
        );
        assert!(result
            .group("location")
            .is_some_and(|location| location.contains("World: (1370,92,-738)")));
    }

    #[test]
    fn the_keywords_of_a_crash_report_are_read() {
        let report = log("crash-report/mod/customnpc.txt").expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from(["npcs".to_string(), "noppes".to_string()])
        );

        let report = log("crash-report/mod/tconstruct.txt").expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from([
                "tconstruct".to_string(),
                "slimeknights".to_string(),
                "smeltery".to_string()
            ])
        );

        let report = log("crash-report/mod/bettersprinting.txt").expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from(["chylex".to_string(), "bettersprinting".to_string()])
        );

        let report = log("crash-report/mod/ic2.txt").expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from(["ic2".to_string()])
        );

        let report = log("crash-report/mod/nei.txt").expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from([
                "nei".to_string(),
                "codechicken".to_string(),
                "guihook".to_string()
            ])
        );

        let report = log("crash-report/mod/twilightforest_optifine_incompatibility.txt")
            .expect("the log should be read");
        assert_eq!(
            find_keywords_from_crash_report(&report),
            BTreeSet::from(["OptiFine".to_string()])
        );
    }

    #[test]
    fn the_crash_report_is_extracted() {
        let log = "before\n---- Minecraft Crash Report ----\nDescription: test\n#@!@# Game crashed! Crash report saved to: #@!@# file.txt\nafter";

        assert_eq!(
            extract_crash_report(log),
            Some("---- Minecraft Crash Report ----\nDescription: test\n")
        );
        assert_eq!(extract_crash_report("no report"), None);
    }

    #[test]
    fn the_place_of_a_crash_report_is_read() {
        let directory =
            std::env::temp_dir().join(format!("hmcl-rs-crash-report-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the directory should be made");
        let report = directory.join("crash.txt");
        std::fs::write(&report, "the report").expect("the report should be written");

        let log = format!(
            "#@!@# Game crashed! Crash report saved to: #@!@# {}",
            report.display()
        );
        assert_eq!(
            find_crash_report(&log).expect("the report should be read"),
            Some("the report".to_string())
        );
        assert_eq!(
            find_crash_report("no report").expect("the log should be read"),
            None
        );

        let _ = std::fs::remove_dir_all(&directory);
    }

    #[test]
    fn the_version_of_java_of_a_class_file_is_read() {
        assert_eq!(java_version_from_major_version(45), -1);
        assert_eq!(java_version_from_major_version(46), 2);
        assert_eq!(java_version_from_major_version(52), 8);
        assert_eq!(java_version_from_major_version(61), 17);
        assert_eq!(java_version_from_major_version(65), 21);
        assert_eq!(java_version_from_major_version(44), -1);
    }

    #[test]
    fn a_normal_log_holds_no_message() {
        assert!(analyze("The game started and stopped").is_empty());
    }
}
