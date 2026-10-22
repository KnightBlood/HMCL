using HMCL.NET.Util;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// A modal dialog showing a message and a single button closing it.
/// </summary>
public sealed class MessageDialog : DialogWindow
{
    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    public MessageDialog(string message, string? title = null)
        : base(title ?? Lang.Get("hmcl.messagedialog.447"))
    {
        AddMessage(message);
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.messagedialog.button_ok"), true, () => Close()));
        AttachButtons();
    }
}
