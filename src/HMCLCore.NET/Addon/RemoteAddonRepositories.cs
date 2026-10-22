using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net.Http;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Serialization;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Addon;

/// <summary>
/// Extensions around remote addon repositories for resolaing concrete files.
/// </summary>
public abstract class RemoteAddonRepositoryEx : RemoteAddonRepository
{
    /// <summary>
    /// Resolves the concrete downloadable file for an addon, honoring the
    /// target Minecraft version and mod loader when possible.
    /// </summary>
    /// <param name="addon">The addon to resolve.</param>
    /// <param name="minecraftVersion">The Minecraft version of the target instance.</param>
    /// <param name="loader">The mod loader of the target instance (may be None).</param>
    /// <returns>An addon with <see cref="RemoteAddon.DownloadUrl"/> and version set, or null when no file matches.</returns>
    public abstract Task<RemoteAddon?> ResolveFileAsync(RemoteAddon addon, string minecraftVersion, ModLoaderType loader);
}

/// <summary>
/// Remote addon repository backed by the public Modrinth API v2.
/// </summary>
public sealed class ModrinthRepository : RemoteAddonRepositoryEx
{
    private static readonly HttpClient HttpClient = CreateHttpClient();

    /// <summary>
    /// The name of this repository.
    /// </summary>
    public override string Name => "Modrinth";

    /// <summary>
    /// The Modrinth project type filter: mod, modpack, resourcepack, shader or datapack.
    /// </summary>
    public string ProjectType { get; set; } = "mod";

    /// <summary>
    /// Searches Modrinth projects by query string.
    /// </summary>
    /// <param name="query">The free-text search query.</param>
    public override async Task<List<RemoteAddon>> SearchAsync(string query)
    {
        var facets = JsonSerializer.Serialize(new[] { new[] { $"project_type:{ProjectType}" } }, LauncherJsonContext.Default.StringArrayArray);
        var url = $"https://api.modrinth.com/v2/search?query={Uri.EscapeDataString(query)}&limit=30" +
                  $"&facets={Uri.EscapeDataString(facets)}";
        var json = await HttpClientProvider.Shared.GetStringAsync(url);
        var response = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.ModrinthSearchResponse);

        return response?.Hits?.Select(fit => new RemoteAddon
        {
            Id = fit.ProjectId ?? string.Empty,
            Name = fit.Title ?? string.Empty,
            Description = fit.Description,
            SupportedVersions = new List<string>()
        }).ToList() ?? new List<RemoteAddon>();
    }

    /// <summary>
    /// Returns project details for a Modrinth project id.
    /// </summary>
    public override async Task<RemoteAddon?> GetAddonAsync(string id)
    {
        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync($"https://api.modrinth.com/v2/project/{id}");
            var project = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.ModrinthProject);
            if (project == null)
                return null;

            return new RemoteAddon
            {
                Id = id,
                Name = project.Title ?? id,
                Description = project.Description,
                ModLoader = ModLoaderType.None
            };
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Resolves the latest Modrinth version file matching the game version and loader.
    /// </summary>
    public override async Task<RemoteAddon?> ResolveFileAsync(RemoteAddon addon, string minecraftVersion, ModLoaderType loader)
    {
        try
        {
            var url = $"https://api.modrinth.com/v2/project/{addon.Id}/version";

            // Filter by game version when it is known, and by loader when specified.
            var filters = new List<string>();
            if (!string.IsNullOrEmpty(minecraftVersion))
                filters.Add($@"game_versions:[""{minecraftVersion}""]");
            if (loader != ModLoaderType.None && loader != ModLoaderType.LiteLoader)
                filters.Add($@"categories:""{loader.ToString().ToLowerInvariant()}""");

            if (filters.Count > 0)
            {
                var facetJson = JsonSerializer.Serialize(new[] { filters }, LauncherJsonContext.Default.StringArrayArray);
                url += "?facets=" + Uri.EscapeDataString(facetJson);
            }

            var json = await HttpClientProvider.Shared.GetStringAsync(url);
            var versions = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.ListModrinthVersion);
            var version = versions?.FirstOrDefault(a => a.Files != null && a.Files.Count > 0);
            if (version?.Files is not { Count: > 0 } files)
                return null;

            var file = files.FirstOrDefault(f => f.Primary == true) ?? files[0];
            return new RemoteAddon
            {
                Id = addon.Id,
                Name = addon.Name,
                Description = addon.Description,
                DownloadUrl = file.Url,
                Version = version.VersionNumber ?? version.Name,
                ModLoader = loader
            };
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Downloads an addon file by resolaing its latest matching version first.
    /// </summary>
    public override async Task<bool> DownloadAsync(RemoteAddon addon, string destination)
    {
        var resolved = await ResolveFileAsync(addon, string.Empty, ModLoaderType.None);
        if (resolved?.DownloadUrl == null)
            return false;

        var task = new ModInstallTask(resolved, Path.GetDirectoryName(destination) ?? ".", string.Empty);
        await task.Run();
        return true;
    }

    /// <summary>
    /// Creates the HTTP client every Modrinth request shares.
    /// </summary>
    private static HttpClient CreateHttpClient()
    {
        var client = HttpClientProvider.CreateClient();
        client.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");
        return client;
    }
}

/// <summary>
/// Remote addon repository backed by the CurseForge API.
/// A valid API key is required; until one is configured the repo reports no results.
/// </summary>
public sealed class CurseForgeRepository : RemoteAddonRepositoryEx
{

    /// <summary>
    /// The base URL of the CurseForge core API.
    /// </summary>
    public const string BaseUrl = "https://api.curseforge.com";

    /// <summary>
    /// The Minecraft game id used by CurseForge.
    /// </summary>
    public const int MinecraftGameId = 432;

    /// <summary>
    /// The API key used to authenticate requests; replace with a valid key to enable.
    /// </summary>
    public string ApiKey { get; set; } = string.Empty;

    /// <summary>
    /// The name of this repository.
    /// </summary>
    public override string Name => "CurseForge";

    /// <summary>
    /// The CurseForge class id: 6 mods, 12 resource packs, 17 worlds,
    /// 6552 shaders, 6945 data packs.
    /// </summary>
    public int ClassId { get; set; } = 6;

    /// <inheritdoc />
    public override async Task<List<RemoteAddon>> SearchAsync(string query)
    {
        try
        {
            if (string.IsNullOrEmpty(ApiKey))
                return new List<RemoteAddon>();

            var request = new HttpRequestMessage(HttpMethod.Get,
                $"{BaseUrl}/v1/mods/search?gameId={MinecraftGameId}&searchFilter={Uri.EscapeDataString(query)}&classId={ClassId}&sortField=2&sortOrder=desc&pageSize=30");
            request.Headers.Add("x-api-key", ApiKey);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            var data = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.CurseForgeSearchResponse);

            return data?.Data?.Select(mod => new RemoteAddon
            {
                Id = mod.ModId?.ToString() ?? string.Empty,
                Name = mod.Name ?? string.Empty,
                Description = mod.Summary,
                ModLoader = ModLoaderType.None
            }).ToList() ?? new List<RemoteAddon>();
        }
        catch
        {
            return new List<RemoteAddon>();
        }
    }

    /// <inheritdoc />
    public override async Task<RemoteAddon?> GetAddonAsync(string id)
    {
        try
        {
            if (string.IsNullOrEmpty(ApiKey) || !int.TryParse(id, out var modId))
                return null;

            var request = new HttpRequestMessage(HttpMethod.Get, $"{BaseUrl}/v1/mods/{modId}");
            request.Headers.Add("x-api-key", ApiKey);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            var data = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.CurseForgeModResponse);

            return data?.Data == null ? null : new RemoteAddon
            {
                Id = id,
                Name = data.Data.Name ?? id,
                Description = data.Data.Summary
            };        }
        catch
        {
            return null;
        }
    }

    /// <inheritdoc />
    public override async Task<RemoteAddon?> ResolveFileAsync(RemoteAddon addon, string minecraftVersion, ModLoaderType loader)
    {
        try
        {
            if (string.IsNullOrEmpty(ApiKey) || !int.TryParse(addon.Id, out var modId))
                return null;

            var url = $"{BaseUrl}/v1/mods/{modId}/files?pageSize=20&sort=dateDescending";
            if (!string.IsNullOrEmpty(minecraftVersion))
                url += $"&gameVersion={Uri.EscapeDataString(minecraftVersion)}";

            var request = new HttpRequestMessage(HttpMethod.Get, url);
            request.Headers.Add("x-api-key", ApiKey);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            var data = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.CurseForgeFilesResponse);
            var file = data?.Data?.FirstOrDefault(f => !string.IsNullOrEmpty(f.DownloadUrl));
            if (file == null)
                return null;

            return new RemoteAddon
            {
                Id = addon.Id,
                Name = addon.Name,
                Description = addon.Description,
                DownloadUrl = file.DownloadUrl,
                Version = file.DisplayName,
                ModLoader = loader
            };
        }
        catch
        {
            return null;
        }
    }

    /// <summary>
    /// Resolves the direct download URL of a specific CurseForge file, typically
    /// referenced by a modpack manifest through its project and file ids.
    /// </summary>
    /// <param name="projectId">The CurseForge project id.</param>
    /// <param name="fileId">The CurseForge file id.</param>
    /// <returns>The download URL, or null when the key is missing or the request fails.</returns>
    public async Task<string?> GetFileDownloadUrlAsync(int projectId, int fileId)
    {
        try
        {
            if (string.IsNullOrEmpty(ApiKey))
                return null;

            var request = new HttpRequestMessage(HttpMethod.Get,
                $"{BaseUrl}/v1/mods/{projectId}/files/{fileId}/download-url");
            request.Headers.Add("x-api-key", ApiKey);

            using var response = await HttpClientProvider.Shared.SendAsync(request);
            response.EnsureSuccessStatusCode();

            var json = await response.Content.ReadAsStringAsync();
            var data = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.CurseForgeDownloadUrlResponse);
            return data?.Data;
        }
        catch
        {
            return null;
        }
    }

    /// <inheritdoc />
    public override async Task<bool> DownloadAsync(RemoteAddon addon, string destination)
    {
        if (string.IsNullOrEmpty(addon.DownloadUrl))
            return false;

        var task = new ModInstallTask(addon, Path.GetDirectoryName(destination) ?? ".", string.Empty);
        await task.Run();
        return true;
    }
}

/// <summary>
/// Modrinth search response.
/// </summary>
public sealed class ModrinthSearchResponse
{
    /// <summary>
    /// The matched projects.
    /// </summary>
    [JsonPropertyName("hits")]
    public List<ModrinthHit>? Hits { get; init; }
}

/// <summary>
/// A single Modrinth search fit.
/// </summary>
public sealed class ModrinthHit
{
    /// <summary>
    /// The project id.
    /// </summary>
    [JsonPropertyName("project_id")]
    public string? ProjectId { get; init; }

    /// <summary>
    /// The project title.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    /// <summary>
    /// Short project description.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; init; }
}

/// <summary>
/// Modrinth project details.
/// </summary>
public sealed class ModrinthProject
{
    /// <summary>
    /// The project title.
    /// </summary>
    [JsonPropertyName("title")]
    public string? Title { get; init; }

    /// <summary>
    /// The full project description.
    /// </summary>
    [JsonPropertyName("description")]
    public string? Description { get; init; }
}

/// <summary>
/// A Modrinth project version.
/// </summary>
public sealed class ModrinthVersion
{
    /// <summary>
    /// The version number.
    /// </summary>
    [JsonPropertyName("version_number")]
    public string? VersionNumber { get; init; }

    /// <summary>
    /// The display name.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The files attached to this version.
    /// </summary>
    [JsonPropertyName("files")]
    public List<ModrinthFile>? Files { get; init; }
}

/// <summary>
/// A Modrinth version file.
/// </summary>
public sealed class ModrinthFile
{
    /// <summary>
    /// Whether this file is the primary download.
    /// </summary>
    [JsonPropertyName("primary")]
    public bool Primary { get; init; }

    /// <summary>
    /// The download URL.
    /// </summary>
    [JsonPropertyName("url")]
    public string? Url { get; init; }
}

/// <summary>
/// CurseForge search response wrapper.
/// </summary>
public sealed class CurseForgeSearchResponse
{
    /// <summary>
    /// The matched mods.
    /// </summary>
    [JsonPropertyName("data")]
    public List<CurseForgeMod>? Data { get; init; }
}

/// <summary>
/// A CurseForge mod.
/// </summary>
public sealed class CurseForgeMod
{
    /// <summary>
    /// The numeric mod id.
    /// </summary>
    [JsonPropertyName("id")]
    public int? ModId { get; init; }

    /// <summary>
    /// The mod name.
    /// </summary>
    [JsonPropertyName("name")]
    public string? Name { get; init; }

    /// <summary>
    /// The mod summary.
    /// </summary>
    [JsonPropertyName("summary")]
    public string? Summary { get; init; }
}

/// <summary>
/// CurseForge mod response wrapper.
/// </summary>
public sealed class CurseForgeModResponse
{
    /// <summary>
    /// The mod payload.
    /// </summary>
    [JsonPropertyName("data")]
    public CurseForgeMod? Data { get; init; }
}

/// <summary>
/// CurseForge files response wrapper.
/// </summary>
public sealed class CurseForgeFilesResponse
{
    /// <summary>
    /// The mod files.
    /// </summary>
    [JsonPropertyName("data")]
    public List<CurseForgeFile>? Data { get; init; }
}

/// <summary>
/// A CurseForge mod file.
/// </summary>
public sealed class CurseForgeFile
{
    /// <summary>
    /// The display name of the file.
    /// </summary>
    [JsonPropertyName("displayName")]
    public string? DisplayName { get; init; }

    /// <summary>
    /// The download URL.
    /// </summary>
    [JsonPropertyName("downloadUrl")]
    public string? DownloadUrl { get; init; }
}

/// <summary>
/// CurseForge download-url response wrapper; the simple download-url endpoint
/// returns the resolved URL as a plain string payload.
/// </summary>
public sealed class CurseForgeDownloadUrlResponse
{
    /// <summary>
    /// The resolved download URL.
    /// </summary>
    [JsonPropertyName("data")]
    public string? Data { get; init; }
}
