using System;

namespace HMCLCore.NET.Tasks;

/// <summary>
/// An <see cref="IProgress{T}"/> that reports its values synchronously on
/// the thread that reports them.
/// </summary>
/// <remarks>
/// The progress of a download has to reach the task that owns it before the
/// download continues, otherwise the reported values would be delivered in an
/// arbitrary order and after the task finished.
/// </remarks>
/// <typeparam name="T">The type of the reported values.</typeparam>
public sealed class InlineProgress<T> : IProgress<T>
{
    private readonly Action<T> _handler;

    /// <summary>
    /// Creates a new InlineProgress.
    /// </summary>
    /// <param name="handler">The handler receiving the reported values.</param>
    public InlineProgress(Action<T> handler)
    {
        _handler = handler ?? throw new ArgumentNullException(nameof(handler));
    }

    /// <summary>
    /// Reports a value to the handler.
    /// </summary>
    /// <param name="value">The value to report.</param>
    public void Report(T value) => _handler(value);
}
