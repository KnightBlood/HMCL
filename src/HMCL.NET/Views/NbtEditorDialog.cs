using System;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Layout;
using Avalonia.Media;
using Avalonia.Threading;
using HMCL.NET.ViewModels;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Dialog for viewing and editing Minecraft NBT files (level.dat, etc.).
/// Shows the NBT tag tree and allows editing scalar values.
/// </summary>
public sealed class NbtEditorDialog : Window
{
    private readonly NbtEditorViewModel _viewModel;
    private readonly TreeView _treeView = new();
    private readonly TextBox _valueBox = new();
    private readonly TextBlock _statusText = new()
    {
        FontSize = 12,
        Foreground = Brushes.Gray,
        TextWrapping = TextWrapping.Wrap,
        Margin = new Thickness(0, 8, 0, 0)
    };
    private readonly Button _saveButton;
    private readonly Button _cancelButton;

    /// <summary>
    /// Creates the NBT editor dialog for the given file path.
    /// </summary>
    /// <param name="filePath">Path to the .nbt or level.dat file.</param>
    public NbtEditorDialog(string filePath)
    {
        Title = $"NBT 编辑器 - {System.IO.Path.GetFileName(filePath)}";
        Width = 700;
        Height = 500;
        SizeToContent = SizeToContent.Manual;
        CanResize = true;
        MinWidth = 500;
        MinHeight = 400;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _viewModel = new NbtEditorViewModel();
        if (!_viewModel.Load(filePath))
        {
            // File load failed; still show dialog with error status.
        }

        DataContext = _viewModel;

        _treeView.Classes.Add("nbt-tree");
        _treeView.HorizontalAlignment = HorizontalAlignment.Stretch;
        _treeView.VerticalAlignment = VerticalAlignment.Stretch;
        _treeView.SelectionChanged += OnTreeSelectionChanged;

        _valueBox.PlaceholderText = Lang.Get("hmcl.nbteditordialog.1072_");
        _valueBox.IsVisible = false;
        _valueBox.Margin = new Thickness(0, 4, 0, 0);
        _valueBox.KeyDown += (_, e) =>
        {
            if (e.Key == Avalonia.Input.Key.Enter)
                CommitEdit();
            else if (e.Key == Avalonia.Input.Key.Escape)
                CancelEdit();
        };

        _saveButton = CreateDialogButton(Lang.Get("hmcl.nbteditordialog.button_save"), "#2B579A", true);
        _saveButton.Click += (_, _) =>
        {
            if (_viewModel.Save())
                Close(true);
        };

        _cancelButton = CreateDialogButton(Lang.Get("hmcl.nbteditordialog.button_cancel"), "#757575", false);
        _cancelButton.Click += (_, _) => Close(false);

        var buttonPanel = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right,
            Margin = new Thickness(16, 8, 16, 16)
        };
        buttonPanel.Children.Add(_saveButton);
        buttonPanel.Children.Add(_cancelButton);

        var root = new DockPanel
        {
            LastChildFill = true,
            Margin = new Thickness(16)
        };

        var infoPanel = new StackPanel
        {
            Spacing = 4,
            Margin = new Thickness(0, 0, 0, 8)
        };
        infoPanel.Children.Add(new TextBlock
        {
            Text = $"文件: {_viewModel.FilePath}",
            FontWeight = FontWeight.Bold,
            TextWrapping = TextWrapping.Wrap
        });
        infoPanel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.nbteditordialog.1078_"),
            Foreground = Brushes.Gray,
            FontSize = 12,
            TextWrapping = TextWrapping.Wrap
        });
        root.Children.Add(infoPanel);
        DockPanel.SetDock(infoPanel, Dock.Top);

        root.Children.Add(_treeView);
        DockPanel.SetDock(_treeView, Dock.Top);

        var editPanel = new StackPanel
        {
            Spacing = 8,
            Margin = new Thickness(0, 8, 0, 0)
        };
        editPanel.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.nbteditordialog.1080_"),
            FontWeight = FontWeight.Bold,
            Margin = new Thickness(0, 0, 0, 4)
        });
        editPanel.Children.Add(_valueBox);
        editPanel.Children.Add(_statusText);
        root.Children.Add(editPanel);
        DockPanel.SetDock(editPanel, Dock.Bottom);

        Content = root;
        this.EnableEscapeToClose();

        BuildTree(_treeView, _viewModel.RootNode);

        // Bind view model status changes to the status text.
        _viewModel.PropertyChanged += (_, e) =>
        {
            if (e.PropertyName == nameof(NbtEditorViewModel.StatusMessage))
                _statusText.Text = _viewModel.StatusMessage;
            if (e.PropertyName == nameof(NbtEditorViewModel.HasChanges))
                _saveButton.IsEnabled = _viewModel.HasChanges;
        };

        _saveButton.IsEnabled = false;
    }

    private static Button CreateDialogButton(string content, string backgroundHex, bool isDefault)
    {
        return new Button
        {
            Content = content,
            Padding = new Thickness(16, 6),
            MinWidth = 80,
            Background = new SolidColorBrush(Color.Parse(backgroundHex)),
            Foreground = Brushes.White,
            IsDefault = isDefault
        };
    }

    private static void BuildTree(TreeView treeView, NbtTreeNode? root)
    {
        if (root is null)
            return;

        treeView.ItemsSource = new[] { root };

        treeView.ItemTemplate = new FuncTreeDataTemplate<NbtTreeNode>((node, _) =>
        {
            var stack = new StackPanel
            {
                Orientation = Orientation.Horizontal,
                Spacing = 8,
                Margin = new Thickness(2)
            };

            var nameText = new TextBlock
            {
                Text = node.Name,
                FontWeight = FontWeight.Bold,
                MinWidth = 120,
                VerticalAlignment = VerticalAlignment.Center
            };
            stack.Children.Add(nameText);

            var typeText = new TextBlock
            {
                Text = $"[{node.TagType}]",
                Foreground = Brushes.Gray,
                MinWidth = 80,
                VerticalAlignment = VerticalAlignment.Center
            };
            stack.Children.Add(typeText);

            var valueText = new TextBlock
            {
                Text = node.DisplayValue,
                VerticalAlignment = VerticalAlignment.Center,
                TextTrimming = Avalonia.Media.TextTrimming.CharacterEllipsis
            };
            stack.Children.Add(valueText);

            return stack;
        }, node => node.Children);
    }

    private void OnTreeSelectionChanged(object? sender, SelectionChangedEventArgs e)
    {
        _valueBox.IsVisible = false;
        _valueBox.Text = string.Empty;

        if (_treeView.SelectedItem is NbtTreeNode node && node.IsEditable)
        {
            _valueBox.IsVisible = true;
            _valueBox.Text = node.EditText;
            _valueBox.Focus();
            _statusText.Text = $"正在编辑: {node.Name} ({node.TagType})";
        }
        else
        {
            _statusText.Text = string.Empty;
        }
    }

    private void CommitEdit()
    {
        if (_treeView.SelectedItem is not NbtTreeNode node || !node.IsEditable)
            return;

        node.EditText = _valueBox.Text ?? string.Empty;
        node.DisplayValue = _valueBox.Text ?? string.Empty;
        _viewModel.HasChanges = true;
        _valueBox.IsVisible = false;
        _statusText.Text = $"已修改: {node.Name}";
    }

    private void CancelEdit()
    {
        _valueBox.IsVisible = false;
        _statusText.Text = string.Empty;
    }
}
