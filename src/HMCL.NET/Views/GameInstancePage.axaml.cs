using Avalonia;
using Avalonia.Controls;
using HMCL.NET.ViewModels;
using HMCL.NET.Views;
using System.IO;

namespace HMCL.NET.Views;

/// <summary>
/// Instance detail page showing tabs for settings, installers, mods, resource packs,
/// data packs, shaders, screenshots and folders. Mirrors the Java HMCL instance page.
/// Data context is <see cref="MainWindowViewModel"/>.
/// </summary>
public partial class GameInstancePage : UserControl
{
    /// <summary>
    /// Creates a new GameInstancePage.
    /// </summary>
    public GameInstancePage()
    {
        InitializeComponent();

        OpenGameSettingsButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel vm && vm.SelectedInstance is { } instance)
            {
                var dialog = new GameSettingsDialog(instance, vm);
                if (TopLevel.GetTopLevel(this) is Window owner)
                    await dialog.ShowDialog(owner);
            }
        };

        ExportModpackButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel vm)
            {
                await vm.ExportModpackAsync();
            }
        };

        OpenModsButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel vm && vm.SelectedInstance is { } instance)
            {
                var dialog = new ModsDialog(instance, vm);
                if (TopLevel.GetTopLevel(this) is Window owner)
                    await dialog.ShowDialog(owner);
            }
        };

        OpenWorldsButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel vm && vm.SelectedInstance is { } instance)
            {
                var dialog = new WorldListWindow(instance, vm);
                if (TopLevel.GetTopLevel(this) is Window owner)
                    await dialog.ShowDialog(owner);
            }
        };

        OpenNbtEditorButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel vm && vm.SelectedInstance is { } instance)
            {
                var levelDat = Path.Combine(instance.GameDirectory, "level.dat");
                var dialog = new NbtEditorDialog(levelDat);
                if (TopLevel.GetTopLevel(this) is Window owner)
                    await dialog.ShowDialog(owner);
            }
        };

        UpdateVersionButton!.Click += async (_, _) =>
        {
            if (DataContext is not MainWindowViewModel viewModel || viewModel.SelectedInstance is not { } instance)
                return;

            // The list of the versions is fetched when it is still empty.
            if (viewModel.Download.Versions.Count == 0)
                await viewModel.Download.RefreshVersionsAsync();

            var dialog = new UpdateInstanceDialog(viewModel.Download.Versions, instance.Version);
            if (TopLevel.GetTopLevel(this) is not Window owner)
                return;

            if (!await dialog.ShowDialog<bool>(owner))
                return;

            var versionId = dialog.SelectedVersionId;
            if (string.IsNullOrEmpty(versionId))
                return;

            await viewModel.UpdateInstanceVersionAsync(versionId);
        };

        GenerateLaunchScriptButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel viewModel)
                await viewModel.GenerateLaunchScriptAsync();
        };

        TestLaunchButton!.Click += async (_, _) =>
        {
            if (DataContext is MainWindowViewModel viewModel)
                await viewModel.TestLaunchAsync();
        };
    }

    private void OnTabSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        if (InstanceTabs?.SelectedIndex is { } index)
        {
            switch (index)
            {
                case 1: // 安装器
                    PageHost.Content = new InstallerListPage { DataContext = DataContext };
                    break;
                case 3: // 资源包
                    PageHost.Content = new ResourcePackListPage { DataContext = DataContext };
                    break;
                case 4: // 数据包
                    PageHost.Content = new DataPackListPage { DataContext = DataContext };
                    break;
                case 5: // 着色器
                    PageHost.Content = new ShaderListPage { DataContext = DataContext };
                    break;
                case 6: // 截图
                    PageHost.Content = new ScreenshotsPage { DataContext = DataContext };
                    break;
                case 7: // 蓝图
                    PageHost.Content = new BlueprintListPage { DataContext = DataContext };
                    break;
                case 8: // 文件夹
                    PageHost.Content = new InstanceFoldersPage { DataContext = DataContext };
                    break;
                default:
                    break;
            }
        }
    }
}
