using System;

namespace HMCLCore.NET.Launch;

/// <summary>
/// The exception raised when the game process cannot be started, for example
/// when the launcher has no permission to run the Java runtime.
/// </summary>
public sealed class GameLaunchException : Exception
{
    /// <summary>
    /// Creates the exception with the given message.
    /// </summary>
    /// <param name="message">The message describing the failure.</param>
    public GameLaunchException(string message) : base(message)
    {
    }

    /// <summary>
    /// Creates the exception with the given message and cause.
    /// </summary>
    /// <param name="message">The message describing the failure.</param>
    /// <param name="innerException">The exception that caused the failure.</param>
    public GameLaunchException(string message, Exception innerException)
        : base(message, innerException)
    {
    }
}
