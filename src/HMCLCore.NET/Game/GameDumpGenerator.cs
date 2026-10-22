using System;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace HMCLCore.NET.Game;

/// <summary>
/// Writes a dump of the threads of the running game, like the dump generator
/// of Java HMCL.
/// </summary>
/// <remarks>
/// The dump is written with the <c>jcmd</c> tool of the Java runtime used by
/// the game, so that no extra tool is needed on the machine of the user.
/// </remarks>
public static class GameDumpGenerator
{
    /// <summary>
    /// The name of the tool of the Java runtime writing the thread dump.
    /// </summary>
    private const string JcmdToolName = "jcmd";

    /// <summary>
    /// Writes the dump of the threads of the given process into a file.
    /// </summary>
    /// <param name="processId">The identifier of the game process.</param>
    /// <param name="javaHome">
    /// The directory of the Java runtime used by the game; the tool of the
    /// runtime is used when it is given.
    /// </param>
    /// <param name="outputPath">The path of the written file.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>Whether the dump was written.</returns>
    public static async Task<bool> WriteDumpToAsync(
        int processId, string? javaHome, string outputPath, CancellationToken cancellationToken = default)
    {
        if (processId <= 0 || string.IsNullOrWhiteSpace(outputPath))
            return false;

        var tool = ResolveJcmd(javaHome);
        if (tool is null)
            return false;

        try
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = tool,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            startInfo.ArgumentList.Add(processId.ToString());
            startInfo.ArgumentList.Add("Thread.print");

            using var process = Process.Start(startInfo);
            if (process is null)
                return false;

            var output = new StringBuilder();
            var error = new StringBuilder();

            var outputTask = process.StandardOutput.ReadToEndAsync(cancellationToken);
            var errorTask = process.StandardError.ReadToEndAsync(cancellationToken);

            await process.WaitForExitAsync(cancellationToken);

            output.Append(await outputTask);
            error.Append(await errorTask);

            var directory = Path.GetDirectoryName(outputPath);
            if (!string.IsNullOrEmpty(directory))
                Directory.CreateDirectory(directory);

            await File.WriteAllTextAsync(outputPath, output.ToString(), cancellationToken);

            if (error.Length > 0)
                await File.AppendAllTextAsync(outputPath, "\n" + error, cancellationToken);

            return true;
        }
        catch (Exception ex) when (ex is IOException or System.ComponentModel.Win32Exception
                                      or OperationCanceledException or InvalidOperationException)
        {
            return false;
        }
    }

    /// <summary>
    /// Returns the path of the tool writing the thread dump.
    /// </summary>
    /// <param name="javaHome">The directory of the Java runtime used by the game.</param>
    /// <returns>The path of the tool, or null when it cannot be found.</returns>
    private static string? ResolveJcmd(string? javaHome)
    {
        if (!string.IsNullOrWhiteSpace(javaHome))
        {
            var directory = Path.GetDirectoryName(javaHome);
            if (!string.IsNullOrEmpty(directory))
            {
                var candidate = Path.Combine(directory,
                    JcmdToolName + (HMCLCore.NET.Util.Platform.CurrentOs == HMCLCore.NET.Util.OsType.Windows ? ".exe" : string.Empty));
                if (File.Exists(candidate))
                    return candidate;
            }
        }

        // The tool of the Java runtime found in the path of the system is used
        // when the runtime of the game is unknown.
        return JcmdToolName;
    }
}
