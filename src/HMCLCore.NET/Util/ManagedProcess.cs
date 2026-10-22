using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace HMCLCore.NET.Util;

/// <summary>
/// Represents a managed process with output streaming and lifecycle control.
/// Ported from Java ManagedProcess.
/// </summary>
public sealed class ManagedProcess : IDisposable
{
    private readonly Process _process;
    private readonly StringBuilder _stdOut = new();
    private readonly StringBuilder _stdErr = new();
    private bool _disposed;

    /// <summary>
    /// Raised for each line written to the process' standard output.
    /// </summary>
    public event Action<string>? OutputLineReceived;

    /// <summary>
    /// Raised for each line written to the process' standard error.
    /// </summary>
    public event Action<string>? ErrorLineReceived;

    /// <summary>
    /// Raised once when the process has exited.
    /// </summary>
    public event Action? Exited;

    private ManagedProcess(Process process)
    {
        _process = process;
        _process.EnableRaisingEvents = true;
        _process.Exited += (_, _) => Exited?.Invoke();
    }

    /// <summary>
    /// The underlying process.
    /// </summary>
    public Process Process => _process;

    /// <summary>
    /// Returns whether the process has exited.
    /// </summary>
    public bool HasExited => _process.HasExited;

    /// <summary>
    /// The exit code of the process, if it has exited.
    /// </summary>
    public int? ExitCode => _process.HasExited ? _process.ExitCode : null;

    /// <summary>
    /// The process ID.
    /// </summary>
    public int Id => _process.Id;

    /// <summary>
    /// Returns the standard output captured so far.
    /// </summary>
    public string StandardOutput => _stdOut.ToString();

    /// <summary>
    /// Returns the standard error captured so far.
    /// </summary>
    public string StandardError => _stdErr.ToString();

    /// <summary>
    /// Starts a process with the specified start info.
    /// </summary>
    public static ManagedProcess Start(ProcessStartInfo startInfo)
    {
        startInfo.UseShellExecute = false;
        startInfo.RedirectStandardOutput = true;
        startInfo.RedirectStandardError = true;
        startInfo.RedirectStandardInput = true;
        startInfo.CreateNoWindow = true;

        var process = Process.Start(startInfo);
        if (process == null)
            throw new InvalidOperationException("Failed to start process");

        return new ManagedProcess(process);
    }

    /// <summary>
    /// Writes a line to the standard input of the process, followed by a newline.
    /// Used to send commands to a running game.
    /// </summary>
    /// <param name="line">The line to send.</param>
    /// <returns>Whether the line could be written.</returns>
    public bool SendInput(string line)
    {
        try
        {
            if (_process.HasExited)
                return false;

            _process.StandardInput.WriteLine(line);
            _process.StandardInput.Flush();
            return true;
        }
        catch (IOException)
        {
            // The game closed its standard input.
            return false;
        }
        catch (ObjectDisposedException)
        {
            // The process was already disposed.
            return false;
        }
        catch (InvalidOperationException)
        {
            // Standard input is not redirected.
            return false;
        }
    }

    /// <summary>
    /// Waits for the process to exit.
    /// </summary>
    public void WaitForExit() => _process.WaitForExit();

    /// <summary>
    /// Waits for the process to exit with a timeout.
    /// </summary>
    public bool WaitForExit(int milliseconds) => _process.WaitForExit(milliseconds);

    /// <summary>
    /// Waits for the process to exit and returns its exit code.
    /// </summary>
    /// <param name="token">Cancellation token; the wait is not interrupted by it.</param>
    /// <returns>The exit code of the process.</returns>
    public async Task<int> WaitForExitAsync(CancellationToken token = default)
    {
        await _process.WaitForExitAsync(token);
        return _process.ExitCode;
    }

    /// <summary>
    /// Kills the process.
    /// </summary>
    public void Kill()
    {
        try
        {
            if (!_process.HasExited)
                _process.Kill();
        }
        catch (InvalidOperationException)
        {
            // Process already exited
        }
    }

    /// <summary>
    /// Streams standard output and error line by line until both pipes close,
    /// appending each line to the captured buffers and raising the
    /// <see cref="OutputLineReceived"/> / <see cref="ErrorLineReceived"/> events.
    /// </summary>
    public async Task ReadOutputAsync(CancellationToken token = default)
    {
        var stdout = PumpLinesAsync(_process.StandardOutput, _stdOut, OutputLineReceived, token);
        var stderr = PumpLinesAsync(_process.StandardError, _stdErr, ErrorLineReceived, token);
        await Task.WhenAll(stdout, stderr);
    }

    /// <summary>
    /// Reads one text stream line by line, forwarding each line to the buffer and handler.
    /// </summary>
    private static async Task PumpLinesAsync(StreamReader reader, StringBuilder buffer, Action<string>? handler, CancellationToken token)
    {
        while (await reader.ReadLineAsync(token) is { } line)
        {
            buffer.AppendLine(line);
            handler?.Invoke(line);
        }
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            try
            {
                if (!_process.HasExited)
                    _process.Kill();
            }
            catch (InvalidOperationException)
            {
                // Process already exited
            }

            _process.Dispose();
            _disposed = true;
        }
    }
}