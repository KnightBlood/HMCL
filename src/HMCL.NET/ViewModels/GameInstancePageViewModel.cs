using System.Collections.Generic;
using Avalonia.Controls;
using HMCLCore.NET.Game;

namespace HMCL.NET.ViewModels;

/// <summary>
/// View model for the instance detail page (GameInstancePage).
/// Holds the selected instance and manages tab navigation.
/// </summary>
public sealed class GameInstancePageViewModel : ViewModelBase
{
    private GameInstance? _instance;
    private int _selectedTabIndex;

    /// <summary>
    /// The game instance being viewed.
    /// </summary>
    public GameInstance? Instance
    {
        get => _instance;
        set => Set(ref _instance, value);
    }

    /// <summary>
    /// The currently selected tab index.
    /// </summary>
    public int SelectedTabIndex
    {
        get => _selectedTabIndex;
        set => Set(ref _selectedTabIndex, value);
    }

    /// <summary>
    /// Creates a new GameInstancePageViewModel.
    /// </summary>
    public GameInstancePageViewModel()
    {
    }
}
