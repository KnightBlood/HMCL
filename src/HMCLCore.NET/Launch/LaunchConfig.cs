using System;
using System.Diagnostics;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using HMCLCore.NET.Auth;
using HMCLCore.NET.Game;
using HMCLCore.NET.Java;
using HMCLCore.NET.Setting;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Launch;

/// <summary>
/// Represents the configuration for launching Minecraft.
/// </summary>
public sealed class LaunchConfig
{
    /// <summary>
    /// The game instance to launch.
    /// </summary>
    public GameInstance Instance { get; init; } = new();

    /// <summary>
    /// The account to use for launching.
    /// </summary>
    public Account Account { get; init; } = new OfflineAccount("Player");

    /// <summary>
    /// The Java runtime to use.
    /// </summary>
    public JavaRuntime? JavaRuntime { get; set; }

    /// <summary>
    /// The JVM arguments.
    /// </summary>
    public List<string> JvmArguments { get; set; } = new();

    /// <summary>
    /// The game arguments.
    /// </summary>
    public List<string> GameArguments { get; set; } = new();

    /// <summary>
    /// Effective game settings (preset merged with instance overrides).
    /// </summary>
    public GameSettingsEffective? GameSettings { get; set; }

    /// <summary>
    /// The memory allocation in MB.
    /// </summary>
    public int Memory { get; set; } = 2048;

    /// <summary>
    /// The working directory for the game.
    /// </summary>
    public string WorkingDirectory { get; init; } = "";

    /// <summary>
    /// Wrapper command executed before the game (optional).
    /// </summary>
    public string? Wrapper { get; set; }

    /// <summary>
    /// Pre-launch command executed before the game starts (optional).
    /// </summary>
    public string? PreCommand { get; set; }

    /// <summary>
    /// Post-launch command executed after the game starts (optional).
    /// </summary>
    public string? PostCommand { get; set; }

    /// <summary>
    /// Running directory override for the game process (optional).
    /// </summary>
    public string? RunningDirectory { get; set; }

    /// <summary>
    /// The game directory.
    /// </summary>
    public string GameDirectory { get; init; } = "";

    /// <summary>
    /// The libraries directory.
    /// </summary>
    public string LibrariesDirectory { get; init; } = "";

    /// <summary>
    /// The natives directory.
    /// </summary>
    public string NativesDirectory { get; set; } = "";

    /// <summary>
    /// The size of the permanent generation area of the Java virtual machine,
    /// in MiB; it is only used by the Java 8 and older runtimes.
    /// </summary>
    public int? PermSize { get; set; }

    /// <summary>
    /// Path of the authlib-injector agent JAR; set when launching with a
    /// third-party Yggdrasil account that requires session redirection.
    /// </summary>
    public string? AuthlibInjectorJarPath { get; set; }

    /// <summary>
    /// The third-party Yggdrasil server URL handed to authlib-injector.
    /// </summary>
    public string? AuthlibInjectorServerUrl { get; set; }

    /// <summary>
    /// Base64-encoded root endpoint response of the authlib-injector server,
    /// used to skip the game's initial server metadata request when launching.
    /// </summary>
    public string? AuthlibInjectorPrefetchData { get; set; }

    /// <summary>
    /// The classpath.
    /// </summary>
    public string ClassPath { get; set; } = "";

    /// <summary>
    /// The main class.
    /// </summary>
    public string MainClass { get; set; } = "net.minecraft.client.main.Main";

    /// <summary>
    /// The version name.
    /// </summary>
    public string VersionName { get; init; } = "";

    /// <summary>
    /// The asset index.
    /// </summary>
    public string? AssetIndex { get; set; }

    /// <summary>
    /// The version type (e.g., "release", "snapshot").
    /// </summary>
    public string VersionType { get; set; } = "release";

    /// <summary>
    /// The assets directory.
    /// </summary>
    public string AssetsDirectory { get; init; } = "";

    /// <summary>
    /// The path of the game JAR of the launched version, used for the
    /// <c>${primary_jar}</c> placeholder of the manifest arguments.
    /// </summary>
    public string? PrimaryJarPath { get; set; }

    /// <summary>
    /// Whether to use the custom JVM arguments from settings.
    /// </summary>
    public bool UseCustomJvmArgs { get; set; } = false;

    /// <summary>
    /// Custom JVM arguments from settings.
    /// </summary>
    public string? CustomJvmArgs { get; set; }

    /// <summary>
    /// Window width.
    /// </summary>
    public int WindowWidth { get; set; } = 854;

    /// <summary>
    /// Window height.
    /// </summary>
    public int WindowHeight { get; set; } = 480;

    /// <summary>
    /// Server address.
    /// </summary>
    public string? ServerAddress { get; set; }

    /// <summary>
    /// Server port.
    /// </summary>
    public int ServerPort { get; set; } = 25565;

    /// <summary>
    /// Process priority for the launched game process.
    /// </summary>
    public HMCLCore.NET.Setting.ProcessPriority? ProcessPriority { get; set; }

    /// <summary>
    /// Quick play mode controlling how the game enters a session directly.
    /// </summary>
    public QuickPlayMode QuickPlayMode { get; set; } = QuickPlayMode.None;

    /// <summary>
    /// Quick play argument: a save folder name for single player, a server
    /// address for multiplayer, or a Realms world id for Realms.
    /// </summary>
    public string? QuickPlayArgument { get; set; }

    /// <summary>
    /// Custom environment variables applied to the game process.
    /// </summary>
    public Dictionary<string, string> EnvironmentVariables { get; set; } = new(StringComparer.Ordinal);

    /// <summary>
    /// Builds the full command line for launching from the aggregated arguments.
    /// Memory and custom JVM options are provided through <see cref="JvmArguments"/>
    /// by the launch argument generator, so they are not added here.
    /// </summary>
    public string BuildCommandLine()
    {
        var args = new List<string>();

        // Java executable
        args.Add(JavaRuntime?.Path ?? "java");

        // JVM arguments (memory, custom args, manifest args)
        args.AddRange(JvmArguments);

        // Classpath, unless the JVM arguments already contain it.
        if (!JvmArguments.Contains("-cp"))
        {
            args.Add("-cp");
            args.Add(ClassPath);
        }

        // Main class
        args.Add(MainClass);

        // Game arguments
        args.AddRange(GameArguments);

        return string.Join(" ", args);
    }

    /// <summary>
    /// Creates a ProcessStartInfo for launching the game. Individual arguments are
    /// passed through <see cref="ProcessStartInfo.ArgumentList"/> so paths containing
    /// spaces (e.g. the classpath) are preserved without shell quoting.
    /// </summary>
    public ProcessStartInfo CreateProcessStartInfo()
    {
        if (JavaRuntime == null)
            throw new InvalidOperationException("No Java runtime specified");

        var psi = new ProcessStartInfo
        {
            FileName = JavaRuntime.Path,
            WorkingDirectory = string.IsNullOrEmpty(RunningDirectory) ? WorkingDirectory : RunningDirectory,
            UseShellExecute = false,
            CreateNoWindow = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        // The arguments of the game process: the JVM arguments, the classpath,
        // the main class and the game arguments.
        var arguments = new List<string>();

        // JVM arguments (memory, custom args, manifest args)
        arguments.AddRange(JvmArguments);

        // Classpath, unless the JVM arguments already contain it.
        if (!JvmArguments.Contains("-cp"))
        {
            arguments.Add("-cp");
            arguments.Add(ClassPath);
        }

        // Main class
        arguments.Add(MainClass);

        // Game arguments
        arguments.AddRange(GameArguments);

        // The wrapper wraps the game process: it is started with the command of
        // the game as its arguments.
        var wrapper = CommandBuilder.Parse(Wrapper);
        if (wrapper.Count > 0)
        {
            psi.FileName = wrapper[0];

            for (var i = 1; i < wrapper.Count; i++)
                psi.ArgumentList.Add(wrapper[i]);

            psi.ArgumentList.Add(JavaRuntime.Path);
        }

        foreach (var argument in arguments)
            psi.ArgumentList.Add(argument);

        // Custom environment variables
        foreach (var kvp in EnvironmentVariables)
        {
            if (string.IsNullOrWhiteSpace(kvp.Key))
                continue;

            try
            {
                psi.Environment[kvp.Key] = kvp.Value ?? string.Empty;
            }
            catch (ArgumentException)
            {
                // Ignore invalid environment variable names.
            }
        }

        return psi;
    }
}