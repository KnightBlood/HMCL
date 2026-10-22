using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using System.Xml.Linq;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Download;

/// <summary>
/// The description of an installer of a mod loader, shown in the download page.
/// </summary>
public sealed class InstallerInfo
{
    /// <summary>
    /// Creates the description of an installer.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="versions">The versions of the loader, the newest first.</param>
    public InstallerInfo(LoaderKind kind, IReadOnlyList<string> versions)
    {
        Kind = kind;
        Versions = versions;
    }

    /// <summary>
    /// The kind of the loader.
    /// </summary>
    public LoaderKind Kind { get; }

    /// <summary>
    /// The name of the loader shown to the user.
    /// </summary>
    public string Name => Kind switch
    {
        LoaderKind.Fabric => "Fabric",
        LoaderKind.Quilt => "Quilt",
        LoaderKind.LegacyFabric => "LegacyFabric",
        LoaderKind.Forge => "Forge",
        LoaderKind.NeoForge => "NeoForge",
        LoaderKind.OptiFine => "OptiFine",
        LoaderKind.LiteLoader => "LiteLoader",
        LoaderKind.Cleanroom => "Cleanroom",
        _ => Kind.ToString()
    };

    /// <summary>
    /// The versions of the loader, the newest first.
    /// </summary>
    public IReadOnlyList<string> Versions { get; }

    /// <summary>
    /// Whether the loader has at least one version.
    /// </summary>
    public bool HasVersions => Versions.Count > 0;
}

/// <summary>
/// Lists the installers of the mod loaders available for a version of the
/// game, like the installer page of Java HMCL.
/// </summary>
public static class InstallerCatalog
{
    /// <summary>
    /// The address of the metadata of the versions of Forge.
    /// </summary>
    private const string ForgeMetadata =
        "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";

    /// <summary>
    /// The address of the metadata of the versions of NeoForge.
    /// </summary>
    private const string NeoForgeMetadata =
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";

    /// <summary>
    /// Lists the installers available for the given version of the game.
    /// </summary>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>
    /// The installers of the loaders, the ones without any version are left
    /// out.
    /// </returns>
    public static async Task<IReadOnlyList<InstallerInfo>> GetAsync(
        string gameVersion, CancellationToken cancellationToken = default)
    {
        var loaders = new List<LoaderKind>
        {
            LoaderKind.Fabric,
            LoaderKind.Quilt,
            LoaderKind.LegacyFabric,
            LoaderKind.Forge,
            LoaderKind.NeoForge,
            LoaderKind.OptiFine,
            LoaderKind.LiteLoader,
            LoaderKind.Cleanroom
        };

        var tasks = loaders.Select(async kind =>
        {
            try
            {
                // A loader whose versions cannot be read in time does not hold
                // back the other ones.
                using var timeout = new CancellationTokenSource(TimeSpan.FromSeconds(20));
                using var linked = CancellationTokenSource.CreateLinkedTokenSource(
                    cancellationToken, timeout.Token);

                var versions = await GetVersionsAsync(kind, gameVersion, linked.Token);
                return versions.Count > 0 ? new InstallerInfo(kind, versions) : null;
            }
            catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException
                                          or System.Text.Json.JsonException
                                          or OperationCanceledException)
            {
                // The versions of the loader cannot be read; the loader is left
                // out of the list.
                return null;
            }
        });

        var results = await Task.WhenAll(tasks);
        return results.Where(info => info != null).Select(info => info!).ToList();
    }

    /// <summary>
    /// Returns the versions of the given loader for the given version of the
    /// game.
    /// </summary>
    /// <param name="kind">The kind of the loader.</param>
    /// <param name="gameVersion">The version of the game.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The versions of the loader, the newest first.</returns>
    public static async Task<IReadOnlyList<string>> GetVersionsAsync(
        LoaderKind kind, string gameVersion, CancellationToken cancellationToken = default)
    {
        switch (kind)
        {
            case LoaderKind.Fabric:
            case LoaderKind.Quilt:
            case LoaderKind.LegacyFabric:
            {
                var versions = await MetaLoaderInstaller.GetLoaderVersionsAsync(kind, gameVersion);
                return versions.Select(version => version.Version)
                    .Where(version => !string.IsNullOrEmpty(version))
                    .ToList();
            }

            case LoaderKind.Forge:
            {
                var versions = await ReadMavenVersionsAsync(ForgeMetadata, cancellationToken);
                return versions
                    .Where(version => version.Contains(gameVersion + "-", StringComparison.OrdinalIgnoreCase))
                    .Select(version => version[(version.IndexOf('-') + 1)..])
                    .ToList();
            }

            case LoaderKind.NeoForge:
                return await ReadMavenVersionsAsync(NeoForgeMetadata, cancellationToken);

            case LoaderKind.OptiFine:
            {
                var version = await LoaderInstaller.GetLoaderVersionAsync(kind, gameVersion, cancellationToken);
                return version is null ? Array.Empty<string>() : new[] { version };
            }

            case LoaderKind.LiteLoader:
            {
                var versions = await LoaderInstaller.GetLoaderVersionsAsync(kind, gameVersion, cancellationToken);
                return versions;
            }

            case LoaderKind.Cleanroom:
                return await ReadMavenVersionsAsync(
                    "https://maven.cleanroommc.net/com/cleanroommc/cleanroom/maven-metadata.xml",
                    cancellationToken);

            default:
                return Array.Empty<string>();
        }
    }

    /// <summary>
    /// Reads the versions published in a maven metadata file.
    /// </summary>
    /// <param name="url">The address of the metadata file.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The versions, the newest first.</returns>
    private static async Task<IReadOnlyList<string>> ReadMavenVersionsAsync(
        string url, CancellationToken cancellationToken)
    {
        var xml = await HttpClientProvider.Shared.GetStringAsync(url, cancellationToken);
        var document = XDocument.Parse(xml);

        var versions = document.Descendants("version")
            .Select(element => element.Value)
            .Where(value => !string.IsNullOrEmpty(value))
            .ToList();

        versions.Reverse();
        return versions;
    }
}
