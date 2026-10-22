using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Util;

/// <summary>
/// The hardware of the machine the launcher runs on.
/// </summary>
public sealed class HardwareInfo
{
    /// <summary>
    /// The name of the processor, when it is known.
    /// </summary>
    public string CpuName { get; init; } = "未知";

    /// <summary>
    /// The number of the logical processors of the machine.
    /// </summary>
    public int ProcessorCount { get; init; } = Environment.ProcessorCount;

    /// <summary>
    /// The total physical memory of the machine, in bytes.
    /// </summary>
    public long TotalPhysicalMemory { get; init; } = Platform.TotalPhysicalMemory ?? 0;

    /// <summary>
    /// The physical memory that can be used right now, in bytes.
    /// </summary>
    public long AvailablePhysicalMemory { get; init; } = Platform.AvailablePhysicalMemory ?? 0;

    /// <summary>
    /// The name of the graphics adapter, when it is known.
    /// </summary>
    public string GpuName { get; init; } = "未知";

    /// <summary>
    /// The name of the operating system of the machine.
    /// </summary>
    public string OperatingSystem { get; init; } = RuntimeInformation.OSDescription;

    /// <summary>
    /// The description of the hardware shown to the user.
    /// </summary>
    public string Description =>
        $"处理器: {CpuName} ({ProcessorCount} 线程)\n" +
        $"内存: {FormatMemory(TotalPhysicalMemory)} (可用 {FormatMemory(AvailablePhysicalMemory)})\n" +
        $"显卡: {GpuName}\n" +
        $"系统: {OperatingSystem}";

    /// <summary>
    /// Reads the hardware of the machine the launcher runs on.
    /// </summary>
    /// <returns>The hardware of the machine.</returns>
    public static HardwareInfo Detect() => new()
    {
        CpuName = DetectCpuName(),
        ProcessorCount = Environment.ProcessorCount,
        TotalPhysicalMemory = Platform.TotalPhysicalMemory ?? 0,
        AvailablePhysicalMemory = Platform.AvailablePhysicalMemory ?? 0,
        GpuName = DetectGpuName(),
        OperatingSystem = RuntimeInformation.OSDescription
    };

    /// <summary>
    /// Returns the name of the processor of the machine.
    /// </summary>
    /// <returns>The name of the processor.</returns>
    private static string DetectCpuName()
    {
        // On Windows the environment holds the description of the processor.
        var identifier = Environment.GetEnvironmentVariable("PROCESSOR_IDENTIFIER");
        if (!string.IsNullOrWhiteSpace(identifier))
            return identifier.Trim();

        // On Linux the model of the processor is read from the proc file system.
        try
        {
            if (File.Exists("/proc/cpuinfo"))
            {
                foreach (var line in File.ReadLines("/proc/cpuinfo"))
                {
                    if (!line.StartsWith("model name", StringComparison.OrdinalIgnoreCase))
                        continue;

                    var separator = line.IndexOf(':');
                    if (separator >= 0)
                    {
                        var name = line[(separator + 1)..].Trim();
                        if (!string.IsNullOrEmpty(name))
                            return name;
                    }
                }
            }
        }
        catch (Exception)
        {
            // The processor cannot be read; the generic name is used.
        }

        return RuntimeInformation.ProcessArchitecture.ToString();
    }

    /// <summary>
    /// Returns the name of the graphics adapter of the machine.
    /// </summary>
    /// <returns>The name of the graphics adapter.</returns>
    private static string DetectGpuName()
    {
        try
        {
            if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            {
                var name = RunCommand(
                    "powershell.exe",
                    "-NoProfile -Command \"(Get-CimInstance Win32_VideoController | Select-Object -First 1).Name\"");

                if (!string.IsNullOrWhiteSpace(name))
                    return name.Trim();
            }
            else if (File.Exists("/sys/class/drm/card0/device/product_name"))
            {
                var name = File.ReadAllText("/sys/class/drm/card0/device/product_name").Trim();
                if (!string.IsNullOrEmpty(name))
                    return name;
            }
        }
        catch (Exception)
        {
            // The graphics adapter cannot be read; the generic name is used.
        }

        return "未知";
    }

    /// <summary>
    /// Runs a command and returns its output.
    /// </summary>
    /// <param name="fileName">The program to run.</param>
    /// <param name="arguments">The arguments of the program.</param>
    /// <returns>The output of the command, or an empty string when it fails.</returns>
    private static string RunCommand(string fileName, string arguments)
    {
        try
        {
            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = fileName,
                Arguments = arguments,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            });

            if (process is null)
                return string.Empty;

            if (!process.WaitForExit(3000))
            {
                try
                {
                    process.Kill();
                }
                catch (Exception)
                {
                    // The process cannot be stopped; its output is ignored.
                }

                return string.Empty;
            }

            return process.StandardOutput.ReadToEnd();
        }
        catch (Exception)
        {
            // The command cannot be run; the generic name is used.
            return string.Empty;
        }
    }

    /// <summary>
    /// Formats a number of bytes as a readable size.
    /// </summary>
    /// <param name="bytes">The number of bytes.</param>
    /// <returns>The formatted size.</returns>
    public static string FormatMemory(long bytes)
    {
        if (bytes <= 0)
            return "未知";

        string[] units = { "B", "KB", "MB", "GB", "TB" };
        double value = bytes;
        var unit = 0;

        while (value >= 1024 && unit < units.Length - 1)
        {
            value /= 1024;
            unit++;
        }

        return $"{value:F2} {units[unit]}";
    }
}
