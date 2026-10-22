using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using HMCLCore.NET.Util;
using System.Text.Json.Nodes;
using System.Text.Json.Serialization;
using HMCLCore.NET.Auth.AuthlibInjector;

namespace HMCLCore.NET.Setting;

/// <summary>
/// The kind of the background of the launcher.
/// </summary>
public enum BackgroundType
{
    /// <summary>
    /// The launcher uses the background of the theme when it has one, and one
    /// of the wallpapers shipped with the launcher otherwise.
    /// </summary>
    Default,

    /// <summary>
    /// The launcher has no background image.
    /// </summary>
    None,

    /// <summary>
    /// The background is a local image chosen by the user.
    /// </summary>
    Image,

    /// <summary>
    /// The background is an image downloaded from the network.
    /// </summary>
    Network,

    /// <summary>
    /// The background is one of the wallpapers shipped with the launcher.
    /// </summary>
    Builtin,

    /// <summary>
    /// The background is a plain color.
    /// </summary>
    Color
}

/// <summary>
/// Represents the launcher settings.
/// </summary>
public sealed class LauncherSettings
{
    /// <summary>
    /// The last selected game instance ID.
    /// </summary>
    public string LastInstanceId { get; set; } = string.Empty;

    /// <summary>
    /// The last selected account username.
    /// </summary>
    public string LastUsername { get; set; } = string.Empty;

    /// <summary>
    /// The memory allocation in MB.
    /// </summary>
    public int Memory { get; set; } = 2048;

    /// <summary>
    /// The Java executable path.
    /// </summary>
    public string JavaPath { get; set; } = "java";

    /// <summary>
    /// The game directory.
    /// </summary>
    public string GameDirectory { get; set; } = string.Empty;

    /// <summary>
    /// Whether to use custom JVM arguments.
    /// </summary>
    public bool UseCustomJvmArgs { get; set; }

    /// <summary>
    /// Custom JVM arguments string.
    /// </summary>
    public string CustomJvmArgs { get; set; } = string.Empty;

    /// <summary>
    /// The window width.
    /// </summary>
    public int WindowWidth { get; set; } = 854;

    /// <summary>
    /// The window height.
    /// </summary>
    public int WindowHeight { get; set; } = 480;

    /// <summary>
    /// The position of the left edge of the window, or null when the position
    /// is not known.
    /// </summary>
    public double? WindowLeft { get; set; }

    /// <summary>
    /// The position of the top edge of the window, or null when the position
    /// is not known.
    /// </summary>
    public double? WindowTop { get; set; }

    /// <summary>
    /// Whether the window was maximized when the launcher was closed.
    /// </summary>
    public bool WindowMaximized { get; set; }

    /// <summary>
    /// Whether to show the console window.
    /// </summary>
    public bool ShowConsole { get; set; }

    /// <summary>
    /// The selected download provider name.
    /// </summary>
    public string DownloadProvider { get; set; } = "Mojang Official";

    /// <summary>
    /// The source used to download the version lists of the game.
    /// </summary>
    public DownloadSource VersionListSource { get; set; } = DownloadSource.Default;

    /// <summary>
    /// The source used to download the files of the game.
    /// </summary>
    public DownloadSource FileDownloadSource { get; set; } = DownloadSource.Default;

    /// <summary>
    /// Whether the download concurrency is chosen automatically.
    /// </summary>
    public bool AutoDownloadThreads { get; set; } = true;

    /// <summary>
    /// The number of parallel downloads used when <see cref="AutoDownloadThreads"/> is false.
    /// </summary>
    public int DownloadThreads { get; set; } = 8;

    /// <summary>
    /// The CurseForge API key used to query and resolve mods.
    /// </summary>
    public string CurseForgeApiKey { get; set; } = string.Empty;

    /// <summary>
    /// The proxy configuration.
    /// </summary>
    public ProxyConfig? ProxyConfig { get; set; }

    /// <summary>
    /// The language/locale setting.
    /// </summary>
    /// <remarks>
    /// An empty value means that the language of the operating system is used.
    /// </remarks>
    public string Language { get; set; } = string.Empty;

    /// <summary>
    /// The global game settings preset shared by all instances.
    /// Instance-specific settings override this preset.
    /// </summary>
    public GameSettingsPreset GameSettingsPreset { get; set; } = new();

    /// <summary>
    /// The game settings presets of the launcher; the instances inherit from
    /// the preset they name.
    /// </summary>
    public List<GameSettingsPreset> GameSettingsPresets { get; set; } = new();

    /// <summary>
    /// The identifier of the preset used by the instances that do not name
    /// one.
    /// </summary>
    public string DefaultGameSettingsPresetId { get; set; } = string.Empty;

    /// <summary>
    /// The game directories known to the launcher.
    /// </summary>
    public List<GameDirectory> GameDirectories { get; set; } = new();

    /// <summary>
    /// The identifier of the selected game directory.
    /// </summary>
    public string? SelectedGameDirectoryId { get; set; }

    /// <summary>
    /// The interface theme: "跟随系统" (follow the system), "浅色" (light) or
    /// "深色" (dark).
    /// </summary>
    public string Theme { get; set; } = "跟随系统";

    /// <summary>
    /// The font family used by the launcher interface, or null to use the
    /// system default font.
    /// </summary>
    public string? FontFamily { get; set; }

    /// <summary>
    /// The base font size used by the launcher interface.
    /// </summary>
    public double FontSize { get; set; } = 13;

    /// <summary>
    /// The path of the image used as the launcher background, or null when no
    /// custom background is used.
    /// </summary>
    public string? BackgroundImagePath { get; set; }

    /// <summary>
    /// Whether the launcher window may use transparent backgrounds when the
    /// platform supports it.
    /// </summary>
    public bool EnableTransparencyEffects { get; set; }

    /// <summary>
    /// The channel used to look for updates: <c>stable</c> or <c>development</c>.
    /// </summary>
    public string UpdateChannel { get; set; } = "stable";

    /// <summary>
    /// The custom theme color of the launcher, in the "#RRGGBB" format; it is
    /// empty when the color of the system is used.
    /// </summary>
    public string ThemeColor { get; set; } = string.Empty;

    /// <summary>
    /// The kind of the background of the launcher.
    /// </summary>
    public BackgroundType BackgroundType { get; set; } = BackgroundType.Default;

    /// <summary>
    /// The opacity of the background image of the launcher, between 0 and 1.
    /// </summary>
    public double BackgroundOpacity { get; set; } = 1.0;

    /// <summary>
    /// The address of the background image downloaded from the network.
    /// </summary>
    public string NetworkBackgroundImageUrl { get; set; } = string.Empty;

    /// <summary>
    /// The identifier of the built-in wallpaper used as the background of the
    /// launcher; it is empty when no built-in wallpaper is used.
    /// </summary>
    public string BuiltinWallpaperId { get; set; } = "2016-02-25";

    /// <summary>
    /// The color style used to derive the colors of the launcher from the seed
    /// color.
    /// </summary>
    public ThemeColorStyle ThemeColorStyle { get; set; } = ThemeColorStyle.Default;

    /// <summary>
    /// The contrast level of the colors of the launcher.
    /// </summary>
    public ThemeContrast ThemeContrast { get; set; } = ThemeContrast.Default;

    /// <summary>
    /// Whether the animations of the user interface are disabled.
    /// </summary>
    public bool DisableAnimations { get; set; }

    /// <summary>
    /// The way the text of the user interface is rendered: <c>default</c>,
    /// <c>antialias</c>, <c>subpixel</c> or <c>alias</c>.
    /// </summary>
    public string TextRenderingMode { get; set; } = "default";

    /// <summary>
    /// The font family of the log window; the system font is used when it is
    /// empty.
    /// </summary>
    public string LogFontFamily { get; set; } = string.Empty;

    /// <summary>
    /// The size of the font of the log window, in points.
    /// </summary>
    public double LogFontSize { get; set; } = 12;

    /// <summary>
    /// Whether the title bar of the launcher is transparent.
    /// </summary>
    public bool TitleBarTransparent { get; set; } = true;

    /// <summary>
    /// Whether the anonymous usage statistics are sent to the developer of the
    /// launcher.
    /// </summary>
    public bool EnableAnalytics { get; set; }

    /// <summary>
    /// The anonymous identifier of the machine used by the usage statistics.
    /// </summary>
    public string AnalyticsDeviceId { get; set; } = string.Empty;

    /// <summary>
    /// The address of the Countly server receiving the usage statistics.
    /// </summary>
    public string AnalyticsServerUrl { get; set; } = string.Empty;

    /// <summary>
    /// The key of the application on the Countly server.
    /// </summary>
    public string AnalyticsAppKey { get; set; } = string.Empty;

    /// <summary>
    /// Whether the April Fools' mode of the launcher is disabled.
    /// </summary>
    public bool DisableAprilFools { get; set; }

    /// <summary>
    /// Whether the launcher shows the diagnostic information of the game
    /// processes it starts.
    /// </summary>
    public bool Debug { get; set; }

    /// <summary>
    /// The authlib-injector servers saved by the user.
    /// </summary>
    public List<AuthlibInjectorServerEntry> AuthServers { get; set; } = new();

    /// <summary>
    /// The paths of the Java runtimes the user disabled; they are ignored
    /// when the Java runtimes are scanned.
    /// </summary>
    public List<string> DisabledJava { get; set; } = new();
}

/// <summary>
/// Proxy configuration for downloads.
/// </summary>
public sealed class ProxyConfig
{
    /// <summary>
    /// Whether to use a proxy.
    /// </summary>
    public bool Enabled { get; set; }

    /// <summary>
    /// The proxy type (http, socks4, socks5).
    /// </summary>
    public string Type { get; set; } = "http";

    /// <summary>
    /// The proxy host.
    /// </summary>
    public string Host { get; set; } = string.Empty;

    /// <summary>
    /// The proxy port.
    /// </summary>
    public int Port { get; set; } = 8080;

    /// <summary>
    /// The proxy username.
    /// </summary>
    public string? Username { get; set; }

    /// <summary>
    /// The proxy password.
    /// </summary>
    public string? Password { get; set; }

    /// <summary>
    /// Returns whether the proxy requires authentication.
    /// </summary>
    public bool RequiresAuthentication => !string.IsNullOrEmpty(Username);
}

/// <summary>
/// Manages application settings with persistence to disk.
/// </summary>
public sealed class SettingsManager
{
    private readonly string _settingsFile;
    private readonly JsonSerializerOptions _jsonOptions = new() { WriteIndented = true };
    private string? _preservedSchema;
    private bool _readOnly;

    /// <summary>
    /// The current settings.
    /// </summary>
    public LauncherSettings Settings { get; private set; } = new();

    /// <summary>
    /// The path of the file holding the settings.
    /// </summary>
    public string SettingsFile => _settingsFile;

    /// <summary>
    /// The error that occurred while loading or saving the settings, if any.
    /// </summary>
    public string? LastError { get; private set; }

    /// <summary>
    /// Whether the settings file was written by a newer version of the launcher
    /// and must not be overwritten.
    /// </summary>
    public bool IsReadOnly => _readOnly;

    /// <summary>
    /// Backs up the settings file and overwrites it with the settings
    /// currently loaded by this version of the launcher.
    /// </summary>
    /// <returns>
    /// The path of the backup file, or null when the file could not be backed
    /// up.
    /// </returns>
    public string? ForceWrite()
    {
        try
        {
            string? backup = null;

            if (File.Exists(_settingsFile))
            {
                backup = $"{_settingsFile}.{DateTime.Now:yyyyMMdd-HHiiss}.bak";
                File.Copy(_settingsFile, backup, true);
            }

            _readOnly = false;
            _preservedSchema = null;
            Save();

            return backup;
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            LastError = ex.Message;
            return null;
        }
    }

    /// <summary>
    /// Creates a new SettingsManager that loads/saves to the specified file.
    /// </summary>
    public SettingsManager(string SettingsFile)
    {
        _settingsFile = SettingsFile ?? throw new ArgumentNullException(nameof(SettingsFile));
        var dir = Path.GetDirectoryName(SettingsFile);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);
    }

    /// <summary>
    /// Loads settings from disk, or creates defaults if the file doesn't exist.
    /// </summary>
    /// <remarks>
    /// The settings files written by the older versions of the launcher are
    /// migrated to the current format. The files that were written by an
    /// incompatible version of the launcher are rejected, and the defaults are
    /// used instead.
    /// </remarks>
    public void Load()
    {
        Settings = new LauncherSettings();
        LastError = null;
        _preservedSchema = null;
        _readOnly = false;

        if (!File.Exists(_settingsFile))
            return;

        string json;
        try
        {
            json = File.ReadAllText(_settingsFile);
        }
        catch (IOException ex)
        {
            LastError = ex.Message;
            return;
        }

        try
        {
            using var document = JsonDocument.Parse(json, new JsonDocumentOptions
            {
                AllowTrailingCommas = true,
                CommentHandling = JsonCommentHandling.Skip
            });

            if (document.RootElement.ValueKind != JsonValueKind.Object)
            {
                LastError = "设置文件的格式不正确";
                return;
            }

            var compatibility = JsonSchema.Check(document.RootElement, SettingsMigrations.CurrentSchema);
            if (!compatibility.Readable &&
                compatibility.Status != JsonSchema.CompatibilityResult.CompatibilityStatus.Missing)
            {
                LastError = $"设置文件的格式不受支持: {compatibility.Status}";
                return;
            }

            // Only the files written by a newer minor version of the launcher
            // must not be overwritten; the files without a schema marker are
            // migrated and saved with the current schema.
            _readOnly = compatibility.Status ==
                        JsonSchema.CompatibilityResult.CompatibilityStatus.ReadOnlyPreserveSchema;
            if (compatibility.PreserveSchema && compatibility.Actual is not null)
                _preservedSchema = compatibility.Actual.Value;

            var node = JsonNode.Parse(json, documentOptions: new JsonDocumentOptions
            {
                AllowTrailingCommas = true,
                CommentHandling = JsonCommentHandling.Skip
            }) as JsonObject;

            if (node is null)
            {
                LastError = "设置文件的格式不正确";
                return;
            }

            var version = compatibility.Actual?.Version ?? SettingsMigrations.LegacyVersion;
            var migrated = SettingsMigrations.Migrate(node, version);

            Settings = migrated.Deserialize(SettingsJsonContext.Default.LauncherSettings)
                       ?? new LauncherSettings();

            // The files written by the older versions of the launcher are saved
            // again with the current schema, so that the migration is applied
            // only once.
            if (!_readOnly && version.CompareTo(SettingsMigrations.CurrentSchema.Version!) < 0)
                Save();
        }
        catch (JsonException ex)
        {
            LastError = ex.Message;
            Settings = new LauncherSettings();
        }
    }

    /// <summary>
    /// Makes sure that the presets of the game settings are consistent: the
    /// list of the presets always holds at least one preset, and the default
    /// preset is always one of them.
    /// </summary>
    public void EnsureGameSettingsPresets()
    {
        var settings = Settings;

        if (settings.GameSettingsPresets.Count == 0)
        {
            // The settings of the older versions of the launcher only hold a
            // single preset; it becomes the default preset of the list.
            var preset = settings.GameSettingsPreset;
            if (string.IsNullOrEmpty(preset.Id))
                preset.Id = Guid.NewGuid().ToString("N");
            if (string.IsNullOrEmpty(preset.Name))
                preset.Name = "默认";

            settings.GameSettingsPresets.Add(preset);

            if (string.IsNullOrEmpty(settings.DefaultGameSettingsPresetId))
                settings.DefaultGameSettingsPresetId = preset.Id;
        }
        else
        {
            foreach (var preset in settings.GameSettingsPresets)
            {
                if (string.IsNullOrEmpty(preset.Id))
                    preset.Id = Guid.NewGuid().ToString("N");
                if (string.IsNullOrEmpty(preset.Name))
                    preset.Name = "默认";
            }

            if (string.IsNullOrEmpty(settings.DefaultGameSettingsPresetId) ||
                settings.GameSettingsPresets.All(p => p.Id != settings.DefaultGameSettingsPresetId))
            {
                settings.DefaultGameSettingsPresetId = settings.GameSettingsPresets[0].Id;
            }
        }
    }

    /// <summary>
    /// Returns the preset of the game settings inherited by the instance.
    /// </summary>
    /// <param name="instance">
    /// The settings of the instance, or null when the instance does not have
    /// any settings of its own.
    /// </param>
    /// <returns>
    /// The preset named by the instance, the default preset when the instance
    /// does not name one.
    /// </returns>
    public GameSettingsPreset GetGameSettingsPreset(GameSettingsInstance? instance)
    {
        EnsureGameSettingsPresets();

        var presets = Settings.GameSettingsPresets;

        if (instance is { } settings && !string.IsNullOrEmpty(settings.PresetId))
        {
            var preset = presets.FirstOrDefault(p => p.Id == settings.PresetId);
            if (preset is not null)
                return preset;
        }

        var defaultId = Settings.DefaultGameSettingsPresetId;
        var defaultPreset = presets.FirstOrDefault(p => p.Id == defaultId);
        return defaultPreset ?? presets[0];
    }

    /// <summary>
    /// Copies the values of the default preset into the single preset of the
    /// settings, keeping both of them in sync.
    /// </summary>
    public void SyncDefaultGameSettingsPreset()
    {
        EnsureGameSettingsPresets();

        var preset = GetGameSettingsPreset(null);
        var target = Settings.GameSettingsPreset;

        GameSettings.CopyValues(preset, target);
        target.Id = preset.Id;
        target.Name = preset.Name;
    }

    /// <summary>
    /// Saves the current settings to disk.
    /// </summary>
    public void Save()
    {
        if (_readOnly)
        {
            LastError = "设置文件由更高版本的启动器创建，无法保存";
            return;
        }

        var dir = Path.GetDirectoryName(_settingsFile);
        if (!string.IsNullOrEmpty(dir))
            Directory.CreateDirectory(dir);

        var node = JsonSerializer.SerializeToNode(Settings, SettingsJsonContext.Default.LauncherSettings) as JsonObject
                   ?? new JsonObject();

        node[JsonSchema.PropertySchema] = _preservedSchema ?? SettingsMigrations.CurrentSchema.Url;

        File.WriteAllText(_settingsFile, node.ToJsonString(_jsonOptions));
    }

    /// <summary>
    /// Updates a setting and saves immediately.
    /// </summary>
    public void Update(Action<LauncherSettings> updateAction)
    {
        updateAction(Settings);
        Save();
    }

    /// <summary>
    /// Resets settings to defaults and saves.
    /// </summary>
    public void Reset()
    {
        Settings = new LauncherSettings();
        _preservedSchema = null;
        Save();
    }
}

/// <summary>
/// Observable setting wrapper for data binding.
/// </summary>
public sealed class ObservableSetting<T>
{
    private T _value;
    private readonly List<Action<T>> _subscribers = new();

    /// <summary>
    /// The current value.
    /// </summary>
    public T Value
    {
        get => _value;
        set
        {
            if (!EqualityComparer<T>.Default.Equals(_value, value))
            {
                _value = value;
                NotifySubscribers();
            }
        }
    }

    /// <summary>
    /// Creates a new ObservableSetting with an initial value.
    /// </summary>
    public ObservableSetting(T initialValue)
    {
        _value = initialValue;
    }

    /// <summary>
    /// Subscribes to value changes.
    /// </summary>
    public void Subscribe(Action<T> callback)
    {
        _subscribers.Add(callback);
        callback(_value);
    }

    private void NotifySubscribers()
    {
        foreach (var callback in _subscribers)
            callback(_value);
    }
}
