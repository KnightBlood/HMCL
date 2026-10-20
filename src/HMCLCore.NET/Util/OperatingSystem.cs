using System;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Util;

/// <summary>
/// Operating system detection.
/// </summary>
public static class OperatingSystem
{
    public static readonly OsType CurrentOs = GetCurrentOs();

    public static bool IsWindows => CurrentOs == OsType.Windows;
    public static bool IsLinux => CurrentOs == OsType.Linux;
    public static bool IsMacOs => CurrentOs == OsType.MacOs;

    public static string SystemName => RuntimeInformation.OSDescription;
    public static string SystemVersion => Environment.OSVersion.Version.ToString();

    private static OsType GetCurrentOs()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows)) return OsType.Windows;
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX)) return OsType.MacOs;
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Linux)) return OsType.Linux;
        return OsType.Unknown;
    }

    public enum OsType
    {
        Windows,
        Linux,
        MacOs,
        Unknown
    }
}