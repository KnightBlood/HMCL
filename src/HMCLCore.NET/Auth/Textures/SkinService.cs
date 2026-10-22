using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Net.Http.Headers;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Auth.Textures;

/// <summary>
/// A skin of a player stored by Mojang.
/// </summary>
public sealed class AccountSkin
{
    /// <summary>
    /// The identifier of the skin.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The state of the skin, for example "ACTIVE" or "INACTIVE".
    /// </summary>
    public string State { get; init; } = string.Empty;

    /// <summary>
    /// The variant of the skin, "classic" or "slim".
    /// </summary>
    public string Variant { get; init; } = string.Empty;

    /// <summary>
    /// The address of the file of the skin.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The date the skin was created at.
    /// </summary>
    public DateTimeOffset? CreatedAt { get; init; }

    /// <summary>
    /// Whether the skin is the one currently worn by the player.
    /// </summary>
    public bool IsActive => string.Equals(State, "ACTIVE", StringComparison.OrdinalIgnoreCase);
}

/// <summary>
/// Reads and changes the skins of the players of Mojang accounts.
/// </summary>
public sealed class SkinService
{
    /// <summary>
    /// The address of the skins of the profile of the player.
    /// </summary>
    private const string SkinsUrl = "https://api.minecraftservices.com/minecraft/profile/skins";

    /// <summary>
    /// Returns the skins of the player owning the given access token.
    /// </summary>
    /// <param name="accessToken">The Minecraft access token of the account.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The skins of the player, empty when they cannot be read.</returns>
    public async Task<IReadOnlyList<AccountSkin>> GetSkinsAsync(string accessToken, CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(accessToken))
            return Array.Empty<AccountSkin>();

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Get, SkinsUrl);
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", accessToken);

            using var response = await HttpClientProvider.Shared.SendAsync(request, token);
            if (!response.IsSuccessStatusCode)
                return Array.Empty<AccountSkin>();

            var json = await response.Content.ReadAsStringAsync(token);
            using var document = JsonDocument.Parse(json);

            var skins = new List<AccountSkin>();
            if (document.RootElement.ValueKind != JsonValueKind.Array)
                return skins;

            foreach (var element in document.RootElement.EnumerateArray())
            {
                skins.Add(new AccountSkin
                {
                    Id = GetString(element, "id") ?? string.Empty,
                    State = GetString(element, "state") ?? string.Empty,
                    Variant = GetString(element, "variant") ?? "classic",
                    Url = GetString(element, "url") ?? string.Empty,
                    CreatedAt = element.TryGetProperty("createdAt", out var createdAt) &&
                                createdAt.TryGetDateTimeOffset(out var date)
                        ? date
                        : null
                });
            }

            return skins;
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException)
        {
            return Array.Empty<AccountSkin>();
        }
    }

    /// <summary>
    /// Uploads a skin for the player owning the given access token.
    /// </summary>
    /// <param name="accessToken">The Minecraft access token of the account.</param>
    /// <param name="filePath">The path of the PNG file holding the skin.</param>
    /// <param name="variant">The variant of the skin, "classic" or "slim".</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>Whether the skin was uploaded.</returns>
    public async Task<bool> UploadSkinAsync(
        string accessToken, string filePath, string variant = "classic", CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(accessToken) || !File.Exists(filePath))
            return false;

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Put, SkinsUrl);
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", accessToken);

            var content = new MultipartFormDataContent();
            var fileContent = new ByteArrayContent(await File.ReadAllBytesAsync(filePath, token));
            fileContent.Headers.ContentType = new MediaTypeHeaderValue("image/png");
            content.Add(fileContent, "file", Path.GetFileName(filePath));
            content.Add(new StringContent(variant), "variant");

            request.Content = content;

            using var response = await HttpClientProvider.Shared.SendAsync(request, token);
            return response.IsSuccessStatusCode;
        }
        catch (Exception ex) when (ex is HttpRequestException or IOException or TaskCanceledException)
        {
            return false;
        }
    }

    /// <summary>
    /// Removes the given skin of the player owning the given access token.
    /// </summary>
    /// <param name="accessToken">The Minecraft access token of the account.</param>
    /// <param name="skinId">The identifier of the skin to remove.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>Whether the skin was removed.</returns>
    public async Task<bool> ResetSkinAsync(string accessToken, string skinId, CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(accessToken) || string.IsNullOrWhiteSpace(skinId))
            return false;

        try
        {
            using var request = new HttpRequestMessage(HttpMethod.Delete, $"{SkinsUrl}/{skinId}");
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", accessToken);

            using var response = await HttpClientProvider.Shared.SendAsync(request, token);
            return response.IsSuccessStatusCode;
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException)
        {
            return false;
        }
    }

    /// <summary>
    /// Reads a string member of a JSON object.
    /// </summary>
    /// <param name="element">The object to read.</param>
    /// <param name="name">The name of the member.</param>
    /// <returns>The value of the member, or null when it is missing.</returns>
    private static string? GetString(JsonElement element, string name)
        => element.TryGetProperty(name, out var value) && value.ValueKind == JsonValueKind.String
            ? value.GetString()
            : null;
}
