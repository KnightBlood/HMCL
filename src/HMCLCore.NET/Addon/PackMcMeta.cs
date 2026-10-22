using System.Text.Json.Serialization;

namespace HMCLCore.NET.Addon;

/// <summary>
/// Pack.mcmeta file structure for resource packs.
/// </summary>
public sealed class PackMcMeta
{
    /// <summary>
    /// The pack metadata section.
    /// </summary>
    [JsonPropertyName("pack")]
    public PackData? Pack { get; init; }

    /// <summary>
    /// The pack section containing description and format.
    /// </summary>
    public sealed class PackData
    {
        /// <summary>
        /// The pack description.
        /// </summary>
        [JsonPropertyName("description")]
        public string? Description { get; init; }

        /// <summary>
        /// The pack format version.
        /// </summary>
        [JsonPropertyName("pack_format")]
        public int PackFormat { get; init; }
    }
}
