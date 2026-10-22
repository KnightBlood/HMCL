using System;
using System.Collections.Generic;
using System.Formats.Tar;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Util;
using HMCLCore.NET.Util.Versioning;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Java;

/// <summary>
/// A Java runtime offered by the Disco API of foojay.io.
/// </summary>
public sealed class DiscoJavaRuntime
{
    /// <summary>
    /// The distribution publishing the runtime.
    /// </summary>
    public DiscoJavaDistribution Distribution { get; init; }

    /// <summary>
    /// The identifier of the package in the Disco API.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The full Java version, for example <c>17.0.20.1+1</c>.
    /// </summary>
    public string JavaVersion { get; init; } = string.Empty;

    /// <summary>
    /// The version of the distribution, for example <c>17.0.20.1</c>.
    /// </summary>
    public string DistributionVersion { get; init; } = string.Empty;

    /// <summary>
    /// The major Java version of the runtime.
    /// </summary>
    public int JdkVersion { get; init; }

    /// <summary>
    /// The major version of the Java specification the runtime implements.
    /// </summary>
    public int MajorVersion { get; init; }

    /// <summary>
    /// The package type of the runtime, either <c>jdk</c> or <c>jre</c>.
    /// </summary>
    public string PackageType { get; init; } = string.Empty;

    /// <summary>
    /// Whether the runtime bundles JavaFX.
    /// </summary>
    public bool JavaFXBundled { get; init; }

    /// <summary>
    /// The name of the archive holding the runtime.
    /// </summary>
    public string FileName { get; init; } = string.Empty;

    /// <summary>
    /// The URL of the archive holding the runtime.
    /// </summary>
    public string DownloadUrl { get; init; } = string.Empty;

    /// <summary>
    /// The size of the archive in bytes.
    /// </summary>
    public long Size { get; init; }

    /// <summary>
    /// The release status of the runtime, for example <c>ga</c>.
    /// </summary>
    public string ReleaseStatus { get; init; } = string.Empty;

    /// <summary>
    /// The term of support of the runtime, for example <c>lts</c>.
    /// </summary>
    public string TermOfSupport { get; init; } = string.Empty;

    /// <summary>
    /// Whether the runtime is a long term support release.
    /// </summary>
    public bool IsLTS => string.Equals(TermOfSupport, "lts", StringComparison.OrdinalIgnoreCase);

    /// <summary>
    /// A label describing the runtime, shown in the user interface.
    /// </summary>
    public string DisplayName
    {
        get
        {
            var name = string.IsNullOrEmpty(JavaVersion) ? DistributionVersion : JavaVersion;
            var type = string.Equals(PackageType, "jre", StringComparison.OrdinalIgnoreCase) ? "JRE" : "JDK";
            var suffix = IsLTS ? " LTS" : string.Empty;
            return $"{DiscoJavaDistributions.GetDisplayName(Distribution)} {type} {name}{suffix}";
        }
    }
}

/// <summary>
/// Downloads the Java runtimes published through the Disco API of foojay.io
/// and installs them into a directory of the launcher.
/// </summary>
/// <remarks>
/// The Disco API is the service behind the
/// <a href="https://github.com/foojayio/discoapi">discoapi</a> project; it
/// aggregates the download metadata of the Java distributions of several
/// vendors.
/// </remarks>
public static class DiscoJavaRuntimeDownloader
{
    /// <summary>
    /// The root URL of the Disco API.
    /// </summary>
    public const string ApiRoot = "https://api.foojay.io/disco/v3.0";

    private static readonly HttpClient HttpClient = CreateHttpClient();

    /// <summary>
    /// Returns the Java runtimes the given distribution publishes for the
    /// current platform.
    /// </summary>
    /// <param name="distribution">The distribution to query.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>
    /// The available runtimes, newest Java version first.
    /// </returns>
    public static async Task<List<DiscoJavaRuntime>> GetAvailableRuntimesAsync(
        DiscoJavaDistribution distribution,
        CancellationToken token = default)
    {
        var os = Platform.CurrentOs;
        var arch = Platform.CurrentArch;

        if (os == OsType.Unknown || !DiscoJavaDistributions.Supports(distribution, os, arch))
            return new List<DiscoJavaRuntime>();

        var archiveType = os == OsType.Windows ? "zip" : "tar.gz";
        var parameters = new List<string>
        {
            "distribution=" + Uri.EscapeDataString(DiscoJavaDistributions.GetApiParameter(distribution)),
            "operating_system=" + Uri.EscapeDataString(GetApiOsName(os)),
            "architecture=" + Uri.EscapeDataString(GetApiArchName(arch)),
            "archive_type=" + Uri.EscapeDataString(archiveType),
            "directly_downloadable=true"
        };

        if (os == OsType.Linux)
            parameters.Add("lib_c_type=glibc");

        var url = $"{ApiRoot}/packages?{string.Join("&", parameters)}";
        var json = await HttpClientProvider.Shared.GetStringAsync(url, token);

        return ParseRuntimes(json, distribution, archiveType);
    }

    /// <summary>
    /// Parses the response of the packages endpoint of the Disco API.
    /// </summary>
    /// <param name="json">The JSON document returned by the API.</param>
    /// <param name="distribution">The distribution the runtimes belong to.</param>
    /// <param name="archiveType">The archive type the runtimes have to use.</param>
    /// <returns>
    /// The runtimes of the distribution, newest Java version first.
    /// </returns>
    public static List<DiscoJavaRuntime> ParseRuntimes(string json, DiscoJavaDistribution distribution, string archiveType)
    {
        var result = new List<DiscoJavaRuntime>();

        using var document = JsonDocument.Parse(json);
        if (!document.RootElement.TryGetProperty("result", out var resultElement) ||
            resultElement.ValueKind != JsonValueKind.Array)
        {
            return result;
        }

        var apiParameter = DiscoJavaDistributions.GetApiParameter(distribution);

        foreach (var element in resultElement.EnumerateArray())
        {
            var runtime = ParseRuntime(element, distribution, apiParameter, archiveType);
            if (runtime != null)
                result.Add(runtime);
        }

        // Only the newest build of every Java version is offered, the same way
        // the official launcher does.
        var latest = new Dictionary<int, DiscoJavaRuntime>();
        foreach (var runtime in result)
        {
            if (!latest.TryGetValue(runtime.JdkVersion, out var existing) ||
                VersionNumber.Compare(runtime.DistributionVersion, existing.DistributionVersion) > 0)
            {
                latest[runtime.JdkVersion] = runtime;
            }
        }

        return latest.Values
            .OrderByDescending(r => r.JdkVersion)
            .ThenByDescending(r => r.DistributionVersion, StringComparer.Ordinal)
            .ToList();
    }

    /// <summary>
    /// Parses a single package of the Disco API.
    /// </summary>
    /// <param name="element">The JSON element describing the package.</param>
    /// <param name="distribution">The distribution the package belongs to.</param>
    /// <param name="apiParameter">The API parameter of the distribution.</param>
    /// <param name="archiveType">The archive type the package has to use.</param>
    /// <returns>The runtime, or null when the package has to be skipped.</returns>
    private static DiscoJavaRuntime? ParseRuntime(
        JsonElement element,
        DiscoJavaDistribution distribution,
        string apiParameter,
        string archiveType)
    {
        if (!string.Equals(GetString(element, "distribution"), apiParameter, StringComparison.OrdinalIgnoreCase))
            return null;

        if (!GetBoolean(element, "directly_downloadable"))
            return null;

        if (!string.Equals(GetString(element, "archive_type"), archiveType, StringComparison.OrdinalIgnoreCase))
            return null;

        var fileName = GetString(element, "filename");
        var packageType = GetString(element, "package_type");

        // The lite builds of Liberica do not contain a full runtime.
        if (distribution == DiscoJavaDistribution.Liberica &&
            (fileName.EndsWith("-lite.tar.gz", StringComparison.OrdinalIgnoreCase) ||
             fileName.EndsWith("-lite.zip", StringComparison.OrdinalIgnoreCase)))
        {
            return null;
        }

        if (string.Equals(packageType, "jdk", StringComparison.OrdinalIgnoreCase))
        {
            if (!DiscoJavaDistributions.SupportsJdk(distribution))
                return null;
        }
        else if (string.Equals(packageType, "jre", StringComparison.OrdinalIgnoreCase))
        {
            if (!DiscoJavaDistributions.SupportsJre(distribution))
                return null;
        }
        else
        {
            return null;
        }

        var downloadUrl = string.Empty;
        if (element.TryGetProperty("links", out var links) && links.ValueKind == JsonValueKind.Object)
            downloadUrl = GetString(links, "pkg_download_redirect");

        if (string.IsNullOrEmpty(downloadUrl))
            return null;

        return new DiscoJavaRuntime
        {
            Distribution = distribution,
            Id = GetString(element, "id"),
            JavaVersion = GetString(element, "java_version"),
            DistributionVersion = GetString(element, "distribution_version"),
            JdkVersion = GetInt(element, "jdk_version"),
            MajorVersion = GetInt(element, "major_version"),
            PackageType = packageType,
            JavaFXBundled = GetBoolean(element, "javafx_bundled"),
            FileName = fileName,
            DownloadUrl = downloadUrl,
            Size = GetInt(element, "size"),
            ReleaseStatus = GetString(element, "release_status"),
            TermOfSupport = GetString(element, "term_of_support")
        };
    }

    /// <summary>
    /// Downloads the archive of the given runtime and installs it into the
    /// destination directory.
    /// </summary>
    /// <param name="runtime">The runtime to install.</param>
    /// <param name="destinationDirectory">The directory receiving the runtime files.</param>
    /// <param name="progress">Receives progress updates.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The path of the installed Java executable, or null on failure.</returns>
    public static async Task<string?> InstallAsync(
        DiscoJavaRuntime runtime,
        string destinationDirectory,
        IProgress<JavaDownloadProgress>? progress = null,
        CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(destinationDirectory) || string.IsNullOrEmpty(runtime.DownloadUrl))
            return null;

        var workingDirectory = Path.Combine(Path.GetTempPath(), "hmcl-disco-" + Guid.NewGuid().ToString("N"));
        var archivePath = Path.Combine(workingDirectory, runtime.FileName);
        var extractDirectory = Path.Combine(workingDirectory, "extract");

        try
        {
            Directory.CreateDirectory(workingDirectory);
            Directory.CreateDirectory(extractDirectory);

            await DownloadAsync(runtime, archivePath, progress, token);

            progress?.Report(new JavaDownloadProgress
            {
                CompletedFiles = 0,
                TotalFiles = 1,
                CurrentFile = "解压 " + runtime.FileName,
                CompletedBytes = runtime.Size,
                TotalBytes = runtime.Size
            });

            if (runtime.FileName.EndsWith(".zip", StringComparison.OrdinalIgnoreCase))
                ExtractZip(archivePath, extractDirectory);
            else
                ExtractTarGz(archivePath, extractDirectory);

            var sourceRoot = GetArchiveRoot(extractDirectory);
            CopyDirectory(sourceRoot, destinationDirectory);

            var executable = FindJavaExecutable(destinationDirectory);
            if (executable != null)
                MakeExecutable(executable);

            progress?.Report(new JavaDownloadProgress
            {
                CompletedFiles = 1,
                TotalFiles = 1,
                CurrentFile = runtime.FileName,
                CompletedBytes = runtime.Size,
                TotalBytes = runtime.Size
            });

            return executable;
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch
        {
            return null;
        }
        finally
        {
            try
            {
                if (Directory.Exists(workingDirectory))
                    Directory.Delete(workingDirectory, true);
            }
            catch (IOException)
            {
                // The temporary files could not be removed; they are left in the
                // temporary directory of the system.
            }
        }
    }

    /// <summary>
    /// Downloads the archive of a runtime.
    /// </summary>
    /// <param name="runtime">The runtime to download.</param>
    /// <param name="archivePath">The path of the archive to write.</param>
    /// <param name="progress">Receives progress updates.</param>
    /// <param name="token">Cancellation token.</param>
    private static async Task DownloadAsync(
        DiscoJavaRuntime runtime,
        string archivePath,
        IProgress<JavaDownloadProgress>? progress,
        CancellationToken token)
    {
        using var response = await HttpClientProvider.Shared.GetAsync(
            runtime.DownloadUrl, HttpCompletionOption.ResponseHeadersRead, token);
        response.EnsureSuccessStatusCode();

        var totalBytes = response.Content.Headers.ContentLength ?? runtime.Size;
        var completedBytes = 0L;

        await using var source = await response.Content.ReadAsStreamAsync(token);
        await using var destination = File.Create(archivePath);

        var buffer = new byte[81920];
        int read;
        while ((read = await source.ReadAsync(buffer, token)) > 0)
        {
            await destination.WriteAsync(buffer.AsMemory(0, read), token);
            completedBytes += read;

            progress?.Report(new JavaDownloadProgress
            {
                CompletedFiles = 0,
                TotalFiles = 1,
                CurrentFile = runtime.FileName,
                CompletedBytes = completedBytes,
                TotalBytes = totalBytes
            });
        }
    }

    /// <summary>
    /// Extracts a ZIP archive.
    /// </summary>
    /// <param name="archivePath">The archive to extract.</param>
    /// <param name="targetDirectory">The directory receiving the files.</param>
    private static void ExtractZip(string archivePath, string targetDirectory)
    {
        ZipFile.ExtractToDirectory(archivePath, targetDirectory, true);
    }

    /// <summary>
    /// Extracts a GZip compressed TAR archive.
    /// </summary>
    /// <param name="archivePath">The archive to extract.</param>
    /// <param name="targetDirectory">The directory receiving the files.</param>
    private static void ExtractTarGz(string archivePath, string targetDirectory)
    {
        using var fileStream = File.OpenRead(archivePath);
        using var gzipStream = new GZipStream(fileStream, CompressionMode.Decompress);
        TarFile.ExtractToDirectory(gzipStream, targetDirectory, true);
    }

    /// <summary>
    /// Returns the directory holding the files of the extracted archive.
    /// </summary>
    /// <param name="extractDirectory">The directory the archive was extracted to.</param>
    /// <returns>The directory holding the files of the runtime.</returns>
    private static string GetArchiveRoot(string extractDirectory)
    {
        var entries = Directory.GetFileSystemEntries(extractDirectory);
        if (entries.Length == 1 && Directory.Exists(entries[0]))
            return entries[0];

        return extractDirectory;
    }

    /// <summary>
    /// Copies the content of a directory into another one.
    /// </summary>
    /// <param name="sourceDirectory">The directory to copy from.</param>
    /// <param name="targetDirectory">The directory to copy to.</param>
    private static void CopyDirectory(string sourceDirectory, string targetDirectory)
    {
        Directory.CreateDirectory(targetDirectory);

        foreach (var directory in Directory.GetDirectories(sourceDirectory, "*", SearchOption.AllDirectories))
        {
            var relative = Path.GetRelativePath(sourceDirectory, directory);
            Directory.CreateDirectory(Path.Combine(targetDirectory, relative));
        }

        foreach (var file in Directory.GetFiles(sourceDirectory, "*", SearchOption.AllDirectories))
        {
            var relative = Path.GetRelativePath(sourceDirectory, file);
            var target = Path.Combine(targetDirectory, relative);
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            File.Copy(file, target, true);
        }
    }

    /// <summary>
    /// Searches the Java executable of an installed runtime.
    /// </summary>
    /// <param name="directory">The directory holding the runtime.</param>
    /// <returns>The path of the Java executable, or null when it was not found.</returns>
    private static string? FindJavaExecutable(string directory)
    {
        var executableName = Platform.JavaExecutable;

        var candidate = Path.Combine(directory, "bin", executableName);
        if (File.Exists(candidate))
            return candidate;

        foreach (var file in Directory.GetFiles(directory, executableName, SearchOption.AllDirectories))
            return file;

        return null;
    }

    /// <summary>
    /// Marks a file as executable on Unix-like systems.
    /// </summary>
    /// <param name="path">The file to mark.</param>
    private static void MakeExecutable(string path)
    {
        if (OperatingSystem.IsWindows())
            return;

        try
        {
            var mode = File.GetUnixFileMode(path);
            File.SetUnixFileMode(path,
                mode | UnixFileMode.UserExecute | UnixFileMode.GroupExecute | UnixFileMode.OtherExecute);
        }
        catch (IOException)
        {
            // Best effort: some file systems do not support the file modes.
        }
        catch (UnauthorizedAccessException)
        {
            // Best effort: the file modes are not accessible.
        }
    }

    /// <summary>
    /// Returns the operating system name used by the Disco API.
    /// </summary>
    /// <param name="os">The operating system of the platform.</param>
    /// <returns>The API name of the operating system.</returns>
    private static string GetApiOsName(OsType os) => os switch
    {
        OsType.Windows => "windows",
        OsType.Linux => "linux",
        OsType.MacOs => "macos",
        OsType.FreeBsd => "freebsd",
        _ => "linux"
    };

    /// <summary>
    /// Returns the architecture name used by the Disco API.
    /// </summary>
    /// <param name="arch">The architecture of the platform.</param>
    /// <returns>The API name of the architecture.</returns>
    private static string GetApiArchName(Arch arch) => arch switch
    {
        Arch.X86_64 => "x64",
        Arch.X86 => "x86",
        Arch.ARM64 => "aarch64",
        Arch.ARM32 => "arm",
        Arch.RISCV64 => "riscv64",
        Arch.RISCV32 => "riscv32",
        Arch.PPC64 => "ppc64",
        Arch.PPC64LE => "ppc64le",
        Arch.S390X => "s390x",
        _ => "x64"
    };

    /// <summary>
    /// Reads a string property of a JSON element.
    /// </summary>
    /// <param name="element">The element to read.</param>
    /// <param name="name">The name of the property.</param>
    /// <returns>The value of the property, or an empty string.</returns>
    private static string GetString(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() ?? string.Empty
            : string.Empty;

    /// <summary>
    /// Reads a boolean property of a JSON element.
    /// </summary>
    /// <param name="element">The element to read.</param>
    /// <param name="name">The name of the property.</param>
    /// <returns>The value of the property, or false.</returns>
    private static bool GetBoolean(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) &&
        (value.ValueKind == JsonValueKind.True || value.ValueKind == JsonValueKind.String && value.GetString() == "true");

    /// <summary>
    /// Reads an integer property of a JSON element.
    /// </summary>
    /// <param name="element">The element to read.</param>
    /// <param name="name">The name of the property.</param>
    /// <returns>The value of the property, or zero.</returns>
    private static int GetInt(JsonElement element, string name) =>
        element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.Number && value.TryGetInt32(out var result)
            ? result
            : 0;

    /// <summary>
    /// Creates the shared HTTP client with a descriptive user agent.
    /// </summary>
    /// <returns>The HTTP client used to talk to the Disco API.</returns>
    private static HttpClient CreateHttpClient()
    {
        var client = HttpClientProvider.CreateClient();
        client.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");
        return client;
    }
}
