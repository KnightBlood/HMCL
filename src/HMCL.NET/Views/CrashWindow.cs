using System;
using System.Diagnostics;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Util;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The window shown when the launcher crashes, like the crash window of Java
/// HMCL.
/// </summary>
public sealed class CrashWindow : Window
{
    private readonly CrashReport _report;

    private readonly TextBlock _statusText = new()
    {
        Foreground = Brushes.Gray,
        FontSize = 12,
        TextWrapping = TextWrapping.Wrap
    };

    /// <summary>
    /// Creates the window.
    /// </summary>
    /// <param name="report">The report of the crash.</param>
    /// <param name="onSend">
    /// The action sending the report to the developer of the launcher; it is
    /// not called when the user does not want to send it.
    /// </param>
    public CrashWindow(CrashReport report, Action<CrashReport>? onSend = null)
    {
        _report = report ?? throw new ArgumentNullException(nameof(report));

        Title = Lang.Get("launcher.crash", "启动器崩溃");
        Width = 720;
        Height = 520;
        WindowStartupLocation = WindowStartupLocation.CenterScreen;

        var reportBox = new TextBox
        {
            Text = report.GetDisplayText(),
            IsReadOnly = true,
            AcceptsReturn = true,
            TextWrapping = TextWrapping.Wrap,
            FontFamily = new FontFamily("Consolas"),
            FontSize = 12,
            Height = 320
        };

        var copyButton = new Button
        {
            Content = Lang.Get("button.copy", "复制"),
            Padding = new Thickness(16, 6)
        };
        copyButton.Click += (_, _) => CopyReport();

        var sendButton = new Button
        {
            Content = Lang.Get("launcher.crash.send", "发送报告"),
            Padding = new Thickness(16, 6)
        };
        sendButton.Click += (_, _) =>
        {
            onSend?.Invoke(_report);
            _statusText.Text = Lang.Get("launcher.crash.sent", "崩溃报告已发送");
        };

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(16, 6)
        };
        closeButton.Click += (_, _) => Close();

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 10,
            Children = { copyButton, sendButton, closeButton }
        };

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };
        root.Children.Add(new TextBlock
        {
            Text = Lang.Get("launcher.crash.desc", "启动器遇到了一个错误，您可以复制下面的报告以便反馈问题。"),
            TextWrapping = TextWrapping.Wrap
        });
        root.Children.Add(reportBox);
        root.Children.Add(_statusText);

        var content = new Grid { RowDefinitions = new RowDefinitions("*,Auto") };
        content.Children.Add(root);

        var buttonRow = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 10,
            Margin = new Thickness(16, 0, 16, 16),
            Children = { buttons }
        };
        Grid.SetRow(buttonRow, 1);
        content.Children.Add(buttonRow);

        Content = content;

        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Copies the report of the crash into the clipboard.
    /// </summary>
    private async void CopyReport()
    {
        try
        {
            var copied = await ClipboardUtils.SetTextAsync(
                TopLevel.GetTopLevel(this), _report.GetDisplayText());

            _statusText.Text = copied
                ? Lang.Get("launcher.crash.copied", "崩溃报告已复制")
                : Lang.Get("launcher.crash.copy_failed", "无法复制崩溃报告");
        }
        catch (Exception)
        {
            _statusText.Text = Lang.Get("launcher.crash.copy_failed", "无法复制崩溃报告");
        }
    }
}
