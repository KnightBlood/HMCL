using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Events;
using HMCLCore.NET.Game;
using HMCLCore.NET.Launch;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Downloads and parses the remote version manifest from Mojang.
/// </summary>
public sealed class VersionRemoteFetchTask : TaskBase
{

    /// <summary>
    /// The URL of the version manifest.
    /// </summary>
    public string ManifestUrl { get; init; } = "https://launchermeta.mojang.com/mc/game/version_manifest.json";

    /// <summary>
    /// The destination path for the manifest JSON.
    /// </summary>
    public string DestinationPath { get; init; } = string.Empty;

    /// <summary>
    /// The parsed version metadata entries.
    /// </summary>
    public List<VersionMetadata> Versions { get; } = new();

    /// <summary>
    /// Creates a new VersionRemoteFetchTask.
    /// </summary>
    public VersionRemoteFetchTask(string destinationPath)
    {
        DestinationPath = destinationPath;
        Name = "获取远程版本列表";
    }

    /// <summary>
    /// Downloads and parses the version manifest.
    /// </summary>
    public override async Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            Directory.CreateDirectory(Path.GetDirectoryName(DestinationPath)!);

            var manifestUrl = ManifestUrl;

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                MirrorSource.GetVersionListUris(manifestUrl), DestinationPath, null, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {manifestUrl}");

            // Parse the downloaded JSON
            var json = File.ReadAllText(DestinationPath);
            ParseManifest(json);

            OnProgressChanged(100, 100);
            IsCompleted = true;
            IsSucceeded = true;
        }
        catch (OperationCanceledException)
        {
            IsCompleted = true;
            IsSucceeded = false;
            throw new OperationCanceledException("Version fetch cancelled");
        }
        catch (Exception ex)
        {
            IsCompleted = true;
            IsSucceeded = false;
            Exception = ex;
            throw;
        }
    }

    /// <summary>
    /// Parses the version manifest JSON into VersionMetadata entries.
    /// </summary>
    private void ParseManifest(string json)
    {
        Versions.Clear();

        using var doc = JsonDocument.Parse(json);
        if (!doc.RootElement.TryGetProperty("versions", out var versionsElement))
            return;

        foreach (var version in versionsElement.EnumerateArray())
        {
            Versions.Add(new VersionMetadata
            {
                Id = version.GetProperty("id").GetString() ?? string.Empty,
                Type = version.GetProperty("type").GetString() ?? "release",
                ReleaseTime = version.GetProperty("releaseTime").GetDateTime(),
                Url = version.GetProperty("url").GetString() ?? string.Empty,
                Sha1 = version.TryGetProperty("sha1", out var sha1Prop) ? sha1Prop.GetString() ?? string.Empty : string.Empty,
                Size = version.TryGetProperty("size", out var sizeProp) ? sizeProp.GetInt64() : 0
            });
        }
    }
}

/// <summary>
/// Downloads a specific version JSON from Mojang's servers.
/// </summary>
public sealed class VersionJsonDownloadTask : TaskBase
{
    /// <summary>
    /// The URL of the version JSON.
    /// </summary>
    public Uri Url { get; init; } = new("about:blank");

    /// <summary>
    /// The destination path for the version JSON.
    /// </summary>
    public string DestinationPath { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new VersionJsonDownloadTask.
    /// </summary>
    public VersionJsonDownloadTask(Uri url, string destinationPath)
    {
        Url = url;
        DestinationPath = destinationPath;
        Name = "下载版本JSON";
    }

    /// <summary>
    /// Downloads the version JSON file.
    /// </summary>
    public override async Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            Directory.CreateDirectory(Path.GetDirectoryName(DestinationPath)!);

            var progress = new InlineProgress<DownloadProgressEventArgs>(
                p => OnProgressChanged(p.DownloadedBytes, p.TotalBytes));

            var succeeded = await FileDownloader.DownloadAsync(
                MirrorSource.GetVersionListUris(Url.ToString()), DestinationPath, null, progress, token);

            if (!succeeded)
                throw new IOException($"下载失败: {Url}");

            OnProgressChanged(100, 100);
            IsCompleted = true;
            IsSucceeded = true;
        }
        catch (OperationCanceledException)
        {
            IsCompleted = true;
            IsSucceeded = false;
            throw new OperationCanceledException("Version JSON download cancelled");
        }
        catch (Exception ex)
        {
            IsCompleted = true;
            IsSucceeded = false;
            Exception = ex;
            throw;
        }
    }
}

/// <summary>
/// Generates the launch arguments for Minecraft based on the version manifest and account.
/// </summary>
public sealed class LaunchArgumentGenerator
{
    /// <summary>
    /// Generates the JVM arguments for the game launch.
    /// </summary>
    /// <remarks>
    /// The arguments of the version manifest are used when the manifest
    /// defines them; their placeholders, for example <c>${natives_directory}</c>,
    /// are resolved with the values of the launch configuration. Manifests
    /// without JVM arguments, which is the case for the versions before 1.13,
    /// fall back to the arguments of the official launcher.
    /// </remarks>
    /// <param name="config">The launch configuration.</param>
    /// <param name="manifest">The manifest of the launched version.</param>
    /// <returns>The JVM arguments of the launch.</returns>
    public List<string> GenerateJvmArguments(LaunchConfig config, GameInstanceManifest manifest)
    {
        var args = new List<string>();
        var substitutor = LaunchArgumentSubstitutor.Create(config, manifest);

        // Memory settings: honor game settings (auto/manual memory).
        var effectiveSettings = config.GameSettings;
        bool autoMemory = true;
        int configuredMemory = config.Memory;
        if (effectiveSettings is { } settings)
        {
            autoMemory = settings.AutoMemory;
            if (!autoMemory && settings.MaxMemory is { } max && max > 0)
                configuredMemory = max;
        }

        args.Add($"-Xmx{configuredMemory}m");
        args.Add($"-Xms{Math.Min(configuredMemory, 512)}m");

        // The JVM arguments of the manifest, with their placeholders resolved.
        var manifestArguments = Substitute(manifest.Arguments.Jvm, substitutor);
        if (manifestArguments.Count > 0 && effectiveSettings?.NoJvmOptions != true)
        {
            args.AddRange(manifestArguments);
        }
        else
        {
            // The default arguments of the official launcher. The heap dumps of
            // the game are kept in the working directory on Windows.
            if (HMCLCore.NET.Util.Platform.CurrentOs == HMCLCore.NET.Util.OsType.Windows)
                args.Add("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump");

            args.Add($"-Djava.library.path={config.NativesDirectory}");
            args.Add($"-Dminecraft.launcher.brand={HMCLCore.NET.Metadata.SHORT_TITLE}");
            args.Add($"-Dminecraft.launcher.version={HMCLCore.NET.Metadata.VERSION}");
            args.Add("-cp");
            args.Add(config.ClassPath);
        }

        // Make sure the native libraries and the classpath are part of the
        // arguments, even when the manifest does not mention them.
        if (!args.Any(a => a.StartsWith("-Djava.library.path=", StringComparison.Ordinal)))
            args.Add($"-Djava.library.path={config.NativesDirectory}");

        if (!args.Contains("-cp"))
        {
            args.Add("-cp");
            args.Add(config.ClassPath);
        }

        // authlib-injector agent for third-party Yggdrasil session redirection.
        if (!string.IsNullOrEmpty(config.AuthlibInjectorJarPath))
        {
            args.Add($"-javaagent:{config.AuthlibInjectorJarPath}={config.AuthlibInjectorServerUrl}");
            args.Add("-Dauthlibinjector.side=client");
        }

        // Server metadata pre-fetch: base64 of the server root response body.
        if (!string.IsNullOrEmpty(config.AuthlibInjectorPrefetchData))
            args.Add($"-Dauthlibinjector.yggdrasil.prefetched={config.AuthlibInjectorPrefetchData}");

        // Custom JVM arguments
        if (config.UseCustomJvmArgs && !string.IsNullOrWhiteSpace(config.CustomJvmArgs))
        {
            foreach (var arg in config.CustomJvmArgs.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
                args.Add(arg);
        }

        // Game settings JVM options (override/additional JVM flags).
        if (effectiveSettings is { } settings2 && !string.IsNullOrWhiteSpace(settings2.JvmOptions))
        {
            foreach (var arg in settings2.JvmOptions.Split(' ', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
                args.Add(arg);
        }

        // The size of the permanent generation area of the virtual machine,
        // which is only used by the Java 8 and older runtimes.
        if (config.PermSize is { } permSize && permSize > 0)
            args.Add($"-XX:PermSize={permSize}m");

        return args;
    }

    /// <summary>
    /// Generates the game arguments for the Minecraft launch.
    /// </summary>
    /// <remarks>
    /// The arguments of the version manifest are used when the manifest
    /// defines them; their placeholders, for example
    /// <c>${auth_player_name}</c>, are resolved with the values of the account
    /// and of the launch configuration. Manifests without game arguments fall
    /// back to the arguments of the official launcher.
    /// </remarks>
    /// <param name="config">The launch configuration.</param>
    /// <param name="manifest">The manifest of the launched version.</param>
    /// <returns>The game arguments of the launch.</returns>
    public List<string> GenerateGameArguments(LaunchConfig config, GameInstanceManifest manifest)
    {
        var args = new List<string>();
        var substitutor = LaunchArgumentSubstitutor.Create(config, manifest);

        // The game arguments of the manifest, with their placeholders resolved.
        var manifestArguments = Substitute(manifest.Arguments.Game, substitutor);
        if (manifestArguments.Count > 0)
        {
            args.AddRange(manifestArguments);
        }
        else
        {
            // The default arguments of the official launcher.
            args.Add("--username");
            args.Add(config.Account.Username);
            args.Add("--uuid");
            args.Add(ResolveUuid(config));
            args.Add("--accessToken");
            args.Add(ResolveAccessToken(config));
            args.Add("--userType");
            args.Add(ResolveUserType(config));
            args.Add("--versionType");
            args.Add(config.VersionType);
            args.Add("--version");
            args.Add(config.VersionName);
            args.Add("--gameDir");
            args.Add(config.GameDirectory);
            args.Add("--assetsDir");
            args.Add(config.AssetsDirectory);
            args.Add("--assetIndex");
            args.Add(config.AssetIndex ?? string.Empty);
        }

        // User properties (empty JSON)
        if (!args.Contains("--userProperties"))
        {
            args.Add("--userProperties");
            args.Add("{}");
        }

        // Demo mode
        if (config.Account is HMCLCore.NET.Auth.OfflineAccount && manifest.Demo)
            args.Add("--demo");

        // Custom resolution
        if (manifest.CustomResolution)
        {
            args.Add("--width");
            args.Add(config.WindowWidth.ToString());
            args.Add("--height");
            args.Add(config.WindowHeight.ToString());
        }

        // Server address
        if (!string.IsNullOrEmpty(config.ServerAddress))
        {
            args.Add("--server");
            args.Add(config.ServerAddress);
            args.Add("--port");
            args.Add(config.ServerPort.ToString());
        }

        // Quick play: enter a single player save, a multiplayer server or a Realms
        // world directly instead of showing the title screen.
        if (!string.IsNullOrWhiteSpace(config.QuickPlayArgument))
        {
            switch (config.QuickPlayMode)
            {
                case HMCLCore.NET.Setting.QuickPlayMode.Singleplayer:
                    args.Add("--quickPlayPath");
                    args.Add(config.QuickPlayArgument);
                    break;
                case HMCLCore.NET.Setting.QuickPlayMode.Multiplayer:
                    args.Add("--quickPlayMultiplayer");
                    args.Add(config.QuickPlayArgument);
                    break;
                case HMCLCore.NET.Setting.QuickPlayMode.Realms:
                    args.Add("--quickPlayRealms");
                    args.Add(config.QuickPlayArgument);
                    break;
            }
        }

        // Additional game arguments
        foreach (var arg in config.GameArguments)
            args.Add(arg);

        return args;
    }

    /// <summary>
    /// Resolves the placeholders of the given arguments and drops the
    /// arguments that cannot be resolved.
    /// </summary>
    /// <param name="arguments">The arguments of the version manifest.</param>
    /// <param name="substitutor">The substitutor holding the placeholder values.</param>
    /// <returns>The resolved arguments.</returns>
    private static List<string> Substitute(IReadOnlyList<string> arguments, LaunchArgumentSubstitutor substitutor)
    {
        var result = new List<string>(arguments.Count);

        foreach (var argument in arguments)
        {
            if (string.IsNullOrWhiteSpace(argument))
                continue;

            if (!substitutor.TrySubstitute(argument, out var substituted))
            {
                // The argument holds an unknown placeholder and cannot be used.
                // When it is the value of a preceding option, the option is
                // dropped as well, so the game does not read the next argument
                // as its value.
                if (result.Count > 0 && result[^1].StartsWith("--", StringComparison.Ordinal))
                    result.RemoveAt(result.Count - 1);

                continue;
            }

            result.Add(substituted);
        }

        return result;
    }

    /// <summary>
    /// Returns the access token of the account of the launch configuration.
    /// </summary>
    /// <param name="config">The launch configuration.</param>
    /// <returns>The access token.</returns>
    private static string ResolveAccessToken(LaunchConfig config) => config.Account switch
    {
        HMCLCore.NET.Auth.Microsoft.MicrosoftAccount microsoft => microsoft.MinecraftAccessToken ?? string.Empty,
        HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount yggdrasil => yggdrasil.YggdrasilAccessToken ?? string.Empty,
        HMCLCore.NET.Auth.OAuthAccount oauth => oauth.AccessToken ?? string.Empty,
        _ => "0"
    };

    /// <summary>
    /// Returns the profile UUID of the account of the launch configuration.
    /// </summary>
    /// <param name="config">The launch configuration.</param>
    /// <returns>The UUID.</returns>
    private static string ResolveUuid(LaunchConfig config) => config.Account switch
    {
        HMCLCore.NET.Auth.Microsoft.MicrosoftAccount microsoft => microsoft.MinecraftUuid ?? string.Empty,
        HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount yggdrasil => yggdrasil.ProfileUuid ?? string.Empty,
        _ => LaunchArgumentSubstitutor.ComputeOfflineUuid(config.Account.Username)
    };

    /// <summary>
    /// Returns the user type of the account of the launch configuration.
    /// </summary>
    /// <param name="config">The launch configuration.</param>
    /// <returns>The user type.</returns>
    private static string ResolveUserType(LaunchConfig config) => config.Account switch
    {
        HMCLCore.NET.Auth.Microsoft.MicrosoftAccount => "msa",
        HMCLCore.NET.Auth.Yggdrasil.YggdrasilAccount => "mojang",
        _ => "legacy"
    };
}

/// <summary>
/// Crash report analyzer for Minecraft crashes.
/// </summary>
public sealed class CrashReportAnalyzer
{
    /// <summary>
    /// Analyzes a crash report file and returns a summary.
    /// </summary>
    public CrashReportSummary Analyze(string crashReportPath)
    {
        if (!File.Exists(crashReportPath))
            return new CrashReportSummary { Error = "Crash report file not found" };

        var content = File.ReadAllText(crashReportPath);
        return AnalyzeContent(content);
    }

    /// <summary>
    /// Analyzes crash report content and returns a summary.
    /// </summary>
    public CrashReportSummary AnalyzeContent(string content)
    {
        var summary = new CrashReportSummary();

        // Extract Minecraft version
        if (content.Contains("Minecraft Version:"))
        {
            var start = content.IndexOf("Minecraft Version:") + "Minecraft Version:".Length;
            var end = content.IndexOf('\n', start);
            summary.MinecraftVersion = end > start ? content[start..end].Trim() : string.Empty;
        }

        // Extract crash cause
        if (content.Contains("Crash Report"))
        {
            summary.HasCrashReport = true;
        }

        // Check for common error types
        if (content.Contains("OutOfMemoryError"))
            summary.ErrorType = "OutOfMemory";
        else if (content.Contains("NullPointerException"))
            summary.ErrorType = "NullPointerException";
        else if (content.Contains("UnsatisfiedLinkError"))
            summary.ErrorType = "NativeLibrary";
        else if (content.Contains("ClassNotFoundException"))
            summary.ErrorType = "MissingClass";
        else if (content.Contains("NoSuchMethodError"))
            summary.ErrorType = "VersionMismatch";
        else
            summary.ErrorType = "Unknown";

        return summary;
    }
}

/// <summary>
/// Summary of a Minecraft crash report analysis.
/// </summary>
public sealed class CrashReportSummary
{
    /// <summary>
    /// The Minecraft version that crashed.
    /// </summary>
    public string MinecraftVersion { get; set; } = string.Empty;

    /// <summary>
    /// Whether this is a valid crash report.
    /// </summary>
    public bool HasCrashReport { get; set; }

    /// <summary>
    /// The type of error detected.
    /// </summary>
    public string ErrorType { get; set; } = "Unknown";

    /// <summary>
    /// Any additional error message.
    /// </summary>
    public string? Error { get; set; }
}
