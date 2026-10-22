using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Util;

/// <summary>
/// An announcement shown by the launcher on its main page.
/// </summary>
public sealed class Announcement
{
    /// <summary>
    /// The title of the announcement.
    /// </summary>
    public string Title { get; init; } = string.Empty;

    /// <summary>
    /// The text of the announcement.
    /// </summary>
    public string Content { get; init; } = string.Empty;

    /// <summary>
    /// The address of the page of the announcement.
    /// </summary>
    public string? Url { get; init; }

    /// <summary>
    /// The date of the announcement.
    /// </summary>
    public DateTime? Date { get; init; }

    /// <summary>
    /// The announcement shown by the launcher.
    /// </summary>
    public override string ToString() => string.IsNullOrEmpty(Title) ? Content : Title;
}

/// <summary>
/// Reads the announcements shown by the launcher on its main page.
/// </summary>
public sealed class AnnouncementService
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        PropertyNameCaseInsensitive = true
    };

    /// <summary>
    /// The address of the file holding the announcements.
    /// </summary>
    public string? AnnouncementsUrl { get; set; }

    /// <summary>
    /// Creates the service.
    /// </summary>
    /// <param name="announcementsUrl">
    /// The address of the file holding the announcements; when it is empty the
    /// announcements are not fetched.
    /// </param>
    public AnnouncementService(string? announcementsUrl = null)
    {
        AnnouncementsUrl = announcementsUrl;
    }

    /// <summary>
    /// Reads the announcements of the launcher.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The announcements, empty when they cannot be read.</returns>
    public async Task<IReadOnlyList<Announcement>> GetAnnouncementsAsync(CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(AnnouncementsUrl))
            return Array.Empty<Announcement>();

        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(AnnouncementsUrl!, token);
            var announcements = JsonSerializer.Deserialize(json, LauncherJsonContext.Default.ListAnnouncement);
            return announcements ?? (IReadOnlyList<Announcement>)Array.Empty<Announcement>();
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException)
        {
            return Array.Empty<Announcement>();
        }
    }

    /// <summary>
    /// Builds the announcement of a release, used when the announcements cannot
    /// be fetched.
    /// </summary>
    /// <param name="title">The title of the release.</param>
    /// <param name="notes">The notes of the release.</param>
    /// <param name="url">The address of the page of the release.</param>
    /// <returns>The announcement of the release.</returns>
    public static Announcement CreateReleaseAnnouncement(string title, string notes, string? url)
        => new()
        {
            Title = title,
            Content = notes,
            Url = url
        };
}
