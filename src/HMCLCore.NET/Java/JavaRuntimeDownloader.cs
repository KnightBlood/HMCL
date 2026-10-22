using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Runtime.InteropServices;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Java;

/// <summary>
/// A Java runtime component listed by Mojang's runtime metadata.
/// </summary>
public sealed class MojangJavaRuntime
{
    /// <summary>
    /// The Mojang component name (e.g., <c>java-runtime-gamma</c>).
    /// </summary>
    public string Component { get; init; } = string.Empty;

    /// <summary>
    /// The runtime version (e.g., <c>17.0.8</c>).
    /// </summary>
    public string Version { get; init; } = string.Empty;

    /// <summary>
    /// The URL of the runtime file manifest.
    /// </summary>
    public string ManifestUrl { get; init; } = string.Empty;

    /// <summary>
    /// A display label combining component and version.
    /// </summary>
    public string DisplayName => $"{Component} ({Version})";
}

/// <summary>
/// Reports progress while downloading a Java runtime.
/// </summary>
public sealed class JavaDownloadProgress
{
    /// <summary>
    /// The number of files already processed.
    /// </summary>
    public int CompletedFiles { get; init; }

    /// <summary>
    /// The total number of files to process.
    /// </summary>
    public int TotalFiles { get; init; }

    /// <summary>
    /// The file currently being processed.
    /// </summary>
    public string CurrentFile { get; init; } = string.Empty;

    /// <summary>
    /// The number of bytes already downloaded, for downloads of a single
    /// archive.
    /// </summary>
    public long CompletedBytes { get; init; }

    /// <summary>
    /// The total number of bytes of the download, or zero when the size of the
    /// download is unknown.
    /// </summary>
    public long TotalBytes { get; init; }
}

/// <summary>
/// Downloads Mojang Java runtimes (java-runtime-gamma/delta/etc.) for the
/// current platform and writes their files to a destination directory.
/// </summary>
public static class JavaRuntimeDownloader
{
    private const string AllRuntimesUrl =
        "https://launchermeta.mojang.com/v1/products/java-runtime/2ec0cc96c44e5a76b9c8b7c39df7210883d12871/all.json";

    private static readonly HttpClient HttpClient = CreateHttpClient();

    /// <summary>
    /// Returns the Java runtimes Mojang publishes for the current platform.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The available runtimes, newest listing first.</returns>
    public static async Task<List<MojangJavaRuntime>> GetAvailableRuntimesAsync(
        CancellationToken token = default)
    {
        var platformKey = GetPlatformKey();
        if (string.IsNullOrEmpty(platformKey))
            return new List<MojangJavaRuntime>();

        var json = await HttpClientProvider.Shared.GetStringAsync(AllRuntimesUrl, token);
        using var document = JsonDocument.Parse(json);

        if (!document.RootElement.TryGetProperty(platformKey, out var platformElement) ||
            platformElement.ValueKind != JsonValueKind.Object)
            return new List<MojangJavaRuntime>();

        var result = new List<MojangJavaRuntime>();
        foreach (var component in platformElement.EnumerateObject())
        {
            if (component.Value.ValueKind != JsonValueKind.Array)
                continue;

            foreach (var entry in component.Value.EnumerateArray())
            {
                if (!entry.TryGetProperty("manifest", out var manifest) ||
                    !manifest.TryGetProperty("url", out var urlElement))
                    continue;

                var url = urlElement.GetString();
                var version = entry.TryGetProperty("version", out var versionElement) &&
                              versionElement.TryGetProperty("name", out var nameElement)
                    ? nameElement.GetString() ?? string.Empty
                    : string.Empty;

                if (!string.IsNullOrEmpty(url))
                {
                    result.Add(new MojangJavaRuntime
                    {
                        Component = component.Name,
                        Version = version,
                        ManifestUrl = url
                    });
                }
            }
        }

        return result;
    }

    /// <summary>
    /// Installs a Mojang runtime into the destination directory.
    /// </summary>
    /// <param name="runtime">The runtime to install.</param>
    /// <param name="destinationDirectory">The directory receiving the runtime files.</param>
    /// <param name="progress">Receives file progress updates.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The path of the installed Java executable, or null on failure.</returns>
    public static async Task<string?> InstallAsync(
        MojangJavaRuntime runtime,
        string destinationDirectory,
        IProgress<JavaDownloadProgress>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(destinationDirectory))
            return null;

        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(runtime.ManifestUrl, token);
            using var document = JsonDocument.Parse(json);

            if (!document.RootElement.TryGetProperty("files", out var files) ||
                files.ValueKind != JsonValueKind.Object)
                return null;

            var entries = new List<(string Path, JsonElement Value)>();
            foreach (var property in files.EnumerateObject())
                entries.Add((property.Name, property.Value));

            var completed = 0;
            foreach (var (relativePath, value) in entries)
            {
                token.ThrowIfCancellationRequested();

                var type = value.TryGetProperty("type", out var typeElement)
                    ? typeElement.GetString()
                    : null;

                var targetPath = Path.Combine(destinationDirectory, relativePath);

                if (string.Equals(type, "directory", StringComparison.OrdinalIgnoreCase))
                {
                    Directory.CreateDirectory(targetPath);
                    completed++;
                    continue;
                }

                if (!string.Equals(type, "file", StringComparison.OrdinalIgnoreCase))
                {
                    completed++;
                    continue;
                }

                Directory.CreateDirectory(Path.GetDirectoryName(targetPath)!);

                if (File.Exists(targetPath))
                {
                    completed++;
                    progress?.Report(new JavaDownloadProgress
                    {
                        CompletedFiles = completed,
                        TotalFiles = entries.Count,
                        CurrentFile = relativePath
                    });
                    continue;
                }

                if (!value.TryGetProperty("downloads", out var downloads) ||
                    !downloads.TryGetProperty("raw", out var raw) ||
                    !raw.TryGetProperty("url", out var urlElement) ||
                    urlElement.GetString() is not { } fileUrl)
                {
                    completed++;
                    continue;
                }

                progress?.Report(new JavaDownloadProgress
                {
                    CompletedFiles = completed,
                    TotalFiles = entries.Count,
                    CurrentFile = relativePath
                });

                await using (var source = await HttpClientProvider.Shared.GetStreamAsync(fileUrl, token))
                await using (var destination = File.Create(targetPath))
                {
                    await source.CopyToAsync(destination, token);
                }

                // Preserve the executable bit on Unix-like systems.
                if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows) &&
                    value.TryGetProperty("executable", out var executable) &&
                    executable.ValueKind == JsonValueKind.True)
                {
                    try
                    {
                        var mode = File.GetUnixFileMode(targetPath);
                        File.SetUnixFileMode(targetPath,
                            mode | UnixFileMode.UserExecute | UnixFileMode.GroupExecute | UnixFileMode.OtherExecute);
                    }
                    catch
                    {
                        // Best effort: some file systems do not support modes.
                    }
                }

                completed++;
                progress?.Report(new JavaDownloadProgress
                {
                    CompletedFiles = completed,
                    TotalFiles = entries.Count,
                    CurrentFile = relativePath
                });
            }

            var executablePath = RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
                ? Path.Combine(destinationDirectory, "bin", "java.exe")
                : Path.Combine(destinationDirectory, "bin", "java");

            return File.Exists(executablePath) ? executablePath : null;
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Returns the Mojang platform key for the current machine.
    /// </summary>
    private static string GetPlatformKey()
    {
        var is64 = Environment.Is64BitProcess;
        var isArm = RuntimeInformation.ProcessArchitecture == Architecture.Arm64;

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            if (isArm)
                return "windows-arm64";
            return is64 ? "windows-x64" : "windows-x86";
        }

        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
            return isArm ? "mac-os-arm64" : "mac-os";

        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux))
            return isArm ? "linux-arm64" : is64 ? "linux" : "linux-i386";

        return string.Empty;
    }

    /// <summary>
    /// Creates the shared HTTP client with a descriptive user agent.
    /// </summary>
    private static HttpClient CreateHttpClient()
    {
        var client = HttpClientProvider.CreateClient();
        client.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");
        return client;
    }
}
