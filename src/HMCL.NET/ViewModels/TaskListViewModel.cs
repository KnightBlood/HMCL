using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Avalonia.Threading;
using HMCLCore.NET.Tasks;

namespace HMCL.NET.ViewModels;

/// <summary>
/// A single task shown in the task list, exposing the tracked task state as
/// bindable properties.
/// </summary>
public sealed class TaskItemViewModel : ViewModelBase
{
    private readonly TrackedTask _tracked;

    /// <summary>
    /// Creates a view model for the given tracked task.
    /// </summary>
    /// <param name="tracked">The tracked task to display.</param>
    public TaskItemViewModel(TrackedTask tracked)
    {
        _tracked = tracked;
    }

    /// <summary>
    /// The tracked task behind this item.
    /// </summary>
    public TrackedTask Tracked => _tracked;

    /// <summary>
    /// The display name of the task.
    /// </summary>
    public string Name => _tracked.Name;

    /// <summary>
    /// The completion percentage between 0 and 100, or null when unknown.
    /// </summary>
    public double? Percentage => _tracked.Percentage;

    /// <summary>
    /// Whether the progress bar should be shown in indeterminate mode.
    /// </summary>
    public bool IsIndeterminate => _tracked.State == TrackedTaskState.Running && _tracked.Percentage == null;

    /// <summary>
    /// The progress bar value between 0 and 100; zero when the total is unknown.
    /// </summary>
    public double ProgressValue => _tracked.Percentage ?? 0;

    /// <summary>
    /// The human-readable progress summary.
    /// </summary>
    public string ProgressText => _tracked.ProgressText;

    /// <summary>
    /// The error message when the task failed, otherwise null.
    /// </summary>
    public string? ErrorMessage => _tracked.ErrorMessage;

    /// <summary>
    /// Whether the task is still running and can therefore be cancelled.
    /// </summary>
    public bool CanCancel => _tracked.State == TrackedTaskState.Running;

    /// <summary>
    /// The status icon of the task: a clock while running, a check on success,
    /// a cross on failure and a slash when cancelled.
    /// </summary>
    public string StatusIcon => _tracked.State switch
    {
        TrackedTaskState.Succeeded => "✓",
        TrackedTaskState.Failed => "✗",
        TrackedTaskState.Cancelled => "⊘",
        _ => "⧗"
    };

    /// <summary>
    /// Refreshes the bindable properties from the tracked task.
    /// </summary>
    public void Refresh()
    {
        OnPropertyChanged(nameof(Name));
        OnPropertyChanged(nameof(Percentage));
        OnPropertyChanged(nameof(ProgressValue));
        OnPropertyChanged(nameof(IsIndeterminate));
        OnPropertyChanged(nameof(ProgressText));
        OnPropertyChanged(nameof(ErrorMessage));
        OnPropertyChanged(nameof(CanCancel));
        OnPropertyChanged(nameof(StatusIcon));
    }

    /// <summary>
    /// Requests cancellation of the task.
    /// </summary>
    public void Cancel() => _tracked.Cancel();
}

/// <summary>
/// View model backing the task list page and the task progress indicator in
/// the sidebar. It mirrors the Java HMCL task list, showing every running task
/// together with its progress.
/// </summary>
public sealed class TaskListViewModel : ViewModelBase
{
    private readonly TaskManager _manager;
    private readonly Dictionary<TrackedTask, TaskItemViewModel> _items = new();

    private bool _isBusy;

    /// <summary>
    /// Creates a view model observing the given task manager.
    /// </summary>
    /// <param name="manager">The task manager to observe.</param>
    public TaskListViewModel(TaskManager manager)
    {
        _manager = manager ?? throw new ArgumentNullException(nameof(manager));
        _manager.TasksChanged += OnTasksChanged;
    }

    /// <summary>
    /// The tasks shown in the list.
    /// </summary>
    public ObservableCollection<TaskItemViewModel> Tasks { get; } = new();

    /// <summary>
    /// Whether at least one task is currently running.
    /// </summary>
    public bool IsBusy
    {
        get => _isBusy;
        private set => Set(ref _isBusy, value);
    }

    /// <summary>
    /// The number of tasks that are currently running.
    /// </summary>
    public int RunningCount => _manager.RunningCount;

    /// <summary>
    /// The name of the most recently started running task, or an empty string.
    /// </summary>
    public string CurrentTaskName
    {
        get
        {
            var tasks = _manager.Tasks;
            for (var i = tasks.Count - 1; i >= 0; i--)
            {
                if (tasks[i].State == TrackedTaskState.Running)
                    return tasks[i].Name;
            }

            return string.Empty;
        }
    }

    /// <summary>
    /// The progress of the most recently started running task, or null when
    /// there is no running task or its progress is unknown.
    /// </summary>
    public double? CurrentTaskPercentage
    {
        get
        {
            var tasks = _manager.Tasks;
            for (var i = tasks.Count - 1; i >= 0; i--)
            {
                if (tasks[i].State == TrackedTaskState.Running)
                    return tasks[i].Percentage;
            }

            return null;
        }
    }

    /// <summary>
    /// Whether the progress of the most recently started running task is
    /// indeterminate because its total size is unknown.
    /// </summary>
    public bool CurrentTaskIsIndeterminate => CurrentTaskPercentage == null;

    /// <summary>
    /// The progress bar value of the most recently started running task;
    /// zero when there is no running task or its total size is unknown.
    /// </summary>
    public double CurrentTaskProgressValue => CurrentTaskPercentage ?? 0;

    /// <summary>
    /// Cancels every running task.
    /// </summary>
    public void CancelAll() => _manager.CancelAll();

    /// <summary>
    /// Removes the finished tasks from the list.
    /// </summary>
    public void ClearFinished() => _manager.ClearFinished();

    private void OnTasksChanged(object? sender, EventArgs e)
    {
        // The manager raises its events from worker threads, so the observable
        // collection must only be touched on the UI thread.
        if (Dispatcher.UIThread.CheckAccess())
            Refresh();
        else
            Dispatcher.UIThread.Post(Refresh);
    }

    private void Refresh()
    {
        var current = _manager.Tasks;

        // Remove the items whose task is no longer tracked.
        var stale = new List<TaskItemViewModel>();
        foreach (var item in Tasks)
        {
            if (!current.Contains(item.Tracked))
                stale.Add(item);
        }

        foreach (var item in stale)
        {
            Tasks.Remove(item);
            _items.Remove(item.Tracked);
        }

        // Add the items for newly started tasks.
        foreach (var task in current)
        {
            if (!_items.TryGetValue(task, out var item))
            {
                item = new TaskItemViewModel(task);
                _items[task] = item;
                Tasks.Add(item);
            }

            item.Refresh();
        }

        IsBusy = _manager.RunningCount > 0;
        OnPropertyChanged(nameof(RunningCount));
        OnPropertyChanged(nameof(CurrentTaskName));
        OnPropertyChanged(nameof(CurrentTaskPercentage));
        OnPropertyChanged(nameof(CurrentTaskIsIndeterminate));
        OnPropertyChanged(nameof(CurrentTaskProgressValue));
    }
}
