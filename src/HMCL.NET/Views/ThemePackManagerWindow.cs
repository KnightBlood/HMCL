using System;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using HMCLCore.NET.Setting;
using HMCL.NET.Util;
using HMCL.NET.ViewModels;
using HMCL.NET.Views.Dialogs;

namespace HMCL.NET.Views;

/// <summary>
/// The window managing the theme packs installed in the launcher, like the
/// theme pack management page of Java HMCL.
/// </summary>
public sealed class ThemePackManagerWindow : Window
{
    private readonly SettingsViewModel _viewModel;

    private readonly ListBox _packList = new() { MinWidth = 260 };
    private readonly ListBox _themeList = new() { MinWidth = 220 };
    private readonly TextBlock _statusText = new()
    {
        Foreground = Brushes.Gray,
        FontSize = 12,
        TextWrapping = TextWrapping.Wrap
    };

    /// <summary>
    /// Creates the window.
    /// </summary>
    /// <param name="viewModel">The view model owning the theme packs.</param>
    public ThemePackManagerWindow(SettingsViewModel viewModel)
    {
        _viewModel = viewModel ?? throw new ArgumentNullException(nameof(viewModel));

        Title = Lang.Get("theme_pack.manage", "主题包管理");
        Width = 640;
        Height = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _viewModel.LoadThemePacks();

        _packList.ItemsSource = _viewModel.ThemePacks;
        _packList.SelectionChanged += (_, _) => UpdateThemes();

        var importButton = new Button
        {
            Content = Lang.Get("theme_pack.import", "导入主题包"),
            Padding = new Thickness(12, 6)
        };
        importButton.Click += async (_, _) => await ImportAsync();

        var deleteButton = new Button
        {
            Content = Lang.Get("theme_pack.delete", "删除主题包"),
            Padding = new Thickness(12, 6)
        };
        deleteButton.Click += async (_, _) => await DeleteAsync();

        var applyButton = new Button
        {
            Content = Lang.Get("theme_pack.apply", "应用主题"),
            Padding = new Thickness(12, 6)
        };
        applyButton.Click += (_, _) => ApplyTheme();

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(12, 6)
        };
        closeButton.Click += (_, _) => Close();

        var lists = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto,*") };
        lists.Children.Add(_packList);
        Grid.SetColumn(_themeList, 2);
        lists.Children.Add(_themeList);

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };
        root.Children.Add(new TextBlock
        {
            Text = Lang.Get("theme_pack.manage.desc", "导入、删除与应用主题包。"),
            FontSize = 12,
            Foreground = Brushes.Gray,
            TextWrapping = TextWrapping.Wrap
        });
        root.Children.Add(lists);
        root.Children.Add(_statusText);

        var content = new Grid { RowDefinitions = new RowDefinitions("*,Auto") };
        content.Children.Add(root);

        var buttonRow = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Spacing = 8,
            Margin = new Thickness(16, 0, 16, 16),
            Children = { importButton, deleteButton, applyButton, closeButton }
        };
        Grid.SetRow(buttonRow, 1);
        content.Children.Add(buttonRow);

        Content = content;

        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Shows the themes of the selected theme pack.
    /// </summary>
    private void UpdateThemes()
    {
        _themeList.ItemsSource = (_packList.SelectedItem as InstalledThemePack)?.Manifest.Themes;
        _statusText.Text = string.Empty;
    }

    /// <summary>
    /// Asks the user for a theme pack and installs it.
    /// </summary>
    private async Task ImportAsync()
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("theme_pack.import.title", "导入主题包"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType(Lang.Get("theme_pack.file", "主题包"))
                {
                    Patterns = new[] { "*.zip", "*.hmcltheme" }
                },
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1302_", "所有文件")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        if (_viewModel.InstallThemePack(files[0].Path.LocalPath))
            _statusText.Text = Lang.Get("theme_pack.import.success", "主题包导入成功");
        else
            _statusText.Text = Lang.Get("theme_pack.import.failed", "主题包导入失败");
    }

    /// <summary>
    /// Asks the user to confirm and then removes the selected theme pack.
    /// </summary>
    private async Task DeleteAsync()
    {
        if (_packList.SelectedItem is not InstalledThemePack pack)
            return;

        if (pack.Builtin)
        {
            _statusText.Text = Lang.Get("theme_pack.delete.failed", "主题包删除失败");
            return;
        }

        var confirmed = await DialogService.ConfirmAsync(
            this,
            string.Format(
                Lang.Get("theme_pack.delete.confirm", "确定要删除主题包 {0} 吗？"),
                pack.Name),
            Lang.Get("theme_pack.delete", "删除主题包"));

        if (!confirmed)
            return;

        if (_viewModel.DeleteThemePack(pack))
            _statusText.Text = Lang.Get("theme_pack.delete.success", "主题包删除成功");
        else
            _statusText.Text = Lang.Get("theme_pack.delete.failed", "主题包删除失败");
    }

    /// <summary>
    /// Applies the theme selected in the list of the themes.
    /// </summary>
    private void ApplyTheme()
    {
        if (_packList.SelectedItem is not InstalledThemePack pack ||
            _themeList.SelectedItem is not ThemePackTheme theme)
        {
            return;
        }

        _viewModel.ApplyThemePackTheme(pack, theme);
        _statusText.Text = Lang.Get("theme_pack.apply.success", "主题已应用");
    }
}
