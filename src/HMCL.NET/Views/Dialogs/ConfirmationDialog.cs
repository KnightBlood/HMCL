using HMCL.NET.Util;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// A modal dialog asking the user to confirm an action; the dialog result is
/// <c>true</c> when the action was confirmed and <c>false</c> when it was
/// cancelled.
/// </summary>
public sealed class ConfirmationDialog : DialogWindow
{
    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    public ConfirmationDialog(string message, string? title = null)
        : base(title ?? Lang.Get("hmcl.confirmationdialog.438"))
    {
        AddMessage(message);
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.confirmationdialog.button_ok"), true, () => Close(true)));
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.confirmationdialog.button_cancel"), false, () => Close(false)));
        AttachButtons();
    }
}
