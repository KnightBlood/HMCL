using System;
using System.Collections.Generic;
using System.IO;
using HMCLCore.NET.Game;

namespace HMCLCore.NET.Setting;

/// <summary>
/// A game directory: the root folder that holds the instances, versions,
/// libraries and assets belonging to one or more game instances.
/// </summary>
public sealed class GameDirectory
{
    /// <summary>
    /// The stable identifier of the game directory.
    /// </summary>
    public string Id { get; init; } = string.Empty;

    /// <summary>
    /// The custom display name of the game directory, or null when unnamed.
    /// </summary>
    public string? Name { get; set; }

    /// <summary>
    /// The root path of the game directory.
    /// </summary>
    public string Path { get; set; } = string.Empty;

    /// <summary>
    /// Whether the path of the game directory is relative to the directory of
    /// the launcher, so that the launcher may be moved.
    /// </summary>
    public bool UseRelativePath { get; set; }

    /// <summary>
    /// The name shown to the user: the custom name when set, otherwise the path.
    /// </summary>
    public string DisplayName => !string.IsNullOrWhiteSpace(Name) ? Name : Path;

    /// <summary>
    /// Returns the absolute path of the game directory.
    /// </summary>
    /// <param name="homeDirectory">The directory of the launcher.</param>
    /// <returns>The absolute path of the game directory.</returns>
    public string GetAbsolutePath(string homeDirectory)
    {
        if (string.IsNullOrWhiteSpace(Path))
            return Path;

        if (!UseRelativePath || Path.Length >= 2 && Path[1] == ':')
            return Path;

        return System.IO.Path.GetFullPath(System.IO.Path.Combine(homeDirectory, Path));
    }
}

/// <summary>
/// Owns the list of game directories and the repository backing each of them,
/// mirroring the Java HMCL game directory manager.
/// </summary>
public sealed class GameDirectoryManager
{
    private readonly List<GameDirectory> _directories = new();
    private readonly Dictionary<string, DefaultGameRepository> _repositories = new(StringComparer.Ordinal);
    private readonly Func<GameDirectory, DefaultGameRepository> _repositoryFactory;
    private GameDirectory _selected;

    /// <summary>
    /// Creates a manager for the given game directories.
    /// </summary>
    /// <param name="directories">The known game directories.</param>
    /// <param name="selectedId">The identifier of the selected directory, or null to select the first one.</param>
    /// <param name="repositoryFactory">Creates the repository backing a game directory.</param>
    public GameDirectoryManager(
        IEnumerable<GameDirectory> directories,
        string? selectedId,
        Func<GameDirectory, DefaultGameRepository> repositoryFactory)
    {
        _repositoryFactory = repositoryFactory ?? throw new ArgumentNullException(nameof(repositoryFactory));

        foreach (var directory in directories)
            _directories.Add(directory);

        if (_directories.Count == 0)
            throw new ArgumentException("At least one game directory is required", nameof(directories));

        _selected = _directories.FirstOrDefault(d => d.Id == selectedId) ?? _directories[0];
    }

    /// <summary>
    /// The known game directories.
    /// </summary>
    public IReadOnlyList<GameDirectory> Directories => _directories;

    /// <summary>
    /// The currently selected game directory.
    /// </summary>
    public GameDirectory Selected => _selected;

    /// <summary>
    /// The repository backing the currently selected game directory.
    /// </summary>
    public DefaultGameRepository Repository => GetRepository(_selected);

    /// <summary>
    /// Raised when the selected game directory changes.
    /// </summary>
    public event EventHandler? SelectionChanged;

    /// <summary>
    /// Returns the repository backing the given game directory, creating it on demand.
    /// </summary>
    /// <param name="directory">The game directory.</param>
    /// <returns>The repository for the directory.</returns>
    public DefaultGameRepository GetRepository(GameDirectory directory)
    {
        if (!_repositories.TryGetValue(directory.Id, out var repository))
        {
            repository = _repositoryFactory(directory);
            _repositories[directory.Id] = repository;
        }

        return repository;
    }

    /// <summary>
    /// Adds a game directory pointing at the given path.
    /// </summary>
    /// <param name="path">The root path of the new game directory.</param>
    /// <param name="name">The optional custom display name.</param>
    /// <returns>The created game directory.</returns>
    public GameDirectory Add(string path, string? name = null)
    {
        var directory = new GameDirectory
        {
            Id = Guid.NewGuid().ToString("N"),
            Name = name,
            Path = path
        };

        _directories.Add(directory);
        return directory;
    }

    /// <summary>
    /// Removes a game directory. The last remaining directory cannot be removed.
    /// </summary>
    /// <param name="directory">The game directory to remove.</param>
    /// <returns>Whether the directory was removed.</returns>
    public bool Remove(GameDirectory directory)
    {
        if (_directories.Count <= 1)
            return false;

        if (!_directories.Remove(directory))
            return false;

        _repositories.Remove(directory.Id);

        if (ReferenceEquals(_selected, directory))
            Select(_directories[0]);

        return true;
    }

    /// <summary>
    /// Updates the name and path of a game directory.
    /// </summary>
    /// <param name="directory">The game directory to update.</param>
    /// <param name="name">The new custom display name, or null to clear it.</param>
    /// <param name="path">The new root path.</param>
    /// <returns>Whether the directory was updated.</returns>
    public bool Update(GameDirectory directory, string? name, string path)
    {
        if (!_directories.Contains(directory))
            return false;

        directory.Name = name;
        directory.Path = path;

        // The repository caches the resolved layout, so it has to be recreated.
        _repositories.Remove(directory.Id);
        return true;
    }

    /// <summary>
    /// Selects a game directory.
    /// </summary>
    /// <param name="directory">The game directory to select.</param>
    public void Select(GameDirectory directory)
    {
        if (ReferenceEquals(_selected, directory))
            return;

        if (!_directories.Contains(directory))
            return;

        _selected = directory;
        SelectionChanged?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>
    /// Creates the directory tree of a game directory if it does not exist yet.
    /// </summary>
    /// <param name="directory">The game directory.</param>
    /// <param name="homeDirectory">The directory of the launcher.</param>
    public static void EnsureLayout(GameDirectory directory, string homeDirectory)
    {
        var path = directory.GetAbsolutePath(homeDirectory);
        if (string.IsNullOrWhiteSpace(path))
            return;

        Directory.CreateDirectory(Path.Combine(path, "instances"));
        Directory.CreateDirectory(Path.Combine(path, "versions"));
        Directory.CreateDirectory(Path.Combine(path, "libraries"));
        Directory.CreateDirectory(Path.Combine(path, "assets"));
    }
}
