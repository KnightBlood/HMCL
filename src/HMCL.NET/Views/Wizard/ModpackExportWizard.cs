using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Game;
using HMCLCore.NET.Modpack;
using HMCL.NET.Util;

namespace HMCL.NET.Views.Wizard;

/// <summary>
/// The state shared by the steps of the modpack export wizard.
/// </summary>
public sealed class ModpackExportContext
{
    /// <summary>
    /// Creates the context for the given instance.
    /// </summary>
    /// <param name="instance">The instance being exported.</param>
    /// <param name="defaultPath">The default path of the exported archive.</param>
    public ModpackExportContext(GameInstance instance, string defaultPath)
    {
        Instance = instance ?? throw new ArgumentNullException(nameof(instance));
        ExportPath = defaultPath;

        Info = new ModpackExportInfo
        {
            Name = instance.GetDisplayName(),
            Version = "1.0",
            Author = Environment.UserName,
            Description = string.Empty
        };
    }

    /// <summary>
    /// The instance being exported.
    /// </summary>
    public GameInstance Instance { get; }

    /// <summary>
    /// The information written into the manifest of the exported pack.
    /// </summary>
    public ModpackExportInfo Info { get; }

    /// <summary>
    /// The type of the exported pack.
    /// </summary>
    public ModpackExportType Type { get; set; } = ModpackExportType.Modrinth;

    /// <summary>
    /// The path of the exported archive.
    /// </summary>
    public string ExportPath { get; set; }

    /// <summary>
    /// The names of the entries of the instance copied into the exported pack.
    /// </summary>
    public HashSet<string> IncludedEntries { get; } = new(StringComparer.OrdinalIgnoreCase);

    /// <summary>
    /// Whether the optional (disabled) mods are copied into the exported pack.
    /// </summary>
    public bool IncludeOptional { get; set; }

    /// <summary>
    /// Builds the options of the export from the state of the wizard.
    /// </summary>
    /// <returns>The options of the export.</returns>
    public ModpackExportOptions BuildOptions() => new()
    {
        Info = Info,
        Type = Type,
        IncludedEntries = IncludedEntries.ToList(),
        IncludeOptional = IncludeOptional
    };
}

/// <summary>
/// The first step of the wizard: choosing the type of the exported pack.
/// </summary>
public sealed class ModpackTypeSelectionStep : IWizardStep
{
    private readonly ModpackExportContext _context;
    private readonly RadioButton _mcbbsButton;
    private readonly RadioButton _modrinthButton;
    private readonly RadioButton _curseForgeButton;
    private readonly RadioButton _serverButton;

    /// <summary>
    /// Creates the type selection step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public ModpackTypeSelectionStep(ModpackExportContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _mcbbsButton = CreateButton(Lang.Get("hmcl.modpackexportwizard.1196_"), Lang.Get("hmcl.modpackexportwizard.1198_"), ModpackExportType.Mcbbs);
        _modrinthButton = CreateButton(Lang.Get("hmcl.modpackexportwizard.1200_"), Lang.Get("hmcl.modpackexportwizard.1202_"), ModpackExportType.Modrinth);
        _curseForgeButton = CreateButton(Lang.Get("hmcl.modpackexportwizard.1204_"), Lang.Get("hmcl.modpackexportwizard.1206_"), ModpackExportType.CurseForge);
        _serverButton = CreateButton(Lang.Get("hmcl.modpackexportwizard.1208_"), Lang.Get("hmcl.modpackexportwizard.1210_"), ModpackExportType.Server);

        var panel = new StackPanel { Spacing = 10 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.modpackexportwizard.1212_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });
        panel.Children.Add(_mcbbsButton);
        panel.Children.Add(_modrinthButton);
        panel.Children.Add(_curseForgeButton);
        panel.Children.Add(_serverButton);

        Content = panel;
    }

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.modpackexportwizard.1214_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// Whether a type is selected.
    /// </summary>
    public bool CanGoNext => true;

    /// <summary>
    /// Does nothing when entering the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnEnterAsync() => Task.CompletedTask;

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Creates a radio button choosing a type of pack.
    /// </summary>
    /// <param name="title">The title of the type.</param>
    /// <param name="description">The description of the type.</param>
    /// <param name="type">The type chosen by the button.</param>
    /// <returns>The created button.</returns>
    private RadioButton CreateButton(string title, string description, ModpackExportType type)
    {
        var button = new RadioButton
        {
            GroupName = "ModpackExportType",
            IsChecked = _context.Type == type,
            Padding = new Thickness(4, 6)
        };

        var content = new StackPanel { Spacing = 2 };
        content.Children.Add(new TextBlock { Text = title, FontWeight = FontWeight.Bold });
        content.Children.Add(new TextBlock
        {
            Text = description,
            Foreground = new SolidColorBrush(Color.Parse("#666")),
            FontSize = 12
        });

        button.Content = content;
        button.IsCheckedChanged += (_, _) =>
        {
            if (button.IsChecked == true)
                _context.Type = type;
        };

        return button;
    }
}

/// <summary>
/// The second step of the wizard: filling the information shown by the
/// manifest of the exported pack.
/// </summary>
public sealed class ModpackInfoStep : IWizardStep, IWizardStepNotifier
{
    private readonly ModpackExportContext _context;
    private readonly TextBox _nameBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.modpack_name") };
    private readonly TextBox _versionBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1218_") };
    private readonly TextBox _authorBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.archive_author") };
    private readonly TextBox _urlBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1222_") };
    private readonly TextBox _fileApiBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1224_") };
    private readonly TextBox _descriptionBox = new()
    {
        PlaceholderText = Lang.Get("hmcl.modpackexportwizard.modpack_description"),
        AcceptsReturn = true,
        Height = 90,
        TextWrapping = Avalonia.Media.TextWrapping.Wrap
    };
    private readonly NumericUpDown _minMemoryBox = new()
    {
        Minimum = 0,
        Maximum = 65536,
        Increment = 256,
        FormatString = "0"
    };
    private readonly TextBox _launchArgumentsBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1228_") };
    private readonly TextBox _javaArgumentsBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1230_") };
    private readonly CheckBox _forceUpdateBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.1232_") };
    private readonly CheckBox _packWithLauncherBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.1234_") };
    private readonly CheckBox _noCreateRemoteFilesBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.1236_") };
    private readonly CheckBox _skipCurseForgeRemoteFilesBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.1238_") };

    /// <summary>
    /// Creates the information step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public ModpackInfoStep(ModpackExportContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _nameBox.Text = _context.Info.Name;
        _versionBox.Text = _context.Info.Version;
        _authorBox.Text = _context.Info.Author;
        _descriptionBox.Text = _context.Info.Description;
        _minMemoryBox.Value = _context.Info.MinMemory;

        _nameBox.TextChanged += (_, _) => { _context.Info.Name = _nameBox.Text ?? string.Empty; RaiseCanGoNextChanged(); };
        _versionBox.TextChanged += (_, _) => { _context.Info.Version = _versionBox.Text ?? string.Empty; RaiseCanGoNextChanged(); };
        _authorBox.TextChanged += (_, _) => _context.Info.Author = _authorBox.Text ?? string.Empty;
        _urlBox.TextChanged += (_, _) => _context.Info.Url = _urlBox.Text ?? string.Empty;
        _fileApiBox.TextChanged += (_, _) => _context.Info.FileApi = _fileApiBox.Text ?? string.Empty;
        _descriptionBox.TextChanged += (_, _) => _context.Info.Description = _descriptionBox.Text ?? string.Empty;
        _minMemoryBox.ValueChanged += (_, _) => _context.Info.MinMemory = (int)(_minMemoryBox.Value ?? 0);
        _launchArgumentsBox.TextChanged += (_, _) => _context.Info.LaunchArguments = _launchArgumentsBox.Text ?? string.Empty;
        _javaArgumentsBox.TextChanged += (_, _) => _context.Info.JavaArguments = _javaArgumentsBox.Text ?? string.Empty;
        _forceUpdateBox.IsCheckedChanged += (_, _) => _context.Info.ForceUpdate = _forceUpdateBox.IsChecked == true;
        _packWithLauncherBox.IsCheckedChanged += (_, _) => _context.Info.PackWithLauncher = _packWithLauncherBox.IsChecked == true;
        _noCreateRemoteFilesBox.IsCheckedChanged += (_, _) => _context.Info.NoCreateRemoteFiles = _noCreateRemoteFilesBox.IsChecked == true;
        _skipCurseForgeRemoteFilesBox.IsCheckedChanged += (_, _) => _context.Info.SkipCurseForgeRemoteFiles = _skipCurseForgeRemoteFilesBox.IsChecked == true;

        var panel = new StackPanel { Spacing = 10 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.modpackexportwizard.1240_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.mods_name"), _nameBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.archive_version"), _versionBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.archive_author_"), _authorBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.schematics_info_description"), _descriptionBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.1250_"), _urlBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.1252_"), _fileApiBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.1254_"), _minMemoryBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.1256_"), _launchArgumentsBox));
        panel.Children.Add(CreateField(Lang.Get("hmcl.modpackexportwizard.1258_"), _javaArgumentsBox));
        panel.Children.Add(_forceUpdateBox);
        panel.Children.Add(_packWithLauncherBox);
        panel.Children.Add(_noCreateRemoteFilesBox);
        panel.Children.Add(_skipCurseForgeRemoteFilesBox);

        var scroll = new ScrollViewer
        {
            Content = panel,
            HorizontalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Disabled
        };

        Content = scroll;
    }

    /// <summary>
    /// Raised when the ability to continue changed.
    /// </summary>
    public event EventHandler? CanGoNextChanged;

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.modpackexportwizard.1260_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// Whether the name and the version of the pack are filled in.
    /// </summary>
    public bool CanGoNext =>
        !string.IsNullOrWhiteSpace(_context.Info.Name) &&
        !string.IsNullOrWhiteSpace(_context.Info.Version);

    /// <summary>
    /// Does nothing when entering the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnEnterAsync() => Task.CompletedTask;

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Notifies the wizard that the ability to continue changed.
    /// </summary>
    private void RaiseCanGoNextChanged() => CanGoNextChanged?.Invoke(this, EventArgs.Empty);

    /// <summary>
    /// Creates a labelled field.
    /// </summary>
    /// <param name="title">The title of the field.</param>
    /// <param name="editor">The editor of the field.</param>
    /// <returns>The created field.</returns>
    private static Control CreateField(string title, Control editor)
    {
        var panel = new StackPanel { Spacing = 4 };
        panel.Children.Add(new TextBlock { Text = title, FontWeight = FontWeight.SemiBold });
        panel.Children.Add(editor);
        return panel;
    }
}

/// <summary>
/// The third step of the wizard: choosing the files of the instance that are
/// copied into the exported pack and the destination of the archive.
/// </summary>
public sealed class ModpackFileSelectionStep : IWizardStep, IWizardStepNotifier, IWizardSummaryProvider
{
    private readonly ModpackExportContext _context;
    private readonly TextBox _pathBox = new() { PlaceholderText = Lang.Get("hmcl.modpackexportwizard.1262_") };
    private readonly CheckBox _includeOptionalBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.1264_") };
    private readonly StackPanel _entriesPanel = new() { Spacing = 2 };
    private readonly TextBlock _statusText = new() { Foreground = new SolidColorBrush(Color.Parse("#888")) };
    private readonly CheckBox _selectAllBox = new() { Content = Lang.Get("hmcl.modpackexportwizard.menu_selectall") };
    private bool _loaded;

    /// <summary>
    /// Creates the file selection step.
    /// </summary>
    /// <param name="context">The shared wizard state.</param>
    public ModpackFileSelectionStep(ModpackExportContext context)
    {
        _context = context ?? throw new ArgumentNullException(nameof(context));

        _pathBox.Text = _context.ExportPath;
        _pathBox.TextChanged += (_, _) =>
        {
            _context.ExportPath = _pathBox.Text ?? string.Empty;
            RaiseCanGoNextChanged();
        };

        _includeOptionalBox.IsCheckedChanged += (_, _) =>
        {
            _context.IncludeOptional = _includeOptionalBox.IsChecked == true;
        };

        _selectAllBox.IsCheckedChanged += (_, _) => ApplySelectAll();

        var browseButton = new Button { Content = Lang.Get("hmcl.modpackexportwizard.1268_"), Padding = new Thickness(12, 6) };
        browseButton.Click += async (_, _) => await BrowseAsync();

        var pathPanel = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto") };
        pathPanel.Children.Add(_pathBox);
        var browseBorder = new Border { Margin = new Thickness(8, 0, 0, 0) };
        Grid.SetColumn(browseBorder, 1);
        browseBorder.Child = browseButton;
        pathPanel.Children.Add(browseBorder);

        var panel = new StackPanel { Spacing = 10 };
        panel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.modpackexportwizard.1270_"),
            Foreground = new SolidColorBrush(Color.Parse("#666"))
        });
        panel.Children.Add(new TextBlock { Text = Lang.Get("hmcl.modpackexportwizard.1272_"), FontWeight = FontWeight.SemiBold });
        panel.Children.Add(pathPanel);
        panel.Children.Add(_includeOptionalBox);
        panel.Children.Add(new TextBlock { Text = Lang.Get("hmcl.modpackexportwizard.1274_"), FontWeight = FontWeight.SemiBold });
        panel.Children.Add(_selectAllBox);
        panel.Children.Add(_entriesPanel);
        panel.Children.Add(_statusText);

        var scroll = new ScrollViewer
        {
            Content = panel,
            HorizontalScrollBarVisibility = Avalonia.Controls.Primitives.ScrollBarVisibility.Disabled
        };

        Content = scroll;
    }

    /// <summary>
    /// Raised when the ability to continue changed.
    /// </summary>
    public event EventHandler? CanGoNextChanged;

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => Lang.Get("hmcl.modpackexportwizard.1276_");

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// Whether a destination path is chosen.
    /// </summary>
    public bool CanGoNext => !string.IsNullOrWhiteSpace(_context.ExportPath);

    /// <summary>
    /// Shows the summary of the export, so that the user confirms the choices
    /// made before the pack is written.
    /// </summary>
    /// <returns>The summary of the export.</returns>
    public WizardSummary GetSummary() => new(
        Lang.Get("hmcl.modpackexportwizard.1280_", "导出确认"),
        new[]
        {
            (Lang.Get("hmcl.modpackexportwizard.1204_", "类型"), _context.Type.ToString()),
            (Lang.Get("hmcl.modpackexportwizard.1206_", "名称"), _context.Info.Name),
            (Lang.Get("hmcl.modpackexportwizard.1208_", "版本"), _context.Info.Version ?? string.Empty),
            (Lang.Get("hmcl.modpackexportwizard.1210_", "作者"), _context.Info.Author ?? string.Empty),
            (Lang.Get("hmcl.modpackexportwizard.1212_", "说明"), _context.Info.Description ?? string.Empty),
            (Lang.Get("hmcl.modpackexportwizard.1262_", "导出路径"), _context.ExportPath),
            (Lang.Get("hmcl.modpackexportwizard.1264_", "包含可选文件"),
                _context.IncludeOptional ? Lang.Get("common.yes", "是") : Lang.Get("common.no", "否"))
        });

    /// <summary>
    /// Lists the files of the instance when the step becomes visible.
    /// </summary>
    /// <returns>A task that completes when the files are listed.</returns>
    public Task OnEnterAsync()
    {
        if (!_loaded)
        {
            LoadEntries();
            _loaded = true;
        }

        return Task.CompletedTask;
    }

    /// <summary>
    /// Does nothing when leaving the step.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnLeaveAsync() => Task.CompletedTask;

    /// <summary>
    /// Notifies the wizard that the ability to continue changed.
    /// </summary>
    private void RaiseCanGoNextChanged() => CanGoNextChanged?.Invoke(this, EventArgs.Empty);

    /// <summary>
    /// Lists the entries of the directory of the instance that may be copied
    /// into the exported pack.
    /// </summary>
    private void LoadEntries()
    {
        _entriesPanel.Children.Clear();

        var instanceDir = _context.Instance.GameDirectory;
        if (string.IsNullOrEmpty(instanceDir) || !Directory.Exists(instanceDir))
        {
            _statusText.Text = Lang.Get("hmcl.modpackexportwizard.1278_");
            return;
        }

        var entries = Directory.GetFileSystemEntries(instanceDir)
            .Select(Path.GetFileName)
            .Where(name => !string.IsNullOrEmpty(name))
            .Select(name => name!)
            .Where(name => !ModpackExporter.ExcludedEntries.Contains(name, StringComparer.OrdinalIgnoreCase))
            .OrderBy(name => name, StringComparer.OrdinalIgnoreCase)
            .ToList();

        foreach (var entry in entries)
        {
            var isDirectory = Directory.Exists(Path.Combine(instanceDir, entry));
            var suggested = ModpackExporter.SuggestedEntries.Contains(entry, StringComparer.OrdinalIgnoreCase);

            var checkBox = new CheckBox
            {
                Content = isDirectory ? $"{entry}/" : entry,
                IsChecked = suggested
            };

            if (suggested)
                _context.IncludedEntries.Add(entry);

            checkBox.IsCheckedChanged += (_, _) =>
            {
                if (checkBox.IsChecked == true)
                    _context.IncludedEntries.Add(entry);
                else
                    _context.IncludedEntries.Remove(entry);
            };

            _entriesPanel.Children.Add(checkBox);
        }

        _statusText.Text = $"共 {entries.Count} 个可导出的项目，已选择 {_context.IncludedEntries.Count} 个。";
    }

    /// <summary>
    /// Checks or unchecks every entry of the list.
    /// </summary>
    private void ApplySelectAll()
    {
        var selectAll = _selectAllBox.IsChecked == true;

        foreach (var child in _entriesPanel.Children.OfType<CheckBox>())
            child.IsChecked = selectAll;
    }

    /// <summary>
    /// Asks the user for the path of the exported archive.
    /// </summary>
    /// <returns>A task that completes when the dialog is closed.</returns>
    private async Task BrowseAsync()
    {
        if (TopLevel.GetTopLevel(Content) is not { } topLevel)
            return;

        var fileType = new Avalonia.Platform.Storage.FilePickerFileType(Lang.Get("hmcl.modpackexportwizard.modpack"))
        {
            Patterns = new[] { "*.zip", "*.mrpack" }
        };

        var file = await topLevel.StorageProvider.SaveFilePickerAsync(new Avalonia.Platform.Storage.FilePickerSaveOptions
        {
            Title = Lang.Get("hmcl.modpackexportwizard.1282_"),
            SuggestedFileName = Path.GetFileName(_context.ExportPath),
            FileTypeChoices = new[] { fileType }
        });

        if (file == null || file.Path.Scheme != "file")
            return;

        _pathBox.Text = file.Path.LocalPath;
    }
}
