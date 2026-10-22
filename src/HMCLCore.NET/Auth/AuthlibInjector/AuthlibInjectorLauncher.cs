using System;
using System.IO;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Auth.AuthlibInjector;

/// <summary>
/// Prepares the authlib-injector agent JAR for launching with third-party
/// Yggdrasil accounts: downloads it once into a cache directory and reuses
/// the cached copy on later launches.
/// </summary>
public sealed class AuthlibInjectorLauncher
{

    /// <summary>
    /// Ensures the authlib-injector JAR exists under the given cache directory,
    /// downloading the latest GitHub release when missing or empty.
    /// </summary>
    /// <param name="cacheDirectory">Directory that stores <c>authlib-injector.jar</c>.</param>
    /// <param name="token">A token to cancel the download.</param>
    /// <returns>The absolute path of the JAR.</returns>
    /// <exception cref="HttpRequestException">The download failed.</exception>
    public async Task<string> EnsureJarAsync(string cacheDirectory, CancellationToken token = default)
    {
        Directory.CreateDirectory(cacheDirectory);
        var jarPath = Path.Combine(cacheDirectory, "authlib-injector.jar");

        // A non-empty cached JAR is reused across launches.
        if (File.Exists(jarPath) && new FileInfo(jarPath).Length > 0)
            return jarPath;

        var url = SimpleAuthlibInjectorArtifactProvider.DefaultUrl;
        using var response = await HttpClientProvider.Shared.GetAsync(url, HttpCompletionOption.ResponseHeadersRead, token);
        response.EnsureSuccessStatusCode();

        await using var content = await response.Content.ReadAsStreamAsync(token);
        var tempPath = jarPath + ".download";
        await using (var file = File.Create(tempPath))
        {
            await content.CopyToAsync(file, token);
        }

        File.Move(tempPath, jarPath, overwrite: true);
        return jarPath;
    }

    /// <summary>
    /// Fetches the server's root endpoint and base64-encodes its body so the
    /// game can skip the initial metadata request when launching. The result is
    /// cached in <paramref name="cacheDirectory"/> keyed by the server URL, so
    /// repeated launches do not re-fetch the same metadata.
    /// </summary>
    /// <param name="serverUrl">The Yggdrasil/authlib-injector server URL.</param>
    /// <param name="cacheDirectory">Directory used to persist the prefetch cache.</param>
    /// <param name="token">A token to cancel the request.</param>
    /// <returns>Base64 of the root response body, or empty when fetching fails.</returns>
    public async Task<string> FetchPrefetchDataAsync(string serverUrl, string cacheDirectory,
        CancellationToken token = default)
    {
        var cacheFile = GetPrefetchCachePath(cacheDirectory, serverUrl);

        // Return cached metadata without hitting the network.
        if (File.Exists(cacheFile))
        {
            try
            {
                return await File.ReadAllTextAsync(cacheFile, token);
            }
            catch
            {
                // Corrupt cache file: fall through and re-fetch.
            }
        }

        try
        {
            var requestUrl = serverUrl.TrimEnd('/') + "/";
            using var response = await HttpClientProvider.Shared.GetAsync(requestUrl, HttpCompletionOption.ResponseHeadersRead, token);
            response.EnsureSuccessStatusCode();

            await using var content = await response.Content.ReadAsStreamAsync(token);
            using var memory = new MemoryStream();
            await content.CopyToAsync(memory, token);
            var base64 = Convert.ToBase64String(memory.ToArray());

            // Persist for subsequent launches (best-effort).
            try
            {
                Directory.CreateDirectory(cacheDirectory);
                await File.WriteAllTextAsync(cacheFile, base64, token);
            }
            catch
            {
                // Cache write failure is not fatal: the value is still returned.
            }

            return base64;
        }
        catch
        {
            // Prefetch is a best-effort optimization: a failure never blocks launch.
            return string.Empty;
        }
    }

    /// <summary>
    /// Returns the cache file path for a given server URL (SHA1 of the URL as
    /// file name to avoid illegal characters).
    /// </summary>
    private static string GetPrefetchCachePath(string cacheDirectory, string serverUrl)
    {
        using var sha1 = SHA1.Create();
        var hash = Convert.ToHexString(sha1.ComputeHash(Encoding.UTF8.GetBytes(serverUrl)));
        return Path.Combine(cacheDirectory, $"prefetch-{hash}.b64");
    }
}
