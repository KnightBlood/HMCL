using Avalonia.Controls;

namespace HMCL.NET.Views;

/// <summary>
/// The launch page letting the user choose an account and game instance, then start the game.
/// All bindings resolve against the owning <see cref="MainWindowViewModel"/> data context.
/// </summary>
public partial class LaunchPage : UserControl
{
    /// <summary>
    /// Creates a new LaunchPage.
    /// </summary>
    public LaunchPage()
    {
        InitializeComponent();
    }
}
