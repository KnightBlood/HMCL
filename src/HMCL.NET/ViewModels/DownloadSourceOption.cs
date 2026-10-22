using HMCLCore.NET.Setting;

namespace HMCL.NET.ViewModels;

/// <summary>
/// An option of the download sources offered in the settings interface.
/// </summary>
public sealed class DownloadSourceOption
{
    /// <summary>
    /// The source of the downloads chosen by the option.
    /// </summary>
    public DownloadSource Value { get; }

    /// <summary>
    /// The name of the source shown to the user.
    /// </summary>
    public string Name { get; }

    /// <summary>
    /// Creates the option.
    /// </summary>
    /// <param name="value">The source of the downloads chosen by the option.</param>
    /// <param name="name">The name of the source shown to the user.</param>
    public DownloadSourceOption(DownloadSource value, string name)
    {
        Value = value;
        Name = name;
    }

    /// <summary>
    /// Returns the name of the source, so that it is shown by the combo boxes
    /// of the settings interface.
    /// </summary>
    /// <returns>The name of the source.</returns>
    public override string ToString() => Name;
}
