using System;

namespace HMCL.NET.Views;

/// <summary>
/// A folder entry with a display name and filesystem path, used by <see cref="InstanceFoldersPage"/>.
/// </summary>
public sealed class FolderEntry
{
    /// <summary>
    /// The folder display name.
    /// </summary>
    public string Name { get; init; } = string.Empty;

    /// <summary>
    /// The folder filesystem path.
    /// </summary>
    public string Path { get; init; } = string.Empty;
}
