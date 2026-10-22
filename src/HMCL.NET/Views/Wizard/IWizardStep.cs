using System;
using System.Threading.Tasks;
using Avalonia.Controls;

namespace HMCL.NET.Views.Wizard;

/// <summary>
/// A single page of a wizard dialog. Mirrors the concept of Java HMCL's
/// wizard pages, where each page contributes its own content and decides
/// whether the user may continue.
/// </summary>
public interface IWizardStep
{
    /// <summary>
    /// The title of the step, shown in the wizard header.
    /// </summary>
    string Title { get; }

    /// <summary>
    /// The content displayed while the step is active.
    /// </summary>
    Control Content { get; }

    /// <summary>
    /// Whether the wizard may continue to the next step.
    /// </summary>
    bool CanGoNext { get; }

    /// <summary>
    /// Called when the step becomes visible.
    /// </summary>
    /// <returns>A task that completes when the step is ready.</returns>
    Task OnEnterAsync();

    /// <summary>
    /// Called when the user leaves the step, before the next step is shown.
    /// </summary>
    /// <returns>A task that completes when the step has been left.</returns>
    Task OnLeaveAsync();
}

/// <summary>
/// Implemented by wizard steps whose ability to continue changes while the
/// step is visible, so the wizard can update its navigation buttons.
/// </summary>
public interface IWizardStepNotifier
{
    /// <summary>
    /// Raised when the value of <see cref="IWizardStep.CanGoNext"/> changed.
    /// </summary>
    event EventHandler? CanGoNextChanged;
}

/// <summary>
/// Implemented by the wizard steps whose content can be refreshed, like the
/// refreshable pages of Java HMCL.
/// </summary>
public interface IWizardRefreshable
{
    /// <summary>
    /// Whether the step can be refreshed.
    /// </summary>
    bool CanRefresh { get; }

    /// <summary>
    /// Refreshes the content of the step.
    /// </summary>
    /// <returns>A task that completes when the step is refreshed.</returns>
    Task RefreshAsync();
}

/// <summary>
/// Implemented by the wizard steps that run a long operation when the user
/// confirms the wizard, like the task executor of Java HMCL.
/// </summary>
public interface IWizardTaskStep
{
    /// <summary>
    /// The title of the dialog showing the progress of the operation.
    /// </summary>
    string TaskTitle { get; }

    /// <summary>
    /// The message shown in the dialog while the operation runs.
    /// </summary>
    string TaskMessage { get; }

    /// <summary>
    /// Runs the operation of the step.
    /// </summary>
    /// <param name="progress">Receives the progress of the operation.</param>
    /// <returns>A task that completes when the operation is done.</returns>
    Task RunTaskAsync(IProgress<double> progress);
}
