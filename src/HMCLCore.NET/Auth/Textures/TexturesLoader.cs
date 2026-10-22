using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Download;

namespace HMCLCore.NET.Auth.Textures;

/// <summary>
/// A texture of a player that has been loaded, holding the file of the texture
/// and its metadata.
/// </summary>
public sealed class LoadedTexture
{
    /// <summary>
    /// Creates a loaded texture.
    /// </summary>
    /// <param name="filePath">The path of the file holding the texture.</param>
    /// <param name="metadata">The metadata of the texture.</param>
    public LoadedTexture(string filePath, IReadOnlyDictionary<string, string> metadata)
    {
        FilePath = filePath;
        Metadata = metadata;
    }

    /// <summary>
    /// The path of the file holding the texture.
    /// </summary>
    public string FilePath { get; }

    /// <summary>
    /// The metadata of the texture.
    /// </summary>
    public IReadOnlyDictionary<string, string> Metadata { get; }
}

/// <summary>
/// Loads the skins and the capes of the players, caching them in the
/// directory of the launcher.
/// </summary>
public sealed class TexturesLoader
{
    private static readonly object _lock = new();

    /// <summary>
    /// The directory holding the cached textures.
    /// </summary>
    public string TexturesDirectory { get; }

    /// <summary>
    /// Creates a loader caching the textures in the given directory.
    /// </summary>
    /// <param name="texturesDirectory">The directory holding the cached textures.</param>
    public TexturesLoader(string texturesDirectory)
    {
        TexturesDirectory = texturesDirectory;
    }

    /// <summary>
    /// Returns the path of the file caching the given texture.
    /// </summary>
    /// <param name="url">The address of the texture.</param>
    /// <returns>The path of the file caching the texture.</returns>
    public string GetTexturePath(string url)
    {
        var slash = url.LastIndexOf('/');
        var dot = url.LastIndexOf('.');
        if (dot < slash)
            dot = url.Length;

        var hash = slash >= 0 && dot > slash ? url[(slash + 1)..dot] : url;
        var prefix = hash.Length > 2 ? hash[..2] : "xx";

        return Path.Combine(TexturesDirectory, prefix, hash);
    }

    /// <summary>
    /// Loads the given texture, downloading it when it is not cached yet.
    /// </summary>
    /// <param name="texture">The texture to load.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The loaded texture.</returns>
    /// <exception cref="IOException">Thrown when the texture cannot be loaded.</exception>
    public async Task<LoadedTexture> LoadTextureAsync(Texture texture, CancellationToken token = default)
    {
        if (string.IsNullOrWhiteSpace(texture.Url))
            throw new IOException("The address of the texture is empty.");

        var file = GetTexturePath(texture.Url);

        if (!File.Exists(file))
        {
            var candidates = MirrorSource.GetFileUris(texture.Url);

            var succeeded = await FileDownloader.DownloadAsync(
                candidates, file, null, null, token);

            if (!succeeded && !File.Exists(file))
                throw new IOException($"Failed to download the texture: {texture.Url}");
        }

        return new LoadedTexture(file, texture.Metadata);
    }

    /// <summary>
    /// Loads the given texture, falling back to the default skin of the player
    /// when it cannot be loaded.
    /// </summary>
    /// <param name="texture">The texture to load.</param>
    /// <param name="fallback">The texture used when the texture cannot be loaded.</param>
    /// <param name="token">Cancellation token.</param>
    /// <returns>The loaded texture.</returns>
    public async Task<LoadedTexture> LoadTextureOrDefaultAsync(
        Texture texture, LoadedTexture fallback, CancellationToken token = default)
    {
        try
        {
            return await LoadTextureAsync(texture, token);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            // The default skin is used when the texture cannot be loaded.
            return fallback;
        }
    }

    /// <summary>
    /// Returns the metadata of the default skin of the given player.
    /// </summary>
    /// <param name="uuid">The identifier of the player.</param>
    /// <returns>The metadata of the default skin.</returns>
    public static IReadOnlyDictionary<string, string> GetDefaultSkinMetadata(string uuid)
        => new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["model"] = Texture.GuessModel(uuid) == TextureModel.Slim ? "slim" : "wide"
        };

    /// <summary>
    /// Removes the cached textures that are not used any more.
    /// </summary>
    /// <param name="keep">The addresses of the textures that must be kept.</param>
    /// <returns>The number of removed files.</returns>
    public int ClearCache(IEnumerable<string> keep)
    {
        var kept = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        foreach (var url in keep)
            kept.Add(GetTexturePath(url));

        var removed = 0;
        lock (_lock)
        {
            if (!Directory.Exists(TexturesDirectory))
                return 0;

            foreach (var file in Directory.GetFiles(TexturesDirectory, "*", SearchOption.AllDirectories))
            {
                if (kept.Contains(file))
                    continue;

                try
                {
                    File.Delete(file);
                    removed++;
                }
                catch (IOException)
                {
                    // A locked file is kept.
                }
                catch (UnauthorizedAccessException)
                {
                    // Same as above: the file is kept.
                }
            }
        }

        return removed;
    }
}
