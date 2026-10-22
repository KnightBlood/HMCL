using System;
using HMCLCore.NET.Setting;

namespace HMCL.NET.ViewModels;

/// <summary>
/// One of the themes shipped with the launcher, shown in the list of the
/// themes of the appearance settings.
/// </summary>
public sealed class ThemeItem
{
    /// <summary>
    /// Creates the item.
    /// </summary>
    /// <param name="theme">The theme of the item.</param>
    /// <param name="name">The name of the theme shown to the user.</param>
    /// <param name="description">The description of the theme shown to the user.</param>
    public ThemeItem(ThemePackTheme theme, string name, string description)
    {
        Theme = theme ?? throw new ArgumentNullException(nameof(theme));
        Name = name ?? string.Empty;
        Description = description ?? string.Empty;
    }

    /// <summary>
    /// The theme of the item.
    /// </summary>
    public ThemePackTheme Theme { get; }

    /// <summary>
    /// The name of the theme shown to the user.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// The description of the theme shown to the user.
    /// </summary>
    public string Description { get; }

    /// <inheritdoc/>
    public override string ToString() => Name;
}
