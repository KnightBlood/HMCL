using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using HMCLCore.NET.Events;

namespace HMCLCore.NET.Tasks;

/// <summary>
/// The lifecycle state of a task tracked by the <see cref="TaskManager"/>.
/// </summary>
public enum TrackedTaskState
{
    /// <summary>
    /// The task is currently running.
    /// </summary>
    Running = 0,

    /// <summary>
    /// The task finished successfully.
    /// </summary>
    Succeeded = 1,

    /// <summary>
    /// The task failed with an error.
    /// </summary>
    Failed = 2,

    /// <summary>
    /// The task was cancelled by the user.
    /// </summary>
    Cancelled = 3
}

/// <summary>
/// A task tracked by the <see cref="TaskManager"/>, exposing its live progress.
/// </summary>
public sealed class TrackedTask
{
    private readonly CancellationTokenSource _cts;

    /// <summary>
    /// Creates a tracked task wrapper.
    /// </summary>
    /// <param name="task">The task being tracked.</param>
    /// <param name="cts">The cancellation source controlling the task.</param>
    internal TrackedTask(TaskBase task, CancellationTokenSource cts)
    {
        Task = task;
        _cts = cts;
    }

    /// <summary>
    /// The tracked task.
    /// </summary>
    public TaskBase Task { get; }

    /// <summary>
    /// The display name of the task.
    /// </summary>
    public string Name => Task.Name;

    /// <summary>
    /// The number of bytes processed so far.
    /// </summary>
    public long DownloadedBytes { get; internal set; }

    /// <summary>
    /// The total number of bytes to process, or null when unknown.
    /// </summary>
    public long? TotalBytes { get; internal set; }

    /// <summary>
    /// The completion percentage between 0 and 100, or null when the total is unknown.
    /// </summary>
    public double? Percentage => TotalBytes is { } total && total > 0
        ? Math.Clamp((double)DownloadedBytes / total * 100.0, 0, 100)
        : null;

    /// <summary>
    /// The current lifecycle state of the task.
    /// </summary>
    public TrackedTaskState State { get; internal set; } = TrackedTaskState.Running;

    /// <summary>
    /// The error message when the task failed, otherwise null.
    /// </summary>
    public string? ErrorMessage { get; internal set; }

    /// <summary>
    /// A human-readable summary of the current progress.
    /// </summary>
    public string ProgressText
    {
        get
        {
            if (State != TrackedTaskState.Running)
                return State switch
                {
                    TrackedTaskState.Succeeded => "已完成",
                    TrackedTaskState.Failed => "失败",
                    _ => "已取消"
                };

            if (Percentage is { } percentage)
                return $"{percentage:F1}%";

            if (TotalBytes is { } total && total > 0)
                return $"{DownloadedBytes / 1024} / {total / 1024} KB";

            return "正在处理...";
        }
    }

    /// <summary>
    /// Requests cancellation of the task.
    /// </summary>
    public void Cancel()
    {
        Task.Cancel();
        try
        {
            _cts.Cancel();
        }
        catch (ObjectDisposedException)
        {
            // The task already finished and released its cancellation source.
        }
    }
}

/// <summary>
/// Tracks the launcher tasks that are running so the user interface can show
/// their names and progress, mirroring the Java HMCL task list.
/// </summary>
public sealed class TaskManager
{
    private readonly List<TrackedTask> _tasks = new();
    private readonly object _sync = new();

    /// <summary>
    /// The tasks currently tracked, in the order they were started.
    /// </summary>
    public IReadOnlyList<TrackedTask> Tasks
    {
        get
        {
            lock (_sync)
                return _tasks.ToArray();
        }
    }

    /// <summary>
    /// The number of tasks that are currently running.
    /// </summary>
    public int RunningCount
    {
        get
        {
            lock (_sync)
                return _tasks.Count(t => t.State == TrackedTaskState.Running);
        }
    }

    /// <summary>
    /// Raised whenever a task is added, updated or removed.
    /// </summary>
    public event EventHandler? TasksChanged;

    /// <summary>
    /// Runs a task while tracking its progress and lifetime.
    /// </summary>
    /// <param name="task">The task to run.</param>
    /// <param name="token">A token that cancels the task in addition to the tracked cancellation source.</param>
    /// <returns>Whether the task finished successfully.</returns>
    /// <exception cref="OperationCanceledException">The task was cancelled.</exception>
    public async Task<bool> RunAsync(TaskBase task, CancellationToken token = default)
    {
        var cts = CancellationTokenSource.CreateLinkedTokenSource(token);
        var tracked = new TrackedTask(task, cts);

        task.ProgressChanged += OnTaskProgressChanged;

        lock (_sync)
            _tasks.Add(tracked);
        OnTasksChanged();

        try
        {
            await task.Run(cts.Token);
            tracked.State = TrackedTaskState.Succeeded;
            EventBus.Instance.Publish(new TaskCompletedEvent(task.Name, true));
            return true;
        }
        catch (OperationCanceledException)
        {
            tracked.State = TrackedTaskState.Cancelled;
            EventBus.Instance.Publish(new TaskCompletedEvent(task.Name, false, null));
            throw;
        }
        catch (Exception ex)
        {
            tracked.State = TrackedTaskState.Failed;
            tracked.ErrorMessage = ex.Message;
            EventBus.Instance.Publish(new TaskCompletedEvent(task.Name, false, ex));
            throw;
        }
        finally
        {
            task.ProgressChanged -= OnTaskProgressChanged;
            lock (_sync)
                _tasks.Remove(tracked);
            OnTasksChanged();
            cts.Dispose();
        }
    }

    /// <summary>
    /// Cancels all running tasks.
    /// </summary>
    public void CancelAll()
    {
        foreach (var task in Tasks)
        {
            if (task.State == TrackedTaskState.Running)
                task.Cancel();
        }
    }

    /// <summary>
    /// Removes the finished tasks from the list, keeping the running ones.
    /// </summary>
    public void ClearFinished()
    {
        lock (_sync)
            _tasks.RemoveAll(t => t.State != TrackedTaskState.Running);
        OnTasksChanged();
    }

    private void OnTaskProgressChanged(object? sender, Events.DownloadProgressEventArgs e)
    {
        lock (_sync)
        {
            foreach (var task in _tasks)
            {
                if (ReferenceEquals(task.Task, sender))
                {
                    task.DownloadedBytes = e.DownloadedBytes;
                    task.TotalBytes = e.TotalBytes;
                    break;
                }
            }
        }

        OnTasksChanged();
    }

    private void OnTasksChanged() => TasksChanged?.Invoke(this, EventArgs.Empty);
}
