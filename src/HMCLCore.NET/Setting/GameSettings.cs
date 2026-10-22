using System;
using System.Collections.Generic;

namespace HMCLCore.NET.Setting;

/// <summary>
/// Game launch settings shared by presets and instance-specific overrides.
/// </summary>
public abstract class GameSettings : GameSettingsBase
{
    /// <summary>Property name for Java selection type.</summary>
    public const string PropertyJavaType = "javaType";

    /// <summary>Property name for custom Java version.</summary>
    public const string PropertyCustomJavaVersion = "customJavaVersion";

    /// <summary>Property name for custom Java path.</summary>
    public const string PropertyCustomJavaPath = "customJavaPath";

    /// <summary>Property name for detected Java.</summary>
    public const string PropertyDetectedJava = "detectedJava";

    /// <summary>Property name for not using javaw.</summary>
    public const string PropertyNotUseJavaw = "notUseJavaw";

    /// <summary>Property name for JVM options.</summary>
    public const string PropertyJvmOptions = "jvmOptions";

    /// <summary>Property name for disabling JVM options.</summary>
    public const string PropertyNoJvmOptions = "noJVMOptions";

    /// <summary>Property name for disabling optimizing JVM options.</summary>
    public const string PropertyNoOptimizingJvmOptions = "noOptimizingJvmOptions";

    /// <summary>Property name for skipping JVM check.</summary>
    public const string PropertyNotCheckJvm = "notCheckJVM";

    /// <summary>Property name for skipping game file check.</summary>
    public const string PropertyNotCheckGame = "notCheckGame";

    /// <summary>Property name for automatic memory allocation.</summary>
    public const string PropertyAutoMemory = "autoMemory";

    /// <summary>Property name for minimum memory.</summary>
    public const string PropertyMinMemory = "minMemory";

    /// <summary>Property name for maximum memory.</summary>
    public const string PropertyMaxMemory = "maxMemory";

    /// <summary>Property name for PermGen/Metaspace size.</summary>
    public const string PropertyPermSize = "permSize";

    /// <summary>Property name for game window width.</summary>
    public const string PropertyWidth = "width";

    /// <summary>Property name for game window height.</summary>
    public const string PropertyHeight = "height";

    /// <summary>Property name for fullscreen mode.</summary>
    public const string PropertyFullscreen = "fullscreen";

    /// <summary>Property name for custom run directory.</summary>
    public const string PropertyRunningDirectory = "runningDirectory";

    /// <summary>Property name for process priority.</summary>
    public const string PropertyProcessPriority = "processPriority";

    /// <summary>Property name for launcher visibility.</summary>
    public const string PropertyLauncherVisibility = "launcherVisibility";

    /// <summary>Property name for allowing Java agents.</summary>
    public const string PropertyAllowAutoAgent = "allowAutoAgent";

    /// <summary>Property name for disabling automatic game options.</summary>
    public const string PropertyDisableAutoGameOptions = "disableAutoGameOptions";

    /// <summary>Property name for wrapper command.</summary>
    public const string PropertyWrapper = "wrapper";

    /// <summary>Property name for pre-launch command.</summary>
    public const string PropertyPreCommand = "preCommand";

    /// <summary>Property name for post-launch command.</summary>
    public const string PropertyPostCommand = "postCommand";

    /// <summary>Property name for custom game arguments.</summary>
    public const string PropertyGameArguments = "gameArguments";

    /// <summary>Property name for graphics backend.</summary>
    public const string PropertyGraphicsBackend = "graphicsBackend";

    /// <summary>Property name for using custom natives directory.</summary>
    public const string PropertyUseCustomNatives = "useCustomNatives";

    /// <summary>Property name for custom natives directory.</summary>
    public const string PropertyNativesDirectory = "nativesDirectory";

    /// <summary>Property name for the quick play mode.</summary>
    public const string PropertyQuickPlayMode = "quickPlayMode";

    /// <summary>Property name for the quick play argument.</summary>
    public const string PropertyQuickPlayArgument = "quickPlayArgument";

    /// <summary>Property name for the custom environment variables.</summary>
    public const string PropertyEnvironmentVariables = "environmentVariables";

    /// <summary>Property name for whether the log window is shown.</summary>
    public const string PropertyShowLogs = "showLogs";

    /// <summary>Property name for whether the natives are patched.</summary>
    public const string PropertyNotPatchNatives = "notPatchNatives";

    /// <summary>Suggested maximum heap memory in MiB.</summary>
    public static readonly int SuggestedMemory = ComputeSuggestedMemory();

    private static int ComputeSuggestedMemory()
    {
        try
        {
            var totalMb = GC.GetGCMemoryInfo().TotalAvailableMemoryBytes / 1024.0 / 1024.0;
            if (totalMb >= 32768) return 8192;
            var suggested = (int)Math.Round(totalMb / 4.0 / 128.0) * 128;
            return suggested < 256 ? 256 : suggested;
        }
        catch
        {
            return 2048;
        }
    }

    /// <summary>How to select Java runtime.</summary>
    public JavaVersionType JavaType { get; set; } = JavaVersionType.Auto;

    /// <summary>Target Java major version when <see cref="JavaType"/> is <see cref="JavaVersionType.Version"/>.</summary>
    public string CustomJavaVersion { get; set; } = string.Empty;

    /// <summary>Custom Java executable path when <see cref="JavaType"/> is <see cref="JavaVersionType.Custom"/>.</summary>
    public string CustomJavaPath { get; set; } = string.Empty;

    /// <summary>Detected Java reference when <see cref="JavaType"/> is <see cref="JavaVersionType.Detected"/>.</summary>
    public DetectedJava DetectedJava { get; set; } = DetectedJava.Empty;

    /// <summary>Whether to not use javaw on Windows.</summary>
    public bool NotUseJavaw { get; set; }

    /// <summary>Additional JVM options.</summary>
    public string JvmOptions { get; set; } = string.Empty;

    /// <summary>Ignore JVM options from version JSON.</summary>
    public bool NoJvmOptions { get; set; }

    /// <summary>Ignore HMCL's optimizing JVM options.</summary>
    public bool NoOptimizingJvmOptions { get; set; }

    /// <summary>Skip JVM compatibility checks.</summary>
    public bool NotCheckJvm { get; set; }

    /// <summary>Skip game file checks before launch.</summary>
    public bool NotCheckGame { get; set; }

    /// <summary>Automatically allocate memory.</summary>
    public bool AutoMemory { get; set; } = true;

    /// <summary>Minimum memory in MiB (nullable = inherited/default).</summary>
    public int? MinMemory { get; set; }

    /// <summary>Maximum memory in MiB.</summary>
    public int? MaxMemory { get; set; }

    /// <summary>PermSize in MiB (Java 8 and below).</summary>
    public int? PermSize { get; set; }

    /// <summary>Game window width.</summary>
    public double Width { get; set; } = 854;

    /// <summary>Game window height.</summary>
    public double Height { get; set; } = 480;

    /// <summary>Start in fullscreen mode.</summary>
    public bool Fullscreen { get; set; }

    /// <summary>Custom game working directory.</summary>
    public string RunningDirectory { get; set; } = string.Empty;

    /// <summary>Wrapper command executed before the game.</summary>
    public string Wrapper { get; set; } = string.Empty;

    /// <summary>Pre-launch command executed before the game starts.</summary>
    public string PreCommand { get; set; } = string.Empty;

    /// <summary>Post-launch command executed after the game starts.</summary>
    public string PostCommand { get; set; } = string.Empty;

    /// <summary>Process priority for the game.</summary>
    public ProcessPriority ProcessPriority { get; set; } = ProcessPriority.Normal;

    /// <summary>Launcher visibility after game starts.</summary>
    public LauncherVisibility LauncherVisibility { get; set; } = LauncherVisibility.Hide;

    /// <summary>Allow HMCL to attach Java agents.</summary>
    public bool AllowAutoAgent { get; set; }

    /// <summary>Do not generate game options automatically.</summary>
    public bool DisableAutoGameOptions { get; set; }

    /// <summary>Custom game arguments.</summary>
    public string GameArguments { get; set; } = string.Empty;

    /// <summary>Graphics backend preference.</summary>
    public GraphicsBackend GraphicsBackend { get; set; } = GraphicsBackend.Auto;

    /// <summary>Use custom natives directory.</summary>
    public bool UseCustomNatives { get; set; }

    /// <summary>Custom natives directory.</summary>
    public string NativesDirectory { get; set; } = string.Empty;

    /// <summary>
    /// Quick play mode controlling how the game enters a session directly.
    /// </summary>
    public QuickPlayMode QuickPlayMode { get; set; } = QuickPlayMode.None;

    /// <summary>
    /// Quick play argument: a save folder name for single player, a server
    /// address for multiplayer, or a Realms world id for Realms.
    /// </summary>
    public string QuickPlayArgument { get; set; } = string.Empty;

    /// <summary>
    /// Custom environment variables applied to the game process.
    /// Keys are variable names without the platform-specific prefix.
    /// </summary>
    public Dictionary<string, string> EnvironmentVariables { get; set; } = new(StringComparer.Ordinal);

    /// <summary>
    /// Whether the window showing the log of the game is opened when the game
    /// starts.
    /// </summary>
    public bool ShowLogs { get; set; } = true;

    /// <summary>
    /// Whether the native libraries of the game are patched.
    /// </summary>
    public bool NotPatchNatives { get; set; }

    /// <summary>
    /// Marks a property as overridden on this settings object.
    /// </summary>
    public void Override(string propertyName)
    {
        MarkOverridden(propertyName);
    }

    /// <summary>
    /// Copies the values of the settings into another settings object; the
    /// environment variables are copied into a new dictionary so that the two
    /// settings do not share it.
    /// </summary>
    /// <param name="source">The settings the values are copied from.</param>
    /// <param name="target">The settings the values are copied into.</param>
    public static void CopyValues(GameSettings source, GameSettings target)
    {
        foreach (var property in typeof(GameSettings).GetProperties())
        {
            if (!property.CanRead || !property.CanWrite)
                continue;

            if (property.GetCustomAttributes(typeof(ObsoleteAttribute), false).Length > 0)
                continue;

            property.SetValue(target, property.GetValue(source));
        }

        target.EnvironmentVariables =
            new Dictionary<string, string>(source.EnvironmentVariables, StringComparer.Ordinal);
    }
}

/// <summary>
/// Global/game directory preset settings.
/// </summary>
public sealed class GameSettingsPreset : GameSettings
{
    /// <summary>
    /// The identifier of the preset, used by the instances that inherit from
    /// it.
    /// </summary>
    public string Id { get; set; } = Guid.NewGuid().ToString("N");

    /// <summary>
    /// The name of the preset shown to the user.
    /// </summary>
    public string Name { get; set; } = "默认";
}

/// <summary>
/// Instance-specific game settings with per-property overrides.
/// </summary>
public sealed class GameSettingsInstance : GameSettings
{
    /// <summary>
    /// The identifier of the preset the instance inherits from; an empty value
    /// means that the default preset is used.
    /// </summary>
    public string PresetId { get; set; } = string.Empty;
}

/// <summary>
/// Resolved effective game settings after merging preset and instance overrides.
/// </summary>
public sealed class GameSettingsEffective
{
    private readonly GameSettingsPreset _preset;
    private readonly GameSettingsInstance? _instance;

    /// <summary>
    /// Creates a new effective settings resolver.
    /// </summary>
    public GameSettingsEffective(GameSettingsPreset preset, GameSettingsInstance? instance)
    {
        _preset = preset ?? throw new ArgumentNullException(nameof(preset));
        _instance = instance;
    }

    private T Get<T>(Func<GameSettings, T> getter, string propertyName)
    {
        if (_instance != null && _instance.IsOverridden(propertyName))
            return getter(_instance);

        return getter(_preset);
    }

    /// <summary>Gets Java selection type.</summary>
    public JavaVersionType JavaType => Get(s => s.JavaType, GameSettings.PropertyJavaType);

    /// <summary>Gets custom Java version.</summary>
    public string CustomJavaVersion => Get(s => s.CustomJavaVersion, GameSettings.PropertyCustomJavaVersion);

    /// <summary>Gets custom Java path.</summary>
    public string CustomJavaPath => Get(s => s.CustomJavaPath, GameSettings.PropertyCustomJavaPath);

    /// <summary>Gets detected Java reference.</summary>
    public DetectedJava DetectedJava => Get(s => s.DetectedJava, GameSettings.PropertyDetectedJava);

    /// <summary>Gets whether to not use javaw.</summary>
    public bool NotUseJavaw => Get(s => s.NotUseJavaw, GameSettings.PropertyNotUseJavaw);

    /// <summary>Gets JVM options.</summary>
    public string JvmOptions => Get(s => s.JvmOptions, GameSettings.PropertyJvmOptions);

    /// <summary>Gets whether JVM options are disabled.</summary>
    public bool NoJvmOptions => Get(s => s.NoJvmOptions, GameSettings.PropertyNoJvmOptions);

    /// <summary>Gets whether optimizing JVM options are disabled.</summary>
    public bool NoOptimizingJvmOptions => Get(s => s.NoOptimizingJvmOptions, GameSettings.PropertyNoOptimizingJvmOptions);

    /// <summary>Gets whether JVM check is skipped.</summary>
    public bool NotCheckJvm => Get(s => s.NotCheckJvm, GameSettings.PropertyNotCheckJvm);

    /// <summary>Gets whether game check is skipped.</summary>
    public bool NotCheckGame => Get(s => s.NotCheckGame, GameSettings.PropertyNotCheckGame);

    /// <summary>Gets whether auto memory is enabled.</summary>
    public bool AutoMemory => Get(s => s.AutoMemory, GameSettings.PropertyAutoMemory);

    /// <summary>Gets minimum memory.</summary>
    public int? MinMemory => Get(s => s.MinMemory, GameSettings.PropertyMinMemory);

    /// <summary>Gets maximum memory.</summary>
    public int? MaxMemory => Get(s => s.MaxMemory, GameSettings.PropertyMaxMemory);

    /// <summary>Gets PermSize.</summary>
    public int? PermSize => Get(s => s.PermSize, GameSettings.PropertyPermSize);

    /// <summary>Gets window width.</summary>
    public double Width => Get(s => s.Width, GameSettings.PropertyWidth);

    /// <summary>Gets window height.</summary>
    public double Height => Get(s => s.Height, GameSettings.PropertyHeight);

    /// <summary>Gets whether fullscreen is enabled.</summary>
    public bool Fullscreen => Get(s => s.Fullscreen, GameSettings.PropertyFullscreen);

    /// <summary>Gets custom running directory.</summary>
    public string RunningDirectory => Get(s => s.RunningDirectory, GameSettings.PropertyRunningDirectory);

    /// <summary>Gets process priority.</summary>
    public ProcessPriority ProcessPriority => Get(s => s.ProcessPriority, GameSettings.PropertyProcessPriority);

    /// <summary>Gets launcher visibility.</summary>
    public LauncherVisibility LauncherVisibility => Get(s => s.LauncherVisibility, GameSettings.PropertyLauncherVisibility);

    /// <summary>Gets whether auto agents are allowed.</summary>
    public bool AllowAutoAgent => Get(s => s.AllowAutoAgent, GameSettings.PropertyAllowAutoAgent);

    /// <summary>Gets whether automatic game options are disabled.</summary>
    public bool DisableAutoGameOptions => Get(s => s.DisableAutoGameOptions, GameSettings.PropertyDisableAutoGameOptions);

    /// <summary>Gets custom game arguments.</summary>
    public string GameArguments => Get(s => s.GameArguments, GameSettings.PropertyGameArguments);

    /// <summary>Gets graphics backend.</summary>
    public GraphicsBackend GraphicsBackend => Get(s => s.GraphicsBackend, GameSettings.PropertyGraphicsBackend);

    /// <summary>Gets wrapper command.</summary>
    public string Wrapper => Get(s => s.Wrapper, GameSettings.PropertyWrapper);

    /// <summary>Gets pre-launch command.</summary>
    public string PreCommand => Get(s => s.PreCommand, GameSettings.PropertyPreCommand);

    /// <summary>Gets post-launch command.</summary>
    public string PostCommand => Get(s => s.PostCommand, GameSettings.PropertyPostCommand);

    /// <summary>Gets whether custom natives directory is used.</summary>
    public bool UseCustomNatives => Get(s => s.UseCustomNatives, GameSettings.PropertyUseCustomNatives);

    /// <summary>Gets custom natives directory.</summary>
    public string NativesDirectory => Get(s => s.NativesDirectory, GameSettings.PropertyNativesDirectory);

    /// <summary>Gets the quick play mode.</summary>
    public QuickPlayMode QuickPlayMode => Get(s => s.QuickPlayMode, GameSettings.PropertyQuickPlayMode);

    /// <summary>Gets the quick play argument.</summary>
    public string QuickPlayArgument => Get(s => s.QuickPlayArgument, GameSettings.PropertyQuickPlayArgument);

    /// <summary>Gets whether the log window is shown when the game starts.</summary>
    public bool ShowLogs => Get(s => s.ShowLogs, GameSettings.PropertyShowLogs);

    /// <summary>Gets whether the native libraries of the game are patched.</summary>
    public bool NotPatchNatives => Get(s => s.NotPatchNatives, GameSettings.PropertyNotPatchNatives);

    /// <summary>
    /// Gets the custom environment variables applied to the game process.
    /// </summary>
    public IReadOnlyDictionary<string, string> EnvironmentVariables =>
        Get(s => s.EnvironmentVariables, GameSettings.PropertyEnvironmentVariables);
}
