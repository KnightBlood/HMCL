using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Download;

/// <summary>
/// Installs the mod loaders distributed as an installer jar, like OptiFine,
/// LiteLoader and Cleanroom. The installer is run with the
/// <c>--install</c> option, so that it writes its version into the launcher
/// directory.
/// </summary>
public static class LoaderInstaller
{
    /// <summary>
    /// The address of the metadata of the versions of Cleanroom.
    /// </summary>
    private const string CleanroomMetadata = "https://maven.cleanroommc.net/com/cleanroommc/cleanroom/maven-metadata.xml";

    /// <summary>
    /// The address of the versions of LiteLoader.
    /// </summary>
    private const string LiteLoaderApi = "https://www.liteloader.com/api/versions/";

    /// <summary>
    /// Returns the address of the installer of the given loader.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="loaderVersion">The version of the loader.</param>
    /// <returns>The address of the installer, or null when it is unknown.</returns>
    public static string? GetInstallerUrl(LoaderKind kind, string gameVersion, string? loaderVersion)
    {
        return kind switch
        {
            LoaderKind.OptiFine => $"https://optifine.net/download?f=OptiFine_{gameVersion}_HD_U_I7.jar",
            LoaderKind.LiteLoader => string.IsNullOrEmpty(loaderVersion)
                ? null
                : $"http://dl.liteloader.com/versions/com/mumfrey/liteloader/{loaderVersion}/" +
                  $"liteloader-{loaderVersion}-{gameVersion}-installer.jar",
            LoaderKind.Cleanroom => string.IsNullOrEmpty(loaderVersion)
                ? null
                : $"https://maven.cleanroommc.net/com/cleanroommc/cleanroom/{loaderVersion}/" +
                  $"cleanroom-{loaderVersion}-installer.jar",
            _ => null
        };
    }

    /// <summary>
    /// Returns the versions of the given loader for the given version of the
    /// game.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The versions of the loader, the newest first.</returns>
    public static async Task<IReadOnlyList<string>> GetLoaderVersionsAsync(
        LoaderKind kind, string gameVersion, CancellationToken token = default)
    {
        if (kind != LoaderKind.LiteLoader)
        {
            var version = await GetLoaderVersionAsync(kind, gameVersion, token);
            return version is null ? Array.Empty<string>() : new[] { version };
        }

        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(
                $"https://www.liteloader.com/api/versions/{gameVersion}", token);

            using var document = JsonDocument.Parse(json);
            if (document.RootElement.ValueKind != JsonValueKind.Array)
                return Array.Empty<string>();

            var versions = new List<string>();
            foreach (var element in document.RootElement.EnumerateArray())
            {
                var version = element.TryGetProperty("version", out var versionValue) &&
                              versionValue.ValueKind == JsonValueKind.String
                    ? versionValue.GetString()
                    : null;

                if (!string.IsNullOrEmpty(version))
                    versions.Add(version!);
            }

            return versions;
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException)
        {
            return Array.Empty<string>();
        }
    }

    /// <summary>
    /// Returns the newest version of the given loader for the given version of
    /// the game.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The version of the loader, or null when it cannot be found.</returns>
    public static async Task<string?> GetLoaderVersionAsync(
        LoaderKind kind, string gameVersion, CancellationToken token = default)
    {
        try
        {
            return kind switch
            {
                LoaderKind.OptiFine => "HD_U_I7",
                LoaderKind.LiteLoader => await GetLiteLoaderVersionAsync(gameVersion, token),
                LoaderKind.Cleanroom => await GetCleanroomVersionAsync(token),
                _ => null
            };
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException)
        {
            return null;
        }
    }

    /// <summary>
    /// Returns the newest version of LiteLoader for the given version of the
    /// game.
    /// </summary>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The version of the loader, or null when it cannot be found.</returns>
    private static async Task<string?> GetLiteLoaderVersionAsync(string gameVersion, CancellationToken token)
    {
        var json = await HttpClientProvider.Shared.GetStringAsync($"{LiteLoaderApi}{gameVersion}", token);
        using var document = JsonDocument.Parse(json);

        if (document.RootElement.ValueKind == JsonValueKind.Array)
        {
            foreach (var element in document.RootElement.EnumerateArray())
            {
                if (element.TryGetProperty("version", out var version) && version.ValueKind == JsonValueKind.String)
                    return version.GetString();
            }
        }

        return null;
    }

    /// <summary>
    /// Returns the newest version of Cleanroom.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The version of the loader, or null when it cannot be found.</returns>
    private static async Task<string?> GetCleanroomVersionAsync(CancellationToken token)
    {
        var xml = await HttpClientProvider.Shared.GetStringAsync(CleanroomMetadata, token);

        var document = System.Xml.Linq.XDocument.Parse(xml);
        var versions = document.Descendants("version")
            .Select(element => element.Value)
            .Where(value => !string.IsNullOrEmpty(value))
            .ToList();

        return versions.Count > 0 ? versions[^1] : null;
    }

    /// <summary>
    /// Installs the given loader into the launcher directory.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="baseDirectory">The root directory of the launcher.</param>
    /// <param name="javaPath">The Java executable used to run the installer.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="loaderVersion">
    /// The version of the loader; when it is empty the newest version is used.
    /// </param>
    /// <param name="progress">Receives the output of the installer.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The identifier of the installed version, or null on failure.</returns>
    public static async Task<string?> InstallAsync(
        LoaderKind kind,
        string baseDirectory,
        string javaPath,
        string gameVersion,
        string? loaderVersion = null,
        IProgress<string>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(baseDirectory) ||
            string.IsNullOrWhiteSpace(javaPath) ||
            string.IsNullOrWhiteSpace(gameVersion))
            return null;

        var version = loaderVersion;
        if (string.IsNullOrEmpty(version))
            version = await GetLoaderVersionAsync(kind, gameVersion, token);

        var url = GetInstallerUrl(kind, gameVersion, version);
        if (url is null)
            return null;

        var cacheDirectory = Path.Combine(baseDirectory, "cache", "installers");
        Directory.CreateDirectory(cacheDirectory);

        var installerPath = Path.Combine(
            cacheDirectory, $"{kind.ToString().ToLowerInvariant()}-{gameVersion}-{version}-installer.jar");

        if (!File.Exists(installerPath) || new FileInfo(installerPath).Length == 0)
        {
            progress?.Report($"正在下载安装器: {Path.GetFileName(installerPath)}");

            try
            {
                await using var content = await HttpClientProvider.Shared.GetStreamAsync(url, token);
                await using var file = File.Create(installerPath);
                await content.CopyToAsync(file, token);
            }
            catch (Exception ex)
            {
                progress?.Report($"下载安装器失败: {ex.Message}");
                return null;
            }
        }

        return await InstallFromFileAsync(baseDirectory, javaPath, installerPath, gameVersion, progress, token);
    }

    /// <summary>
    /// Installs a loader from a local installer file, without downloading
    /// anything.
    /// </summary>
    /// <param name="baseDirectory">The root directory of the launcher.</param>
    /// <param name="javaPath">The Java executable used to run the installer.</param>
    /// <param name="installerPath">The path of the installer.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="progress">Receives the output of the installer.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The identifier of the installed version, or null on failure.</returns>
    public static async Task<string?> InstallFromFileAsync(
        string baseDirectory,
        string javaPath,
        string installerPath,
        string gameVersion,
        IProgress<string>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(baseDirectory) ||
            string.IsNullOrWhiteSpace(javaPath) ||
            !File.Exists(installerPath))
            return null;

        var versionsDirectory = Path.Combine(baseDirectory, "versions");
        var existing = Directory.Exists(versionsDirectory)
            ? Directory.GetDirectories(versionsDirectory)
                .Select(Path.GetFileName)
                .ToHashSet(StringComparer.OrdinalIgnoreCase)
            : new HashSet<string?>(StringComparer.OrdinalIgnoreCase);

        var exitCode = await RunInstallerAsync(
            javaPath, installerPath, baseDirectory, progress, token);

        if (exitCode != 0)
        {
            progress?.Report($"安装器退出码 {exitCode}");
            return null;
        }

        Directory.CreateDirectory(versionsDirectory);

        var candidates = Directory.GetDirectories(versionsDirectory)
            .Select(Path.GetFileName)
            .Where(name => !string.IsNullOrEmpty(name) &&
                           !existing.Contains(name) &&
                           File.Exists(Path.Combine(versionsDirectory, name!, $"{name}.json")))
            .ToList();

        if (candidates.Count > 0)
            return candidates[0];

        var expected = $"{gameVersion}-OptiFine_HD_U_I7";
        if (File.Exists(Path.Combine(versionsDirectory, expected, $"{expected}.json")))
            return expected;

        return null;
    }

    /// <summary>
    /// Runs the installer with the <c>--install</c> option and streams its
    /// output.
    /// </summary>
    /// <param name="javaPath">The Java executable used to run the installer.</param>
    /// <param name="installerPath">The path of the installer.</param>
    /// <param name="baseDirectory">The root directory of the launcher.</param>
    /// <param name="progress">Receives the output of the installer.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The exit code of the installer.</returns>
    private static async Task<int> RunInstallerAsync(
        string javaPath,
        string installerPath,
        string baseDirectory,
        IProgress<string>? progress,
        CancellationToken token)
    {
        var startInfo = new ProcessStartInfo
        {
            FileName = javaPath,
            WorkingDirectory = baseDirectory,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            UseShellExecute = false,
            CreateNoWindow = true
        };

        startInfo.ArgumentList.Add("-jar");
        startInfo.ArgumentList.Add(installerPath);
        startInfo.ArgumentList.Add("--install");
        startInfo.ArgumentList.Add(baseDirectory);

        using var process = new Process { StartInfo = startInfo };
        var output = new StringBuilder();

        process.OutputDataReceived += (_, e) =>
        {
            if (string.IsNullOrEmpty(e.Data))
                return;
            progress?.Report(e.Data);
            output.AppendLine(e.Data);
        };

        process.ErrorDataReceived += (_, e) =>
        {
            if (string.IsNullOrEmpty(e.Data))
                return;
            progress?.Report(e.Data);
            output.AppendLine(e.Data);
        };

        if (!process.Start())
            return -1;

        process.BeginOutputReadLine();
        process.BeginErrorReadLine();

        try
        {
            await process.WaitForExitAsync(token);
        }
        catch (OperationCanceledException)
        {
            try
            {
                process.Kill(entireProcessTree: true);
            }
            catch (Exception)
            {
                // The installer cannot be stopped; the cancellation is reported.
            }

            throw;
        }

        if (process.ExitCode != 0 && output.Length > 0)
            progress?.Report(output.ToString());

        return process.ExitCode;
    }
}
