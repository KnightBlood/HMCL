using System;
using System.Threading;
using System.Threading.Tasks;

namespace HMCLCore.NET.Task;

/// <summary>
/// Base task abstraction.
/// </summary>
public abstract class TaskBase
{
    public abstract System.Threading.Tasks.Task Run(CancellationToken token = default);
}

/// <summary>
/// Generic task.
/// </summary>
public abstract class Task<TResult> : TaskBase where TResult : notnull
{
    public TResult? Result { get; protected set; }
    public override System.Threading.Tasks.Task Run(CancellationToken token = default) => DoRun(token);

    public abstract System.Threading.Tasks.Task DoRun(CancellationToken token = default);
}