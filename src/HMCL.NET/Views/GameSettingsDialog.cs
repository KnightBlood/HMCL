using System;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Game;
using HMCLCore.NET.Setting;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;

namespace HMCL.NET.Views;

/// <summary>
/// Edits an instance's game settings (overrides of the global preset).
/// </summary>
public sealed class GameSettingsDialog : Window
{
    private readonly GameInstance _instance;
    private readonly MainWindowViewModel _viewModel;

    private readonly GameSettingsInstance _settings;
    private readonly TextBox _jvmOptionsBox = new();
    private readonly CheckBox _autoMemoryBox = new() { Content = Lang.Get("hmcl.gamesettingsdialog.settings_memory_auto_allocate") };
    private readonly NumericUpDown _minMemoryBox = new() { Minimum = 0, Maximum = 65536, Width = 140 };
    private readonly NumericUpDown _maxMemoryBox = new() { Minimum = 0, Maximum = 65536, Width = 140 };
    private readonly TextBox _gameArgsBox = new();
    private readonly TextBox _runningDirBox = new();
    private readonly ComboBox _quickPlayModeBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly ComboBox _presetBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly TextBox _quickPlayArgBox = new();
    private readonly TextBox _envVarsBox = new() { AcceptsReturn = true, TextWrapping = TextWrapping.Wrap, MinHeight = 80, Width = 320 };
    private readonly ComboBox _javaTypeBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly TextBox _customJavaVersionBox = new() { Width = 200 };
    private readonly TextBox _customJavaPathBox = new() { Width = 320 };
    private readonly NumericUpDown _widthBox = new() { Minimum = 1, Maximum = 9999, Increment = 1, FormatString = "0", Width = 120 };
    private readonly NumericUpDown _heightBox = new() { Minimum = 1, Maximum = 9999, Increment = 1, FormatString = "0", Width = 120 };
    private readonly CheckBox _fullscreenBox = new() { Content = Lang.Get("hmcl.settingspage.358_", "全屏") };
    private readonly ComboBox _processPriorityBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly ComboBox _launcherVisibilityBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly ComboBox _graphicsBackendBox = new() { Width = 200, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Left };
    private readonly TextBox _wrapperBox = new() { Width = 320 };
    private readonly TextBox _preCommandBox = new() { Width = 320 };
    private readonly TextBox _postCommandBox = new() { Width = 320 };
    private readonly CheckBox _showLogsBox = new() { Content = Lang.Get("settings.game.show_logs", "显示游戏日志窗口") };
    private readonly CheckBox _noJvmOptionsBox = new() { Content = Lang.Get("settings.advanced.no_jvm_options", "禁用版本自带的 JVM 参数") };
    private readonly CheckBox _noOptimizingJvmOptionsBox = new() { Content = Lang.Get("settings.advanced.no_optimizing_jvm_options", "禁用优化后的 JVM 参数") };
    private readonly CheckBox _notCheckJvmBox = new() { Content = Lang.Get("settings.advanced.not_check_jvm", "不检查 Java 运行时") };
    private readonly CheckBox _notCheckGameBox = new() { Content = Lang.Get("settings.advanced.not_check_game", "不检查游戏文件") };
    private readonly NumericUpDown _permSizeBox = new() { Minimum = 0, Maximum = 1024, Increment = 16, Width = 140 };
    private readonly CheckBox _useCustomNativesBox = new() { Content = Lang.Get("settings.advanced.use_custom_natives", "使用自定义原生库目录") };
    private readonly TextBox _nativesDirectoryBox = new() { Width = 320 };
    private readonly Button _saveButton = new() { Content = Lang.Get("hmcl.gamesettingsdialog.button_save"), Width = 100 };
    private readonly TextBlock _statusText = new() { Foreground = Brushes.Gray, FontSize = 12, TextWrapping = TextWrapping.Wrap };

    public GameSettingsDialog(GameInstance instance, MainWindowViewModel viewModel)
    {
        _instance = instance ?? throw new ArgumentNullException(nameof(instance));
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

        Title = $"实例游戏设置 - {instance.Id}";
        Width = 520;
        Height = 520;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var manifest = _viewModel.LoadInstanceManifest(instance.Id) ?? new GameInstanceManifest();
        _settings = manifest.GameSettings;

        // The instance inherits from the preset it names; the dialog shows the
        // effective values, so that the user sees the settings the game is
        // launched with.
        var presets = _viewModel.GetGameSettingsPresets();
        _presetBox.ItemsSource = presets;
        _presetBox.SelectedItem = presets.FirstOrDefault(item => item.Id == _settings.PresetId)
                                 ?? presets.FirstOrDefault();

        // The lists of the values the user may choose.
        _quickPlayModeBox.ItemsSource = new List<QuickPlayMode>
        {
            QuickPlayMode.None,
            QuickPlayMode.Singleplayer,
            QuickPlayMode.Multiplayer,
            QuickPlayMode.Realms
        };

        _javaTypeBox.ItemsSource = new List<JavaVersionType>
        {
            JavaVersionType.Auto,
            JavaVersionType.Version,
            JavaVersionType.Custom,
            JavaVersionType.Detected
        };

        _processPriorityBox.ItemsSource = new List<HMCLCore.NET.Setting.ProcessPriority>
        {
            HMCLCore.NET.Setting.ProcessPriority.High,
            HMCLCore.NET.Setting.ProcessPriority.AboveNormal,
            HMCLCore.NET.Setting.ProcessPriority.Normal,
            HMCLCore.NET.Setting.ProcessPriority.BelowNormal,
            HMCLCore.NET.Setting.ProcessPriority.Low
        };

        _launcherVisibilityBox.ItemsSource = new List<LauncherVisibility>
        {
            LauncherVisibility.Hide,
            LauncherVisibility.HideAndReopen,
            LauncherVisibility.Close,
            LauncherVisibility.CloseAndReopen,
            LauncherVisibility.Keep
        };

        _graphicsBackendBox.ItemsSource = new List<GraphicsBackend>
        {
            GraphicsBackend.Auto,
            GraphicsBackend.OpenGL,
            GraphicsBackend.Vulkan,
            GraphicsBackend.Software
        };

        LoadValues();

        // The values of the preset follow the preset chosen by the user.
        _presetBox.SelectionChanged += (_, _) => LoadValues();

        _autoMemoryBox.IsCheckedChanged += (_, _) => UpdateMemoryControls();
        UpdateMemoryControls();

        _saveButton.Click += async (_, _) => await SaveAsync();

        _jvmOptionsBox.HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Stretch;
        _gameArgsBox.HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Stretch;
        _runningDirBox.HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Stretch;

        var cancelButton = new Button { Content = Lang.Get("hmcl.gamesettingsdialog.button_cancel"), Width = 80 };
        cancelButton.Click += (_, _) => Close();

        var buttonRow = new StackPanel
        {
            Orientation = Avalonia.Layout.Orientation.Horizontal,
            HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Right,
            Spacing = 10,
            Children = { cancelButton, _saveButton }
        };

        var content = new ScrollViewer
        {
            Content = new StackPanel
            {
                Spacing = 10,
                Margin = new Thickness(16),
                Children =
                {
                    new TextBlock { Text = $"实例 {instance.Id} 的游戏设置覆盖项", FontSize = 16, FontWeight = FontWeight.Bold },
                    new TextBlock { Text = Lang.Get("hmcl.gamesettingsdialog.934_"), Foreground = Brushes.Gray, FontSize = 12, TextWrapping = TextWrapping.Wrap },
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.936_"), _jvmOptionsBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.938_"), new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 8,
                        Children =
                        {
                            _autoMemoryBox,
                            new TextBlock { Text = Lang.Get("hmcl.gamesettingsdialog.940_"), VerticalAlignment = Avalonia.Layout.VerticalAlignment.Center },
                            _minMemoryBox,
                            new TextBlock { Text = Lang.Get("hmcl.gamesettingsdialog.942_"), VerticalAlignment = Avalonia.Layout.VerticalAlignment.Center },
                            _maxMemoryBox,
                        }
                    }),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.944_"), _gameArgsBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.946_"), _runningDirBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.948_"), _quickPlayModeBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.950_"), _quickPlayArgBox),
                    Labeled(Lang.Get("GameSettingsPreset.Inherit", "实例继承的预设"), _presetBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.936_"), _jvmOptionsBox),

                    // The settings the instance may override, like the game
                    // settings page of Java HMCL.
                    new TextBlock
                    {
                        Text = Lang.Get("hmcl.gamesettingsdialog.settings_memory_auto_allocate"),
                        FontWeight = FontWeight.Bold,
                        Margin = new Thickness(0, 8, 0, 0)
                    },
                    Labeled(Lang.Get("hmcl.settingspage.336_", "Java 版本类型"), _javaTypeBox),
                    Labeled(Lang.Get("hmcl.settingspage.338_", "Java 版本号"), _customJavaVersionBox),
                    Labeled(Lang.Get("hmcl.settingspage.340_", "Java 路径"), _customJavaPathBox),
                    Labeled(Lang.Get("hmcl.settingspage.358_", "全屏"), _fullscreenBox),
                    Labeled(Lang.Get("hmcl.settingspage.360_", "窗口尺寸"), new StackPanel
                    {
                        Orientation = Orientation.Horizontal,
                        Spacing = 8,
                        Children =
                        {
                            _widthBox,
                            new TextBlock { Text = "x", VerticalAlignment = VerticalAlignment.Center },
                            _heightBox
                        }
                    }),
                    Labeled(Lang.Get("settings.advanced.process_priority", "进程优先级"), _processPriorityBox),
                    Labeled(Lang.Get("hmcl.settingspage.364_", "启动器可见性"), _launcherVisibilityBox),
                    Labeled(Lang.Get("hmcl.settingspage.366_", "图形后端"), _graphicsBackendBox),
                    Labeled(Lang.Get("hmcl.settingspage.368_", "包装器"), _wrapperBox),
                    Labeled(Lang.Get("hmcl.settingspage.372_", "前置命令"), _preCommandBox),
                    Labeled(Lang.Get("hmcl.settingspage.376_", "后置命令"), _postCommandBox),
                    Labeled(Lang.Get("settings.game.show_logs", "显示游戏日志窗口"), _showLogsBox),

                    // The advanced options of the launch.
                    new TextBlock
                    {
                        Text = Lang.Get("settings.advanced.game_options", "高级启动选项"),
                        FontWeight = FontWeight.Bold,
                        Margin = new Thickness(0, 8, 0, 0)
                    },
                    Labeled(Lang.Get("settings.advanced.no_jvm_options", "禁用版本自带的 JVM 参数"), _noJvmOptionsBox),
                    Labeled(Lang.Get("settings.advanced.no_optimizing_jvm_options", "禁用优化后的 JVM 参数"), _noOptimizingJvmOptionsBox),
                    Labeled(Lang.Get("settings.advanced.not_check_jvm", "不检查 Java 运行时"), _notCheckJvmBox),
                    Labeled(Lang.Get("settings.advanced.not_check_game", "不检查游戏文件"), _notCheckGameBox),
                    Labeled(Lang.Get("settings.advanced.perm_size", "PermGen / 元空间大小"), _permSizeBox),
                    Labeled(Lang.Get("settings.advanced.use_custom_natives", "使用自定义原生库目录"), _useCustomNativesBox),
                    Labeled(Lang.Get("settings.advanced.natives_directory", "原生库目录"), _nativesDirectoryBox),
                    Labeled(Lang.Get("hmcl.gamesettingsdialog.952_"), _envVarsBox),
                    buttonRow,
                    _statusText
                }
            }
        };

        Content = content;
        this.EnableEscapeToClose();
    }

    private static Panel Labeled(string label, Control control)
    {
        return new StackPanel
        {
            Spacing = 4,
            Children =
            {
                new TextBlock { Text = label, FontWeight = FontWeight.Bold },
                control
            }
        };
    }

    /// <summary>
    /// Shows the effective values of the game settings in the fields of the
    /// dialog.
    /// </summary>
    private void LoadValues()
    {
        var preset = (_presetBox.SelectedItem as GameSettingsPreset) ?? new GameSettingsPreset();
        var effective = new GameSettingsEffective(preset, _settings);

        _jvmOptionsBox.Text = effective.JvmOptions;
        _autoMemoryBox.IsChecked = effective.AutoMemory;
        _minMemoryBox.Value = effective.MinMemory ?? 0;
        _maxMemoryBox.Value = effective.MaxMemory ?? 0;
        _gameArgsBox.Text = effective.GameArguments;
        _runningDirBox.Text = effective.RunningDirectory;
        _quickPlayModeBox.SelectedItem = effective.QuickPlayMode;
        _quickPlayArgBox.Text = effective.QuickPlayArgument;
        _envVarsBox.Text = EnvironmentVariableText.Format(effective.EnvironmentVariables);
        _javaTypeBox.SelectedItem = effective.JavaType;
        _customJavaVersionBox.Text = effective.CustomJavaVersion;
        _customJavaPathBox.Text = effective.CustomJavaPath;
        _widthBox.Value = (int)effective.Width;
        _heightBox.Value = (int)effective.Height;
        _fullscreenBox.IsChecked = effective.Fullscreen;
        _processPriorityBox.SelectedItem = effective.ProcessPriority;
        _launcherVisibilityBox.SelectedItem = effective.LauncherVisibility;
        _graphicsBackendBox.SelectedItem = effective.GraphicsBackend;
        _wrapperBox.Text = effective.Wrapper;
        _preCommandBox.Text = effective.PreCommand;
        _postCommandBox.Text = effective.PostCommand;
        _showLogsBox.IsChecked = effective.ShowLogs;
        _noJvmOptionsBox.IsChecked = effective.NoJvmOptions;
        _noOptimizingJvmOptionsBox.IsChecked = effective.NoOptimizingJvmOptions;
        _notCheckJvmBox.IsChecked = effective.NotCheckJvm;
        _notCheckGameBox.IsChecked = effective.NotCheckGame;
        _permSizeBox.Value = effective.PermSize ?? 0;
        _useCustomNativesBox.IsChecked = effective.UseCustomNatives;
        _nativesDirectoryBox.Text = effective.NativesDirectory;
    }

    private void UpdateMemoryControls()
    {
        var manual = _autoMemoryBox.IsChecked == false;
        _minMemoryBox.IsEnabled = manual;
        _maxMemoryBox.IsEnabled = manual;
    }

    /// <summary>
    /// Marks the property as overridden when its value differs from the one of
    /// the preset the instance inherits from.
    /// </summary>
    /// <typeparam name="T">The type of the value of the property.</typeparam>
    /// <param name="propertyName">The name of the property.</param>
    /// <param name="value">The value chosen by the user.</param>
    /// <param name="inherited">
    /// The value of the property in the preset, or null when the instance
    /// inherits from no preset.
    /// </param>
    private void ApplyOverride<T>(string propertyName, T value, T? inherited)
    {
        // The environment variables are compared by their content, as they are
        // stored in a dictionary.
        if (value is IReadOnlyDictionary<string, string> variables &&
            inherited is IReadOnlyDictionary<string, string> inheritedVariables)
        {
            var same = variables.Count == inheritedVariables.Count &&
                       variables.All(pair => inheritedVariables.TryGetValue(pair.Key, out var inheritedValue) &&
                                             inheritedValue == pair.Value);

            if (same)
                _settings.OverrideProperties.Remove(propertyName);
            else
                _settings.Override(propertyName);

            return;
        }

        if (inherited is { } inheritedValue && EqualityComparer<T>.Default.Equals(value, inheritedValue))
            _settings.OverrideProperties.Remove(propertyName);
        else
            _settings.Override(propertyName);
    }

    private async Task SaveAsync()
    {
        try
        {
            _settings.JvmOptions = _jvmOptionsBox.Text ?? string.Empty;
            _settings.AutoMemory = _autoMemoryBox.IsChecked ?? true;
            _settings.MinMemory = _minMemoryBox.Value > 0 ? (int?)_minMemoryBox.Value : null;
            _settings.MaxMemory = _maxMemoryBox.Value > 0 ? (int?)_maxMemoryBox.Value : null;
            _settings.GameArguments = _gameArgsBox.Text ?? string.Empty;
            _settings.RunningDirectory = _runningDirBox.Text ?? string.Empty;
            _settings.QuickPlayMode = _quickPlayModeBox.SelectedItem is QuickPlayMode mode ? mode : QuickPlayMode.None;
            _settings.QuickPlayArgument = _quickPlayArgBox.Text ?? string.Empty;
            _settings.EnvironmentVariables = EnvironmentVariableText.Parse(_envVarsBox.Text);
            _settings.PresetId = (_presetBox.SelectedItem as GameSettingsPreset)?.Id ?? string.Empty;

            _settings.JavaType = _javaTypeBox.SelectedItem is JavaVersionType javaType
                ? javaType
                : JavaVersionType.Auto;
            _settings.CustomJavaVersion = _customJavaVersionBox.Text ?? string.Empty;
            _settings.CustomJavaPath = _customJavaPathBox.Text ?? string.Empty;
            _settings.Width = (double)(_widthBox.Value ?? 854);
            _settings.Height = (double)(_heightBox.Value ?? 480);
            _settings.Fullscreen = _fullscreenBox.IsChecked ?? false;
            _settings.ProcessPriority = _processPriorityBox.SelectedItem is HMCLCore.NET.Setting.ProcessPriority priority
                ? priority
                : HMCLCore.NET.Setting.ProcessPriority.Normal;
            _settings.LauncherVisibility = _launcherVisibilityBox.SelectedItem is LauncherVisibility visibility
                ? visibility
                : LauncherVisibility.Hide;
            _settings.GraphicsBackend = _graphicsBackendBox.SelectedItem is GraphicsBackend backend
                ? backend
                : GraphicsBackend.Auto;
            _settings.Wrapper = _wrapperBox.Text ?? string.Empty;
            _settings.PreCommand = _preCommandBox.Text ?? string.Empty;
            _settings.PostCommand = _postCommandBox.Text ?? string.Empty;
            _settings.ShowLogs = _showLogsBox.IsChecked ?? true;
            _settings.NoJvmOptions = _noJvmOptionsBox.IsChecked ?? false;
            _settings.NoOptimizingJvmOptions = _noOptimizingJvmOptionsBox.IsChecked ?? false;
            _settings.NotCheckJvm = _notCheckJvmBox.IsChecked ?? false;
            _settings.NotCheckGame = _notCheckGameBox.IsChecked ?? false;
            _settings.PermSize = _permSizeBox.Value > 0 ? (int?)_permSizeBox.Value : null;
            _settings.UseCustomNatives = _useCustomNativesBox.IsChecked ?? false;
            _settings.NativesDirectory = _nativesDirectoryBox.Text ?? string.Empty;

            // Only the values which differ from the ones of the preset are
            // overridden, so that the instance keeps inheriting the other
            // settings of the preset.
            var preset = (_presetBox.SelectedItem as GameSettingsPreset);
            ApplyOverride(GameSettings.PropertyJvmOptions, _settings.JvmOptions, preset?.JvmOptions);
            ApplyOverride(GameSettings.PropertyAutoMemory, _settings.AutoMemory, preset?.AutoMemory);
            ApplyOverride(GameSettings.PropertyMinMemory, _settings.MinMemory, preset?.MinMemory);
            ApplyOverride(GameSettings.PropertyMaxMemory, _settings.MaxMemory, preset?.MaxMemory);
            ApplyOverride(GameSettings.PropertyGameArguments, _settings.GameArguments, preset?.GameArguments);
            ApplyOverride(GameSettings.PropertyRunningDirectory, _settings.RunningDirectory, preset?.RunningDirectory);
            ApplyOverride(GameSettings.PropertyQuickPlayMode, _settings.QuickPlayMode, preset?.QuickPlayMode);
            ApplyOverride(GameSettings.PropertyQuickPlayArgument, _settings.QuickPlayArgument, preset?.QuickPlayArgument);
            ApplyOverride(GameSettings.PropertyEnvironmentVariables, _settings.EnvironmentVariables, preset?.EnvironmentVariables);
            ApplyOverride(GameSettings.PropertyJavaType, _settings.JavaType, preset?.JavaType);
            ApplyOverride(GameSettings.PropertyCustomJavaVersion, _settings.CustomJavaVersion, preset?.CustomJavaVersion);
            ApplyOverride(GameSettings.PropertyCustomJavaPath, _settings.CustomJavaPath, preset?.CustomJavaPath);
            ApplyOverride(GameSettings.PropertyWidth, _settings.Width, preset?.Width);
            ApplyOverride(GameSettings.PropertyHeight, _settings.Height, preset?.Height);
            ApplyOverride(GameSettings.PropertyFullscreen, _settings.Fullscreen, preset?.Fullscreen);
            ApplyOverride(GameSettings.PropertyProcessPriority, _settings.ProcessPriority, preset?.ProcessPriority);
            ApplyOverride(GameSettings.PropertyLauncherVisibility, _settings.LauncherVisibility, preset?.LauncherVisibility);
            ApplyOverride(GameSettings.PropertyGraphicsBackend, _settings.GraphicsBackend, preset?.GraphicsBackend);
            ApplyOverride(GameSettings.PropertyWrapper, _settings.Wrapper, preset?.Wrapper);
            ApplyOverride(GameSettings.PropertyPreCommand, _settings.PreCommand, preset?.PreCommand);
            ApplyOverride(GameSettings.PropertyPostCommand, _settings.PostCommand, preset?.PostCommand);
            ApplyOverride(GameSettings.PropertyShowLogs, _settings.ShowLogs, preset?.ShowLogs);
            ApplyOverride(GameSettings.PropertyNoJvmOptions, _settings.NoJvmOptions, preset?.NoJvmOptions);
            ApplyOverride(GameSettings.PropertyNoOptimizingJvmOptions, _settings.NoOptimizingJvmOptions, preset?.NoOptimizingJvmOptions);
            ApplyOverride(GameSettings.PropertyNotCheckJvm, _settings.NotCheckJvm, preset?.NotCheckJvm);
            ApplyOverride(GameSettings.PropertyNotCheckGame, _settings.NotCheckGame, preset?.NotCheckGame);
            ApplyOverride(GameSettings.PropertyPermSize, _settings.PermSize, preset?.PermSize);
            ApplyOverride(GameSettings.PropertyUseCustomNatives, _settings.UseCustomNatives, preset?.UseCustomNatives);
            ApplyOverride(GameSettings.PropertyNativesDirectory, _settings.NativesDirectory, preset?.NativesDirectory);

            var manifest = new GameInstanceManifest
            {
                Version = _instance.Version,
                MainClass = "net.minecraft.client.main.Main",
                GameSettings = _settings
            };

            if (_viewModel.SaveInstanceManifest(_instance.Id, manifest))
            {
                _statusText.Text = Lang.Get("hmcl.gamesettingsdialog.954_");
                _statusText.Foreground = Brushes.Green;
            }
            else
            {
                _statusText.Text = Lang.Get("hmcl.gamesettingsdialog.956_");
                _statusText.Foreground = Brushes.Red;
            }
        }
        catch (Exception ex)
        {
            _statusText.Text = $"保存出错：{ex.Message}";
            _statusText.Foreground = Brushes.Red;
        }
    }
}
