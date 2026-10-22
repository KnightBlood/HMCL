using System;
using System.Collections.Generic;
using Avalonia.Controls;
using Avalonia.Input;
using Avalonia.Platform.Storage;

namespace HMCL.NET.Util;

/// <summary>
/// Helpers for handling files dropped from the operating system onto the
/// launcher interface.
/// </summary>
public static class FileDropHelper
{
    /// <summary>
    /// Enables dropping files on the given control and invokes the callback
    /// with the local paths of the dropped files.
    /// </summary>
    /// <param name="control">The control accepting the dropped files.</param>
    /// <param name="onFiles">The callback invoked with the dropped file paths.</param>
    public static void EnableFileDrop(Control control, Action<IReadOnlyList<string>> onFiles)
    {
        if (control == null)
            throw new ArgumentNullException(nameof(control));
        if (onFiles == null)
            throw new ArgumentNullException(nameof(onFiles));

        DragDrop.SetAllowDrop(control, true);

        DragDrop.AddDragOverHandler(control, (_, e) =>
        {
            e.DragEffects = e.DataTransfer.TryGetFiles() is { Length: > 0 }
                ? DragDropEffects.Copy
                : DragDropEffects.None;
            e.Handled = true;
        });

        DragDrop.AddDropHandler(control, (_, e) =>
        {
            var files = GetDroppedFiles(e.DataTransfer);
            if (files.Count == 0)
                return;

            e.Handled = true;
            onFiles(files);
        });
    }

    /// <summary>
    /// Returns the local file paths carried by a data transfer.
    /// </summary>
    /// <param name="dataTransfer">The data transfer of the drop event.</param>
    /// <returns>The local paths of the dropped files.</returns>
    public static IReadOnlyList<string> GetDroppedFiles(IDataTransfer? dataTransfer)
    {
        if (dataTransfer == null)
            return Array.Empty<string>();

        var items = dataTransfer.TryGetFiles();
        if (items == null || items.Length == 0)
            return Array.Empty<string>();

        var result = new List<string>(items.Length);
        foreach (var item in items)
        {
            if (item is null)
                continue;

            var path = item.TryGetLocalPath();
            if (!string.IsNullOrWhiteSpace(path))
                result.Add(path);
        }

        return result;
    }
}
