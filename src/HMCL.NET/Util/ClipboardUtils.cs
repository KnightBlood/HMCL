using System.Threading.Tasks;
using Avalonia.Controls;
using Avalonia.Input;

namespace HMCL.NET.Util;

/// <summary>
/// Helpers for the clipboard of the system.
/// </summary>
public static class ClipboardUtils
{
    /// <summary>
    /// Copies the given text into the clipboard of the given window.
    /// </summary>
    /// <param name="topLevel">The window owning the clipboard; the clipboard
    /// is not available when it is <c>null</c>.</param>
    /// <param name="text">The text to copy.</param>
    /// <returns>Whether the text was copied.</returns>
    public static async Task<bool> SetTextAsync(TopLevel? topLevel, string text)
    {
        var clipboard = topLevel?.Clipboard;
        if (clipboard is null)
            return false;

        var data = new DataTransfer();
        data.Add(DataTransferItem.CreateText(text));

        await clipboard.SetDataAsync(data);

        return true;
    }
}
