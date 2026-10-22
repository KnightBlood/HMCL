using Avalonia.Controls;
using Avalonia.Markup.Xaml;

namespace HMCL.NET.Strings;

/// <summary>
/// The English resources of the launcher.
/// </summary>
/// <remarks>
/// The dictionary is loaded from <c>English.axaml</c>; the keys of the
/// dictionary are the identifiers used by the <c>DynamicResource</c>
/// references of the user interface.
/// </remarks>
public partial class English : ResourceDictionary
{
    /// <summary>
    /// Creates the dictionary and loads its resources.
    /// </summary>
    public English()
    {
        AvaloniaXamlLoader.Load(this);
    }
}
