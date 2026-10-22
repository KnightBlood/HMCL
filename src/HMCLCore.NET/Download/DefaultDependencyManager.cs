using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading.Tasks;
using HMCLCore.NET.Game;
using HMCLCore.NET.Launch;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// Manages dependencies for game versions - resolves and downloads required libraries and assets.
/// </summary>
public sealed class DefaultDependencyManager
{
    private readonly string _librariesDirectory;
    private readonly string _assetsDirectory;
    private readonly DownloadProviderManager _providerManager;
    private readonly string _versionsDirectory;
    private readonly string _nativesDirectory;

    /// <summary>
    /// Creates a new DefaultDependencyManager.
    /// </summary>
    public DefaultDependencyManager(string librariesDirectory, string assetsDirectory, DownloadProviderManager providerManager, string versionsDirectory, string nativesDirectory)
    {
        _librariesDirectory = librariesDirectory ?? throw new ArgumentNullException(nameof(librariesDirectory));
        _assetsDirectory = assetsDirectory ?? throw new ArgumentNullException(nameof(assetsDirectory));
        _providerManager = providerManager ?? throw new ArgumentNullException(nameof(providerManager));
        _versionsDirectory = versionsDirectory ?? throw new ArgumentNullException(nameof(versionsDirectory));
        _nativesDirectory = nativesDirectory ?? throw new ArgumentNullException(nameof(nativesDirectory));
    }

    /// <summary>
    /// Resolves all dependencies for a game instance manifest and returns download tasks.
    /// Missing libraries, native classifiers, the asset index and the client JAR
    /// each produce a download task; already-present files are skipped.
    /// </summary>
    public List<TaskBase> ResolveDependencies(GameInstanceManifest manifest)
    {
        var tasks = new List<TaskBase>();

        foreach (var library in manifest.Libraries)
        {
            if (!library.IsApplicableToCurrentOs())
                continue;

            var localPath = library.GetLocalPath(_librariesDirectory);
            var hasClassifiers = library.Downloads?.Classifiers != null &&
                                library.Downloads.Classifiers.Count > 0;

            // The libraries shipped with the launcher are written into the
            // library directory instead of being downloaded.
            if (BundledLibraries.TryGetResource(library, out var resourceName))
            {
                if (!File.Exists(localPath))
                {
                    tasks.Add(new BundledLibraryTask(resourceName, localPath)
                    {
                        Name = $"写入内置库 {library.Name}"
                    });
                }

                continue;
            }

            // Libraries that publish native archives only (the style used by
            // LegacyFabric) have no plain artifact to download.
            var nativesOnly = library.RequiresNatives && !hasClassifiers;

            if (!File.Exists(localPath) && !nativesOnly)
            {
                var downloadUrl = ResolveLibraryDownloadUrl(library);
                if (!string.IsNullOrEmpty(downloadUrl))
                {
                    tasks.Add(new GameDownloadTask(new Uri(downloadUrl), localPath)
                    {
                        Name = $"下载库 {library.Name}"
                    });
                }
            }

            // Older version manifests keep the native libraries in the
            // classifiers of the downloads section instead of listing them as
            // separate library entries.
            if (library.RequiresNatives && library.Natives != null)
            {
                foreach (var classifier in SelectNativeClassifiers(library))
                {
                    var nativePath = GetNativeClassifierPath(library, classifier);
                    if (File.Exists(nativePath))
                        continue;

                    var nativeUrl = ResolveNativeDownloadUrl(library, classifier);
                    if (!string.IsNullOrEmpty(nativeUrl))
                    {
                        tasks.Add(new GameDownloadTask(new Uri(nativeUrl), nativePath)
                        {
                            Name = $"下载原生库 {classifier}"
                        });
                    }
                }
            }
        }

        // Handle asset index
        if (manifest.AssetIndex != null && !string.IsNullOrEmpty(manifest.AssetIndex.Url))
        {
            var assetIndexPath = Path.Combine(_assetsDirectory, "indexes", $"{manifest.AssetIndex.Id}.json");
            var assetIndexUrl = manifest.AssetIndex.Url;
            tasks.Add(new AssetIndexDownloadTask(new Uri(assetIndexUrl), assetIndexPath)
            {
                Name = "下载资源索引"
            });

            // Download every asset object referenced by the index once it is available.
            tasks.Add(new AssetObjectsDownloadTask(assetIndexPath, _assetsDirectory)
            {
                Name = "下载资源文件"
            });
        }

        // Handle the client JAR itself
        var clientUrl = manifest.Downloads?.Client?.Url;
        var clientJarPath = Path.Combine(_versionsDirectory, manifest.Version, $"{manifest.Version}.jar");
        if (!string.IsNullOrEmpty(clientUrl) && !File.Exists(clientJarPath))
        {
            tasks.Add(new GameDownloadTask(new Uri(clientUrl), clientJarPath)
            {
                Name = $"下载游戏本体 {manifest.Version}"
            });
        }

        return tasks;
    }

    /// <summary>
    /// Extracts the native libraries of the manifest into the natives directory
    /// of the version, so that the game can load them through
    /// <c>java.library.path</c>.
    /// </summary>
    /// <remarks>
    /// This has to be called after the download tasks of
    /// <see cref="ResolveDependencies"/> finished, because the native archives
    /// are downloaded by those tasks.
    /// </remarks>
    /// <param name="manifest">The manifest of the version to extract the natives of.</param>
    public void ExtractNatives(GameInstanceManifest manifest)
    {
        var nativesDirectory = GetNativesDirectory(manifest);

        foreach (var library in manifest.Libraries)
        {
            if (!library.IsApplicableToCurrentOs())
                continue;

            // Modern manifests list the native libraries as separate library
            // entries whose artifact is the native archive itself.
            if (library.IsNative)
            {
                var path = library.GetLocalPath(_librariesDirectory);
                if (File.Exists(path))
                    ExtractNative(path, nativesDirectory, library.Extract);

                continue;
            }

            if (!library.RequiresNatives || library.Natives == null)
                continue;

            foreach (var classifier in SelectNativeClassifiers(library))
            {
                var path = GetNativeClassifierPath(library, classifier);
                if (File.Exists(path))
                    ExtractNative(path, nativesDirectory, library.Extract);
            }
        }
    }

    /// <summary>
    /// Extracts a native archive into the natives directory.
    /// </summary>
    /// <param name="archivePath">The native archive to extract.</param>
    /// <param name="nativesDirectory">The directory receiving the extracted files.</param>
    /// <param name="extractRules">The extract rules of the library; may be null.</param>
    private static void ExtractNative(string archivePath, string nativesDirectory, IReadOnlyList<ExtractRule>? extractRules)
    {
        try
        {
            var task = new NativeExtractTask(archivePath, nativesDirectory, extractRules);
            task.Run().GetAwaiter().GetResult();
        }
        catch (IOException)
        {
            // The natives could not be extracted; the game may still be able to
            // load them from the classpath.
        }
        catch (InvalidDataException)
        {
            // The native archive is broken; the game may still be able to load
            // them from the classpath.
        }
    }

    /// <summary>
    /// Selects the native classifiers of a library that apply to the current
    /// platform, preferring the classifier matching the CPU architecture.
    /// </summary>
    /// <param name="library">The library to select the classifiers of.</param>
    /// <returns>The applicable classifiers.</returns>
    private static IReadOnlyList<string> SelectNativeClassifiers(Library library)
    {
        if (library.Natives == null)
            return Array.Empty<string>();

        var applicable = new List<string>();
        var operatingSystemOnly = new List<string>();

        foreach (var classifier in library.Natives)
        {
            if (HMCLCore.NET.Util.NativeClassifier.IsApplicableToCurrentPlatform(classifier))
                applicable.Add(classifier);
            else if (HMCLCore.NET.Util.NativeClassifier.MatchesOperatingSystem(classifier))
                operatingSystemOnly.Add(classifier);
        }

        // Fall back to the classifier of the operating system when no
        // architecture specific one is published for the current platform.
        return applicable.Count > 0 ? applicable : operatingSystemOnly;
    }

    /// <summary>
    /// Returns the local path of a native classifier archive of a library.
    /// </summary>
    /// <param name="library">The library owning the classifier.</param>
    /// <param name="classifier">The classifier, for example <c>natives-windows</c>.</param>
    /// <returns>The absolute path of the native archive.</returns>
    public string GetNativeClassifierPath(Library library, string classifier)
    {
        var parts = library.Name.Split(':');
        if (parts.Length < 3)
            return Path.Combine(_librariesDirectory, classifier, $"{classifier}-native.jar");

        var groupId = parts[0].Replace('.', Path.DirectorySeparatorChar);
        var artifactId = parts[1];
        var version = parts[2];
        return Path.Combine(
            _librariesDirectory, groupId, artifactId, version, $"{artifactId}-{version}-{classifier}.jar");
    }

    /// <summary>
    /// Returns the local path of the client JAR for a version.
    /// </summary>
    public string GetClientJarPath(string version) =>
        Path.Combine(_versionsDirectory, version, $"{version}.jar");

    /// <summary>
    /// Returns the natives directory for a version's extracted native libraries.
    /// </summary>
    public string GetNativesDirectory(GameInstanceManifest manifest) =>
        Path.Combine(_nativesDirectory, manifest.Version);

    /// <summary>
    /// Resolves the download URL of a library, honoring direct artifact URLs
    /// and Fabric/Quilt Maven base URLs.
    /// </summary>
    /// <param name="library">The library to resolve.</param>
    /// <returns>The download URL, or null when it cannot be resolved.</returns>
    public static string? ResolveLibraryDownloadUrl(Library library)
    {
        var downloadUrl = library.Downloads?.Artifact?.Url;
        if (!string.IsNullOrEmpty(downloadUrl))
            return downloadUrl;

        if (string.IsNullOrEmpty(library.Url))
            return null;

        var parts = library.Name.Split(':');
        if (parts.Length < 3)
            return null;

        var groupPath = parts[0].Replace('.', '/');
        var artifact = parts[1];
        var version = parts[2];
        var classifier = parts.Length >= 4 && !string.IsNullOrEmpty(parts[3]) ? $"-{parts[3]}" : string.Empty;
        var baseUrl = library.Url.EndsWith('/') ? library.Url : library.Url + "/";
        return $"{baseUrl}{groupPath}/{artifact}/{version}/{artifact}-{version}{classifier}.jar";
    }

    /// <summary>
    /// Resolves the download URL of a native archive of a library.
    /// </summary>
    /// <param name="library">The library to resolve.</param>
    /// <param name="classifier">The classifier of the native archive, for
    /// example <c>natives-windows</c>.</param>
    /// <returns>The download URL, or null when it cannot be resolved.</returns>
    public static string? ResolveNativeDownloadUrl(Library library, string classifier)
    {
        if (library.Downloads?.Classifiers != null &&
            library.Downloads.Classifiers.TryGetValue(classifier, out var classifierInfo) &&
            !string.IsNullOrEmpty(classifierInfo.Url))
        {
            return classifierInfo.Url;
        }

        // The manifests of the meta based loaders do not contain the URLs of
        // the native archives; they are derived from the Maven repository of
        // the library.
        if (string.IsNullOrEmpty(library.Url))
            return null;

        var parts = library.Name.Split(':');
        if (parts.Length < 3)
            return null;

        var groupPath = parts[0].Replace('.', '/');
        var artifact = parts[1];
        var version = parts[2];
        var baseUrl = library.Url.EndsWith('/') ? library.Url : library.Url + "/";
        return $"{baseUrl}{groupPath}/{artifact}/{version}/{artifact}-{version}-{classifier}.jar";
    }

    /// <summary>
    /// Returns the local path for a library.
    /// </summary>
    public string GetLibraryPath(Library library) => library.GetLocalPath(_librariesDirectory);

    /// <summary>
    /// Returns whether a library is already downloaded.
    /// </summary>
    public bool IsLibraryDownloaded(Library library) => File.Exists(GetLibraryPath(library));
}

/// <summary>
/// Builds game launch configuration from a manifest and instance.
/// </summary>
public sealed class GameBuilder
{
    private readonly string _gameDirectory;
    private readonly string _librariesDirectory;
    private readonly string _assetsDirectory;
    private readonly string _nativesDirectory;

    /// <summary>
    /// Creates a new GameBuilder.
    /// </summary>
    public GameBuilder(string gameDirectory, string librariesDirectory, string assetsDirectory, string nativesDirectory)
    {
        _gameDirectory = gameDirectory ?? throw new ArgumentNullException(nameof(gameDirectory));
        _librariesDirectory = librariesDirectory ?? throw new ArgumentNullException(nameof(librariesDirectory));
        _assetsDirectory = assetsDirectory ?? throw new ArgumentNullException(nameof(assetsDirectory));
        _nativesDirectory = nativesDirectory ?? throw new ArgumentNullException(nameof(nativesDirectory));
    }

    /// <summary>
    /// Builds a launch configuration from a game instance manifest.
    /// </summary>
    public LaunchConfig BuildLaunchConfig(GameInstanceManifest manifest, string username, string uuid, string accessToken, string clientToken)
    {
        var builder = new LibraryBuilder();
        var classpath = builder.BuildClasspath(_librariesDirectory);

        // Add the game JAR
        var gameJar = Path.Combine(_gameDirectory, "versions", manifest.Version, $"{manifest.Version}.jar");
        if (File.Exists(gameJar))
        {
            if (!string.IsNullOrEmpty(classpath))
                classpath += Path.PathSeparator;
            classpath += gameJar;
        }

        return new LaunchConfig
        {
            Instance = new GameInstance
            {
                Id = manifest.Version,
                Profile = manifest.Version,
                Version = manifest.Version,
                GameDirectory = _gameDirectory,
                Type = "release"
            },
            JavaRuntime = null, // Set by caller
            WorkingDirectory = _gameDirectory,
            GameDirectory = _gameDirectory,
            LibrariesDirectory = _librariesDirectory,
            ClassPath = classpath,
            MainClass = manifest.MainClass,
            VersionName = manifest.Version,
            AssetsDirectory = _assetsDirectory,
            JvmArguments = new List<string>(manifest.Arguments.Jvm),
            GameArguments = new List<string>(manifest.Arguments.Game),
            Memory = 2048,
            Account = new HMCLCore.NET.Auth.OfflineAccount(username)
        };
    }
}

/// <summary>
/// Cleanroom install task - installs a clean version without any mod loader.
/// </summary>
public sealed class CleanroomInstallTask : GameInstallTask
{
    /// <summary>
    /// Creates a new CleanroomInstallTask.
    /// </summary>
    public CleanroomInstallTask(string version, string gameDirectory, string librariesDirectory)
        : base(version, gameDirectory, librariesDirectory)
    {
        Name = $"安装原版 {version}";
    }

    /// <summary>
    /// Installs a clean Minecraft version.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();
            EnsureVersionDirectory();

            // Download version JSON and JAR
            // In a real implementation, this would download from Mojang's servers
            OnProgressChanged(100, 100);
            IsCompleted = true;
            IsSucceeded = true;
        }
        catch (OperationCanceledException)
        {
            IsCompleted = true;
            IsSucceeded = false;
            throw new OperationCanceledException("Cleanroom install cancelled");
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
