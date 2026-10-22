using System;
using System.IO;
using System.Net.Http;
using System.Threading;
using System.Threading.Tasks;
using SkiaSharp;

namespace HMCLCore.NET.Auth;

/// <summary>
/// Draws the skins of the players, like the preview of the skins of Java HMCL.
/// </summary>
/// <remarks>
/// The skins are drawn with SkiaSharp, so no native rendering context is
/// needed and the results can be shown by any user interface of the launcher.
/// </remarks>
public static class SkinRenderer
{
    /// <summary>
    /// The address of the service returning the skin of a player.
    /// </summary>
    public const string SkinServiceUrl = "https://crafatar.com/skins/{0}";

    /// <summary>
    /// Draws the head of a player with the three dimensions.
    /// </summary>
    /// <param name="skin">The bytes of the skin of the player.</param>
    /// <returns>The bytes of the drawn image, or null when the skin is invalid.</returns>
    public static byte[]? RenderHead(byte[]? skin)
    {
        if (skin is null || skin.Length == 0)
            return null;

        try
        {
            using var bitmap = SKBitmap.Decode(skin);
            if (bitmap is null)
                return null;

            using var image = MinecraftSkinRender.Image.Skin3DHeadTypeA.MakeHeadImage(bitmap);
            if (image is null)
                return null;

            using var data = image.Encode(SKEncodedImageFormat.Png, 100);
            return data.ToArray();
        }
        catch (Exception)
        {
            // An invalid skin simply has no preview.
            return null;
        }
    }

    /// <summary>
    /// Draws the whole model of a player.
    /// </summary>
    /// <param name="skin">The bytes of the skin of the player.</param>
    /// <returns>The bytes of the drawn image, or null when the skin is invalid.</returns>
    public static byte[]? RenderSkin(byte[]? skin)
    {
        if (skin is null || skin.Length == 0)
            return null;

        try
        {
            using var bitmap = SKBitmap.Decode(skin);
            if (bitmap is null)
                return null;

            using var drawn = MinecraftSkinRender.Image.Skin2DTypeA.MakeSkinImage(
                bitmap, (MinecraftSkinRender.SkinType?)null);
            if (drawn is null)
                return null;

            using var data = drawn.Encode(SKEncodedImageFormat.Png, 100);
            return data.ToArray();
        }
        catch (Exception)
        {
            // An invalid skin simply has no preview.
            return null;
        }
    }

    /// <summary>
    /// Downloads the skin of a player and draws its head.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The bytes of the drawn image, or null when it cannot be drawn.</returns>
    public static async Task<byte[]?> RenderHeadAsync(string username, CancellationToken cancellationToken = default)
    {
        var skin = await DownloadSkinAsync(username, cancellationToken);
        return RenderHead(skin);
    }

    /// <summary>
    /// Downloads the skin of a player and draws its whole model.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The bytes of the drawn image, or null when it cannot be drawn.</returns>
    public static async Task<byte[]?> RenderSkinAsync(string username, CancellationToken cancellationToken = default)
    {
        var skin = await DownloadSkinAsync(username, cancellationToken);
        return RenderSkin(skin);
    }

    /// <summary>
    /// Downloads the skin of a player.
    /// </summary>
    /// <param name="username">The name of the player.</param>
    /// <param name="cancellationToken">Cancellation token.</param>
    /// <returns>The bytes of the skin, or null when it cannot be downloaded.</returns>
    private static async Task<byte[]?> DownloadSkinAsync(string username, CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(username))
            return null;

        try
        {
            using var client = new HttpClient { Timeout = TimeSpan.FromSeconds(20) };
            return await client.GetByteArrayAsync(
                string.Format(SkinServiceUrl, Uri.EscapeDataString(username)), cancellationToken);
        }
        catch (Exception ex) when (ex is HttpRequestException or TaskCanceledException or OperationCanceledException)
        {
            return null;
        }
    }
}
