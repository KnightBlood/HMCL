using System;
using System.IO;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Media.Imaging;
using Avalonia.Threading;
using HMCLCore.NET.Auth;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The window showing the skin of a player, like the preview of the skins of
/// Java HMCL.
/// </summary>
/// <remarks>
/// The head is drawn with the three dimensions and the whole model is drawn
/// flat; the images are produced by the skin renderer of the core, which needs
/// no native rendering context.
/// </remarks>
public sealed class SkinPreviewWindow : Window
{
    private readonly Image _headImage = new()
    {
        Width = 180,
        Height = 180,
        Stretch = Stretch.Uniform
    };

    private readonly Image _modelImage = new()
    {
        Width = 128,
        Height = 256,
        Stretch = Stretch.Uniform
    };

    private readonly TextBlock _statusText = new()
    {
        Foreground = Brushes.Gray,
        FontSize = 12,
        TextWrapping = TextWrapping.Wrap
    };

    /// <summary>
    /// Creates the window showing the skin of the given player.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <param name="skinPath">The path of the local skin of the player, if any.</param>
    public SkinPreviewWindow(string username, string? skinPath)
    {
        Title = Lang.Get("account.skin.preview", "皮肤预览");
        Width = 420;
        Height = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };
        root.Children.Add(new TextBlock
        {
            Text = username,
            FontSize = 16,
            FontWeight = FontWeight.Bold,
            HorizontalAlignment = HorizontalAlignment.Center
        });

        var images = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 20,
            HorizontalAlignment = HorizontalAlignment.Center
        };
        images.Children.Add(_headImage);
        images.Children.Add(_modelImage);
        root.Children.Add(images);
        root.Children.Add(_statusText);

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(16, 6),
            HorizontalAlignment = HorizontalAlignment.Right
        };
        closeButton.Click += (_, _) => Close();
        root.Children.Add(closeButton);

        Content = new ScrollViewer { Content = root };

        this.EnableEscapeToClose();

        _ = LoadAsync(username, skinPath);
    }

    /// <summary>
    /// Loads the skin of the player and draws it.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <param name="skinPath">The path of the local skin of the player, if any.</param>
    private async Task LoadAsync(string username, string? skinPath)
    {
        try
        {
            byte[]? skin = null;

            // The local skin of the player is used when it is set.
            if (!string.IsNullOrWhiteSpace(skinPath) && File.Exists(skinPath))
            {
                try
                {
                    skin = await File.ReadAllBytesAsync(skinPath);
                }
                catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
                {
                    skin = null;
                }
            }

            if (skin is null)
            {
                _statusText.Text = Lang.Get("account.skin.preview.loading", "正在获取皮肤...");
                skin = await DownloadSkinAsync(username);
            }

            if (skin is null)
            {
                _statusText.Text = Lang.Get("account.skin.preview.failed", "无法获取皮肤");
                return;
            }

            var head = SkinRenderer.RenderHead(skin);
            var model = SkinRenderer.RenderSkin(skin);

            await Dispatcher.UIThread.InvokeAsync(() =>
            {
                if (head is not null)
                    _headImage.Source = LoadBitmap(head);

                if (model is not null)
                    _modelImage.Source = LoadBitmap(model);

                _statusText.Text = head is null && model is null
                    ? Lang.Get("account.skin.preview.failed", "无法获取皮肤")
                    : string.Empty;
            });
        }
        catch (Exception ex)
        {
            _statusText.Text = Lang.Get("account.skin.preview.failed", "无法获取皮肤");
            LauncherLog.Warn($"无法显示皮肤预览: {ex.Message}");
        }
    }

    /// <summary>
    /// Downloads the skin of a player.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <returns>The bytes of the skin, or null when it cannot be downloaded.</returns>
    private static async Task<byte[]?> DownloadSkinAsync(string username)
    {
        try
        {
            using var client = new System.Net.Http.HttpClient { Timeout = TimeSpan.FromSeconds(20) };
            return await client.GetByteArrayAsync(
                string.Format(SkinRenderer.SkinServiceUrl, Uri.EscapeDataString(username)));
        }
        catch (Exception ex) when (ex is System.Net.Http.HttpRequestException or TaskCanceledException)
        {
            return null;
        }
    }

    /// <summary>
    /// Loads the bytes of a PNG file into a bitmap.
    /// </summary>
    /// <param name="data">The bytes of the file.</param>
    /// <returns>The bitmap of the file.</returns>
    private static Bitmap LoadBitmap(byte[] data)
    {
        using var stream = new MemoryStream(data);
        return new Bitmap(stream);
    }
}
