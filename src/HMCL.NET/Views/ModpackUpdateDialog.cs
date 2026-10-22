using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Templates;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Modpack;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// Shows the file differences between the installed modpack and a newer version
/// of it and lets the user confirm the update.
/// </summary>
public sealed class ModpackUpdateDialog : Window
{
    private readonly Button _applyButton;

    /// <summary>
    /// The changes shown in the dialog.
    /// </summary>
    public ObservableCollection<ModpackUpdatePlan> Changes { get; } = new();

    /// <summary>
    /// Creates the dialog for the given update plan.
    /// </summary>
    /// <param name="plan">The computed update plan.</param>
    public ModpackUpdateDialog(ModpackUpdatePlan plan)
    {
        Title = Lang.Get("hmcl.modpackupdatedialog.instance_update");
        Width = 620;
        Height = 460;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        Changes.Add(plan);

        var summary = new TextBlock
        {
            Text = plan.Summary,
            FontWeight = FontWeight.Bold,
            TextWrapping = TextWrapping.Wrap
        };

        var list = new ListBox
        {
            ItemsSource = Changes,
            Height = 280
        };

        var template = new FuncDataTemplate<ModpackUpdatePlan>((item, _) =>
        {
            if (item == null)
                return new TextBlock();

            var panel = new StackPanel { Spacing = 2 };
            panel.Children.Add(new TextBlock
            {
                Text = $"{item.Name}: {item.OldVersion ?? Lang.Get("hmcl.modpackupdatedialog.install_installer_not_installed")} → {item.NewVersion ?? Lang.Get("hmcl.modpackupdatedialog.message_unknown")}",
                FontWeight = FontWeight.Bold
            });
            panel.Children.Add(new TextBlock
            {
                Text = $"新增 {item.AddedCount}，修改 {item.ModifiedCount}，删除 {item.RemovedCount}",
                Foreground = Brushes.Gray,
                FontSize = 12
            });

            foreach (var change in item.Changes.Where(c => c.Kind != ModpackChangeKind.Unchanged))
            {
                panel.Children.Add(new TextBlock
                {
                    Text = change.Description,
                    FontSize = 12,
                    Margin = new Thickness(12, 0, 0, 0)
                });
            }

            return panel;
        });
        list.ItemTemplate = template;

        _applyButton = new Button
        {
            Content = Lang.Get("hmcl.modpackupdatedialog.1066_"),
            Padding = new Thickness(16, 6),
            MinWidth = 96,
            Background = new SolidColorBrush(Color.Parse("#2B579A")),
            Foreground = Brushes.White,
            IsEnabled = plan.HasChanges
        };
        _applyButton.Click += (_, _) => Close(true);

        var cancelButton = new Button
        {
            Content = Lang.Get("hmcl.modpackupdatedialog.button_cancel"),
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            Background = new SolidColorBrush(Color.Parse("#757575")),
            Foreground = Brushes.White
        };
        cancelButton.Click += (_, _) => Close(false);

        var buttonPanel = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        buttonPanel.Children.Add(_applyButton);
        buttonPanel.Children.Add(cancelButton);

        var root = new StackPanel
        {
            Margin = new Thickness(16),
            Spacing = 12
        };
        root.Children.Add(summary);
        root.Children.Add(list);
        root.Children.Add(buttonPanel);

        Content = root;
        this.EnableEscapeToClose();
    }
}
