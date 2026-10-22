using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Game;
using HMCLCore.NET.Java;
using HMCLCore.NET.Launch;
using HMCLCore.NET.Tasks;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Launch;

/// <summary>
/// Executor for launching Minecraft game instances.
/// Ported from Java GameLauncher / GameInstance launcher.
/// </summary>
public sealed class GameLauncherExecutor
{
    private readonly GameLauncher _launcher;
    private readonly JavaManager _javaManager;
    private readonly string _workingDirectory;

    /// <summary>
    /// Creates a new GameLauncherExecutor.
    /// </summary>
    public GameLauncherExecutor(GameLauncher launcher, JavaManager javaManager, string workingDirectory)
    {
        _launcher = launcher ?? throw new ArgumentNullException(nameof(launcher));
        _javaManager = javaManager ?? throw new ArgumentNullException(nameof(javaManager));
        _workingDirectory = workingDirectory ?? throw new ArgumentNullException(nameof(workingDirectory));
    }

    /// <summary>
    /// Launches a Minecraft game instance with the specified configuration.
    /// </summary>
    public async Task<ManagedProcess> LaunchAsync(LaunchConfig config, CancellationToken token = default)
    {
        if (config.JavaRuntime == null)
            throw new InvalidOperationException("No Java runtime specified");

        if (!File.Exists(config.JavaRuntime.Path))
            throw new FileNotFoundException("Java executable not found", config.JavaRuntime.Path);

        var process = await _launcher.LaunchAsync(config, token);
        return process;
    }

    /// <summary>
    /// Builds the command line used to start the game process.
    /// </summary>
    /// <param name="config">The configuration of the launch.</param>
    /// <returns>The command line of the game process.</returns>
    public string BuildLaunchCommand(LaunchConfig config) => _launcher.BuildLaunchCommand(config);

    /// <summary>
    /// Builds a LaunchConfig from a game instance and account.
    /// </summary>
    public LaunchConfig BuildLaunchConfig(GameInstance instance, string accountUsername, string librariesDirectory)
    {
        return new LaunchConfig
        {
            Instance = instance,
            JavaRuntime = _javaManager.PreferredRuntime,
            WorkingDirectory = instance.GameDirectory,
            GameDirectory = instance.GameDirectory,
            LibrariesDirectory = librariesDirectory,
            ClassPath = BuildClasspath(instance, librariesDirectory),
            MainClass = "net.minecraft.client.main.Main",
            VersionName = instance.Version,
            Memory = 2048
        };
    }

    /// <summary>
    /// Builds the classpath string for a game instance. Libraries live under the
    /// libraries directory; the client JAR lives under the versions directory
    /// (sibling of the libraries directory in the repository layout).
    /// </summary>
    private string BuildClasspath(GameInstance instance, string librariesDirectory)
    {
        var classpath = new LibraryBuilder().BuildClasspath(librariesDirectory);

        // Add the game JAR itself from the shared versions directory.
        var versionsDirectory = Path.GetDirectoryName(librariesDirectory.TrimEnd(Path.DirectorySeparatorChar));
        if (!string.IsNullOrEmpty(versionsDirectory))
        {
            var gameJar = Path.Combine(versionsDirectory, "versions", instance.Version, $"{instance.Version}.jar");
            if (File.Exists(gameJar))
            {
                classpath = string.IsNullOrEmpty(classpath)
                    ? gameJar
                    : classpath + Path.PathSeparator + gameJar;
            }
        }

        return classpath;
    }
}

