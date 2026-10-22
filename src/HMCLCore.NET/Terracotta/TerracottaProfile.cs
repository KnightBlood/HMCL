using System;
using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace HMCLCore.NET.Terracotta;

/// <summary>
/// The kind of a player of a session of the online mode.
/// </summary>
public enum ProfileKind
{
    /// <summary>
    /// The player hosting the session.
    /// </summary>
    Host,

    /// <summary>
    /// The player running the launcher.
    /// </summary>
    Local,

    /// <summary>
    /// A player who joined the session.
    /// </summary>
    Guest
}

/// <summary>
/// A player of a session of the online mode.
/// </summary>
public sealed class TerracottaProfile
{
    /// <summary>
    /// The identifier of the machine of the player.
    /// </summary>
    [JsonPropertyName("machine_id")]
    public string MachineId { get; init; } = string.Empty;

    /// <summary>
    /// The name of the player.
    /// </summary>
    [JsonPropertyName("name")]
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The vendor of the machine of the player.
    /// </summary>
    [JsonPropertyName("vendor")]
    public string Vendor { get; init; } = string.Empty;

    /// <summary>
    /// The kind of the player.
    /// </summary>
    [JsonPropertyName("kind")]
    public ProfileKind Kind { get; init; } = ProfileKind.Guest;

    /// <summary>
    /// The name of the player shown to the user.
    /// </summary>
    public string DisplayName => Kind switch
    {
        ProfileKind.Host => $"{Name} ({Vendor})",
        ProfileKind.Local => $"{Name} ({Vendor}, 本机)",
        _ => $"{Name} ({Vendor})"
    };
}

/// <summary>
/// The difficulty of the connection estimated by the service.
/// </summary>
public enum TerracottaDifficulty
{
    /// <summary>
    /// The difficulty is not known.
    /// </summary>
    Unknown,

    /// <summary>
    /// The network is excellent.
    /// </summary>
    Easiest,

    /// <summary>
    /// The network is good.
    /// </summary>
    Simple,

    /// <summary>
    /// The network is average.
    /// </summary>
    Medium,

    /// <summary>
    /// The network is poor.
    /// </summary>
    Tough
}
