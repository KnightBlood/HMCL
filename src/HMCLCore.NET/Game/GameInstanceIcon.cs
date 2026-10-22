using System;
using System.IO;

namespace HMCLCore.NET.Game;

/// <summary>
/// The kind of icon shown for a game instance. Mirrors Java HMCL's
/// <c>GameInstanceIconType</c>.
/// </summary>
public enum GameInstanceIconType
{
    /// <summary>
    /// The icon is derived from the mod loader of the instance.
    /// </summary>
    Default,

    /// <summary>
    /// The icon of a vanilla Minecraft instance.
    /// </summary>
    Vanilla,

    /// <summary>
    /// The icon of a Forge instance.
    /// </summary>
    Forge,

    /// <summary>
    /// The icon of a NeoForge instance.
    /// </summary>
    NeoForge,

    /// <summary>
    /// The icon of a Fabric instance.
    /// </summary>
    Fabric,

    /// <summary>
    /// The icon of a Quilt instance.
    /// </summary>
    Quilt,

    /// <summary>
    /// A custom image chosen by the user.
    /// </summary>
    Custom
}

/// <summary>
/// Manages the custom icon of a game instance. The image is stored as
/// <c>icon.png</c> inside the instance directory, mirroring Java HMCL.
/// </summary>
public static class GameInstanceIcons
{
    /// <summary>
    /// The name of the file holding the custom icon of an instance.
    /// </summary>
    public const string CustomIconFileName = "icon.png";

    /// <summary>
    /// Returns the path of the custom icon of an instance.
    /// </summary>
    /// <param name="instanceDirectory">The directory of the instance.</param>
    /// <returns>The path of the icon, or null when the instance has none.</returns>
    public static string? GetCustomIconPath(string instanceDirectory)
    {
        if (string.IsNullOrWhiteSpace(instanceDirectory))
            return null;

        var path = Path.Combine(instanceDirectory, CustomIconFileName);
        return File.Exists(path) ? path : null;
    }

    /// <summary>
    /// Copies an image into the instance directory as its custom icon.
    /// </summary>
    /// <param name="instanceDirectory">The directory of the instance.</param>
    /// <param name="sourceImagePath">The image to use.</param>
    /// <returns>Whether the icon was stored.</returns>
    public static bool SetCustomIcon(string instanceDirectory, string sourceImagePath)
    {
        if (string.IsNullOrWhiteSpace(instanceDirectory) || !File.Exists(sourceImagePath))
            return false;

        try
        {
            Directory.CreateDirectory(instanceDirectory);
            var target = Path.Combine(instanceDirectory, CustomIconFileName);
            File.Copy(sourceImagePath, target, overwrite: true);
            return true;
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Deletes the custom icon of an instance.
    /// </summary>
    /// <param name="instanceDirectory">The directory of the instance.</param>
    /// <returns>Whether an icon was deleted.</returns>
    public static bool RemoveCustomIcon(string instanceDirectory)
    {
        var path = GetCustomIconPath(instanceDirectory);
        if (path == null)
            return false;

        try
        {
            File.Delete(path);
            return true;
        }
        catch (IOException)
        {
            return false;
        }
        catch (UnauthorizedAccessException)
        {
            return false;
        }
    }

    /// <summary>
    /// Determines the icon type of an instance from its main class.
    /// </summary>
    /// <param name="manifest">The manifest of the instance.</param>
    /// <returns>The icon type of the instance.</returns>
    public static GameInstanceIconType GetIconType(GameInstanceManifest? manifest)
    {
        var mainClass = manifest?.MainClass ?? string.Empty;

        if (mainClass.Contains("quilt", StringComparison.OrdinalIgnoreCase))
            return GameInstanceIconType.Quilt;
        if (mainClass.Contains("fabric", StringComparison.OrdinalIgnoreCase) ||
            mainClass.Contains("knot", StringComparison.OrdinalIgnoreCase))
            return GameInstanceIconType.Fabric;
        if (mainClass.Contains("neoforge", StringComparison.OrdinalIgnoreCase))
            return GameInstanceIconType.NeoForge;
        if (mainClass.Contains("forge", StringComparison.OrdinalIgnoreCase) ||
            mainClass.Contains("bootstraplauncher", StringComparison.OrdinalIgnoreCase))
            return GameInstanceIconType.Forge;

        return GameInstanceIconType.Vanilla;
    }
}
