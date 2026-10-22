using System.Text.Json;
using HMCLCore.NET.Game;

namespace HMCLCore.NET.Util;

/// <summary>
/// Reads and writes the documents of the launcher with the metadata generated
/// when the launcher is compiled, so that the launcher can be published with
/// the native ahead-of-time compiler.
/// </summary>
public static class LauncherJson
{
    /// <summary>
    /// Writes the manifest of a game instance into a JSON document.
    /// </summary>
    /// <param name="manifest">The manifest of the instance.</param>
    /// <returns>The JSON document of the manifest.</returns>
    public static string SerializeManifest(GameInstanceManifest manifest)
        => JsonSerializer.Serialize(manifest, LauncherJsonContext.Default.GameInstanceManifest);

    /// <summary>
    /// Reads the manifest of a game instance from a JSON document.
    /// </summary>
    /// <param name="json">The JSON document of the manifest.</param>
    /// <returns>The manifest, or null when the document cannot be read.</returns>
    public static GameInstanceManifest? DeserializeManifest(string json)
        => JsonSerializer.Deserialize(json, LauncherJsonContext.Default.GameInstanceManifest);
}
