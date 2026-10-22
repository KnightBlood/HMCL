using System;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Util;

/// <summary>
/// Operating system detection utilities.
/// </summary>
public static class OsDetection
{
    /// <summary>
    /// Returns the operating system of the running process.
    /// </summary>
    public static OsType CurrentOs => Platform.CurrentOs;

    /// <summary>
    /// Returns whether this is a Windows system.
    /// </summary>
    public static bool IsWindows => Platform.CurrentOs == OsType.Windows;

    /// <summary>
    /// Returns whether this is a Linux system.
    /// </summary>
    public static bool IsLinux => Platform.CurrentOs == OsType.Linux;

    /// <summary>
    /// Returns whether this is a macOS system.
    /// </summary>
    public static bool IsMacOs => Platform.CurrentOs == OsType.MacOs;

    /// <summary>
    /// Returns whether this is a FreeBSD system.
    /// </summary>
    public static bool IsFreeBsd => Platform.CurrentOs == OsType.FreeBsd;

    /// <summary>
    /// Returns the OS description.
    /// </summary>
    public static string SystemName => RuntimeInformation.OSDescription;

    /// <summary>
    /// Returns the OS version.
    /// </summary>
    public static string SystemVersion => Environment.OSVersion.Version.ToString();
}