using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Diagnostics;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Game;
using HMCLCore.NET.Java;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Launch;

/// <summary>
/// Executes the Minecraft game launch process.
/// </summary>
public sealed class GameLauncher
{
    /// <summary>
    /// Launches Minecraft with the specified configuration.
    /// </summary>
    /// <param name="config">The configuration of the launch.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The process of the game.</returns>
    /// <exception cref="GameLaunchException">
    /// Thrown when the game process cannot be started, for example when the
    /// launcher has no permission to run the Java runtime.
    /// </exception>
    public async Task<ManagedProcess> LaunchAsync(LaunchConfig config, CancellationToken token = default)
    {
        ArgumentNullException.ThrowIfNull(config);

        if (config.JavaRuntime == null)
            throw new InvalidOperationException("No Java runtime specified");

        if (!File.Exists(config.JavaRuntime.Path))
            throw new FileNotFoundException("Java executable not found", config.JavaRuntime.Path);

        // The command executed before the game is started.
        await RunCommandAsync(config.PreCommand, config.WorkingDirectory, token);

        var startInfo = config.CreateProcessStartInfo();

        // Redirect output streams to capture game logs
        startInfo.RedirectStandardOutput = true;
        startInfo.RedirectStandardError = true;

        ManagedProcess process;
        try
        {
            process = ManagedProcess.Start(startInfo);
        }
        catch (UnauthorizedAccessException ex)
        {
            throw new GameLaunchException(
                $"没有权限启动游戏进程，请检查 {config.JavaRuntime.Path} 的访问权限。", ex);
        }
        catch (Win32Exception ex)
        {
            throw new GameLaunchException($"无法启动游戏进程: {ex.Message}", ex);
        }

        // Apply process priority if configured
        if (config.ProcessPriority is { } priority)
        {
            try { process.Process.PriorityClass = ToProcessPriorityClass(priority); }
            catch (InvalidOperationException) { /* Process already exited */ }
            catch (Win32Exception) { /* The launcher is not allowed to change the priority */ }
        }

        // Read output asynchronously
        _ = Task.Run(async () =>
        {
            try
            {
                await process.ReadOutputAsync(token);
            }
            catch (OperationCanceledException)
            {
                // Expected on cancellation
            }
            catch (IOException)
            {
                // The pipe was closed while the process was running.
            }
        }, token);

        // The command executed after the game exited.
        if (!string.IsNullOrWhiteSpace(config.PostCommand))
        {
            process.Exited += () => _ = RunCommandAsync(config.PostCommand, config.WorkingDirectory, CancellationToken.None);
        }

        return process;
    }

    /// <summary>
    /// Runs a shell command and waits for its completion.
    /// </summary>
    /// <param name="command">The command to run; nothing happens when it is empty.</param>
    /// <param name="workingDirectory">The working directory of the command.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the command has finished.</returns>
    private static async Task RunCommandAsync(string? command, string workingDirectory, CancellationToken token)
    {
        if (string.IsNullOrWhiteSpace(command))
            return;

        var startInfo = CreateShellStartInfo(command, workingDirectory);

        try
        {
            using var process = Process.Start(startInfo);
            if (process == null)
                return;

            await process.WaitForExitAsync(token);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException or IOException)
        {
            throw new GameLaunchException($"无法执行命令: {command}", ex);
        }
    }

    /// <summary>
    /// Creates the start info running a command with the shell of the system.
    /// </summary>
    /// <param name="command">The command to run.</param>
    /// <param name="workingDirectory">The working directory of the command.</param>
    /// <returns>The start info of the command.</returns>
    private static ProcessStartInfo CreateShellStartInfo(string command, string workingDirectory)
    {
        var startInfo = new ProcessStartInfo
        {
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        if (!string.IsNullOrEmpty(workingDirectory) && Directory.Exists(workingDirectory))
            startInfo.WorkingDirectory = workingDirectory;

        if (Platform.CurrentOs == OsType.Windows)
        {
            startInfo.FileName = "cmd.exe";
            startInfo.ArgumentList.Add("/c");
            startInfo.ArgumentList.Add(command);
        }
        else
        {
            startInfo.FileName = "/bin/sh";
            startInfo.ArgumentList.Add("-c");
            startInfo.ArgumentList.Add(command);
        }

        return startInfo;
    }

    /// <summary>
    /// Builds the complete launch command line for Minecraft from the aggregated arguments.
    /// Memory, custom JVM and server options are provided through <see cref="LaunchConfig.JvmArguments"/>
    /// and <see cref="LaunchConfig.GameArguments"/> by the launch argument generator.
    /// </summary>
    public string BuildLaunchCommand(LaunchConfig config)
    {
        var builder = new CommandBuilder();

        // Java executable
        builder.AppendQuoted(config.JavaRuntime?.Path ?? "java");

        // Standard JVM arguments (memory, custom args, manifest args)
        foreach (var arg in config.JvmArguments)
            builder.Append(arg);

        // Classpath
        builder.Append("-cp");
        builder.AppendQuoted(config.ClassPath);

        // Main class
        builder.Append(config.MainClass);

        // Game arguments
        foreach (var arg in config.GameArguments)
            builder.Append(arg);

        return builder.ToString();
    }

    /// <summary>
    /// Returns whether the specified Java runtime is compatible with the game version.
    /// </summary>
    public bool IsJavaCompatible(JavaRuntime javaRuntime, string minecraftVersion)
    {
        return javaRuntime.IsCompatibleWithMinecraft(minecraftVersion);
    }

    /// <summary>
    /// Converts the launcher's game-settings process priority enum to
    /// <see cref="System.Diagnostics.ProcessPriorityClass"/>.
    /// </summary>
    private static System.Diagnostics.ProcessPriorityClass ToProcessPriorityClass(
        HMCLCore.NET.Setting.ProcessPriority priority) => priority switch
    {
        HMCLCore.NET.Setting.ProcessPriority.High =>
            System.Diagnostics.ProcessPriorityClass.High,
        HMCLCore.NET.Setting.ProcessPriority.AboveNormal =>
            System.Diagnostics.ProcessPriorityClass.AboveNormal,
        HMCLCore.NET.Setting.ProcessPriority.Normal =>
            System.Diagnostics.ProcessPriorityClass.Normal,
        HMCLCore.NET.Setting.ProcessPriority.BelowNormal =>
            System.Diagnostics.ProcessPriorityClass.BelowNormal,
        HMCLCore.NET.Setting.ProcessPriority.Low =>
            System.Diagnostics.ProcessPriorityClass.BelowNormal,
        _ => System.Diagnostics.ProcessPriorityClass.Normal,
    };
}
