using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Modpack;

/// <summary>
/// The layout of the files of a modpack inside its archive.
/// </summary>
public enum ModpackLayout
{
    /// <summary>
    /// The files of the pack are stored in the "overrides" directory of the
    /// archive, like the Modrinth and the CurseForge packs.
    /// </summary>
    Overrides,

    /// <summary>
    /// The files of the pack are stored at the root of the archive, like the
    /// MCBBS, the MultiMC and the server packs.
    /// </summary>
    Root
}

/// <summary>
/// Represents a modpack with metadata and manifest.
/// </summary>
public sealed class Modpack
{
    /// <summary>
    /// The modpack name.
    /// </summary>
    public string Name { get; set; } = string.Empty;

    /// <summary>
    /// The modpack author.
    /// </summary>
    public string? Author { get; set; }

    /// <summary>
    /// The modpack version.
    /// </summary>
    public string? Version { get; set; }

    /// <summary>
    /// The Minecraft game version this modpack targets.
    /// </summary>
    public string? GameVersion { get; set; }

    /// <summary>
    /// The modpack description.
    /// </summary>
    public string? Description { get; set; }

    /// <summary>
    /// The modpack manifest file.
    /// </summary>
    public ModpackManifest? Manifest { get; set; }

    /// <summary>
    /// The path to the modpack file.
    /// </summary>
    public string? FilePath { get; set; }

    /// <summary>
    /// The layout of the files of the modpack inside its archive.
    /// </summary>
    public ModpackLayout Layout { get; set; } = ModpackLayout.Overrides;

    /// <summary>
    /// The files of the modpack which the user may skip, like the optional
    /// files page of Java HMCL.
    /// </summary>
    public IReadOnlyList<ModpackOptionalFile> OptionalFiles =>
        Manifest?.Files.Where(file => file.Optional).Select(file => new ModpackOptionalFile
        {
            Key = file.Path,
            Path = file.Path,
            FileName = Path.GetFileName(file.Path)
        }).ToList() ?? (IReadOnlyList<ModpackOptionalFile>)Array.Empty<ModpackOptionalFile>();

    /// <summary>
    /// The settings of a MultiMC instance, applied to the instance after the
    /// installation, like the post-install task of Java HMCL.
    /// </summary>
    public MultiMcInstanceSettings? MultiMcSettings { get; set; }

    /// <summary>
    /// Supported icon extensions.
    /// </summary>
    public static readonly HashSet<string> SupportedIconExtensions = new(StringComparer.OrdinalIgnoreCase)
    {
        "png", "jpg", "jpeg", "bmp", "gif", "webp", "apng"
    };

    /// <summary>
    /// Supported icon file names.
    /// </summary>
    public static readonly HashSet<string> SupportedIconNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "icon.png", "icon.jpg", "icon.jpeg", "icon.bmp", "icon.gif", "icon.webp", "icon.apng"
    };
}

/// <summary>
/// Represents a modpack manifest.
/// </summary>
public sealed class ModpackManifest
{
    /// <summary>
    /// The manifest version.
    /// </summary>
    public int ManifestVersion { get; init; }

    /// <summary>
    /// The Minecraft version.
    /// </summary>
    public string MinecraftVersion { get; init; } = string.Empty;

    /// <summary>
    /// The mod loader type and version.
    /// </summary>
    public string? ModLoader { get; init; }

    /// <summary>
    /// The list of mods in the modpack.
    /// </summary>
    public List<ModpackMod> Mods { get; init; } = new();

    /// <summary>
    /// The list of overrides (files to replace).
    /// </summary>
    public List<ModpackOverride> Overrides { get; init; } = new();

    /// <summary>
    /// Remote file transfers (Modrinth pack style): files resolved by URL.
    /// </summary>
    public List<ModpackFileTransfer> Files { get; init; } = new();
}

/// <summary>
/// A remote file to download while installing a Modrinth-style modpack.
/// </summary>
public sealed class ModpackFileTransfer
{
    /// <summary>
    /// The path relative to the game directory where the file is placed.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The first download URL.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The SHA1 hash of the file content, used for reuse detection.
    /// </summary>
    public string Sha1 { get; init; } = string.Empty;

    /// <summary>
    /// Whether the file is optional, so that the user may skip it.
    /// </summary>
    public bool Optional { get; init; }
}

/// <summary>
/// The settings of a MultiMC instance, applied to the instance after the
/// installation.
/// </summary>
public sealed class MultiMcInstanceSettings
{
    /// <summary>The path of the Java runtime used by the instance.</summary>
    public string? JavaPath { get; init; }

    /// <summary>The arguments of the Java virtual machine used by the instance.</summary>
    public string? JvmArgs { get; init; }

    /// <summary>The command wrapping the launch of the game.</summary>
    public string? WrapperCommand { get; init; }

    /// <summary>The command run before the game starts.</summary>
    public string? PreLaunchCommand { get; init; }

    /// <summary>The command run after the game exits.</summary>
    public string? PostExitCommand { get; init; }

    /// <summary>The maximum memory of the game, in MiB.</summary>
    public int? MaxMemory { get; init; }

    /// <summary>The minimum memory of the game, in MiB.</summary>
    public int? MinMemory { get; init; }

    /// <summary>The size of the permanent generation area, in MiB.</summary>
    public int? PermGen { get; init; }

    /// <summary>The width of the window of the game.</summary>
    public int? Width { get; init; }

    /// <summary>The height of the window of the game.</summary>
    public int? Height { get; init; }

    /// <summary>Whether the game starts in the fullscreen mode.</summary>
    public bool Fullscreen { get; init; }

    /// <summary>Whether the console of the game is shown.</summary>
    public bool ShowConsole { get; init; }

    /// <summary>Whether the memory of the instance overrides the one of the launcher.</summary>
    public bool OverrideMemory { get; init; }

    /// <summary>Whether the Java runtime of the instance overrides the one of the launcher.</summary>
    public bool OverrideJavaLocation { get; init; }

    /// <summary>Whether the arguments of the virtual machine override the ones of the launcher.</summary>
    public bool OverrideJavaArgs { get; init; }

    /// <summary>Whether the console of the instance overrides the one of the launcher.</summary>
    public bool OverrideConsole { get; init; }

    /// <summary>Whether the commands of the instance override the ones of the launcher.</summary>
    public bool OverrideCommands { get; init; }

    /// <summary>Whether the window of the instance overrides the one of the launcher.</summary>
    public bool OverrideWindow { get; init; }
}

/// <summary>
/// A file of a modpack which the user may skip when the pack is installed.
/// </summary>
public sealed class ModpackOptionalFile
{
    /// <summary>
    /// The key identifying the file, used to remember the choice of the user.
    /// </summary>
    public string Key { get; init; } = string.Empty;

    /// <summary>
    /// The path of the file relative to the game directory.
    /// </summary>
    public string Path { get; init; } = string.Empty;

    /// <summary>
    /// The name of the file shown to the user.
    /// </summary>
    public string FileName { get; init; } = string.Empty;
}

/// <summary>
/// Represents a mod in a modpack manifest.
/// </summary>
public sealed class ModpackMod
{
    /// <summary>
    /// The mod ID (slug on CurseForge/Modrinth).
    /// </summary>
    public string ProjectId { get; init; } = string.Empty;

    /// <summary>
    /// The mod file ID.
    /// </summary>
    public int FileId { get; init; }

    /// <summary>
    /// The name of the mod, when the manifest holds it.
    /// </summary>
    public string? Name { get; init; }

    /// <summary>
    /// Whether this mod is required (not optional).
    /// </summary>
    public bool Required { get; init; } = true;
}

/// <summary>
/// A mod of a modpack that could not be installed, reported to the user after
/// the installation.
/// </summary>
public sealed class UnresolvedMod
{
    /// <summary>
    /// The identifier of the project of the mod.
    /// </summary>
    public string ProjectId { get; init; } = string.Empty;

    /// <summary>
    /// The identifier of the file of the mod.
    /// </summary>
    public int FileId { get; init; }

    /// <summary>
    /// The name of the mod, when it is known.
    /// </summary>
    public string? Name { get; init; }

    /// <summary>
    /// The description of the mod shown to the user.
    /// </summary>
    public string DisplayName => string.IsNullOrEmpty(Name)
        ? $"{ProjectId} ({FileId})"
        : $"{Name} ({ProjectId}:{FileId})";
}

/// <summary>
/// Represents a file override in a modpack.
/// </summary>
public sealed class ModpackOverride
{
    /// <summary>
    /// The source path within the modpack.
    /// </summary>
    public string Source { get; init; } = string.Empty;

    /// <summary>
    /// The destination path in the game directory.
    /// </summary>
    public string Destination { get; init; } = string.Empty;
}

/// <summary>
/// Provider for modpacks (CurseForge, Modrinth, local files).
/// </summary>
public abstract class ModpackProvider
{
    /// <summary>
    /// The name of the provider.
    /// </summary>
    public abstract string Name { get; }

    /// <summary>
    /// Returns whether this provider can handle the specified URL or path.
    /// </summary>
    public abstract bool CanHandle(string pathOrUrl);

    /// <summary>
    /// Loads a modpack from the specified path or URL.
    /// </summary>
    public abstract Task<Modpack> LoadAsync(string pathOrUrl);
}

/// <summary>
/// Exception thrown when a modpack type is not supported.
/// </summary>
public sealed class UnsupportedModpackException : Exception
{
    public UnsupportedModpackException(string message) : base(message) { }
}

/// <summary>
/// Exception thrown during modpack completion/installation.
/// </summary>
public sealed class ModpackCompletionException : Exception
{
    public ModpackCompletionException(string message) : base(message) { }
    public ModpackCompletionException(string message, Exception inner) : base(message, inner) { }
}
