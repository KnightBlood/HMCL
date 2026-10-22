using System;
using System.Globalization;
using System.Reflection;
using System.Runtime.InteropServices;

namespace HMCLCore.NET.Util;

/// <summary>
/// The report of a crash of the launcher, like the crash report of Java HMCL.
/// </summary>
public sealed class CrashReport
{
    /// <summary>
    /// Creates the report of a crash.
    /// </summary>
    /// <param name="exception">The exception that crashed the launcher.</param>
    /// <param name="thread">The thread the exception was thrown in.</param>
    public CrashReport(Exception exception, string? thread = null)
    {
        Exception = exception ?? throw new ArgumentNullException(nameof(exception));
        Thread = thread ?? "unknown";
        StackTrace = exception.ToString();
    }

    /// <summary>
    /// The exception that crashed the launcher.
    /// </summary>
    public Exception Exception { get; }

    /// <summary>
    /// The thread the exception was thrown in.
    /// </summary>
    public string Thread { get; }

    /// <summary>
    /// The stack trace of the exception.
    /// </summary>
    public string StackTrace { get; }

    /// <summary>
    /// Whether the crash is worth reporting to the developer of the launcher.
    /// </summary>
    /// <remarks>
    /// The crashes caused by the virtual machine itself, like an out of memory
    /// error, are not reported, and neither are the crashes of the code of the
    /// other libraries.
    /// </remarks>
    public bool ShouldBeReported
    {
        get
        {
            if (Exception is OutOfMemoryException or StackOverflowException or AccessViolationException)
                return false;

            return StackTrace.Contains("HMCL", StringComparison.OrdinalIgnoreCase);
        }
    }

    /// <summary>
    /// Returns the text of the report shown to the user and sent to the
    /// developer of the launcher.
    /// </summary>
    /// <returns>The text of the report.</returns>
    public string GetDisplayText()
    {
        var version = Assembly.GetExecutingAssembly().GetName().Version?.ToString() ?? "unknown";

        var builder = new System.Text.StringBuilder();
        builder.AppendLine("---- Hello Minecraft! Crash Report ----");
        builder.AppendLine($"  Version: {version}");
        builder.AppendLine($"  Time: {DateTime.Now:yyyy-MM-dd HH:mm:ss}");
        builder.AppendLine($"  Thread: {Thread}");
        builder.AppendLine();
        builder.AppendLine("  Content: ");
        builder.AppendLine($"    {StackTrace}");
        builder.AppendLine();
        builder.AppendLine("-- System Details --");
        builder.AppendLine($"  Operating System: {RuntimeInformation.OSDescription}");
        builder.AppendLine($"  System Architecture: {RuntimeInformation.OSArchitecture}");
        builder.AppendLine($"  Process Architecture: {RuntimeInformation.ProcessArchitecture}");
        builder.AppendLine($"  Runtime Version: {RuntimeInformation.FrameworkDescription}");
        builder.AppendLine($"  Launcher Version: {version}");
        builder.AppendLine($"  Working Memory: {Environment.WorkingSet / 1024 / 1024} MiB");
        builder.AppendLine($"  Total Memory: {GC.GetTotalMemory(false) / 1024 / 1024} MiB");

        return builder.ToString();
    }

    /// <summary>
    /// Returns the name of the error of the crash.
    /// </summary>
    /// <returns>The name of the error.</returns>
    public string GetErrorName() => Exception.GetType().FullName ?? Exception.GetType().Name;

    /// <summary>
    /// Returns the description of the crash shown in the title of the window.
    /// </summary>
    /// <returns>The description of the crash.</returns>
    public string GetDisplayTitle() => string.Format(
        CultureInfo.CurrentCulture,
        "{0}: {1}",
        GetErrorName(),
        Exception.Message);
}
