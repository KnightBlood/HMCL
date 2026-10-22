using System;
using System.IO;
using System.Net.Http;
using System.Text.Json;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCL.NET.Services;

/// <summary>
/// Information about a GitHub release available for update.
/// </summary>
public sealed class UpdateInfo
{
    /// <summary>The release tag (e.g. "v1.2.3").</summary>
    public string TagName { get; set; } = string.Empty;

    /// <summary>The release HTML URL.</summary>
    public string HtmlUrl { get; set; } = string.Empty;

    /// <summary>The release notes / body.</summary>
    public string Body { get; set; } = string.Empty;

    /// <summary>Direct download URL for the .exe asset (if found).</summary>
    public string? ExeDownloadUrl { get; set; }

    /// <summary>Whether this is a pre-release.</summary>
    public bool IsPrerelease { get; set; }

    /// <summary>Parsed semantic version without the leading 'v'.</summary>
    public Version Version { get; set; } = new(0, 0, 0);
}

/// <summary>
/// Checks for updates via the GitHub Releases API and downloads the new executable.
/// </summary>
public sealed class UpdateChecker
{
    private const string RepoOwner = "KnightBlood";
    private const string RepoName = "HMCL";

    /// <summary>
    /// Creates a new <see cref="UpdateChecker"/>.
    /// </summary>
    public UpdateChecker()
    {
        _httpClient = HttpClientProvider.CreateClient();
        _httpClient.BaseAddress = new Uri("https://api.github.com/");
        _httpClient.DefaultRequestHeaders.Add("Accept", "application/vnd.github+json");
    }

    private readonly HttpClient _httpClient;

    /// <summary>
    /// Checks whether a newer release is available on GitHub.
    /// </summary>
    /// <param name="currentVersion">The currently running assembly version.</param>
    /// <param name="includePrerelease">
    /// Whether the pre-releases are taken into account; the development
    /// channel uses them, the stable channel does not.
    /// </param>
    /// <returns>Update info if a newer release exists, otherwise null.</returns>
    public async Task<UpdateInfo?> CheckForUpdateAsync(Version currentVersion, bool includePrerelease = false)
    {
        try
        {
            var url = includePrerelease
                ? $"repos/{RepoOwner}/{RepoName}/releases"
                : $"repos/{RepoOwner}/{RepoName}/releases/latest";

            var response = await _httpClient.GetAsync(url);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;

            if (includePrerelease)
            {
                // The list of the releases is ordered by the creation date, so
                // the first suitable entry is the newest one.
                if (root.ValueKind != JsonValueKind.Array)
                    return null;

                foreach (var element in root.EnumerateArray())
                {
                    var info = ParseRelease(element, currentVersion, includePrerelease: true);
                    if (info is not null)
                        return info;
                }

                return null;
            }

            return ParseRelease(root, currentVersion, includePrerelease: false);
        }
        catch
        {
            // Best-effort update check; ignore all failures.
            return null;
        }
    }

    /// <summary>
    /// Parses the information of a release.
    /// </summary>
    /// <param name="element">The JSON object of the release.</param>
    /// <param name="currentVersion">The currently running assembly version.</param>
    /// <param name="includePrerelease">Whether the pre-releases are accepted.</param>
    /// <returns>The information of the release, or null when it is not newer.</returns>
    private static UpdateInfo? ParseRelease(JsonElement element, Version currentVersion, bool includePrerelease)
    {
        if (element.ValueKind != JsonValueKind.Object)
            return null;

        var tagName = element.TryGetProperty("tag_name", out var tag) ? tag.GetString() ?? string.Empty : string.Empty;
        if (string.IsNullOrWhiteSpace(tagName))
            return null;

        var isPrerelease = element.TryGetProperty("prerelease", out var prerelease) &&
                           prerelease.ValueKind == JsonValueKind.True;

        if (isPrerelease && !includePrerelease)
            return null;

        if (!Version.TryParse(tagName.Trim().TrimStart('v'), out var latestVersion))
            return null;

        if (latestVersion <= currentVersion)
            return null;

        var htmlUrl = element.TryGetProperty("html_url", out var html) ? html.GetString() ?? string.Empty : string.Empty;
        var body = element.TryGetProperty("body", out var notes) ? notes.GetString() ?? string.Empty : string.Empty;

        string? exeUrl = null;
        if (element.TryGetProperty("assets", out var assets) && assets.ValueKind == JsonValueKind.Array)
        {
            foreach (var asset in assets.EnumerateArray())
            {
                var name = asset.TryGetProperty("name", out var assetName)
                    ? assetName.GetString() ?? string.Empty
                    : string.Empty;
                if (name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
                {
                    exeUrl = asset.TryGetProperty("browser_download_url", out var url)
                        ? url.GetString()
                        : null;
                    break;
                }
            }
        }

        return new UpdateInfo
        {
            TagName = tagName,
            HtmlUrl = htmlUrl,
            Body = body,
            ExeDownloadUrl = exeUrl,
            IsPrerelease = isPrerelease,
            Version = latestVersion
        };
    }

    /// <summary>
    /// Downloads the update executable to a temporary file.
    /// </summary>
    /// <param name="downloadUrl">The direct download URL.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The path to the downloaded file.</returns>
    public async Task<string> DownloadUpdateAsync(string downloadUrl, CancellationToken cancellationToken = default)
    {
        var tempPath = Path.Combine(Path.GetTempPath(), $"HMCL.Update.{Guid.NewGuid():N}.exe");
        using var response = await _httpClient.GetAsync(downloadUrl, HttpCompletionOption.ResponseHeadersRead, cancellationToken);
        response.EnsureSuccessStatusCode();

        await using var fs = new FileStream(tempPath, FileMode.Create, FileAccess.Write, FileShare.None);
        await using var stream = await response.Content.ReadAsStreamAsync(cancellationToken);
        await stream.CopyToAsync(fs, cancellationToken);

        return tempPath;
    }
}
