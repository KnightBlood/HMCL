using System;
using System.Threading.Tasks;
using System.Windows.Input;

namespace HMCL.NET.ViewModels;

/// <summary>
/// A simple synchronous/asynchronous implementation of <see cref="ICommand"/> for Avalonia data binding.
/// The command re-raises <see cref="CanExecuteChanged"/> on demand so that bound controls
/// (e.g. buttons) refresh their enabled state.
/// </summary>
public sealed class RelayCommand : ICommand
{
    private readonly Func<Task>? _executeAsync;
    private readonly Action? _execute;
    private readonly Action<object?>? _executeWithParameter;
    private readonly Func<bool>? _canExecute;

    private bool _isExecuting;

    /// <summary>
    /// Raised when the command's ability to execute changes.
    /// </summary>
    public event EventHandler? CanExecuteChanged;

    /// <summary>
    /// Creates a new RelayCommand that executes an asynchronous action.
    /// </summary>
    /// <param name="execute">The async action to execute.</param>
    /// <param name="canExecute">Optional predicate determining whether the command may execute.</param>
    public RelayCommand(Func<Task> execute, Func<bool>? canExecute = null)
    {
        _executeAsync = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }

    /// <summary>
    /// Creates a new RelayCommand that executes a synchronous action.
    /// </summary>
    /// <param name="execute">The action to execute.</param>
    /// <param name="canExecute">Optional predicate determining whether the command may execute.</param>
    public RelayCommand(Action execute, Func<bool>? canExecute = null)
    {
        _execute = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }

    /// <summary>
    /// Creates a new RelayCommand that executes a synchronous action with the
    /// parameter given to the command.
    /// </summary>
    /// <param name="execute">The action to execute.</param>
    /// <param name="canExecute">Optional predicate determining whether the command may execute.</param>
    public RelayCommand(Action<object?> execute, Func<bool>? canExecute = null)
    {
        _executeWithParameter = execute ?? throw new ArgumentNullException(nameof(execute));
        _canExecute = canExecute;
    }

    /// <summary>
    /// Determines whether the command can execute. Returns false while the command is running.
    /// </summary>
    public bool CanExecute(object? parameter) =>
        !_isExecuting && (_canExecute?.Invoke() ?? true);

    /// <summary>
    /// Executes the command, guarding against re-entrant execution.
    /// </summary>
    public async void Execute(object? parameter)
    {
        if (!CanExecute(parameter))
            return;

        _isExecuting = true;
        RaiseCanExecuteChanged();

        try
        {
            if (_executeAsync is not null)
                await _executeAsync();
            else if (_executeWithParameter is not null)
                _executeWithParameter(parameter);
            else
                _execute!();
        }
        finally
        {
            _isExecuting = false;
            RaiseCanExecuteChanged();
        }
    }

    /// <summary>
    /// Notifies bound controls that <see cref="CanExecute"/> may have changed.
    /// </summary>
    public void RaiseCanExecuteChanged() =>
        CanExecuteChanged?.Invoke(this, EventArgs.Empty);
}
