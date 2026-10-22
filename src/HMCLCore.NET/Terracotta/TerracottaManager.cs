using System;
using System.Collections.Generic;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Terracotta;

/// <summary>
/// Holds the state of the Terracotta service of the launcher and drives the
/// operations of the online mode, like the manager of Java HMCL.
/// </summary>
public sealed class TerracottaManager
{
    private static readonly object _lock = new();

    private static TerracottaState _state = new TerracottaState.Bootstrap();

    /// <summary>
    /// The state of the service.
    /// </summary>
    public static TerracottaState State
    {
        get
        {
            lock (_lock)
                return _state;
        }
    }

    /// <summary>
    /// Raised when the state of the service changed.
    /// </summary>
    public static event Action<TerracottaState>? StateChanged;

    /// <summary>
    /// The directory holding the Terracotta bundles of the launcher.
    /// </summary>
    public static string? DependenciesDirectory { get; set; }

    /// <summary>
    /// Downloads and installs the Terracotta bundle of the current platform,
    /// then starts the service.
    /// </summary>
    /// <param name="progress">Receives the progress of the download.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the service is started.</returns>
    public static async Task<TerracottaState> DownloadAsync(
        IProgress<double>? progress = null, CancellationToken cancellationToken = default)
    {
        if (DependenciesDirectory is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        var metadata = TerracottaMetadata.Current;
        if (metadata is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        var bundle = TerracottaBundle.Create(DependenciesDirectory, metadata);
        if (bundle is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        SetState(new TerracottaState.Preparing());

        try
        {
            var packagePath = Path.Combine(
                Path.GetTempPath(), $"terracotta-{metadata.LatestVersion}.tar.gz");

            var downloaded = await bundle.DownloadAsync(packagePath, progress, cancellationToken);
            if (!downloaded)
            {
                SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Network));
                return State;
            }

            if (!bundle.Install(packagePath))
            {
                SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Install));
                return State;
            }

            TerracottaBundle.RemoveLegacyVersionFiles(DependenciesDirectory, metadata.LatestVersion);

            SetState(new TerracottaState.Launching());
            var port = await bundle.LaunchAsync(cancellationToken);
            SetState(new TerracottaState.Unknown(port));
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            HMCLCore.NET.Util.Logging.Logger.Error("Cannot launch the Terracotta service", ex);
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Terracotta));
        }

        return State;
    }

    /// <summary>
    /// Installs a Terracotta bundle from a local package and starts the
    /// service.
    /// </summary>
    /// <param name="packagePath">The path of the downloaded package.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the service is started.</returns>
    public static async Task<TerracottaState> InstallAsync(
        string packagePath, CancellationToken cancellationToken = default)
    {
        if (DependenciesDirectory is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        var metadata = TerracottaMetadata.Current;
        if (metadata is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        var bundle = TerracottaBundle.Create(DependenciesDirectory, metadata);
        if (bundle is null)
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Unknown));
            return State;
        }

        SetState(new TerracottaState.Preparing());

        if (!bundle.Install(packagePath))
        {
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Install));
            return State;
        }

        try
        {
            SetState(new TerracottaState.Launching());
            var port = await bundle.LaunchAsync(cancellationToken);
            SetState(new TerracottaState.Unknown(port));
        }
        catch (Exception ex)
        {
            HMCLCore.NET.Util.Logging.Logger.Error("Cannot launch the Terracotta service", ex);
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Terracotta));
        }

        return State;
    }

    /// <summary>
    /// Replaces the state of the service.
    /// </summary>
    /// <param name="state">The new state.</param>
    private static void SetState(TerracottaState state)
    {
        lock (_lock)
            _state = state;

        StateChanged?.Invoke(state);
    }

    /// <summary>
    /// Checks the state of the service and starts it when it is installed.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the state is known.</returns>
    public static async Task<TerracottaState> InitializeAsync(CancellationToken cancellationToken = default)
    {
        if (DependenciesDirectory is null)
        {
            SetState(new TerracottaState.Uninitialized(false));
            return State;
        }

        try
        {
            var status = TerracottaBundle.GetStatus(DependenciesDirectory);
            if (status != TerracottaBundle.BundleStatus.Ready)
            {
                SetState(new TerracottaState.Uninitialized(status == TerracottaBundle.BundleStatus.LegacyVersion));
                return State;
            }

            var metadata = TerracottaMetadata.Current;
            var bundle = metadata is null
                ? null
                : TerracottaBundle.Create(DependenciesDirectory, metadata);

            if (bundle is null)
            {
                SetState(new TerracottaState.Uninitialized(false));
                return State;
            }

            SetState(new TerracottaState.Launching());

            var port = await bundle.LaunchAsync(cancellationToken);
            SetState(new TerracottaState.Unknown(port));
        }
        catch (Exception ex)
        {
            HMCLCore.NET.Util.Logging.Logger.Error("Cannot launch the Terracotta service", ex);
            SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Terracotta));
        }

        return State;
    }

    /// <summary>
    /// Reads the state of the service from its local interface.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the state is read.</returns>
    public static async Task<TerracottaState> RefreshAsync(CancellationToken cancellationToken = default)
    {
        if (State.Port is not { } port)
            return State;

        // The service needs a moment to answer once it is started, so the
        // request is retried a few times.
        for (var attempt = 0; attempt < 10; attempt++)
        {
            try
            {
                var json = await HttpClientProvider.Shared.GetStringAsync(
                    $"http://127.0.0.1:{port}/state", cancellationToken);

                var state = ParseState(json, port);
                SetState(state);
                return state;
            }
            catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException
                                          or System.Text.Json.JsonException)
            {
                HMCLCore.NET.Util.Logging.Logger.Warn("Cannot read the state of the Terracotta service: " + ex.Message);
                if (attempt == 9)
                {
                    SetState(new TerracottaState.Fatal(TerracottaState.Fatal.FailureType.Terracotta));
                    return State;
                }

                await Task.Delay(1000, cancellationToken);
            }
        }

        return State;
    }

    /// <summary>
    /// Parses the state reported by the local service.
    /// </summary>
    /// <param name="json">The JSON reported by the service.</param>
    /// <param name="port">The port of the local service.</param>
    /// <returns>The state of the service.</returns>
    private static TerracottaState ParseState(string json, int port)
    {
        using var document = System.Text.Json.JsonDocument.Parse(json);
        var root = document.RootElement;

        var index = root.TryGetProperty("index", out var indexValue) ? indexValue.GetInt32() : 0;
        var state = root.TryGetProperty("state", out var stateValue) ? stateValue.GetString() ?? "waiting" : "waiting";

        var profiles = new List<TerracottaProfile>();
        if (root.TryGetProperty("profiles", out var profilesValue) &&
            profilesValue.ValueKind == System.Text.Json.JsonValueKind.Array)
        {
            foreach (var profile in profilesValue.EnumerateArray())
            {
                profiles.Add(new TerracottaProfile
                {
                    MachineId = profile.TryGetProperty("machine_id", out var machine) ? machine.GetString() ?? string.Empty : string.Empty,
                    Name = profile.TryGetProperty("name", out var name) ? name.GetString() ?? string.Empty : string.Empty,
                    Vendor = profile.TryGetProperty("vendor", out var vendor) ? vendor.GetString() ?? string.Empty : string.Empty,
                    Kind = profile.TryGetProperty("kind", out var kind) && kind.ValueKind == System.Text.Json.JsonValueKind.String
                        ? ParseProfileKind(kind.GetString())
                        : ProfileKind.Guest
                });
            }
        }

        var profileIndex = root.TryGetProperty("profile_index", out var profileIndexValue)
            ? profileIndexValue.GetInt32()
            : -1;

        var room = root.TryGetProperty("room", out var roomValue) ? roomValue.GetString() : null;
        var url = root.TryGetProperty("url", out var urlValue) ? urlValue.GetString() : null;

        var difficulty = TerracottaDifficulty.Unknown;
        if (root.TryGetProperty("difficulty", out var difficultyValue) &&
            difficultyValue.ValueKind == System.Text.Json.JsonValueKind.String)
        {
            difficulty = difficultyValue.GetString()?.ToLowerInvariant() switch
            {
                "easiest" => TerracottaDifficulty.Easiest,
                "simple" => TerracottaDifficulty.Simple,
                "medium" => TerracottaDifficulty.Medium,
                "tough" => TerracottaDifficulty.Tough,
                _ => TerracottaDifficulty.Unknown
            };
        }

        return new TerracottaState.Ready(port, index, state)
        {
            Profiles = profiles,
            ProfileIndex = profileIndex,
            RoomCode = room,
            BackupAddress = url,
            Difficulty = difficulty
        };
    }

    /// <summary>
    /// Parses the kind of a player.
    /// </summary>
    /// <param name="value">The name of the kind.</param>
    /// <returns>The kind of the player.</returns>
    private static ProfileKind ParseProfileKind(string? value) => value?.ToUpperInvariant() switch
    {
        "HOST" => ProfileKind.Host,
        "LOCAL" => ProfileKind.Local,
        _ => ProfileKind.Guest
    };

    /// <summary>
    /// Returns the list of the Terracotta nodes of the network.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The addresses of the nodes.</returns>
    public static Task<IReadOnlyList<string>> GetNodesAsync(CancellationToken cancellationToken = default)
        => TerracottaNodeList.FetchAsync(cancellationToken);

    /// <summary>
    /// Asks the service to look for the rooms of the network.
    /// </summary>
    /// <param name="playerName">The name of the player.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public static async Task<bool> ScanAsync(string playerName, CancellationToken cancellationToken = default)
    {
        if (State.Port is not { } port)
            return false;

        try
        {
            var nodes = await TerracottaNodeList.FetchAsync(cancellationToken);
            var query = BuildQuery(new[] { ("player", playerName) }, nodes);

            using var response = await HttpClientProvider.Shared.GetAsync(
                $"http://127.0.0.1:{port}/state/scanning?{query}", cancellationToken);

            SetState(new TerracottaState.HostScanning());
            return response.IsSuccessStatusCode;
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>
    /// Asks the service to join a room.
    /// </summary>
    /// <param name="room">The name of the room.</param>
    /// <param name="playerName">The name of the player.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>A task that completes when the request is sent.</returns>
    public static async Task<bool> JoinAsync(
        string room, string playerName, CancellationToken cancellationToken = default)
    {
        if (State.Port is not { } port)
            return false;

        try
        {
            var nodes = await TerracottaNodeList.FetchAsync(cancellationToken);
            var query = BuildQuery(new[] { ("room", room), ("player", playerName) }, nodes);

            using var response = await HttpClientProvider.Shared.GetAsync(
                $"http://127.0.0.1:{port}/state/guesting?{query}", cancellationToken);

            SetState(new TerracottaState.GuestConnecting());
            return response.IsSuccessStatusCode;
        }
        catch (Exception)
        {
            return false;
        }
    }

    /// <summary>
    /// Reads the logs of the service.
    /// </summary>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The logs of the service, empty when they cannot be read.</returns>
    public static async Task<string> ExportLogsAsync(CancellationToken cancellationToken = default)
    {
        if (State.Port is not { } port)
            return string.Empty;

        try
        {
            return await HttpClientProvider.Shared.GetStringAsync(
                $"http://127.0.0.1:{port}/log?fetch=true", cancellationToken);
        }
        catch (Exception ex)
        {
            HMCLCore.NET.Util.Logging.Logger.Error("Cannot read the logs of the Terracotta service", ex);
            return string.Empty;
        }
    }

    /// <summary>
    /// Builds the query of a request of the local service.
    /// </summary>
    /// <param name="parameters">The parameters of the query.</param>
    /// <param name="nodes">The nodes of the network.</param>
    /// <returns>The query of the request.</returns>
    private static string BuildQuery(
        (string Key, string Value)[] parameters, IReadOnlyList<string> nodes)
    {
        var query = new List<string>(parameters.Length + nodes.Count);
        foreach (var parameter in parameters)
            query.Add($"{Uri.EscapeDataString(parameter.Key)}={Uri.EscapeDataString(parameter.Value)}");

        foreach (var node in nodes)
            query.Add($"public_nodes={Uri.EscapeDataString(node)}");

        return string.Join('&', query);
    }}
