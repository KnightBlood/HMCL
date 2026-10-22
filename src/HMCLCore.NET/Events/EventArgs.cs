using System;

namespace HMCLCore.NET.Events;

/// <summary>
/// Base class for all launcher events.
/// </summary>
public abstract class EventBase
{
    /// <summary>
    /// The time the event was raised.
    /// </summary>
    public DateTime Timestamp { get; } = DateTime.UtcNow;

    /// <summary>
    /// The source of the event.
    /// </summary>
    public object? Source { get; init; }
}

/// <summary>
/// Event raised when a task completes.
/// </summary>
public sealed class TaskCompletedEvent : EventBase
{
    public TaskCompletedEvent(string taskName, bool succeeded, Exception? exception = null)
    {
        TaskName = taskName;
        Succeeded = succeeded;
        Exception = exception;
    }

    public string TaskName { get; }
    public bool Succeeded { get; }
    public Exception? Exception { get; }
}

/// <summary>
/// Event raised when download progress changes.
/// </summary>
public sealed class DownloadProgressEvent : EventBase
{
    public DownloadProgressEvent(string url, long downloadedBytes, long? totalBytes)
    {
        Url = url;
        DownloadedBytes = downloadedBytes;
        TotalBytes = totalBytes;
    }

    public string Url { get; }
    public long DownloadedBytes { get; }
    public long? TotalBytes { get; }

    /// <summary>
    /// Returns the download percentage, or 0 if total bytes is unknown.
    /// </summary>
    public double Percentage => TotalBytes.HasValue && TotalBytes.Value > 0
        ? (double)DownloadedBytes / TotalBytes.Value * 100.0
        : 0;
}

/// <summary>
/// Event raised when the game launch state changes.
/// </summary>
public sealed class GameLaunchEvent : EventBase
{
    public GameLaunchEvent(string version, LaunchState state)
    {
        Version = version;
        State = state;
    }

    public string Version { get; }
    public LaunchState State { get; }
}

/// <summary>
/// The launch state of the game.
/// </summary>
public enum LaunchState
{
    Preparing,
    Launching,
    Running,
    Exited,
    Failed
}

/// <summary>
/// Event raised when account login state changes.
/// </summary>
public sealed class AccountEvent : EventBase
{
    public AccountEvent(string accountType, string username, AccountEventType eventType)
    {
        AccountType = accountType;
        Username = username;
        EventType = eventType;
    }

    public string AccountType { get; }
    public string Username { get; }
    public AccountEventType EventType { get; }
}

/// <summary>
/// The type of account event.
/// </summary>
public enum AccountEventType
{
    LoggedIn,
    LoggedOut,
    LoginFailed,
    TokenRefreshed,
    TokenExpired
}

/// <summary>
/// Event arguments for download progress events raised by DownloadTask.
/// </summary>
public sealed class DownloadProgressEventArgs : System.EventArgs
{
    /// <summary>
    /// The number of bytes downloaded so far.
    /// </summary>
    public long DownloadedBytes { get; }

    /// <summary>
    /// The total size of the file in bytes, if known.
    /// </summary>
    public long? TotalBytes { get; }

    /// <summary>
    /// Returns the download percentage, or 0 if total bytes is unknown.
    /// </summary>
    public double Percentage => TotalBytes.HasValue && TotalBytes.Value > 0
        ? (double)DownloadedBytes / TotalBytes.Value * 100.0
        : 0;

    public DownloadProgressEventArgs(long downloadedBytes, long? totalBytes)
    {
        DownloadedBytes = downloadedBytes;
        TotalBytes = totalBytes;
    }
}
