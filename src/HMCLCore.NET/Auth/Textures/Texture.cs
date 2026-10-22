using System;

namespace HMCLCore.NET.Auth.Textures;

/// <summary>
/// The model of a skin, telling whether the arms of the player are slim or
/// wide.
/// </summary>
public enum TextureModel
{
    /// <summary>
    /// The slim model, used by the players named Alex, Ari, Efe, Kai, Makena,
    /// Noor and Zuri.
    /// </summary>
    Slim,

    /// <summary>
    /// The wide model, used by the players named Steve and Sunny.
    /// </summary>
    Wide
}

/// <summary>
/// The kind of a texture of a player.
/// </summary>
public enum TextureType
{
    /// <summary>
    /// The skin of the player.
    /// </summary>
    Skin,

    /// <summary>
    /// The cape of the player.
    /// </summary>
    Cape
}

/// <summary>
/// A texture of a player, downloaded from the session server of the account.
/// </summary>
public sealed class Texture
{
    /// <summary>
    /// The address of the texture.
    /// </summary>
    public string Url { get; init; } = string.Empty;

    /// <summary>
    /// The metadata of the texture, for example the model of the skin.
    /// </summary>
    public IReadOnlyDictionary<string, string> Metadata { get; init; } =
        new Dictionary<string, string>(StringComparer.Ordinal);

    /// <summary>
    /// The model of the skin, when it is known.
    /// </summary>
    public TextureModel? Model
    {
        get
        {
            if (Metadata.TryGetValue("model", out var model))
            {
                if (string.Equals(model, "slim", StringComparison.OrdinalIgnoreCase))
                    return TextureModel.Slim;
                if (string.Equals(model, "wide", StringComparison.OrdinalIgnoreCase))
                    return TextureModel.Wide;
            }

            return null;
        }
    }

    /// <summary>
    /// Returns the model of the skin of a player, guessed from the identifier
    /// of the player when it is not known.
    /// </summary>
    /// <param name="uuid">The identifier of the player.</param>
    /// <returns>The model of the skin.</returns>
    public static TextureModel GuessModel(string uuid)
    {
        // The default skins of the game alternate between the slim and the
        // wide model, following the hash of the identifier of the player.
        var hash = 0;
        foreach (var c in uuid)
            hash = unchecked(hash * 31 + c);

        return Math.Abs(hash) % 2 == 0 ? TextureModel.Slim : TextureModel.Wide;
    }
}
