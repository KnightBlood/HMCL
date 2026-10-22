using System;
using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;

namespace HMCLCore.NET.Auth;

/// <summary>
/// The way a protected payload is stored inside its JSON envelope.
/// </summary>
public enum ProtectionMode
{
    /// <summary>
    /// The payload is stored as plain JSON, for development and diagnostics.
    /// </summary>
    Plain,

    /// <summary>
    /// The payload is encrypted with a built-in application key and split into
    /// padded lanes. It avoids storing private data as directly readable JSON,
    /// but it is not a device-bound secret storage.
    /// </summary>
    ObfuscatedV1
}

/// <summary>
/// Stores a JSON payload using the portable protection envelope of HMCL.
/// </summary>
public static class ProtectedPayload
{
    /// <summary>
    /// The JSON member containing the protection marker.
    /// </summary>
    public const string PropertyProtection = "protection";

    /// <summary>
    /// The JSON member containing the envelope payload.
    /// </summary>
    public const string PropertyPayload = "payload";

    /// <summary>
    /// The JSON member containing the Base64-encoded encryption nonce.
    /// </summary>
    public const string PropertyNonce = "nonce";

    /// <summary>
    /// The identifier of the plain protection mode.
    /// </summary>
    public const string PlainProtectionId = "plain";

    /// <summary>
    /// The identifier of the obfuscated protection mode.
    /// </summary>
    public const string ObfuscatedProtectionId = "hmcl-obfuscated-v1";

    /// <summary>
    /// The number of lanes used by the obfuscated payload.
    /// </summary>
    private const int LaneCount = 4;

    /// <summary>
    /// The number of padding elements stored before each lane.
    /// </summary>
    private const int LanePaddingCount = 63;

    /// <summary>
    /// The number of elements written in an obfuscated payload array.
    /// </summary>
    private const int WrittenPayloadSize = 256;

    /// <summary>
    /// The size of the encryption nonce in bytes.
    /// </summary>
    private const int NonceSize = 12;

    /// <summary>
    /// The size of the authentication tag in bytes.
    /// </summary>
    private const int TagSize = 16;

    /// <summary>
    /// The built-in application key used by the obfuscated protection mode.
    /// </summary>
    private static readonly byte[] ProtectionKey =
    {
        0x3c, 0xd8, 0xa2, 0x22, 0x11, 0xd2, 0x8d, 0x89,
        0xb4, 0xf7, 0xd9, 0xb0, 0x65, 0xbc, 0x14, 0x8a,
        0x6e, 0xb0, 0xa9, 0x4d, 0xeb, 0x93, 0x99, 0x6f,
        0x84, 0x07, 0x5a, 0x9e, 0xbd, 0xc8, 0xd1, 0xeb
    };

    /// <summary>
    /// Protects the given payload and returns the envelope holding it.
    /// </summary>
    /// <param name="payload">The payload to protect.</param>
    /// <param name="mode">The protection mode used to store the payload.</param>
    /// <returns>The envelope of the protected payload.</returns>
    public static JsonObject Protect(JsonNode payload, ProtectionMode mode)
    {
        var envelope = new JsonObject();

        if (mode == ProtectionMode.Plain)
        {
            envelope[PropertyProtection] = PlainProtectionId;
            envelope[PropertyPayload] = payload.DeepClone();
            return envelope;
        }

        var nonce = RandomNumberGenerator.GetBytes(NonceSize);
        var payloadBytes = Encoding.UTF8.GetBytes(payload.ToJsonString());
        var encrypted = Encrypt(payloadBytes, nonce);
        var encoded = Convert.ToBase64String(encrypted);

        envelope[PropertyProtection] = ObfuscatedProtectionId;
        envelope[PropertyPayload] = SplitPayload(encoded);
        envelope[PropertyNonce] = Convert.ToBase64String(nonce);

        return envelope;
    }

    /// <summary>
    /// Reveals the payload stored in the given envelope.
    /// </summary>
    /// <param name="envelope">The envelope holding the protected payload.</param>
    /// <returns>The revealed payload, or null when the envelope is malformed.</returns>
    public static JsonNode? Reveal(JsonObject envelope)
    {
        var protection = envelope[PropertyProtection]?.GetValue<string>();
        if (string.IsNullOrEmpty(protection))
            return null;

        if (protection == PlainProtectionId)
            return envelope[PropertyPayload]?.DeepClone();

        if (protection != ObfuscatedProtectionId)
            return null;

        var encodedNonce = envelope[PropertyNonce]?.GetValue<string>();
        if (string.IsNullOrEmpty(encodedNonce))
            return null;

        byte[] nonce;
        try
        {
            nonce = Convert.FromBase64String(encodedNonce);
        }
        catch (FormatException)
        {
            return null;
        }

        if (nonce.Length != NonceSize)
            return null;

        var encoded = JoinPayload(envelope[PropertyPayload] as JsonArray);
        if (encoded is null)
            return null;

        byte[] encrypted;
        try
        {
            encrypted = Convert.FromBase64String(encoded);
        }
        catch (FormatException)
        {
            return null;
        }

        byte[] payloadBytes;
        try
        {
            payloadBytes = Decrypt(encrypted, nonce);
        }
        catch (CryptographicException)
        {
            // The payload was tampered with or written with another key.
            return null;
        }

        try
        {
            return JsonNode.Parse(Encoding.UTF8.GetString(payloadBytes));
        }
        catch (JsonException)
        {
            return null;
        }
    }

    /// <summary>
    /// Encrypts the plain payload bytes.
    /// </summary>
    /// <param name="payload">The plain payload bytes.</param>
    /// <param name="nonce">The encryption nonce.</param>
    /// <returns>The encrypted payload bytes with the authentication tag appended.</returns>
    private static byte[] Encrypt(byte[] payload, byte[] nonce)
    {
        var ciphertext = new byte[payload.Length];
        var tag = new byte[TagSize];

        using var cipher = new ChaCha20Poly1305(ProtectionKey);
        cipher.Encrypt(nonce, payload, ciphertext, tag);

        var result = new byte[payload.Length + TagSize];
        ciphertext.CopyTo(result, 0);
        tag.CopyTo(result, payload.Length);
        return result;
    }

    /// <summary>
    /// Decrypts the protected payload bytes.
    /// </summary>
    /// <param name="payload">The encrypted payload bytes with the authentication tag appended.</param>
    /// <param name="nonce">The encryption nonce.</param>
    /// <returns>The plain payload bytes.</returns>
    private static byte[] Decrypt(byte[] payload, byte[] nonce)
    {
        if (payload.Length < TagSize)
            throw new CryptographicException("The protected payload is too short.");

        var ciphertext = payload.AsSpan(0, payload.Length - TagSize);
        var tag = payload.AsSpan(payload.Length - TagSize, TagSize);
        var result = new byte[ciphertext.Length];

        using var cipher = new ChaCha20Poly1305(ProtectionKey);
        cipher.Decrypt(nonce, ciphertext, tag, result);
        return result;
    }

    /// <summary>
    /// Splits a Base64 payload into padded lanes.
    /// </summary>
    /// <param name="payload">The Base64 payload to split.</param>
    /// <returns>The padded payload lanes.</returns>
    private static JsonArray SplitPayload(string payload)
    {
        var result = new JsonArray();
        var laneLength = payload.Length / LaneCount;

        for (var laneIndex = 0; laneIndex < LaneCount; laneIndex++)
        {
            var start = laneIndex * laneLength;
            var lane = payload.Substring(start, laneLength);

            for (var i = 0; i < LanePaddingCount; i++)
                result.Add((System.Text.Json.Nodes.JsonNode?)null);

            result.Add((System.Text.Json.Nodes.JsonNode)System.Text.Json.Nodes.JsonValue.Create(lane));
        }

        // The array is padded to the size expected by the readers, so that the
        // lanes cannot be located by their position alone.
        while (result.Count < WrittenPayloadSize)
            result.Add((System.Text.Json.Nodes.JsonNode?)null);

        return result;
    }

    /// <summary>
    /// Joins the Base64 payload lanes of an envelope.
    /// </summary>
    /// <param name="lanes">The lanes of the envelope.</param>
    /// <returns>The restored Base64 payload, or null when the lanes are malformed.</returns>
    private static string? JoinPayload(JsonArray? lanes)
    {
        if (lanes is null || lanes.Count < LaneCount)
            return null;

        var effectiveSize = 1;
        while (effectiveSize * 2 <= lanes.Count)
            effectiveSize *= 2;

        var segmentSize = effectiveSize / LaneCount;
        var builder = new StringBuilder();

        for (var laneIndex = 0; laneIndex < LaneCount; laneIndex++)
        {
            var payloadIndex = (laneIndex + 1) * segmentSize - 1;
            if (payloadIndex >= lanes.Count)
                return null;

            if (lanes[payloadIndex] is not JsonValue value || !value.TryGetValue<string>(out var lane))
                return null;

            builder.Append(lane);
        }

        return builder.ToString();
    }
}
