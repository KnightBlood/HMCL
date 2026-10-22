using System;
using System.IO;
using System.Security.Cryptography;

namespace HMCLCore.NET.Util;

/// <summary>
/// Computes the digests of the files, of the streams and of the byte arrays,
/// in the same hexadecimal format as the digests published by Mojang.
/// </summary>
public static class DigestUtils
{
    /// <summary>
    /// Returns whether the given digest is a hexadecimal SHA-1 digest.
    /// </summary>
    /// <param name="digest">The digest to check.</param>
    /// <returns>Whether the digest is a valid SHA-1 digest.</returns>
    public static bool IsSha1Digest(string? digest) => IsHexDigest(digest, 40);

    /// <summary>
    /// Returns whether the given digest is a hexadecimal SHA-512 digest.
    /// </summary>
    /// <param name="digest">The digest to check.</param>
    /// <returns>Whether the digest is a valid SHA-512 digest.</returns>
    public static bool IsSha512Digest(string? digest) => IsHexDigest(digest, 128);

    /// <summary>
    /// Computes the digest of the given data with the given algorithm.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="data">The data to hash.</param>
    /// <returns>The digest of the data.</returns>
    public static byte[] Digest(string algorithm, byte[] data) =>
        CreateAlgorithm(algorithm).ComputeHash(data);

    /// <summary>
    /// Computes the digest of the given file with the given algorithm.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="path">The path of the file to hash.</param>
    /// <returns>The digest of the file.</returns>
    public static byte[] Digest(string algorithm, string path)
    {
        using var stream = File.OpenRead(path);
        return CreateAlgorithm(algorithm).ComputeHash(stream);
    }

    /// <summary>
    /// Computes the digest of the given stream with the given algorithm.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="stream">The stream to hash.</param>
    /// <returns>The digest of the stream.</returns>
    public static byte[] Digest(string algorithm, Stream stream) =>
        CreateAlgorithm(algorithm).ComputeHash(stream);

    /// <summary>
    /// Computes the digest of the given data and returns it in the lower case
    /// hexadecimal notation.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="data">The data to hash.</param>
    /// <returns>The digest of the data.</returns>
    public static string DigestToString(string algorithm, byte[] data) =>
        Convert.ToHexString(Digest(algorithm, data)).ToLowerInvariant();

    /// <summary>
    /// Computes the digest of the given file and returns it in the lower case
    /// hexadecimal notation.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="path">The path of the file to hash.</param>
    /// <returns>The digest of the file.</returns>
    public static string DigestToString(string algorithm, string path) =>
        Convert.ToHexString(Digest(algorithm, path)).ToLowerInvariant();

    /// <summary>
    /// Computes the digest of the given stream and returns it in the lower case
    /// hexadecimal notation.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <param name="stream">The stream to hash.</param>
    /// <returns>The digest of the stream.</returns>
    public static string DigestToString(string algorithm, Stream stream) =>
        Convert.ToHexString(Digest(algorithm, stream)).ToLowerInvariant();

    /// <summary>
    /// Returns whether the given digest is a hexadecimal digest of the given
    /// length.
    /// </summary>
    /// <param name="digest">The digest to check.</param>
    /// <param name="length">The number of the hexadecimal digits of the digest.</param>
    /// <returns>Whether the digest is a valid one.</returns>
    private static bool IsHexDigest(string? digest, int length)
    {
        if (string.IsNullOrEmpty(digest) || digest.Length != length)
            return false;

        foreach (var character in digest)
        {
            var valid = character is >= '0' and <= '9' or >= 'a' and <= 'f' or >= 'A' and <= 'F';
            if (!valid)
                return false;
        }

        return true;
    }

    /// <summary>
    /// Computes the SHA-1 digest of the given file and returns it in the lower
    /// case hexadecimal notation.
    /// </summary>
    /// <param name="path">The path of the file to hash.</param>
    /// <returns>The digest of the file.</returns>
    public static string ComputeSha1(string path) => DigestToString("SHA-1", path);

    /// <summary>
    /// Computes the SHA-1 digest of the given data and returns it in the lower
    /// case hexadecimal notation.
    /// </summary>
    /// <param name="data">The data to hash.</param>
    /// <returns>The digest of the data.</returns>
    public static string ComputeSha1(byte[] data) => DigestToString("SHA-1", data);

    /// <summary>
    /// Computes the MD5 digest of the given data and returns it in the lower
    /// case hexadecimal notation.
    /// </summary>
    /// <param name="data">The data to hash.</param>
    /// <returns>The digest of the data.</returns>
    public static string ComputeMd5(byte[] data) => DigestToString("MD5", data);

    /// <summary>
    /// Computes the SHA-256 digest of the given data and returns it in the
    /// lower case hexadecimal notation.
    /// </summary>
    /// <param name="data">The data to hash.</param>
    /// <returns>The digest of the data.</returns>
    public static string ComputeSha256(byte[] data) => DigestToString("SHA-256", data);

    /// <summary>
    /// Creates the algorithm computing the requested digest.
    /// </summary>
    /// <param name="algorithm">The name of the algorithm, such as "SHA-1".</param>
    /// <returns>The algorithm.</returns>
    private static HashAlgorithm CreateAlgorithm(string algorithm) =>
        algorithm.ToUpperInvariant() switch
        {
            "SHA-1" or "SHA1" => SHA1.Create(),
            "SHA-256" or "SHA256" => SHA256.Create(),
            "SHA-512" or "SHA512" => SHA512.Create(),
            "MD5" => MD5.Create(),
            _ => throw new ArgumentException($"不支持的摘要算法: {algorithm}", nameof(algorithm))
        };
}
