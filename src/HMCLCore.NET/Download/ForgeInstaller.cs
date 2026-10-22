using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Installs Forge/NeoForge by downloading the official installer and running
/// it in client mode against the repository directory. The installer performs
/// the processor steps and writes a version JSON that this launcher then parses.
/// </summary>
public static class ForgeInstaller
{
    private const string ForgeMaven = "https://maven.minecraftforge.net";
    private const string NeoForgeMaven = "https://maven.neoforged.net/releases";
    private const string ForgePromotions = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
    private const string NeoForgeVersions = "https://maven.neoforged.net/api/maven/versions/releases/net/neoforged/neoforge";

    private static readonly HttpClient HttpClient = CreateHttpClient();

    /// <summary>
    /// Returns the recommended or latest Forge version for a Minecraft version.
    /// </summary>
    /// <param name="gameVersion">The Minecraft version.</param>
    /// <param name="latest">True for the latest build, false for recommended.</param>
    /// <returns>The Forge version, or null when none is published.</returns>
    public static async Task<string?> GetForgeVersionAsync(string gameVersion, bool latest = false)
    {
        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(ForgePromotions);
            using var document = JsonDocument.Parse(json);

            // The promotions file uses "promos"; older mirrors used "promotions".
            if (!document.RootElement.TryGetProperty("promos", out var promotions) &&
                !document.RootElement.TryGetProperty("promotions", out promotions))
                return null;

            var key = $"{gameVersion}-{(latest ? "latest" : "recommended")}";
            if (promotions.TryGetProperty(key, out var version))
                return version.GetString();

            // Fall back to the other channel when the requested one is absent.
            var fallbackKey = $"{gameVersion}-{(latest ? "recommended" : "latest")}";
            return promotions.TryGetProperty(fallbackKey, out var fallback)
                ? fallback.GetString()
                : null;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Returns the newest NeoForge version published for a Minecraft version.
    /// </summary>
    /// <param name="gameVersion">The Minecraft version.</param>
    /// <returns>The NeoForge version, or null when none is published.</returns>
    public static async Task<string?> GetNeoForgeVersionAsync(string gameVersion)
    {
        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(NeoForgeVersions);
            using var document = JsonDocument.Parse(json);
            if (document.RootElement.ValueKind != JsonValueKind.Array)
                return null;

            string? best = null;
            long bestOrder = long.MinValue;

            foreach (var element in document.RootElement.EnumerateArray())
            {
                var version = element.GetString();
                if (string.IsNullOrEmpty(version))
                    continue;

                var parts = version.Split('.');
                if (parts.Length < 3)
                    continue;

                // NeoForge versions are named "<major>.<minor>.<patch>.<build>"
                // and target Minecraft "1.<major>.<minor>".
                var target = $"1.{parts[0]}.{parts[1]}";
                if (!target.Equals(gameVersion, StringComparison.Ordinal))
                    continue;

                if (!long.TryParse(parts[2], out var patch))
                    continue;

                var order = patch;
                if (parts.Length > 3 && long.TryParse(parts[3], out var build))
                    order = patch * 100000 + build;

                if (order > bestOrder)
                {
                    bestOrder = order;
                    best = version;
                }
            }

            return best;
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Installs Forge or NeoForge into the repository directory.
    /// </summary>
    /// <param name="kind">The loader kind (Forge or NeoForge).</param>
    /// <param name="baseDirectory">The repository root containing <c>versions</c> and <c>libraries</c>.</param>
    /// <param name="javaPath">The Java executable used to run the installer.</param>
    /// <param name="gameVersion">The Minecraft version.</param>
    /// <param name="loaderVersion">
    /// The loader version; for Forge this may be null to use the recommended build.
    /// </param>
    /// <param name="progress">Receives installer output lines.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The installed version id, or null when installation failed.</returns>
    public static async Task<string?> InstallAsync(
        LoaderKind kind,
        string baseDirectory,
        string javaPath,
        string gameVersion,
        string? loaderVersion,
        IProgress<string>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(baseDirectory) ||
            string.IsNullOrWhiteSpace(javaPath) ||
            string.IsNullOrWhiteSpace(gameVersion))
            return null;

        var resolvedVersion = loaderVersion;
        if (kind == LoaderKind.Forge && string.IsNullOrWhiteSpace(resolvedVersion))
            resolvedVersion = await GetForgeVersionAsync(gameVersion);

        if (string.IsNullOrWhiteSpace(resolvedVersion))
            return null;

        var installerUrl = kind == LoaderKind.Forge
            ? $"{ForgeMaven}/net/minecraftforge/forge/{gameVersion}-{resolvedVersion}/forge-{gameVersion}-{resolvedVersion}-installer.jar"
            : $"{NeoForgeMaven}/net/neoforged/neoforge/{resolvedVersion}/neoforge-{resolvedVersion}-installer.jar";

        var cacheDirectory = Path.Combine(baseDirectory, "cache", "installers");
        Directory.CreateDirectory(cacheDirectory);
        var installerPath = Path.Combine(
            cacheDirectory,
            kind == LoaderKind.Forge
                ? $"forge-{gameVersion}-{resolvedVersion}-installer.jar"
                : $"neoforge-{resolvedVersion}-installer.jar");

        if (!File.Exists(installerPath) || new FileInfo(installerPath).Length == 0)
        {
            progress?.Report($"正在下载安装器: {Path.GetFileName(installerPath)}");
            await using var content = await HttpClientProvider.Shared.GetStreamAsync(installerUrl, token);
            await using var file = File.Create(installerPath);
            await content.CopyToAsync(file, token);
        }

        EnsureLauncherProfiles(baseDirectory);

        return await InstallFromFileAsync(
            baseDirectory, javaPath, installerPath, resolvedVersion, kind, gameVersion, progress, token);
    }

    /// <summary>
    /// Installs a mod loader from a local installer file, without downloading
    /// anything.
    /// </summary>
    /// <param name="baseDirectory">The repository root containing <c>versions</c> and <c>libraries</c>.</param>
    /// <param name="javaPath">The Java executable used to run the installer.</param>
    /// <param name="installerPath">The path of the installer file.</param>
    /// <param name="loaderVersion">
    /// The version of the loader, when it is known; it is only used to guess
    /// the identifier of the installed version.
    /// </param>
    /// <param name="kind">The kind of the loader, when it is known.</param>
    /// <param name="gameVersion">The Minecraft version, when it is known.</param>
    /// <param name="progress">Receives installer output lines.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The installed version id, or null when the installation failed.</returns>
    public static async Task<string?> InstallFromFileAsync(
        string baseDirectory,
        string javaPath,
        string installerPath,
        string? loaderVersion = null,
        LoaderKind? kind = null,
        string? gameVersion = null,
        IProgress<string>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(baseDirectory) ||
            string.IsNullOrWhiteSpace(javaPath) ||
            string.IsNullOrWhiteSpace(installerPath))
            return null;

        if (!File.Exists(installerPath))
            return null;

        EnsureLauncherProfiles(baseDirectory);

        var versionsDirectory = Path.Combine(baseDirectory, "versions");
        HashSet<string?> existing = Directory.Exists(versionsDirectory)
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

        // Prefer a newly created version folder; fall back to the expected id.
        Directory.CreateDirectory(versionsDirectory);
        var candidates = Directory.GetDirectories(versionsDirectory)
            .Select(Path.GetFileName)
            .Where(name => !string.IsNullOrEmpty(name) &&
                           !existing.Contains(name) &&
                           File.Exists(Path.Combine(versionsDirectory, name!, $"{name}.json")))
            .ToList();

        if (candidates.Count > 0)
            return candidates[0];

        if (string.IsNullOrEmpty(loaderVersion) || string.IsNullOrEmpty(gameVersion))
            return null;

        var expected = kind == LoaderKind.NeoForge
            ? $"neoforge-{loaderVersion}"
            : $"{gameVersion}-forge-{loaderVersion}";
        if (File.Exists(Path.Combine(versionsDirectory, expected, $"{expected}.json")))
            return expected;

        // NeoForge installs some game versions under the "<mc>-forge-<ver>" id.
        var neoForgeLegacy = $"{gameVersion}-forge-{loaderVersion}";
        if (kind == LoaderKind.NeoForge &&
            File.Exists(Path.Combine(versionsDirectory, neoForgeLegacy, $"{neoForgeLegacy}.json")))
            return neoForgeLegacy;

        return null;
    }

    /// <summary>
    /// Ensures a launcher_profiles.json exists because the installer requires it.
    /// </summary>
    private static void EnsureLauncherProfiles(string baseDirectory)
    {
        var path = Path.Combine(baseDirectory, "launcher_profiles.json");
        if (File.Exists(path))
            return;

        File.WriteAllText(path,
            "{\"profiles\":{},\"selectedProfile\":\"\",\"clientToken\":\"\"," +
            "\"authenticationDatabase\":{},\"launcherVersion\":{\"name\":\"HMCL.NET\",\"format\":21}," +
            "\"settings\":{},\"version\":3}");
    }

    /// <summary>
    /// Runs the installer with <c>--installClient</c> and streams its output.
    /// </summary>
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
        startInfo.ArgumentList.Add("--installClient");
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
            catch
            {
                // Ignore kill failures.
            }

            throw;
        }

        if (process.ExitCode != 0 && output.Length > 0)
            progress?.Report(output.ToString());

        return process.ExitCode;
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
