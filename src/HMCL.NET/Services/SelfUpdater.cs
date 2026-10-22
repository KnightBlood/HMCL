using System;
using System.Diagnostics;
using System.IO;
using System.Security.Cryptography;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace HMCL.NET.Services;

/// <summary>
/// The result of the verification of a downloaded update.
/// </summary>
public sealed class UpdateVerification
{
    /// <summary>
    /// Creates the result of the verification.
    /// </summary>
    /// <param name="isValid">Whether the file is a valid update.</param>
    /// <param name="sha256">The SHA-256 hash of the file.</param>
    /// <param name="reason">The reason the file was rejected.</param>
    public UpdateVerification(bool isValid, string sha256, string? reason = null)
    {
        IsValid = isValid;
        Sha256 = sha256;
        Reason = reason;
    }

    /// <summary>
    /// Whether the file is a valid update.
    /// </summary>
    public bool IsValid { get; }

    /// <summary>
    /// The SHA-256 hash of the file.
    /// </summary>
    public string Sha256 { get; }

    /// <summary>
    /// The reason the file was rejected, when it was.
    /// </summary>
    public string? Reason { get; }
}

/// <summary>
/// Replaces the running launcher with a newer version downloaded from the
/// releases of the project, like the update handler of Java HMCL.
/// </summary>
public static class SelfUpdater
{
    /// <summary>
    /// Verifies that the given file is a Windows executable and computes its
    /// hash.
    /// </summary>
    /// <param name="filePath">The path of the downloaded file.</param>
    /// <returns>The result of the verification.</returns>
    public static UpdateVerification Verify(string filePath)
    {
        try
        {
            if (!File.Exists(filePath))
                return new UpdateVerification(false, string.Empty, "更新文件不存在");

            var info = new FileInfo(filePath);
            if (info.Length < 2)
                return new UpdateVerification(false, string.Empty, "更新文件无效");

            using var stream = File.OpenRead(filePath);
            var header = new byte[2];
            if (stream.Read(header, 0, 2) != 2 || header[0] != (byte)'M' || header[1] != (byte)'Z')
                return new UpdateVerification(false, string.Empty, "更新文件不是有效的可执行文件");

            using var sha = SHA256.Create();
            var hash = sha.ComputeHash(File.ReadAllBytes(filePath));

            return new UpdateVerification(true, Convert.ToHexString(hash));
        }
        catch (Exception ex)
        {
            return new UpdateVerification(false, string.Empty, ex.Message);
        }
    }

    /// <summary>
    /// Replaces the running launcher with the given executable and starts the
    /// new version.
    /// </summary>
    /// <param name="newExecutablePath">The path of the downloaded executable.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>Whether the update was applied.</returns>
    public static async Task<bool> ApplyAsync(string newExecutablePath, CancellationToken cancellationToken = default)
    {
        var currentExecutable = Environment.ProcessPath;
        if (string.IsNullOrEmpty(currentExecutable) || !File.Exists(currentExecutable))
            return false;

        var verification = Verify(newExecutablePath);
        if (!verification.IsValid)
            return false;

        try
        {
            // The running executable cannot be replaced, so a script waiting
            // for the launcher to exit does it.
            var scriptPath = Path.Combine(Path.GetTempPath(), $"hmcl-update-{Guid.NewGuid():N}.bat");
            var processName = Path.GetFileNameWithoutExtension(currentExecutable);

            var script = new StringBuilder()
                .AppendLine("@echo off")
                .AppendLine(":wait")
                .AppendLine("timeout /t 1 /nobreak >nul")
                .AppendLine($"tasklist /fi \"imagename eq {processName}\" | find \"{processName}\" >nul && goto wait")
                .AppendLine($"copy /y \"{newExecutablePath}\" \"{currentExecutable}\" >nul")
                .AppendLine($"del \"{newExecutablePath}\" >nul 2>&1")
                .AppendLine($"start \"\" \"{currentExecutable}\"")
                .ToString();

            await File.WriteAllTextAsync(scriptPath, script, cancellationToken);

            using var process = Process.Start(new ProcessStartInfo
            {
                FileName = scriptPath,
                UseShellExecute = true,
                CreateNoWindow = true
            });

            return process != null;
        }
        catch (Exception)
        {
            return false;
        }
    }
}
