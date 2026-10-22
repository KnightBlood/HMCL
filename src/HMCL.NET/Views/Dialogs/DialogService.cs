using HMCL.NET.Util;
using System;
using System.Threading.Tasks;
using Avalonia.Controls;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// The entry points of the dialogs of the launcher, so that the views and the
/// view models can show the standard dialogs without knowing their classes.
/// </summary>
public static class DialogService
{
    /// <summary>
    /// Shows a message to the user.
    /// </summary>
    /// <param name="owner">The window owning the dialog.</param>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    /// <returns>A task that completes when the dialog is closed.</returns>
    public static Task ShowMessageAsync(Window owner, string message, string? title = null)
        => new MessageDialog(message, title).ShowDialog(owner);

    /// <summary>
    /// Asks the user to confirm an action.
    /// </summary>
    /// <param name="owner">The window owning the dialog.</param>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    /// <returns>Whether the user confirmed the action.</returns>
    public static Task<bool> ConfirmAsync(Window owner, string message, string? title = null)
        => new ConfirmationDialog(message, title).ShowDialog<bool>(owner);

    /// <summary>
    /// Asks the user for a single line of text.
    /// </summary>
    /// <param name="owner">The window owning the dialog.</param>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    /// <param name="initialValue">The text the input starts with.</param>
    /// <param name="watermark">The hint shown while the input is empty.</param>
    /// <returns>The entered text, or <c>null</c> when the dialog was cancelled.</returns>
    public static Task<string?> InputAsync(
        Window owner,
        string message,
        string? title = null,
        string initialValue = "",
        string? watermark = null)
        => new InputDialog(message, title, initialValue, watermark).ShowDialog<string?>(owner);

    /// <summary>
    /// Runs a long operation in a modal dialog showing its progress.
    /// </summary>
    /// <param name="owner">The window owning the dialog.</param>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    /// <param name="operation">The operation to run.</param>
    /// <returns>Whether the operation finished without an error.</returns>
    public static async Task<bool> RunTaskAsync(
        Window owner,
        string message,
        string title,
        Func<IProgress<double>, Task> operation)
    {
        var dialog = new TaskDialog(message, title);
        var shown = dialog.ShowDialog(owner);
        var succeeded = await dialog.RunAsync(operation);
        await shown;
        return succeeded;
    }
}
