using System;
using System.Collections.Generic;
using System.IO;
using System.Text;

namespace HMCLCore.NET.Util;

/// <summary>
/// File system utility methods.
/// </summary>
public static class FileUtils
{
    /// <summary>
    /// Copies a file from source to destination, creating directories as needed.
    /// </summary>
    public static void CopyFile(string sourcePath, string destinationPath)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(destinationPath)!);
        File.Copy(sourcePath, destinationPath, overwrite: true);
    }

    /// <summary>
    /// Returns the file extension including the leading dot, or empty string if none.
    /// </summary>
    public static string GetExtension(string path)
    {
        return Path.GetExtension(path);
    }

    /// <summary>
    /// Ensures the directory for the given file path exists.
    /// </summary>
    public static void EnsureDirectory(string filePath)
    {
        var dir = Path.GetDirectoryName(filePath);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);
    }

    /// <summary>
    /// Returns whether a path is valid for the current platform.
    /// </summary>
    public static bool IsValidPath(string path)
    {
        try
        {
            Path.GetFullPath(path);
            return true;
        }
        catch
        {
            return false;
        }
    }
}

/// <summary>
/// Utility methods for I/O streams.
/// </summary>
public static class IOUtils
{
    /// <summary>
    /// Reads all bytes from a stream.
    /// </summary>
    public static byte[] ReadAllBytes(Stream stream)
    {
        using var ms = new MemoryStream();
        stream.CopyTo(ms);
        return ms.ToArray();
    }

    /// <summary>
    /// Reads all text from a stream using UTF-8 encoding.
    /// </summary>
    public static string ReadAllText(Stream stream)
    {
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }

    /// <summary>
    /// Copies a stream to a destination file.
    /// </summary>
    public static void CopyToFile(Stream source, string destinationPath)
    {
        FileUtils.EnsureDirectory(destinationPath);
        using var dest = File.Create(destinationPath);
        source.CopyTo(dest);
    }
}
