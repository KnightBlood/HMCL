using System;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Platform.Storage;
using HMCLCore.NET.Game;
using HMCL.NET.Util;
using HMCL.NET.Views.Dialogs;

namespace HMCL.NET.Views;

/// <summary>
/// The dialog showing the information about a world, like the world information
/// page of Java HMCL.
/// </summary>
public sealed class WorldInfoDialog : Window
{
    /// <summary>
    /// The directory of the world shown by the dialog.
    /// </summary>
    private readonly string _worldDirectory;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="worldDirectory">The directory of the world.</param>
    /// <param name="fallbackName">The name of the world, used when it cannot be read.</param>
    public WorldInfoDialog(string worldDirectory, string fallbackName)
    {
        _worldDirectory = worldDirectory;

        Title = Lang.Get("world.info", "世界信息");
        Width = 460;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var details = WorldInfoReader.Read(worldDirectory);

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 8 };
        root.Children.Add(new TextBlock
        {
            Text = details?.Name ?? fallbackName,
            FontSize = 16,
            FontWeight = FontWeight.Bold
        });

        if (details is null)
        {
            root.Children.Add(new TextBlock
            {
                Text = Lang.Get("world.info.failed", "读取世界信息失败"),
                Foreground = new SolidColorBrush(Color.Parse("#C62828"))
            });
        }
        else
        {
            root.Children.Add(CreateIconSection());

            root.Children.Add(CreateSection(Lang.Get("world.info.basic", "基本信息")));
            root.Children.Add(CreateRow(Lang.Get("world.game_version", "游戏版本"), details.GameVersion));
            root.Children.Add(CreateRow(Lang.Get("world.info.last_played", "最后游玩"),
                details.LastPlayed?.ToString("yyyy-MM-dd HH:mm", CultureInfo.CurrentCulture) ?? "-"));
            root.Children.Add(CreateRow(Lang.Get("world.info.random_seed", "种子"),
                details.Seed?.ToString(CultureInfo.CurrentCulture) ?? "-"));
            root.Children.Add(CreateRow(Lang.Get("world.info.time", "游戏时间"), FormatTime(details.Time)));
            root.Children.Add(CreateDifficultySection(details.Difficulty));
            root.Children.Add(CreateRow(Lang.Get("world.info.allow_cheats", "允许命令/作弊"),
                details.AllowCheats ? Lang.Get("common.yes", "是") : Lang.Get("common.no", "否")));
            root.Children.Add(CreateRow(Lang.Get("world.info.generate_features", "生成结构"),
                details.GenerateStructures ? Lang.Get("common.yes", "是") : Lang.Get("common.no", "否")));
            root.Children.Add(CreateRow(Lang.Get("world.info.spawn", "世界出生点"), FormatPosition(details.SpawnPosition)));

            if (details.Player is { } player)
            {
                root.Children.Add(CreateSection(Lang.Get("world.info.player", "玩家信息")));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.game_type", "游戏模式"),
                    Lang.Get("world.info.player.game_type." + player.GameMode, player.GameMode)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.health", "生命值"),
                    player.Health.ToString("F1", CultureInfo.CurrentCulture)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.food_level", "饥饿值"),
                    player.FoodLevel.ToString(CultureInfo.CurrentCulture)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.food_saturation_level", "饱和度"),
                    player.Saturation.ToString("F2", CultureInfo.CurrentCulture)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.xp_level", "经验等级"),
                    player.ExperienceLevel.ToString(CultureInfo.CurrentCulture)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.location", "位置"),
                    FormatPosition(player.Position)));
                root.Children.Add(CreateRow(Lang.Get("world.info.player.spawn", "出生点"),
                    FormatPosition(player.SpawnPosition)));

                if (player.LastDeathPosition.Count > 0)
                {
                    root.Children.Add(CreateRow(Lang.Get("world.info.player.last_death_location", "最后死亡位置"),
                        FormatPosition(player.LastDeathPosition)));
                }
            }
        }

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            HorizontalAlignment = HorizontalAlignment.Right
        };

        closeButton.Click += (_, _) => Close(true);
        root.Children.Add(closeButton);

        Content = new ScrollViewer { Content = root };

        // The escape key closes the dialog.
        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Creates the section showing the icon of the world and the buttons
    /// changing it.
    /// </summary>
    /// <returns>The created section.</returns>
    private Control CreateIconSection()
    {
        var iconPath = Path.Combine(_worldDirectory, WorldEditor.IconFileName);
        var preview = new Image
        {
            Width = 64,
            Height = 64,
            Stretch = Stretch.Uniform,
            HorizontalAlignment = HorizontalAlignment.Left
        };

        if (File.Exists(iconPath))
        {
            try
            {
                preview.Source = new Avalonia.Media.Imaging.Bitmap(iconPath);
            }
            catch (Exception)
            {
                // The icon of the world cannot be shown.
            }
        }

        var changeButton = new Button
        {
            Content = Lang.Get("world.icon.change", "修改世界图标"),
            Padding = new Thickness(12, 6)
        };

        var clearButton = new Button
        {
            Content = Lang.Get("world.icon.clear", "清除图标"),
            Padding = new Thickness(12, 6)
        };

        changeButton.Click += async (_, _) => await ChangeIconAsync(preview, iconPath);
        clearButton.Click += (_, _) =>
        {
            WorldEditor.ClearIcon(_worldDirectory);
            preview.Source = null;
        };

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            Children = { changeButton, clearButton }
        };

        var layout = new StackPanel { Spacing = 6 };
        layout.Children.Add(preview);
        layout.Children.Add(buttons);
        layout.Children.Add(new TextBlock
        {
            Text = Lang.Get("world.icon.change.tip", "请提供一张 64×64 的 PNG 格式图片。"),
            FontSize = 11,
            Foreground = new SolidColorBrush(Color.Parse("#888888")),
            TextWrapping = TextWrapping.Wrap,
            MaxWidth = 380
        });

        var section = new StackPanel { Spacing = 4 };
        section.Children.Add(CreateSection(Lang.Get("world.icon", "世界图标")));
        section.Children.Add(layout);
        return section;
    }

    /// <summary>
    /// Creates the section showing the difficulty of the world, which can be
    /// changed and locked.
    /// </summary>
    /// <param name="currentDifficulty">The name of the current difficulty.</param>
    /// <returns>The created section.</returns>
    private Control CreateDifficultySection(string currentDifficulty)
    {
        var items = new List<DifficultyItem>
        {
            new(WorldDifficulty.Peaceful, "peaceful", Lang.Get("world.info.difficulty.peaceful", "和平")),
            new(WorldDifficulty.Easy, "easy", Lang.Get("world.info.difficulty.easy", "简单")),
            new(WorldDifficulty.Normal, "normal", Lang.Get("world.info.difficulty.normal", "普通")),
            new(WorldDifficulty.Hard, "hard", Lang.Get("world.info.difficulty.hard", "困难"))
        };

        var current = items.FirstOrDefault(item =>
            string.Equals(item.Key, currentDifficulty, StringComparison.OrdinalIgnoreCase))
            ?? items.FirstOrDefault(item => item.Difficulty == WorldDifficulty.Normal)
            ?? items[0];

        var difficultyBox = new ComboBox
        {
            ItemsSource = items,
            SelectedItem = current,
            Width = 160,
            HorizontalAlignment = HorizontalAlignment.Left
        };

        var locked = WorldEditor.IsDifficultyLocked(_worldDirectory) == true;
        var lockBox = new CheckBox
        {
            Content = Lang.Get("world.info.difficulty_lock", "锁定难度"),
            IsChecked = locked
        };

        difficultyBox.SelectionChanged += (_, _) =>
        {
            if (difficultyBox.SelectedItem is not DifficultyItem item)
                return;

            WorldEditor.SetDifficulty(_worldDirectory, item.Difficulty, lockBox.IsChecked == true);
        };

        lockBox.IsCheckedChanged += (_, _) =>
        {
            var selected = difficultyBox.SelectedItem as DifficultyItem;
            WorldEditor.SetDifficulty(_worldDirectory, selected?.Difficulty ?? WorldDifficulty.Normal,
                lockBox.IsChecked == true);
        };

        var content = new StackPanel { Spacing = 6 };
        content.Children.Add(difficultyBox);
        content.Children.Add(lockBox);

        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("160,*") };
        grid.Children.Add(new TextBlock
        {
            Text = Lang.Get("world.info.difficulty", "难度"),
            Foreground = new SolidColorBrush(Color.Parse("#666666")),
            VerticalAlignment = VerticalAlignment.Top
        });

        Grid.SetColumn(content, 1);
        grid.Children.Add(content);

        var section = new StackPanel { Spacing = 4 };
        section.Children.Add(CreateSection(Lang.Get("world.info.basic", "基本信息")));
        section.Children.Add(grid);
        return section;
    }

    /// <summary>
    /// Asks the user for an image and uses it as the icon of the world.
    /// </summary>
    /// <param name="preview">The image showing the icon of the world.</param>
    /// <param name="iconPath">The path of the icon of the world.</param>
    /// <returns>A task that completes when the icon is changed.</returns>
    private async Task ChangeIconAsync(Image preview, string iconPath)
    {
        var files = await StorageProvider.OpenFilePickerAsync(new FilePickerOpenOptions
        {
            Title = Lang.Get("world.icon.choose.title", "选择世界图标"),
            AllowMultiple = false,
            FileTypeFilter = new[]
            {
                new FilePickerFileType("PNG") { Patterns = new[] { "*.png" } },
                new FilePickerFileType(Lang.Get("hmcl.worldlistwindow.axaml.1302_", "所有文件")) { Patterns = new[] { "*" } }
            }
        });

        if (files.Count == 0 || files[0].Path.Scheme != "file")
            return;

        var path = files[0].Path.LocalPath;

        int width;
        int height;
        try
        {
            using var bitmap = new Avalonia.Media.Imaging.Bitmap(path);
            width = bitmap.PixelSize.Width;
            height = bitmap.PixelSize.Height;
        }
        catch (Exception)
        {
            await DialogService.ShowMessageAsync(
                this,
                Lang.Get("world.icon.change.fail.load.text", "该图片似乎已损坏，HMCL 无法解析。"),
                Lang.Get("world.icon.change.fail.load.title", "图片解析失败"));
            return;
        }

        // The game only shows the icons of 64 by 64 pixels.
        if (width != 64 || height != 64)
        {
            await DialogService.ShowMessageAsync(
                this,
                string.Format(
                    CultureInfo.CurrentCulture,
                    Lang.Get("world.icon.change.fail.not_64x64.text",
                        "该图片的分辨率为 %d×%d 而不是 64×64。请提供一张 64×64 的图片并重试。"),
                    width, height),
                Lang.Get("world.icon.change.fail.not_64x64.title", "图片大小错误"));
            return;
        }

        if (!WorldEditor.SetIcon(_worldDirectory, path))
            return;

        preview.Source = new Avalonia.Media.Imaging.Bitmap(iconPath);
    }

    /// <summary>
    /// Creates the title of a section of the dialog.
    /// </summary>
    /// <param name="title">The title of the section.</param>
    /// <returns>The created title.</returns>
    private static Control CreateSection(string title) => new TextBlock
    {
        Text = title,
        FontWeight = FontWeight.Bold,
        Margin = new Thickness(0, 8, 0, 0)
    };

    /// <summary>
    /// Creates a row showing a value of the world.
    /// </summary>
    /// <param name="name">The name of the value.</param>
    /// <param name="value">The value shown to the user.</param>
    /// <returns>The created row.</returns>
    private static Control CreateRow(string name, string value)
    {
        var grid = new Grid { ColumnDefinitions = new ColumnDefinitions("160,*") };

        grid.Children.Add(new TextBlock
        {
            Text = name,
            Foreground = new SolidColorBrush(Color.Parse("#666666")),
            VerticalAlignment = VerticalAlignment.Top
        });

        var valueText = new TextBlock
        {
            Text = value,
            TextWrapping = TextWrapping.Wrap
        };

        Grid.SetColumn(valueText, 1);
        grid.Children.Add(valueText);

        return grid;
    }

    /// <summary>
    /// Formats the position of a player or of a spawn point.
    /// </summary>
    /// <param name="position">The position to format.</param>
    /// <returns>The formatted position.</returns>
    private static string FormatPosition(IReadOnlyList<double> position)
        => position.Count < 3
            ? "-"
            : string.Join(", ", position.Take(3).Select(value =>
                value.ToString("F1", CultureInfo.CurrentCulture)));

    /// <summary>
    /// Formats the time the world was played.
    /// </summary>
    /// <param name="ticks">The time in ticks.</param>
    /// <returns>The formatted time.</returns>
    private static string FormatTime(long ticks)
    {
        var days = ticks / 24000;
        var hours = ticks % 24000 / 1000;
        var minutes = ticks % 1000 / 60;
        var seconds = ticks % 60;

        return string.Format(CultureInfo.CurrentCulture, "{0}d {1}h {2}m {3}s", days, hours, minutes, seconds);
    }

    /// <summary>
    /// A difficulty of a world shown in the list of the difficulties.
    /// </summary>
    private sealed class DifficultyItem
    {
        /// <summary>
        /// Creates the item.
        /// </summary>
        /// <param name="difficulty">The difficulty of the world.</param>
        /// <param name="key">The key of the difficulty used by the game.</param>
        /// <param name="name">The name of the difficulty shown to the user.</param>
        public DifficultyItem(WorldDifficulty difficulty, string key, string name)
        {
            Difficulty = difficulty;
            Key = key;
            Name = name;
        }

        /// <summary>
        /// The difficulty of the world.
        /// </summary>
        public WorldDifficulty Difficulty { get; }

        /// <summary>
        /// The key of the difficulty used by the game.
        /// </summary>
        public string Key { get; }

        /// <summary>
        /// The name of the difficulty shown to the user.
        /// </summary>
        public string Name { get; }

        /// <inheritdoc/>
        public override string ToString() => Name;
    }
}
