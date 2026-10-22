using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media.Imaging;

namespace HMCL.NET.Controls;

/// <summary>
/// Displays the custom icon of a game instance. The image is loaded from disk
/// and released when the icon path changes, so that long running sessions do
/// not keep stale bitmaps alive.
/// </summary>
public sealed class InstanceIcon : Image
{
    /// <summary>
    /// The styled property holding the path of the image to display.
    /// </summary>
    public static readonly StyledProperty<string?> IconPathProperty =
        AvaloniaProperty.Register<InstanceIcon, string?>(nameof(IconPath));

    private Bitmap? _bitmap;

    /// <summary>
    /// Creates a new InstanceIcon.
    /// </summary>
    public InstanceIcon()
    {
        Width = 32;
        Height = 32;
        Stretch = Avalonia.Media.Stretch.UniformToFill;
    }

    /// <summary>
    /// The path of the image to display, or null to show no image.
    /// </summary>
    public string? IconPath
    {
        get => GetValue(IconPathProperty);
        set => SetValue(IconPathProperty, value);
    }

    /// <summary>
    /// Reloads the image when the icon path changes.
    /// </summary>
    /// <param name="change">The changed property.</param>
    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property == IconPathProperty)
            LoadIcon(IconPath);
    }

    /// <summary>
    /// Releases the bitmap when the control is detached from the visual tree.
    /// </summary>
    /// <param name="e">The event arguments.</param>
    protected override void OnDetachedFromVisualTree(VisualTreeAttachmentEventArgs e)
    {
        base.OnDetachedFromVisualTree(e);
        ReleaseBitmap();
    }

    /// <summary>
    /// Loads the image at the given path.
    /// </summary>
    /// <param name="path">The path of the image, or null to show no image.</param>
    private void LoadIcon(string? path)
    {
        ReleaseBitmap();

        if (string.IsNullOrWhiteSpace(path) || !System.IO.File.Exists(path))
            return;

        try
        {
            _bitmap = new Bitmap(path);
            Source = _bitmap;
        }
        catch (Exception)
        {
            // An unreadable image simply leaves the icon empty.
            _bitmap = null;
            Source = null;
        }
    }

    /// <summary>
    /// Releases the currently loaded bitmap.
    /// </summary>
    private void ReleaseBitmap()
    {
        if (_bitmap == null)
            return;

        Source = null;
        _bitmap.Dispose();
        _bitmap = null;
    }
}
