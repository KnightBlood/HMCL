using System;

namespace HMCL.NET.ViewModels;

/// <summary>
/// An option of a list shown in the user interface, with the name shown to the
/// user and the value kept in the settings.
/// </summary>
/// <typeparam name="T">The type of the value of the option.</typeparam>
public sealed class NamedOption<T>
{
    /// <summary>
    /// Creates the option.
    /// </summary>
    /// <param name="value">The value of the option.</param>
    /// <param name="name">The name of the option shown to the user.</param>
    public NamedOption(T value, string name)
    {
        Value = value;
        Name = name;
    }

    /// <summary>
    /// The value of the option.
    /// </summary>
    public T Value { get; }

    /// <summary>
    /// The name of the option shown to the user.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// Returns the name of the option, so that it is shown by the combo boxes
    /// of the user interface.
    /// </summary>
    /// <returns>The name of the option.</returns>
    public override string ToString() => Name;
}
