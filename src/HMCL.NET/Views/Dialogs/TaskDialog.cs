using HMCL.NET.Util;

using System;
using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Media;

namespace HMCL.NET.Views.Dialogs;

/// <summary>
/// A modal dialog running a long operation in the background, showing the
/// progress of the operation.
/// </summary>
public sealed class TaskDialog : DialogWindow
{
    /// <summary>
    /// The progress bar of the dialog.
    /// </summary>
    private readonly ProgressBar _progress;

    /// <summary>
    /// The label showing the current state of the operation.
    /// </summary>
    private readonly TextBlock _status;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="message">The message shown to the user.</param>
    /// <param name="title">The title of the dialog window.</param>
    public TaskDialog(string message, string? title = null)
        : base(title ?? Lang.Get("hmcl.taskdialog.449"))
    {
        AddMessage(message);

        _progress = new ProgressBar
        {
            IsIndeterminate = true,
            Height = 6,
            MinWidth = 320
        };

        _status = new TextBlock
        {
            Text = string.Empty,
            Foreground = new SolidColorBrush(Color.Parse("#888888")),
            FontSize = 12,
            TextWrapping = TextWrapping.Wrap
        };

        AddContent(_progress);
        AddContent(_status);
        ButtonPanel.Children.Add(CreateButton(Lang.Get("hmcl.taskdialog.900_"), false, () => Close()));
        AttachButtons();
    }

    /// <summary>
    /// Runs the given operation, updating the progress of the dialog, and
    /// closes the dialog when the operation finishes.
    /// </summary>
    /// <param name="operation">The operation to run.</param>
    /// <returns>Whether the operation finished without an error.</returns>
    public async Task<bool> RunAsync(Func<IProgress<double>, Task> operation)
    {
        var progress = new Progress<double>(value =>
        {
            _progress.IsIndeterminate = false;
            _progress.Value = Math.Clamp(value, 0, 100);
        });

        try
        {
            await operation(progress);
            _status.Text = Lang.Get("hmcl.taskdialog.902_");
            return true;
        }
        catch (Exception ex)
        {
            _status.Text = $"操作失败: {ex.Message}";
            return false;
        }
        finally
        {
            _progress.IsIndeterminate = false;
            Close();
        }
    }
}
