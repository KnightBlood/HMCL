using System;
using System.Collections.Generic;
using System.Formats.Tar;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Security.Cryptography;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Terracotta;

/// <summary>
/// The Terracotta bundle installed in the directory of the dependencies of the
/// launcher.
/// </summary>
public sealed class TerracottaBundle
{
    private readonly string _root;
    private readonly IReadOnlyList<string> _links;
    private readonly string _hash;
    private readonly IReadOnlyDictionary<string, string> _files;

    private static readonly object _processLock = new();
    private static System.Diagnostics.Process? _runningProcess;

    /// <summary>
    /// Creates the bundle.
    /// </summary>
    /// <param name="root">The directory receiving the files of the bundle.</param>
    /// <param name="links">The addresses of the package.</param>
    /// <param name="hash">The SHA-512 hash of the package.</param>
    /// <param name="files">The SHA-512 hashes of the files of the package.</param>
    public TerracottaBundle(
        string root,
        IReadOnlyList<string> links,
        string hash,
        IReadOnlyDictionary<string, string> files)
    {
        _root = root;
        _links = links;
        _hash = hash;
        _files = files;
    }

    /// <summary>
    /// The directory receiving the files of the bundle.
    /// </summary>
    public string Root => _root;

    /// <summary>
    /// The state of the bundle installed in the directory of the dependencies.
    /// </summary>
    public enum BundleStatus
    {
        /// <summary>
        /// The bundle is not installed.
        /// </summary>
        NotExists,

        /// <summary>
        /// An old version of the bundle is installed.
        /// </summary>
        LegacyVersion,

        /// <summary>
        /// The bundle is installed and up to date.
        /// </summary>
        Ready
    }

    /// <summary>
    /// Returns the state of the bundle installed in the given directory.
    /// </summary>
    /// <param name="dependenciesDirectory">The directory holding the dependencies.</param>
    /// <returns>The state of the bundle.</returns>
    public static BundleStatus GetStatus(string dependenciesDirectory)
    {
        var metadata = TerracottaMetadata.Current;
        if (metadata is null)
            return BundleStatus.NotExists;

        var root = GetRoot(dependenciesDirectory, metadata.LatestVersion);
        if (Directory.Exists(root) && IsLocalBundleValid(root, metadata))
            return BundleStatus.Ready;

        return HasLegacyVersionFiles(dependenciesDirectory, metadata.LatestVersion)
            ? BundleStatus.LegacyVersion
            : BundleStatus.NotExists;
    }

    /// <summary>
    /// Returns the directory of the bundle of the given version.
    /// </summary>
    /// <param name="dependenciesDirectory">The directory holding the dependencies.</param>
    /// <param name="version">The version of the bundle.</param>
    /// <returns>The directory of the bundle.</returns>
    public static string GetRoot(string dependenciesDirectory, string version)
        => Path.Combine(dependenciesDirectory, "terracotta", version);

    /// <summary>
    /// Returns whether the files of an old version of the bundle are still
    /// installed.
    /// </summary>
    /// <param name="dependenciesDirectory">The directory holding the dependencies.</param>
    /// <param name="latestVersion">The newest version of the bundle.</param>
    /// <returns>Whether the old files exist.</returns>
    public static bool HasLegacyVersionFiles(string dependenciesDirectory, string latestVersion)
    {
        try
        {
            var directory = Path.Combine(dependenciesDirectory, "terracotta");
            if (!Directory.Exists(directory))
                return false;

            return Directory.GetDirectories(directory)
                .Select(Path.GetFileName)
                .Any(name => !string.IsNullOrEmpty(name) &&
                             !string.Equals(name, latestVersion, StringComparison.OrdinalIgnoreCase));
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>
    /// Removes the files of the old versions of the bundle.
    /// </summary>
    /// <param name="dependenciesDirectory">The directory holding the dependencies.</param>
    /// <param name="latestVersion">
    /// The newest version of the bundle; its files are kept.
    /// </param>
    public static void RemoveLegacyVersionFiles(string dependenciesDirectory, string? latestVersion = null)
    {
        try
        {
            var directory = Path.Combine(dependenciesDirectory, "terracotta");
            if (!Directory.Exists(directory))
                return;

            foreach (var version in Directory.GetDirectories(directory))
            {
                var name = Path.GetFileName(version);
                if (latestVersion != null &&
                    string.Equals(name, latestVersion, StringComparison.OrdinalIgnoreCase))
                {
                    // The newest version is kept.
                    continue;
                }

                try
                {
                    Directory.Delete(version, true);
                }
                catch (Exception)
                {
                    // A locked directory is kept.
                }
            }
        }
        catch (Exception)
        {
            // The old files stay when they cannot be removed.
        }
    }

    /// <summary>
    /// Verifies the files installed in the given directory.
    /// </summary>
    /// <param name="root">The directory of the bundle.</param>
    /// <param name="metadata">The metadata of the bundles.</param>
    /// <returns>Whether the installed files are up to date.</returns>
    private static bool IsLocalBundleValid(string root, TerracottaMetadata metadata)
    {
        if (TerracottaMetadata.CurrentClassifier is not { } classifier)
            return false;

        if (!metadata.Packages.TryGetValue(classifier, out var package))
            return false;

        foreach (var file in package.Files)
        {
            var path = Path.Combine(root, file.Key);
            if (!File.Exists(path))
                return false;

            try
            {
                var hash = ComputeSha512(path);
                if (!hash.Equals(file.Value, StringComparison.OrdinalIgnoreCase))
                    return false;
            }
            catch (IOException)
            {
                return false;
            }
        }

        return true;
    }

    /// <summary>
    /// Computes the SHA-512 hash of the given file.
    /// </summary>
    /// <param name="path">The path of the file.</param>
    /// <returns>The hash of the file.</returns>
    private static string ComputeSha512(string path)
    {
        using var stream = File.OpenRead(path);
        using var sha = SHA512.Create();
        return Convert.ToHexString(sha.ComputeHash(stream));
    }

    /// <summary>
    /// Creates the bundle of the current platform from the metadata.
    /// </summary>
    /// <param name="dependenciesDirectory">The directory holding the dependencies.</param>
    /// <param name="metadata">The metadata of the bundles.</param>
    /// <returns>The bundle, or null when the platform is not supported.</returns>
    public static TerracottaBundle? Create(
        string dependenciesDirectory, TerracottaMetadata metadata)
    {
        if (TerracottaMetadata.CurrentClassifier is not { } classifier)
            return null;

        if (!metadata.Packages.TryGetValue(classifier, out var package))
            return null;

        return new TerracottaBundle(
            GetRoot(dependenciesDirectory, metadata.LatestVersion),
            metadata.GetDownloadLinks(),
            package.Hash,
            package.Files);
    }

    /// <summary>
    /// Downloads the package of the bundle.
    /// </summary>
    /// <param name="destinationPath">The path of the downloaded package.</param>
    /// <param name="progress">Receives the progress of the download.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>Whether the package was downloaded.</returns>
    public async Task<bool> DownloadAsync(
        string destinationPath, IProgress<double>? progress = null, CancellationToken token = default)
    {
        if (_links.Count == 0)
            return false;

        var directory = Path.GetDirectoryName(destinationPath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        var urls = _links.Select(link => new Uri(link)).ToArray();

        // The package is verified with its SHA-512 hash.
        var succeeded = await FileDownloader.DownloadAsync(
            urls, destinationPath, new FileDownloadOptions { Sha1 = null }, null, token);

        if (!succeeded)
            return false;

        if (string.IsNullOrEmpty(_hash))
            return true;

        var hash = ComputeSha512(destinationPath);
        if (!hash.Equals(_hash, StringComparison.OrdinalIgnoreCase))
        {
            try
            {
                File.Delete(destinationPath);
            }
            catch (IOException)
            {
                // The broken package is kept.
            }

            return false;
        }

        return true;
    }

    /// <summary>
    /// Installs the files of the bundle from the downloaded package.
    /// </summary>
    /// <param name="packagePath">The path of the downloaded package.</param>
    /// <returns>Whether the bundle was installed.</returns>
    public bool Install(string packagePath)
    {
        try
        {
            Directory.CreateDirectory(_root);

            using var file = File.OpenRead(packagePath);
            using var gzip = new GZipStream(file, CompressionMode.Decompress);
            using var tar = new TarReader(gzip);

            while (tar.GetNextEntry() is { } entry)
            {
                var name = entry.Name.TrimStart('/');
                if (string.IsNullOrEmpty(name))
                    continue;

                if (!_files.TryGetValue(name, out var expectedHash))
                    continue;

                var target = Path.Combine(_root, name);
                Directory.CreateDirectory(Path.GetDirectoryName(target)!);

                using (var output = File.Create(target))
                {
                    if (entry.DataStream != null)
                        entry.DataStream.CopyTo(output);
                }

                var hash = ComputeSha512(target);
                if (!hash.Equals(expectedHash, StringComparison.OrdinalIgnoreCase))
                    throw new IOException($"The file {name} of the bundle is broken.");

                if (!OperatingSystem.IsWindows())
                {
                    try
                    {
                        File.SetUnixFileMode(target,
                            UnixFileMode.UserRead | UnixFileMode.UserWrite | UnixFileMode.UserExecute |
                            UnixFileMode.GroupRead | UnixFileMode.GroupExecute |
                            UnixFileMode.OtherRead | UnixFileMode.OtherExecute);
                    }
                    catch (Exception)
                    {
                        // The permissions of the file stay the default ones.
                    }
                }
            }
        }
        catch (Exception)
        {
            // A broken installation leaves no file behind.
            try
            {
                if (Directory.Exists(_root))
                    Directory.Delete(_root, true);
            }
            catch (Exception)
            {
                // The broken files stay.
            }

            return false;
        }

        return true;
    }

    /// <summary>
    /// Stops the service when it is running.
    /// </summary>
    public static void Stop()
    {
        lock (_processLock)
        {
            try
            {
                if (_runningProcess is { HasExited: false })
                    _runningProcess.Kill();
            }
            catch (Exception)
            {
                // The service is already stopped.
            }

            _runningProcess = null;
        }
    }

    /// <summary>
    /// Returns the path of the given file of the bundle.
    /// </summary>
    /// <param name="file">The name of the file.</param>
    /// <returns>The path of the file.</returns>
    public string Locate(string file) => Path.Combine(_root, file);

    /// <summary>
    /// Returns the path of the executable of the bundle on the current
    /// platform.
    /// </summary>
    /// <returns>The path of the executable, or null when it is unknown.</returns>
    public string? LocateExecutable()
    {
        var candidates = _files.Keys
            .Where(name => name.StartsWith("terracotta", StringComparison.OrdinalIgnoreCase))
            .OrderBy(name => name, StringComparer.Ordinal)
            .ToList();

        foreach (var candidate in candidates)
        {
            if (OperatingSystem.IsWindows() && candidate.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                return Locate(candidate);
        }

        foreach (var candidate in candidates)
        {
            if (!candidate.Contains('.'))
                return Locate(candidate);
        }

        return null;
    }

    /// <summary>
    /// Starts the bundle and waits for the port of its local interface.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The port of the local interface of the service.</returns>
    public async Task<int> LaunchAsync(CancellationToken cancellationToken = default)
    {
        var executable = LocateExecutable()
                         ?? throw new InvalidOperationException("The Terracotta bundle has no executable.");

        // The service writes the port of its local interface into this file
        // once it is ready.
        var portTransfer = Path.Combine(
            Path.GetTempPath(), $"hmcl-terracotta-{Guid.NewGuid():N}", "http");
        Directory.CreateDirectory(Path.GetDirectoryName(portTransfer)!);

        var startInfo = new System.Diagnostics.ProcessStartInfo
        {
            FileName = executable,
            WorkingDirectory = Path.GetDirectoryName(executable) ?? string.Empty,
            UseShellExecute = false,
            CreateNoWindow = true,
            RedirectStandardOutput = true,
            RedirectStandardError = true
        };

        startInfo.ArgumentList.Add("--hmcl");
        startInfo.ArgumentList.Add(portTransfer);

        var process = System.Diagnostics.Process.Start(startInfo);
        if (process is null)
            throw new InvalidOperationException("无法启动联机组件");

        // Only one service runs at a time; the previous one is stopped.
        lock (_processLock)
        {
            try
            {
                _runningProcess?.Kill();
            }
            catch (Exception)
            {
                // The previous service is already stopped.
            }

            _runningProcess = process;
        }

        // The output of the service is drained, so that it never blocks on a
        // full pipe.
        _ = Task.Run(async () =>
        {
            try
            {
                await process.StandardOutput.ReadToEndAsync(cancellationToken);
            }
            catch (Exception)
            {
                // The service was stopped.
            }
        }, cancellationToken);

        _ = Task.Run(async () =>
        {
            try
            {
                await process.StandardError.ReadToEndAsync(cancellationToken);
            }
            catch (Exception)
            {
                // The service was stopped.
            }
        }, cancellationToken);

        var deadline = DateTime.UtcNow.AddSeconds(30);

        while (DateTime.UtcNow < deadline)
        {
            cancellationToken.ThrowIfCancellationRequested();

            // The service writes the file holding its port once it is ready.
            if (File.Exists(portTransfer))
            {
                try
                {
                    var json = await File.ReadAllTextAsync(portTransfer, cancellationToken);
                    using var document = System.Text.Json.JsonDocument.Parse(json);

                    if (document.RootElement.TryGetProperty("port", out var port))
                        return port.GetInt32();
                }
                catch (System.Text.Json.JsonException)
                {
                    // The file is not written yet.
                }
                catch (IOException)
                {
                    // The file is being written.
                }
            }

            if (process.HasExited)
                throw new InvalidOperationException("联机组件已退出");

            await Task.Delay(200, cancellationToken);
        }

        throw new TimeoutException("联机组件启动超时");
    }
}
