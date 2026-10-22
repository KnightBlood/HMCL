using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Launch;

/// <summary>
/// The result of the test of a launch.
/// </summary>
public sealed class LaunchTestResult
{
    /// <summary>
    /// Creates the result of the test.
    /// </summary>
    /// <param name="succeeded">Whether the game started correctly.</param>
    /// <param name="message">The message describing the result.</param>
    /// <param name="output">The output written by the game.</param>
    public LaunchTestResult(bool succeeded, string message, string output)
    {
        Succeeded = succeeded;
        Message = message;
        Output = output;
    }

    /// <summary>
    /// Whether the game started correctly.
    /// </summary>
    public bool Succeeded { get; }

    /// <summary>
    /// The message describing the result.
    /// </summary>
    public string Message { get; }

    /// <summary>
    /// The output written by the game.
    /// </summary>
    public string Output { get; }
}

/// <summary>
/// Generates the script starting the game, like the launch scripts of Java
/// HMCL, and tests that the game starts correctly.
/// </summary>
public static class LaunchScriptGenerator
{
    /// <summary>
    /// Returns the command line of the game process.
    /// </summary>
    /// <param name="config">The configuration of the launch.</param>
    /// <returns>The command line of the game.</returns>
    public static string BuildCommandLine(LaunchConfig config)
    {
        var arguments = new List<string>();
        arguments.Add(config.JavaRuntime?.Path ?? "java");
        arguments.AddRange(config.JvmArguments);

        if (!config.JvmArguments.Contains("-cp"))
        {
            arguments.Add("-cp");
            arguments.Add(config.ClassPath);
        }

        arguments.Add(config.MainClass);
        arguments.AddRange(config.GameArguments);

        var builder = new CommandBuilder();
        foreach (var argument in arguments)
            builder.AppendQuoted(argument);

        return builder.ToString();
    }

    /// <summary>
    /// Generates the script starting the game.
    /// </summary>
    /// <param name="config">The configuration of the launch.</param>
    /// <param name="scriptPath">The path of the script to write.</param>
    /// <returns>The content of the script.</returns>
    public static string WriteScript(LaunchConfig config, string scriptPath)
    {
        var commandLine = BuildCommandLine(config);
        var workingDirectory = string.IsNullOrEmpty(config.RunningDirectory)
            ? config.WorkingDirectory
            : config.RunningDirectory;

        string content;

        if (OperatingSystem.IsWindows())
        {
            content = new StringBuilder()
                .AppendLine("@echo off")
                .AppendLine($"cd /d \"{workingDirectory}\"")
                .AppendLine(commandLine)
                .AppendLine("pause")
                .ToString();
        }
        else
        {
            content = new StringBuilder()
                .AppendLine("#!/bin/sh")
                .AppendLine($"cd \"{workingDirectory}\" || exit 1")
                .AppendLine(commandLine)
                .ToString();
        }

        var directory = Path.GetDirectoryName(scriptPath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        File.WriteAllText(scriptPath, content, new UTF8Encoding(false));

        if (!OperatingSystem.IsWindows())
        {
            try
            {
                File.SetUnixFileMode(scriptPath,
                    UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute);
            }
            catch (Exception)
            {
                // The permissions of the script stay the default ones.
            }
        }

        return content;
    }

    /// <summary>
    /// Starts the game and checks that it keeps running, so that the user can
    /// test the settings of an instance.
    /// </summary>
    /// <param name="config">The configuration of the launch.</param>
    /// <param name="duration">The time the game is watched.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The result of the test.</returns>
    public static async Task<LaunchTestResult> TestLaunchAsync(
        LaunchConfig config, TimeSpan? duration = null, CancellationToken token = default)
    {
        if (config.JavaRuntime == null)
            return new LaunchTestResult(false, "未找到 Java 运行时", string.Empty);

        if (!File.Exists(config.JavaRuntime.Path))
            return new LaunchTestResult(false, "未找到 Java 运行时", string.Empty);

        var output = new StringBuilder();

        try
        {
            var startInfo = config.CreateProcessStartInfo();
            startInfo.RedirectStandardOutput = true;
            startInfo.RedirectStandardError = true;

            using var process = new Process { StartInfo = startInfo };
            process.OutputDataReceived += (_, e) =>
            {
                if (!string.IsNullOrEmpty(e.Data))
                    output.AppendLine(e.Data);
            };
            process.ErrorDataReceived += (_, e) =>
            {
                if (!string.IsNullOrEmpty(e.Data))
                    output.AppendLine(e.Data);
            };

            if (!process.Start())
                return new LaunchTestResult(false, "无法启动游戏进程", output.ToString());

            process.BeginOutputReadLine();
            process.BeginErrorReadLine();

            // The game must keep running during the test.
            await Task.Delay(duration ?? TimeSpan.FromSeconds(10), token);

            if (process.HasExited)
            {
                var code = process.ExitCode;
                return new LaunchTestResult(false, $"游戏进程已退出 (退出码 {code})", output.ToString());
            }

            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch (Exception)
            {
                // The process cannot be stopped; the test still succeeded.
            }

            return new LaunchTestResult(true, "游戏启动成功", output.ToString());
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            return new LaunchTestResult(false, $"启动测试失败: {ex.Message}", output.ToString());
        }
    }
}
