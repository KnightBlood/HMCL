using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;

namespace HMCLCore.NET.Game;

/// <summary>
/// The date filter applied to the list of the screenshots.
/// </summary>
public enum ScreenshotFilter
{
    /// <summary>
    /// Every screenshot is shown.
    /// </summary>
    All,

    /// <summary>
    /// Only the screenshots taken today are shown.
    /// </summary>
    Today,

    /// <summary>
    /// Only the screenshots taken this week are shown.
    /// </summary>
    ThisWeek,

    /// <summary>
    /// Only the screenshots taken this month are shown.
    /// </summary>
    ThisMonth
}

/// <summary>
/// A screenshot of the game, with the date it was taken at.
/// </summary>
public sealed class ScreenshotInfo
{
    /// <summary>
    /// Creates the information of a screenshot.
    /// </summary>
    /// <param name="path">The path of the file holding the screenshot.</param>
    /// <param name="takenAt">The date the screenshot was taken at.</param>
    public ScreenshotInfo(string path, DateTime takenAt)
    {
        Path = path;
        TakenAt = takenAt;
    }

    /// <summary>
    /// The path of the file holding the screenshot.
    /// </summary>
    public string Path { get; }

    /// <summary>
    /// The name of the file holding the screenshot.
    /// </summary>
    public string FileName => System.IO.Path.GetFileName(Path);

    /// <summary>
    /// The date the screenshot was taken at.
    /// </summary>
    public DateTime TakenAt { get; }

    /// <summary>
    /// The date the screenshot was taken at, shown to the user.
    /// </summary>
    public string TakenAtText => TakenAt.ToLocalTime().ToString("yyyy-MM-dd HH:mm:ss");
}

/// <summary>
/// Manages the screenshots of a game instance.
/// </summary>
public sealed class ScreenshotManager
{
    private static readonly string[] Extensions = { ".png", ".jpg", ".jpeg", ".bmp" };

    /// <summary>
    /// Creates a manager for the screenshots of the given game directory.
    /// </summary>
    /// <param name="gameDirectory">The game directory holding the screenshots.</param>
    public ScreenshotManager(string gameDirectory)
    {
        GameDirectory = gameDirectory ?? throw new ArgumentNullException(nameof(gameDirectory));
    }

    /// <summary>
    /// The game directory of the instance.
    /// </summary>
    public string GameDirectory { get; }

    /// <summary>
    /// The directory holding the screenshots of the instance.
    /// </summary>
    public string ScreenshotDirectory => System.IO.Path.Combine(GameDirectory, "screenshots");

    /// <summary>
    /// Returns the screenshots of the instance, the newest ones first.
    /// </summary>
    /// <returns>The screenshots of the instance.</returns>
    public IReadOnlyList<ScreenshotInfo> GetScreenshots()
    {
        var directory = ScreenshotDirectory;
        if (!Directory.Exists(directory))
            return Array.Empty<ScreenshotInfo>();

        try
        {
            return Directory.GetFiles(directory)
                .Where(file => Extensions.Contains(System.IO.Path.GetExtension(file), StringComparer.OrdinalIgnoreCase))
                .Select(file => new ScreenshotInfo(file, File.GetLastWriteTime(file)))
                .OrderByDescending(screenshot => screenshot.TakenAt)
                .ToList();
        }
        catch (IOException)
        {
            // A directory that cannot be read holds no screenshot.
            return Array.Empty<ScreenshotInfo>();
        }
        catch (UnauthorizedAccessException)
        {
            // Same as above.
            return Array.Empty<ScreenshotInfo>();
        }
    }

    /// <summary>
    /// Deletes the given screenshot.
    /// </summary>
    /// <param name="path">The path of the screenshot to delete.</param>
    /// <returns>Whether the screenshot was deleted.</returns>
    public bool Delete(string path)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path))
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
    /// Copies the given screenshot into another directory.
    /// </summary>
    /// <param name="path">The path of the screenshot to copy.</param>
    /// <param name="targetDirectory">The directory receiving the copy.</param>
    /// <returns>The path of the copy, or null when the screenshot cannot be copied.</returns>
    public string? Copy(string path, string targetDirectory)
    {
        if (string.IsNullOrWhiteSpace(path) || !File.Exists(path) ||
            string.IsNullOrWhiteSpace(targetDirectory))
            return null;

        try
        {
            Directory.CreateDirectory(targetDirectory);
            var target = System.IO.Path.Combine(targetDirectory, System.IO.Path.GetFileName(path));
            File.Copy(path, target, true);
            return target;
        }
        catch (IOException)
        {
            return null;
        }
        catch (UnauthorizedAccessException)
        {
            return null;
        }
    }
}
