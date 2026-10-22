using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using System.Text.Json;
using HMCLCore.NET.Download;
using HMCLCore.NET.Util;

namespace HMCLCore.NET.Terracotta;

/// <summary>
/// The state of the Terracotta service of the launcher.
/// </summary>
public abstract class TerracottaState
{
    /// <summary>
    /// The name of the state shown to the user.
    /// </summary>
    public abstract string DisplayName { get; }

    /// <summary>
    /// The description of the state shown to the user.
    /// </summary>
    public virtual string Description => string.Empty;

    /// <summary>
    /// Whether the state holds the port of the local service.
    /// </summary>
    public virtual int? Port => null;

    /// <summary>
    /// The state used before the service is checked.
    /// </summary>
    public sealed class Bootstrap : TerracottaState
    {
        /// <inheritdoc />
        public override string DisplayName => "正在检查联机组件";
    }

    /// <summary>
    /// The state used when the service is not installed yet.
    /// </summary>
    public sealed class Uninitialized : TerracottaState
    {
        /// <summary>
        /// Creates the state.
        /// </summary>
        /// <param name="legacy">Whether an old version of the service is installed.</param>
        public Uninitialized(bool legacy)
        {
            Legacy = legacy;
        }

        /// <summary>
        /// Whether an old version of the service is installed.
        /// </summary>
        public bool Legacy { get; }

        /// <inheritdoc />
        public override string DisplayName => "联机组件未安装";

        /// <inheritdoc />
        public override string Description => Legacy
            ? "检测到旧版本的联机组件，需要重新安装。"
            : "需要下载并安装联机组件后才能使用联机功能。";
    }

    /// <summary>
    /// The state used while the service is being installed.
    /// </summary>
    public sealed class Preparing : TerracottaState
    {
        /// <inheritdoc />
        public override string DisplayName => "正在准备联机组件";

        /// <inheritdoc />
        public override string Description => "正在下载并安装联机组件，请稍候。";
    }

    /// <summary>
    /// The state used while the service is starting.
    /// </summary>
    public sealed class Launching : TerracottaState
    {
        /// <inheritdoc />
        public override string DisplayName => "正在启动联机组件";
    }

    /// <summary>
    /// The state used when the port of the service is known.
    /// </summary>
    public class Unknown : TerracottaState
    {
        /// <summary>
        /// Creates the state.
        /// </summary>
        /// <param name="port">The port of the local service.</param>
        public Unknown(int port)
        {
            ServicePort = port;
        }

        /// <summary>
        /// The port of the local service.
        /// </summary>
        public int ServicePort { get; }

        /// <inheritdoc />
        public override string DisplayName => "联机组件已就绪";

        /// <inheritdoc />
        public override int? Port => ServicePort;
    }

    /// <summary>
    /// The state used when the service is ready.
    /// </summary>
    public class Ready : Unknown
    {
        /// <summary>
        /// Creates the state.
        /// </summary>
        /// <param name="port">The port of the local service.</param>
        /// <param name="index">The index of the node of the launcher.</param>
        /// <param name="state">The name of the state reported by the service.</param>
        public Ready(int port, int index, string state = "waiting") : base(port)
        {
            Index = index;
            State = state;
        }

        /// <summary>
        /// The index of the node of the launcher.
        /// </summary>
        public int Index { get; }

        /// <summary>
        /// The name of the state reported by the service.
        /// </summary>
        public string State { get; }

        /// <summary>
        /// The players of the session.
        /// </summary>
        public IReadOnlyList<TerracottaProfile> Profiles { get; init; } = Array.Empty<TerracottaProfile>();

        /// <summary>
        /// The index of the player of the launcher in the list of the players.
        /// </summary>
        public int ProfileIndex { get; init; } = -1;

        /// <summary>
        /// The invitation code of the session, when the launcher hosts it.
        /// </summary>
        public string? RoomCode { get; init; }

        /// <summary>
        /// The backup address of the session, when the launcher joins it.
        /// </summary>
        public string? BackupAddress { get; init; }

        /// <summary>
        /// The difficulty of the connection estimated by the service.
        /// </summary>
        public TerracottaDifficulty Difficulty { get; init; } = TerracottaDifficulty.Unknown;
    }

    /// <summary>
    /// The state used when the launcher waits for the other players.
    /// </summary>
    public sealed class Waiting : TerracottaState
    {
        /// <summary>
        /// Creates the state.
        /// </summary>
        /// <param name="port">The port of the local service.</param>
        /// <param name="index">The index of the node of the launcher.</param>
        /// <param name="reason">The reason of the wait.</param>
        public Waiting(int port, int index, string? reason)
        {
            ServicePort = port;
            Index = index;
            Reason = reason;
        }

        /// <summary>
        /// The port of the local service.
        /// </summary>
        public int ServicePort { get; }

        /// <summary>
        /// The index of the node of the launcher.
        /// </summary>
        public int Index { get; }

        /// <summary>
        /// The reason of the wait.
        /// </summary>
        public string? Reason { get; }

        /// <inheritdoc />
        public override string DisplayName => "正在等待其他玩家";
    }

    /// <summary>
    /// The state used when the launcher looks for the other players.
    /// </summary>
    public sealed class HostScanning : TerracottaState
    {
        /// <inheritdoc />
        public override string DisplayName => "正在寻找房间";
    }

    /// <summary>
    /// The state used when the launcher joins a room.
    /// </summary>
    public sealed class GuestConnecting : TerracottaState
    {
        /// <inheritdoc />
        public override string DisplayName => "正在加入房间";
    }

    /// <summary>
    /// The state used when the service failed.
    /// </summary>
    public sealed class Fatal : TerracottaState
    {
        /// <summary>
        /// The kind of the failure.
        /// </summary>
        public enum FailureType
        {
            /// <summary>
            /// The service cannot be downloaded.
            /// </summary>
            Network,

            /// <summary>
            /// The service cannot be installed.
            /// </summary>
            Install,

            /// <summary>
            /// The service cannot be started.
            /// </summary>
            Terracotta,

            /// <summary>
            /// The failure is not known.
            /// </summary>
            Unknown
        }

        /// <summary>
        /// Creates the state.
        /// </summary>
        /// <param name="type">The kind of the failure.</param>
        public Fatal(FailureType type)
        {
            Type = type;
        }

        /// <summary>
        /// The kind of the failure.
        /// </summary>
        public FailureType Type { get; }

        /// <inheritdoc />
        public override string DisplayName => "联机功能不可用";

        /// <inheritdoc />
        public override string Description => Type switch
        {
            FailureType.Network => "无法下载联机组件，请检查网络连接。",
            FailureType.Install => "无法安装联机组件。",
            FailureType.Terracotta => "联机组件启动失败。",
            _ => "联机功能发生未知错误。"
        };

        /// <summary>
        /// Whether the failure can be recovered by installing the service again.
        /// </summary>
        public bool IsRecoverable => Type is FailureType.Network or FailureType.Install;
    }
}

/// <summary>
/// Reads the list of the Terracotta nodes of the network.
/// </summary>
public static class TerracottaNodeList
{
    /// <summary>
    /// The address of the file holding the list of the nodes.
    /// </summary>
    public const string NodeListUrl = "https://terracotta.glavo.site/nodes";

    private static readonly object _lock = new();
    private static IReadOnlyList<string>? _list;

    /// <summary>
    /// Returns the addresses of the known nodes, caching them after the first
    /// successful read.
    /// </summary>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The addresses of the nodes.</returns>
    public static async Task<IReadOnlyList<string>> FetchAsync(CancellationToken token = default)
    {
        lock (_lock)
        {
            if (_list != null)
                return _list;
        }

        IReadOnlyList<string> list;

        try
        {
            var json = await HttpClientProvider.Shared.GetStringAsync(NodeListUrl, token);
            using var document = JsonDocument.Parse(json);

            var nodes = new List<string>();
            if (document.RootElement.ValueKind == JsonValueKind.Array)
            {
                foreach (var element in document.RootElement.EnumerateArray())
                {
                    if (!element.TryGetProperty("url", out var url) || url.ValueKind != JsonValueKind.String)
                        continue;

                    var address = url.GetString();
                    if (string.IsNullOrWhiteSpace(address))
                        continue;

                    // The nodes of the other regions are only used by the
                    // players of the same region.
                    var region = element.TryGetProperty("region", out var regionValue) &&
                                 regionValue.ValueKind == JsonValueKind.String
                        ? regionValue.GetString()
                        : null;

                    if (!string.IsNullOrWhiteSpace(region) &&
                        !region!.Equals("CN", StringComparison.OrdinalIgnoreCase) &&
                        RegionUtils.IsChinaMainland)
                    {
                        continue;
                    }

                    nodes.Add(address);
                }
            }

            list = nodes;
        }
        catch (Exception ex) when (ex is HttpRequestException or JsonException or TaskCanceledException)
        {
            list = Array.Empty<string>();
        }

        lock (_lock)
            _list = list;

        return list;
    }
}
