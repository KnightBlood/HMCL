using System;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Events;

namespace HMCLCore.NET.Tasks;

/// <summary>
/// Base class for all launcher tasks.
/// </summary>
public abstract class TaskBase
{
    /// <summary>
    /// The task name for display purposes.
    /// </summary>
    public virtual string Name { get; set; } = string.Empty;

    /// <summary>
    /// Returns whether the task has been cancelled.
    /// </summary>
    public bool IsCancelled { get; protected set; }

    /// <summary>
    /// Returns whether the task has completed.
    /// </summary>
    public bool IsCompleted { get; protected set; }

    /// <summary>
    /// Returns whether the task succeeded.
    /// </summary>
    public bool IsSucceeded { get; protected set; }

    /// <summary>
    /// The exception if the task failed, null otherwise.
    /// </summary>
    public Exception? Exception { get; protected set; }

    /// <summary>
    /// Raised when download/task progress changes.
    /// </summary>
    public event EventHandler<DownloadProgressEventArgs>? ProgressChanged;

    /// <summary>
    /// Runs the task asynchronously.
    /// </summary>
    public abstract Task Run(CancellationToken token = default);

    /// <summary>
    /// Cancels the task.
    /// </summary>
    public void Cancel()
    {
        IsCancelled = true;
    }

    /// <summary>
    /// Raises the ProgressChanged event.
    /// </summary>
    protected void OnProgressChanged(long downloadedBytes, long? totalBytes)
    {
        ProgressChanged?.Invoke(this, new DownloadProgressEventArgs(downloadedBytes, totalBytes));
    }
}

/// <summary>
/// Task error information.
/// </summary>
public sealed class TaskError
{
    /// <summary>
    /// The error message.
    /// </summary>
    public string Message { get; init; } = string.Empty;

    /// <summary>
    /// The exception that caused the error.
    /// </summary>
    public Exception? Exception { get; init; }

    /// <summary>
    /// Returns whether this is a cancellation error.
    /// </summary>
    public bool IsCancelled { get; init; }

    private TaskError(string message, Exception? exception, bool isCancelled)
    {
        Message = message;
        Exception = exception;
        IsCancelled = isCancelled;
    }

    /// <summary>
    /// Creates a cancellation error.
    /// </summary>
    public static TaskError Cancelled() => new("Task was cancelled", null, true);

    /// <summary>
    /// Creates a generic error.
    /// </summary>
    public static TaskError Failed(string message, Exception? exception = null) => new(message, exception, false);
}

/// <summary>
/// Cancellation token wrapper for tasks.
/// </summary>
public sealed class CancelToken
{
    private readonly CancellationTokenSource _cts = new();

    /// <summary>
    /// The underlying CancellationToken.
    /// </summary>
    public CancellationToken Token => _cts.Token;

    /// <summary>
    /// Returns whether cancellation has been requested.
    /// </summary>
    public bool IsCancellationRequested => _cts.Token.IsCancellationRequested;

    /// <summary>
    /// Requests cancellation.
    /// </summary>
    public void Cancel() => _cts.Cancel();

    /// <summary>
    /// Disposes the token source.
    /// </summary>
    public void Dispose() => _cts.Dispose();
}
