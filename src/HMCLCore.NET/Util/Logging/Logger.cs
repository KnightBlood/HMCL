using System;

namespace HMCLCore.NET.Util.Logging;

/// <summary>
/// Simple logger.
/// </summary>
public static class Logger
{
    public static void Info(string message) => Console.WriteLine($"[INFO] {DateTime.Now:HH:mm:ss} {message}");
    public static void Warn(string message) => Console.WriteLine($"[WARN] {DateTime.Now:HH:mm:ss} {message}");
    public static void Error(string message, Exception? ex = null)
    {
        Console.WriteLine($"[ERROR] {DateTime.Now:HH:mm:ss} {message}");
        if (ex != null)
            Console.WriteLine(ex);
    }
}