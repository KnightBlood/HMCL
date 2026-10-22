using System;
using System.IO;
using System.Reflection;
using HMCLCore.NET.Game;
using HMCLCore.NET.Tasks;

namespace HMCLCore.NET.Download;

/// <summary>
/// The Java libraries shipped with the launcher, like the MultiMC bootstrap and
/// the transformer discovery service of Java HMCL.
/// </summary>
public static class BundledLibraries
{
    /// <summary>
    /// The group of the libraries shipped with the launcher.
    /// </summary>
    public const string GroupId = "org.jackhuang.hmcl";

    /// <summary>
    /// The artifact of the bootstrap of the MultiMC instances.
    /// </summary>
    public const string MultiMcBootstrapArtifact = "mmc-bootstrap";

    /// <summary>
    /// The artifact of the transformer discovery service of ModLauncher.
    /// </summary>
    public const string TransformerDiscoveryServiceArtifact = "transformer-discovery-service";

    /// <summary>
    /// The resource holding the bootstrap of the MultiMC instances.
    /// </summary>
    public const string MultiMcBootstrapResource = "HMCLCore.NET.Resources.HMCLMultiMCBootstrap-1.0.jar";

    /// <summary>
    /// The resource holding the transformer discovery service.
    /// </summary>
    public const string TransformerDiscoveryServiceResource =
        "HMCLCore.NET.Resources.HMCLTransformerDiscoveryService-1.0.jar";

    /// <summary>
    /// Returns the resource holding the given library, when the library is
    /// shipped with the launcher.
    /// </summary>
    /// <param name="library">The library.</param>
    /// <param name="resourceName">The name of the resource holding the library.</param>
    /// <returns>Whether the library is shipped with the launcher.</returns>
    public static bool TryGetResource(Library library, out string resourceName)
    {
        if (library.Is(GroupId, MultiMcBootstrapArtifact))
        {
            resourceName = MultiMcBootstrapResource;
            return true;
        }

        if (library.Is(GroupId, TransformerDiscoveryServiceArtifact))
        {
            resourceName = TransformerDiscoveryServiceResource;
            return true;
        }

        resourceName = string.Empty;
        return false;
    }
}

/// <summary>
/// Writes a Java library shipped with the launcher into the library directory
/// of the launcher.
/// </summary>
public sealed class BundledLibraryTask : TaskBase
{
    private readonly string _resourceName;
    private readonly string _destinationPath;

    /// <summary>
    /// Creates the task writing the given resource into the given path.
    /// </summary>
    /// <param name="resourceName">The name of the embedded resource.</param>
    /// <param name="destinationPath">The path of the written file.</param>
    public BundledLibraryTask(string resourceName, string destinationPath)
    {
        _resourceName = resourceName;
        _destinationPath = destinationPath;
    }

    /// <inheritdoc />
    public override async Task Run(CancellationToken token = default)
    {
        var directory = Path.GetDirectoryName(_destinationPath);
        if (!string.IsNullOrEmpty(directory))
            Directory.CreateDirectory(directory);

        await using var source = typeof(BundledLibraryTask).GetTypeInfo().Assembly
            .GetManifestResourceStream(_resourceName);

        if (source is null)
            throw new FileNotFoundException($"内置库 {_resourceName} 丢失");

        await using var destination = File.Create(_destinationPath);
        await source.CopyToAsync(destination, token);
    }
}
