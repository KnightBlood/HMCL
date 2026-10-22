using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Util;

/// <summary>
/// Sends the anonymous usage statistics of the launcher to a Countly server.
/// </summary>
/// <remarks>
/// The statistics are only sent when the user enables them; the server is
/// empty by default, so nothing leaves the machine unless the launcher is
/// configured to send them.
/// </remarks>
public sealed class CountlyService
{
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        WriteIndented = false
    };

    /// <summary>
    /// Creates the service.
    /// </summary>
    public CountlyService()
    {
    }

    /// <summary>
    /// Creates the service sending the statistics to the given server.
    /// </summary>
    /// <param name="serverUrl">The address of the Countly server.</param>
    /// <param name="appKey">The key of the application on the server.</param>
    /// <param name="deviceId">The anonymous identifier of the machine.</param>
    public CountlyService(string serverUrl, string appKey, string deviceId)
    {
        ServerUrl = serverUrl;
        AppKey = appKey;
        DeviceId = deviceId;
    }

    /// <summary>
    /// The address of the Countly server.
    /// </summary>
    public string ServerUrl { get; set; } = string.Empty;

    /// <summary>
    /// The key of the application on the Countly server.
    /// </summary>
    public string AppKey { get; set; } = string.Empty;

    /// <summary>
    /// The anonymous identifier of the machine.
    /// </summary>
    public string DeviceId { get; set; } = string.Empty;

    /// <summary>
    /// Whether the statistics are sent to the server.
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// Whether the service is configured and can send the statistics.
    /// </summary>
    public bool IsConfigured =>
        Enabled &&
        !string.IsNullOrWhiteSpace(ServerUrl) &&
        !string.IsNullOrWhiteSpace(AppKey) &&
        !string.IsNullOrWhiteSpace(DeviceId);

    /// <summary>
    /// Sends the start of a session of the launcher.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackSessionStartAsync(CancellationToken token = default)
        => SendAsync(new[] { ("request_session_id", "1") }, null, token);

    /// <summary>
    /// Sends the end of a session of the launcher.
    /// </summary>
    /// <param name="duration">The duration of the session, in seconds.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackSessionEndAsync(int duration, CancellationToken token = default)
        => SendAsync(new[] { ("duration", duration.ToString()) }, null, token);

    /// <summary>
    /// Sends the usage of a page of the launcher.
    /// </summary>
    /// <param name="view">The name of the page.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackViewAsync(string view, CancellationToken token = default)
        => SendAsync(new[] { ("view", view) }, null, token);

    /// <summary>
    /// Sends the use of a feature of the launcher.
    /// </summary>
    /// <param name="key">The name of the feature.</param>
    /// <param name="segmentation">The details of the use of the feature.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackEventAsync(
        string key, IReadOnlyDictionary<string, string>? segmentation = null, CancellationToken token = default)
    {
        var parameters = new List<(string Key, string Value)> { ("event", key), ("count", "1") };

        if (segmentation is { Count: > 0 })
        {
            var json = JsonSerializer.Serialize(segmentation, LauncherJsonContext.Default.IReadOnlyDictionaryStringString);
            parameters.Add(("segmentation", json));
        }

        return SendAsync(parameters.ToArray(), null, token);
    }

    /// <summary>
    /// Sends a crash of the launcher.
    /// </summary>
    /// <param name="exception">The exception that crashed the launcher.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackCrashAsync(Exception exception, CancellationToken token = default)
    {
        ArgumentNullException.ThrowIfNull(exception);

        var parameters = new List<(string Key, string Value)>
        {
            ("crash", exception.GetType().FullName ?? "Error"),
            ("nonfatal", "0")
        };

        return SendAsync(parameters.ToArray(), exception.ToString(), token);
    }

    /// <summary>
    /// Sends the report of a crash of the launcher.
    /// </summary>
    /// <param name="report">The report of the crash.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public Task TrackCrashAsync(CrashReport report, CancellationToken token = default)
    {
        ArgumentNullException.ThrowIfNull(report);

        var parameters = new List<(string Key, string Value)>
        {
            ("crash", report.GetErrorName()),
            ("nonfatal", "0")
        };

        return SendAsync(parameters.ToArray(), report.GetDisplayText(), token);
    }

    /// <summary>
    /// Sends a request to the Countly server.
    /// </summary>
    /// <param name="parameters">The parameters of the request.</param>
    /// <param name="body">The body of the request.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    private async Task SendAsync(
        (string Key, string Value)[] parameters, string? body, CancellationToken token)
    {
        if (!IsConfigured)
            return;

        try
        {
            var query = new List<string>
            {
                $"app_key={Uri.EscapeDataString(AppKey)}",
                $"device_id={Uri.EscapeDataString(DeviceId)}"
            };

            foreach (var parameter in parameters)
                query.Add($"{Uri.EscapeDataString(parameter.Key)}={Uri.EscapeDataString(parameter.Value)}");

            var url = $"{ServerUrl.TrimEnd('/')}/i?{string.Join('&', query)}";

            using var request = new System.Net.Http.HttpRequestMessage(
                System.Net.Http.HttpMethod.Post, url);

            if (!string.IsNullOrEmpty(body))
                request.Content = new System.Net.Http.StringContent(body);

            using var response = await HttpClientProvider.Shared.SendAsync(request, token);
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException
                                      or TaskCanceledException or UriFormatException)
        {
            // The statistics are optional and must never affect the launcher.
        }
    }
}
