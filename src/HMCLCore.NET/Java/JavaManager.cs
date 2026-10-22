using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Java;

/// <summary>
/// Represents a Java installation.
/// </summary>
public sealed record JavaRuntime
{
    /// <summary>
    /// The path to the Java executable (java or javaw).
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The major version of the Java runtime (e.g., 8, 11, 17, 21).
    /// </summary>
    public int MajorVersion { get; init; }

    /// <summary>
    /// The full version string.
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The vendor of the Java runtime (e.g., "Oracle Corporation", "Azul Systems").
    /// </summary>
    public string Vendor { get; init; } = string.Empty;

    /// <summary>
    /// The architecture of the Java runtime.
    /// </summary>
    public string Architecture { get; init; } = string.Empty;

    /// <summary>
    /// Whether this is a Java 8 or later (required for modern Minecraft).
    /// </summary>
    public bool IsJava8OrLater => MajorVersion >= 8;

    /// <summary>
    /// Returns whether this runtime is suitable for the specified Minecraft version.
    /// </summary>
    public bool IsCompatibleWithMinecraft(string minecraftVersion)
    {
        if (VersionHelper.TryParseMinecraftMajorMinor(minecraftVersion, out var major, out var minor))
        {
            // Minecraft 1.20.5+ requires Java 21
            if (major == 1 && minor == 20 && (minor == 5 || minor > 5))
                return MajorVersion >= 21;
            // Minecraft 1.18+ requires Java 17
            if (major == 1 && minor >= 18)
                return MajorVersion >= 17;
            // Minecraft 1.17+ requires Java 16
            if (major == 1 && minor == 17)
                return MajorVersion >= 16;
        }

        // Default to Java 8+
        return IsJava8OrLater;
    }

    /// <summary>
    /// Returns a display string for the Java runtime.
    /// </summary>
    public override string ToString() => $"{Vendor} {Version} ({Architecture}) at {Path}";
}

/// <summary>
/// Helpers for parsing Minecraft version strings.
/// </summary>
internal static class VersionHelper
{
    /// <summary>
    /// Parses the major and minor components of a Minecraft version string.
    /// </summary>
    /// <remarks>
    /// Pre-release suffixes such as `-pre1` are stripped before parsing.
    /// </remarks>
    public static bool TryParseMinecraftMajorMinor(string version, out int major, out int minor)
    {
        major = 0;
        minor = 0;

        if (string.IsNullOrEmpty(version))
            return false;

        var clean = version.Split('-')[0];
        var parts = clean.Split('.');
        if (parts.Length < 2)
            return false;

        if (!int.TryParse(parts[0], out major))
            return false;
        if (!int.TryParse(parts[1], out minor))
            return false;

        return true;
    }
}

/// <summary>
/// Detects and manages Java installations.
/// </summary>
public sealed class JavaManager
{
    private readonly List<JavaRuntime> _knownRuntimes = new();

    /// <summary>
    /// The paths of the Java runtimes the user disabled; they are ignored by
    /// the scan.
    /// </summary>
    private readonly HashSet<string> _disabledPaths = new(StringComparer.OrdinalIgnoreCase);

    private JavaRuntime? _preferredRuntime;

    /// <summary>
    /// All known Java runtimes on the system.
    /// </summary>
    public IReadOnlyList<JavaRuntime> KnownRuntimes => _knownRuntimes.AsReadOnly();

    /// <summary>
    /// The preferred Java runtime for launching games.
    /// </summary>
    public JavaRuntime? PreferredRuntime
    {
        get => _preferredRuntime;
        set
        {
            if (value != null && !_knownRuntimes.Contains(value))
                throw new ArgumentException("Runtime not in known runtimes", nameof(value));
            _preferredRuntime = value;
        }
    }

    /// <summary>
    /// Scans the system for Java installations.
    /// </summary>
    /// <param name="disabledPaths">
    /// The paths of the runtimes the user disabled; they are skipped by the
    /// scan.
    /// </param>
    public void Scan(IReadOnlyCollection<string>? disabledPaths = null)
    {
        _knownRuntimes.Clear();
        _disabledPaths.Clear();

        if (disabledPaths is not null)
        {
            foreach (var path in disabledPaths)
            {
                if (!string.IsNullOrWhiteSpace(path))
                    _disabledPaths.Add(path);
            }
        }

        var javaHome = GetJavaHome();
        if (!string.IsNullOrEmpty(javaHome))
            ScanDirectory(javaHome);

        foreach (var path in GetSystemJavaPaths())
            ScanDirectory(path);

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            ScanWindowsRegistry();

        // Sort by version (newer first)
        _knownRuntimes.Sort((a, b) => b.MajorVersion.CompareTo(a.MajorVersion));
    }

    /// <summary>
    /// Returns whether the runtime at the given path was disabled by the user.
    /// </summary>
    /// <param name="path">The path of the Java executable.</param>
    /// <returns>Whether the runtime is disabled.</returns>
    public bool IsDisabled(string path) =>
        !string.IsNullOrEmpty(path) && _disabledPaths.Contains(path);

    /// <summary>
    /// Selects the best known runtime (highest version) as the preferred runtime.
    /// </summary>
    public bool SelectBestRuntime()
    {
        if (_knownRuntimes.Count == 0)
            return false;

        _preferredRuntime = _knownRuntimes[0];
        return true;
    }

    /// <summary>
    /// Detects a Java runtime at an explicit executable path.
    /// </summary>
    /// <param name="javaPath">The java/javaw executable path.</param>
    /// <returns>The detected runtime, or null when it cannot be read.</returns>
    public static JavaRuntime? DetectFromPath(string javaPath) => DetectJavaRuntime(javaPath);

    /// <summary>
    /// Detects and registers a Java executable so it appears in the runtime list.
    /// </summary>
    /// <param name="javaPath">The java/javaw executable path.</param>
    /// <returns>The registered runtime, or null when detection failed.</returns>
    public JavaRuntime? RegisterRuntime(string javaPath)
    {
        var runtime = DetectJavaRuntime(javaPath);
        if (runtime == null)
            return null;

        var existing = _knownRuntimes.Find(r =>
            string.Equals(r.Path, runtime.Path, StringComparison.OrdinalIgnoreCase));
        if (existing != null)
            return existing;

        _knownRuntimes.Add(runtime);
        _knownRuntimes.Sort((a, b) => b.MajorVersion.CompareTo(a.MajorVersion));
        return runtime;
    }

    /// <summary>
    /// Removes a runtime from the known list.
    /// </summary>
    /// <param name="runtime">The runtime to remove.</param>
    /// <returns>Whether the runtime was present.</returns>
    public bool RemoveRuntime(JavaRuntime runtime)
    {
        var removed = _knownRuntimes.Remove(runtime);
        if (removed && ReferenceEquals(_preferredRuntime, runtime))
            _preferredRuntime = _knownRuntimes.Count > 0 ? _knownRuntimes[0] : null;
        return removed;
    }

    /// <summary>
    /// Ensures a preferred runtime is configured: selects the stored path when it
    /// matches a known runtime, registers and invokes detection for hardcoded external
    /// paths, and falls back to the newest known runtime. Returns whether a
    /// preferred runtime is set after the call.
    /// </summary>
    /// <param name="preferredPath">The stored Java executable path from the settings.</param>
    public bool EnsurePreferredRuntime(string? preferredPath)
    {
        // Exact-path match against the already known runtimes.
        if (!string.IsNullOrEmpty(preferredPath))
        {
            var match = _knownRuntimes.Find(r =>
                string.Equals(r.Path, preferredPath, StringComparison.OrdinalIgnoreCase));
            if (match != null)
            {
                _preferredRuntime = match;
                return true;
            }

            // The user may have typed a custom java path; detect and register it.
            if (File.Exists(preferredPath))
            {
                var runtime = DetectJavaRuntime(preferredPath);
                if (runtime != null)
                {
                    _knownRuntimes.Add(runtime);
                    _preferredRuntime = runtime;
                    return true;
                }
            }
        }

        return _preferredRuntime != null || SelectBestRuntime();
    }

    /// <summary>
    /// Scans a directory for Java installations.
    /// </summary>
    private void ScanDirectory(string path)
    {
        if (string.IsNullOrEmpty(path) || !Directory.Exists(path))
            return;

        try
        {
            foreach (var dir in Directory.GetDirectories(path))
            {
                var javaExe = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                    ? Path.Combine(dir, "bin", "java.exe")
                    : Path.Combine(dir, "bin", "java");

                if (File.Exists(javaExe))
                {
                    if (IsDisabled(javaExe))
                        continue;

                    var runtime = DetectJavaRuntime(javaExe);
                    if (runtime != null)
                        _knownRuntimes.Add(runtime);
                }
            }
        }
        catch (Exception e) when (e is IOException || e is UnauthorizedAccessException)
        {
            // Ignore directories that cannot be read.
        }
    }

    /// <summary>
    /// Gets the JAVA_HOME directory, if it is set.
    /// </summary>
    private static string? GetJavaHome()
    {
        var javaHome = Environment.GetEnvironmentVariable("JAVA_HOME");
        return string.IsNullOrEmpty(javaHome) ? null : javaHome;
    }

    /// <summary>
    /// Gets common system Java paths.
    /// </summary>
    private static IEnumerable<string> GetSystemJavaPaths()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            yield return @"C:\Program Files\Java";
            yield return @"C:\Program Files (x86)\Java";
            yield return @"C:\Program Files\Eclipse Adoptium";
            yield return @"C:\Program Files\Microsoft\jdk";
            yield return @"C:\Program Files\Amazon Corretto";
            yield return @"C:\Program Files\BellSoft\LibericaJDK";
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
        {
            yield return "/usr/lib/jvm";
            yield return "/usr/java";
            yield return "/opt/java";
        }
        else if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            yield return "/Library/Java/JavaVirtualMachines";
            yield return "/System/Library/Frameworks/JavaVM.framework/Versions";
        }
    }

    /// <summary>
    /// Scans the Windows registry for Java installations.
    /// </summary>
    private void ScanWindowsRegistry()
    {
        if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
            return;

        try
        {
            using var baseKey = Microsoft.Win32.Registry.LocalMachine.OpenSubKey(@"SOFTWARE\JavaSoft\Java Runtime Environment");
            if (baseKey == null)
                return;

            foreach (var versionName in baseKey.GetSubKeyNames())
            {
                using var versionKey = baseKey.OpenSubKey(versionName);
                if (versionKey?.GetValue("JavaHome") is string javaHome)
                {
                    var javaExe = Path.Combine(javaHome, "bin", "java.exe");
                    if (File.Exists(javaExe) && !IsDisabled(javaExe))
                    {
                        var runtime = DetectJavaRuntime(javaExe);
                        if (runtime != null)
                            _knownRuntimes.Add(runtime);
                    }
                }
            }
        }
        catch (Exception e) when (e is IOException || e is UnauthorizedAccessException)
        {
            // Ignore registry access errors.
        }
    }

    /// <summary>
    /// Detects the Java runtime information by running java -version.
    /// </summary>
    private static JavaRuntime? DetectJavaRuntime(string javaPath)
    {
        try
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = javaPath,
                Arguments = "-version",
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };

            using var process = Process.Start(startInfo);
            if (process == null)
                return null;

            process.WaitForExit(5000);
            var output = process.StandardError.ReadToEnd() + process.StandardOutput.ReadToEnd();

            return ParseJavaVersion(javaPath, output);
        }
        catch (Exception e) when (e is IOException || e is System.ComponentModel.Win32Exception)
        {
            return null;
        }
    }

    /// <summary>
    /// Parses the output of java -version.
    /// </summary>
    private static JavaRuntime? ParseJavaVersion(string javaPath, string output)
    {
        // Example output:
        // openjdk version "17.0.8" 2023-07-18
        // OpenJDK Runtime Environment (build 17.0.8+7)
        // OpenJDK 64-Bit Server VM (build 17.0.8+7, mixed mode, sharing)

        var lines = output.Split(new[] { '\n', '\r' }, StringSplitOptions.RemoveEmptyEntries);
        if (lines.Length == 0)
            return null;

        var firstLine = lines[0].Trim();
        var versionMatch = System.Text.RegularExpressions.Regex.Match(
            firstLine, "version\\s+\"([^\"]+)\"");

        if (!versionMatch.Success)
            return null;

        var version = versionMatch.Groups[1].Value;
        var majorVersion = ParseMajorVersion(version);

        var vendor = "Unknown";
        if (output.Contains("Oracle", StringComparison.OrdinalIgnoreCase))
            vendor = "Oracle Corporation";
        else if (output.Contains("Eclipse Adoptium", StringComparison.OrdinalIgnoreCase) ||
                 output.Contains("Temurin", StringComparison.OrdinalIgnoreCase))
            vendor = "Eclipse Adoptium";
        else if (output.Contains("Azul", StringComparison.OrdinalIgnoreCase) ||
                 output.Contains("Zulu", StringComparison.OrdinalIgnoreCase))
            vendor = "Azul Systems";
        else if (output.Contains("Amazon", StringComparison.OrdinalIgnoreCase) ||
                 output.Contains("Corretto", StringComparison.OrdinalIgnoreCase))
            vendor = "Amazon";
        else if (output.Contains("BellSoft", StringComparison.OrdinalIgnoreCase) ||
                 output.Contains("Liberica", StringComparison.OrdinalIgnoreCase))
            vendor = "BellSoft";
        else if (output.Contains("Microsoft", StringComparison.OrdinalIgnoreCase))
            vendor = "Microsoft";
        else if (output.Contains("OpenJDK", StringComparison.OrdinalIgnoreCase))
            vendor = "OpenJDK";

        var arch = "x64";
        if (output.Contains("32-Bit", StringComparison.OrdinalIgnoreCase))
            arch = "x86";
        else if (output.Contains("aarch64", StringComparison.OrdinalIgnoreCase) ||
                 output.Contains("ARM64", StringComparison.OrdinalIgnoreCase))
            arch = "aarch64";

        return new JavaRuntime
        {
            Path = javaPath,
            MajorVersion = majorVersion,
            Version = version,
            Vendor = vendor,
            Architecture = arch
        };
    }

    /// <summary>
    /// Parses the major version from a version string.
    /// </summary>
    private static int ParseMajorVersion(string version)
    {
        var parts = version.Split('.');
        if (parts.Length >= 2 && parts[0] == "1")
        {
            // Java 8 and earlier: 1.8.0_292 -> 8
            if (int.TryParse(parts[1], out var minor))
                return minor;
        }
        else if (parts.Length >= 1 && int.TryParse(parts[0], out var major))
        {
            // Java 9+: 17.0.8 -> 17
            return major;
        }

        return 0;
    }
}