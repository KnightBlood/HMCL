using System;
using System.Collections.Generic;
using System.IO;
using System.Net.Http;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Threading;

using HMCLCore.NET.Download;
namespace HMCL.NET.Controls;

/// <summary>
/// A circular account avatar that loads the player's skin face from Crafatar
/// and falls back to the first letter of the username while loading or when
/// the skin cannot be fetched.
/// </summary>
public sealed class AccountAvatar : Border
{
    private static readonly Dictionary<string, Bitmap> Cache = new();

    /// <summary>
    /// The Minecraft account UUID used to resolve the skin face.
    /// </summary>
    public static readonly StyledProperty<string?> UuidProperty =
        AvaloniaProperty.Register<AccountAvatar, string?>(nameof(Uuid));

    /// <summary>
    /// The account username, used for the fallback letter and username lookups.
    /// </summary>
    public static readonly StyledProperty<string?> UsernameProperty =
        AvaloniaProperty.Register<AccountAvatar, string?>(nameof(Username));

    /// <summary>
    /// The path of the local skin of the account; the skin is loaded from this
    /// file instead of the skin server when it is set.
    /// </summary>
    public static readonly StyledProperty<string?> SkinPathProperty =
        AvaloniaProperty.Register<AccountAvatar, string?>(nameof(SkinPath));

    private readonly Image _image = new() { Stretch = Stretch.UniformToFill, IsVisible = false };
    private readonly TextBlock _initial = new()
    {
        Foreground = Brushes.White,
        FontWeight = FontWeight.Bold,
        HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Center,
        VerticalAlignment = Avalonia.Layout.VerticalAlignment.Center
    };

    /// <summary>
    /// Creates a new avatar control.
    /// </summary>
    public AccountAvatar()
    {
        Width = 32;
        Height = 32;
        CornerRadius = new CornerRadius(16);
        Background = new SolidColorBrush(Color.Parse("#2B579A"));
        ClipToBounds = true;
        Child = new Panel { Children = { _image, _initial } };
    }

    /// <summary>
    /// Gets or sets the account UUID.
    /// </summary>
    public string? Uuid
    {
        get => GetValue(UuidProperty);
        set => SetValue(UuidProperty, value);
    }

    /// <summary>
    /// Gets or sets the account username.
    /// </summary>
    public string? Username
    {
        get => GetValue(UsernameProperty);
        set => SetValue(UsernameProperty, value);
    }

    /// <summary>
    /// Gets or sets the path of the local skin of the account.
    /// </summary>
    public string? SkinPath
    {
        get => GetValue(SkinPathProperty);
        set => SetValue(SkinPathProperty, value);
    }

    /// <inheritdoc />
    protected override void OnPropertyChanged(AvaloniaPropertyChangedEventArgs change)
    {
        base.OnPropertyChanged(change);

        if (change.Property == UuidProperty || change.Property == UsernameProperty ||
            change.Property == SkinPathProperty)
            UpdateAvatar();
    }

    /// <summary>
    /// Resets to the fallback letter and starts an async skin load when possible.
    /// </summary>
    private void UpdateAvatar()
    {
        var username = Username ?? string.Empty;
        _initial.Text = username.Length > 0 ? username[..1].ToUpperInvariant() : "?";
        _initial.IsVisible = true;
        _image.IsVisible = false;

        // A local skin of the account is loaded from its own file.
        var skinPath = SkinPath;
        if (!string.IsNullOrWhiteSpace(skinPath))
        {
            var localKey = "local:" + skinPath;
            if (Cache.TryGetValue(localKey, out var local))
            {
                ShowImage(local);
                return;
            }

            _ = LoadLocalSkinAsync(skinPath, localKey);
            return;
        }

        // Prefer the UUID, but offline accounts may only expose a username.
        var key = string.IsNullOrWhiteSpace(Uuid)
            ? username
            : Uuid!.Replace("-", string.Empty);
        if (string.IsNullOrEmpty(key))
            return;

        if (Cache.TryGetValue(key, out var cached))
        {
            ShowImage(cached);
            return;
        }

        _ = LoadAsync(key);
    }

    /// <summary>
    /// Loads the local skin of the account and caches it in memory.
    /// </summary>
    /// <param name="skinPath">The path of the file holding the skin.</param>
    /// <param name="key">The key of the skin in the cache.</param>
    private async Task LoadLocalSkinAsync(string skinPath, string key)
    {
        try
        {
            var bytes = await File.ReadAllBytesAsync(skinPath);
            using var stream = new MemoryStream(bytes);
            var bitmap = new Bitmap(stream);
            Cache[key] = bitmap;

            Dispatcher.UIThread.Post(() =>
            {
                if (SkinPath == skinPath)
                    ShowImage(bitmap);
            });
        }
        catch
        {
            // The skin cannot be read: the initial-letter fallback is kept.
        }
    }

    /// <summary>
    /// Downloads the skin face and caches it in memory.
    /// </summary>
    private async Task LoadAsync(string key)
    {
        try
        {
            var bytes = await HttpClientProvider.Shared.GetByteArrayAsync(
                $"https://crafatar.com/avatars/{key}?size=64&overlay");
            using var stream = new MemoryStream(bytes);
            var bitmap = new Bitmap(stream);
            Cache[key] = bitmap;

            Dispatcher.UIThread.Post(() =>
            {
                var current = string.IsNullOrWhiteSpace(Uuid)
                    ? Username ?? string.Empty
                    : Uuid!.Replace("-", string.Empty);
                if (current == key)
                    ShowImage(bitmap);
            });
        }
        catch
        {
            // Network or API failure: keep the initial-letter fallback.
        }
    }

    /// <summary>
    /// Shows the loaded skin face.
    /// </summary>
    private void ShowImage(Bitmap bitmap)
    {
        _image.Source = bitmap;
        _image.IsVisible = true;
        _initial.IsVisible = false;
    }
}
