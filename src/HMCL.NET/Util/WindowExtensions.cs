using System;
using Avalonia.Controls;
using Avalonia.Input;

namespace HMCL.NET.Util;

/// <summary>
/// Helpers adding the standard keyboard shortcuts to the windows of the
/// launcher.
/// </summary>
public static class WindowExtensions
{
    /// <summary>
    /// Closes the window when the user presses the escape key.
    /// </summary>
    /// <param name="window">The window receiving the shortcut.</param>
    /// <param name="result">The result of the window when it is closed.</param>
    public static void EnableEscapeToClose(this Window window, object? result = null)
    {
        ArgumentNullException.ThrowIfNull(window);

        window.KeyDown += (_, e) =>
        {
            if (e.Key != Key.Escape)
                return;

            e.Handled = true;
            window.Close(result);
        };
    }

    /// <summary>
    /// Runs the given action when the user presses the enter key.
    /// </summary>
    /// <param name="window">The window receiving the shortcut.</param>
    /// <param name="action">The action executed when the key is pressed.</param>
    public static void EnableEnterTo(this Window window, Action action)
    {
        ArgumentNullException.ThrowIfNull(window);
        ArgumentNullException.ThrowIfNull(action);

        window.KeyDown += (_, e) =>
        {
            if (e.Key != Key.Enter)
                return;

            e.Handled = true;
            action();
        };
    }
}
