using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Tasks;

using HMCLCore.NET.Download;
namespace HMCLCore.NET.Addon;

/// <summary>
/// Task for installing a mod from a remote repository.
/// </summary>
public sealed class ModInstallTask : TaskBase
{
    /// <summary>
    /// The mod to install.
    /// </summary>
    public RemoteAddon Addon { get; init; } = new();

    /// <summary>
    /// The destination directory for the mod file.
    /// </summary>
    public string DestinationDirectory { get; init; } = string.Empty;

    /// <summary>
    /// The Minecraft version the mod is for.
    /// </summary>
    public string MinecraftVersion { get; init; } = string.Empty;

    /// <summary>
    /// Creates a new ModInstallTask.
    /// </summary>
    public ModInstallTask(RemoteAddon addon, string destinationDirectory, string minecraftVersion)
    {
        Addon = addon;
        DestinationDirectory = destinationDirectory;
        MinecraftVersion = minecraftVersion;
        Name = $"安装模组 {addon.Name}";
    }

    /// <summary>
    /// Installs the mod by downloading it to the destination directory.
    /// </summary>
    public override async System.Threading.Tasks.Task Run(CancellationToken token = default)
    {
        try
        {
            token.ThrowIfCancellationRequested();

            if (string.IsNullOrEmpty(Addon.DownloadUrl))
                throw new InvalidOperationException("Mod download URL is not available");

            Directory.CreateDirectory(DestinationDirectory);

            var fileName = Path.GetFileName(Addon.DownloadUrl) ?? $"{Addon.Name}.jar";
            var destinationPath = Path.Combine(DestinationDirectory, fileName);

            using var httpClient = HttpClientProvider.CreateClient();
            HttpClientProvider.Shared.DefaultRequestHeaders.Add("User-Agent", "HMCL.NET/1.0");

            using var response = await HttpClientProvider.Shared.GetAsync(Addon.DownloadUrl, HttpCompletionOption.ResponseHeadersRead, token);
            response.EnsureSuccessStatusCode();

            var totalBytes = response.Content.Headers.ContentLength;
            using var fileStream = new FileStream(destinationPath, FileMode.Create, FileAccess.Write, FileShare.None);
            using var contentStream = await response.Content.ReadAsStreamAsync(token);

            var buffer = new byte[8192];
            int bytesRead;
            var totalRead = 0L;

            while ((bytesRead = await contentStream.ReadAsync(buffer, 0, buffer.Length, token)) > 0)
            {
                token.ThrowIfCancellationRequested();
                await fileStream.WriteAsync(buffer, 0, bytesRead, token);
                totalRead += bytesRead;
                OnProgressChanged(totalRead, totalBytes);
            }

            fileStream.Close();
            OnProgressChanged(100, 100);
            IsCompleted = true;
            IsSucceeded = true;
        }
        catch (OperationCanceledException)
        {
            IsCompleted = true;
            IsSucceeded = false;
            throw new OperationCanceledException("Mod install cancelled");
        }
        catch (Exception ex)
        {
            IsCompleted = true;
            IsSucceeded = false;
            Exception = ex;
            throw;
        }
    }
}

