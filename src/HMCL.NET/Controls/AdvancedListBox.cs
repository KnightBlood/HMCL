using Avalonia.Controls;
using Avalonia.Layout;

namespace HMCL.NET.Controls;

/// <summary>
/// A vertical list of the items of a navigation drawer, like the advanced list
/// box of Java HMCL.
/// </summary>
/// <remarks>
/// The items of the list are the children of the control; they are usually
/// <see cref="AdvancedListItem"/> controls, separated by the group titles
/// built with <see cref="ClassTitle"/>.
/// </remarks>
public class AdvancedListBox : StackPanel
{
    /// <summary>
    /// Creates the list.
    /// </summary>
    public AdvancedListBox()
    {
        Spacing = 2;
        HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Stretch;
    }
}
