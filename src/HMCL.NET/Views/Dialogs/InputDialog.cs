using HMCL.NET.Util;

using Avalonia.Controls;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// A modal dialog asking the user for a single line of text; the dialog result
/// is the entered text, or <c>null</c> when the dialog was cancelled.
/// </summary>
public sealed class InputDialog : DialogWindow
{
    /// <summary>
    /// The text box holding the answer of the user.
    /// </summary>
    private readonly TextBox _input;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    /// <param name="initialValue">The text the input starts with.</param>
    /// <param name="watermark">The hint shown while the input is empty.</param>
    public InputDialog(string message, string? title = null, string initialValue = "", string? watermark = null)
        : base(title ?? Lang.Get("hmcl.inputdialog.444"))
    {
        AddMessage(message);

        _input = new TextBox
        {
            Text = initialValue,
            PlaceholderText = watermark,
            MinWidth = 320
        };

        AddContent(_input);
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.inputdialog.button_ok"), true, () => Close(_input.Text ?? string.Empty)));
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.inputdialog.button_cancel"), false, () => Close(null)));
        AttachButtons();
    }
}
