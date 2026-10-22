using System;
using System.Collections.Generic;

namespace HMCLCore.NET.Game;

/// <summary>
/// Builds the libraries and the arguments of an instance, which allows the
/// mod loaders to add their own libraries and arguments to the manifest of the
/// game.
/// </summary>
public sealed class GameInstanceLibraryBuilder
{
    private readonly GameInstanceManifest _manifest;
    private readonly List<string> _gameArguments;
    private readonly List<string> _jvmArguments;
    private readonly List<Library> _libraries;
    private bool _jvmChanged;

    /// <summary>
    /// Creates a builder for a manifest.
    /// </summary>
    /// <param name="manifest">The manifest to build.</param>
    public GameInstanceLibraryBuilder(GameInstanceManifest manifest)
    {
        _manifest = manifest ?? throw new ArgumentNullException(nameof(manifest));
        _libraries = new List<Library>(manifest.Libraries);
        _gameArguments = new List<string>(manifest.Arguments.Game);
        _jvmArguments = new List<string>(manifest.Arguments.Jvm);
    }

    /// <summary>
    /// The game arguments of the builder.
    /// </summary>
    public List<string> GameArguments => _gameArguments;

    /// <summary>
    /// The JVM arguments of the builder.
    /// </summary>
    public List<string> JvmArguments => _jvmArguments;

    /// <summary>
    /// The libraries of the builder.
    /// </summary>
    public List<Library> Libraries => _libraries;

    /// <summary>
    /// Builds the manifest with the modified libraries and arguments.
    /// </summary>
    /// <returns>The built manifest.</returns>
    public GameInstanceManifest Build()
    {
        return new GameInstanceManifest
        {
            Version = _manifest.Version,
            LastPlayed = _manifest.LastPlayed,
            MainClass = _manifest.MainClass,
            JavaVersion = _manifest.JavaVersion,
            Arguments = new GameInstanceManifest.LaunchArguments
            {
                Jvm = new List<string>(_jvmArguments),
                Game = new List<string>(_gameArguments)
            },
            Libraries = new List<Library>(_libraries),
            AssetIndex = _manifest.AssetIndex,
            LoggingConfig = _manifest.LoggingConfig,
            Downloads = _manifest.Downloads,
            Demo = _manifest.Demo,
            CustomResolution = _manifest.CustomResolution,
            GameSettings = _manifest.GameSettings,
            IconType = _manifest.IconType,
            Patches = new List<GameInstancePatch>(_manifest.Patches)
        };
    }

    /// <summary>
    /// Returns whether the arguments contain the given tweak class.
    /// </summary>
    /// <param name="tweakClass">The tweak class to look for.</param>
    /// <returns>Whether the arguments contain the tweak class.</returns>
    public bool HasTweakClass(string tweakClass)
    {
        for (var i = 0; i + 1 < _gameArguments.Count; i++)
        {
            if (_gameArguments[i] == "--tweakClass" && _gameArguments[i + 1] == tweakClass)
                return true;
        }

        return false;
    }

    /// <summary>
    /// Removes the given tweak class from the arguments.
    /// </summary>
    /// <param name="target">The tweak class to remove.</param>
    public void RemoveTweakClass(string target) => ReplaceTweakClass(target, null, true);

    /// <summary>
    /// Replaces the given tweak class in the arguments with another one.
    /// </summary>
    /// <param name="target">The tweak class to replace.</param>
    /// <param name="replacement">The tweak class to replace it with.</param>
    public void ReplaceTweakClass(string target, string replacement)
        => ReplaceTweakClass(target, replacement, true);

    /// <summary>
    /// Adds the given tweak class to the arguments, replacing an existing
    /// tweak class with the same name.
    /// </summary>
    /// <param name="target">The tweak class to replace.</param>
    /// <param name="replacement">The tweak class to add.</param>
    public void AddTweakClass(string target, string replacement)
        => ReplaceTweakClass(target, replacement, false);

    /// <summary>
    /// Replaces a tweak class in the arguments.
    /// </summary>
    /// <param name="target">The tweak class to replace.</param>
    /// <param name="replacement">
    /// The tweak class to replace it with; when it is null, the tweak class is
    /// only removed.
    /// </param>
    /// <param name="inPlace">
    /// Whether the first occurrence of the tweak class is replaced in place;
    /// when it is false, the tweak class is appended to the arguments.
    /// </param>
    public void ReplaceTweakClass(string target, string? replacement, bool inPlace)
    {
        if (replacement is null && inPlace)
            throw new ArgumentNullException(nameof(replacement));

        var replaced = false;

        for (var i = 0; i + 1 < _gameArguments.Count; i++)
        {
            if (_gameArguments[i] != "--tweakClass" || _gameArguments[i + 1] != target)
                continue;

            if (!replaced && inPlace && replacement is not null)
            {
                // The first occurrence of the tweak class is replaced.
                _gameArguments[i + 1] = replacement;
                replaced = true;
            }
            else
            {
                // The duplicate tweak classes are removed.
                _gameArguments.RemoveAt(i + 1);
                _gameArguments.RemoveAt(i);
                i--;
            }
        }

        // When the tweak class is not present, it is appended to the arguments.
        if (!replaced && replacement is not null)
        {
            _gameArguments.Add("--tweakClass");
            _gameArguments.Add(replacement);
        }
    }

    /// <summary>
    /// Adds game arguments.
    /// </summary>
    /// <param name="args">The arguments to add.</param>
    public void AddGameArgument(params string[] args) => _gameArguments.AddRange(args);

    /// <summary>
    /// Adds JVM arguments.
    /// </summary>
    /// <param name="args">The arguments to add.</param>
    public void AddJvmArgument(params string[] args)
    {
        _jvmChanged = true;
        _jvmArguments.AddRange(args);
    }

    /// <summary>
    /// Adds a library.
    /// </summary>
    /// <param name="library">The library to add.</param>
    public void AddLibrary(Library library) => _libraries.Add(library);

    /// <summary>
    /// Returns whether the JVM arguments were changed by the builder.
    /// </summary>
    public bool JvmChanged => _jvmChanged;
}
