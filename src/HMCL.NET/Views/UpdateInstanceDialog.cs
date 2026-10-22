using System;
using System.Collections.Generic;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCLCore.NET.Download;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The dialog letting the user choose the version an instance is updated to.
/// </summary>
public sealed class UpdateInstanceDialog : Window
{
    private readonly ComboBox _versionBox = new() { HorizontalAlignment = HorizontalAlignment.Stretch };
    private readonly TextBlock _statusText = new() { Foreground = new SolidColorBrush(Color.Parse("#666666")) };
    private readonly Button _updateButton;

    /// <summary>
    /// The identifier of the version chosen by the user.
    /// </summary>
    public string SelectedVersionId => (_versionBox.SelectedItem as VersionMetadata)?.Id ?? string.Empty;

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    /// <param name="versions">The versions offered to the user.</param>
    /// <param name="currentVersion">The version of the instance, preselected.</param>
    public UpdateInstanceDialog(IEnumerable<VersionMetadata> versions, string currentVersion)
    {
        Title = Lang.Get("hmcl.updateinstancedialog.1124_");
        Width = 420;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var ordered = versions.OrderByDescending(v => v, VersionMetadataComparer.Instance).ToList();
        _versionBox.ItemsSource = ordered;
        _versionBox.SelectedItem = ordered.FirstOrDefault(v =>
            string.Equals(v.Id, currentVersion, StringComparison.OrdinalIgnoreCase));

        _updateButton = new Button
        {
            Content = Lang.Get("hmcl.updateinstancedialog.update_accept"),
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            Background = new SolidColorBrush(Color.Parse("#2B579A")),
            Foreground = Brushes.White
        };
        _updateButton.Click += (_, _) => Close(true);

        var cancelButton = new Button
        {
            Content = Lang.Get("hmcl.updateinstancedialog.button_cancel"),
            Padding = new Thickness(16, 6),
            MinWidth = 72
        };
        cancelButton.Click += (_, _) => Close(false);

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        buttons.Children.Add(_updateButton);
        buttons.Children.Add(cancelButton);

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 12 };
        root.Children.Add(new TextBlock { Text = Lang.Get("hmcl.updateinstancedialog.1130_"), FontWeight = FontWeight.Bold });
        root.Children.Add(_versionBox);
        root.Children.Add(_statusText);
        root.Children.Add(buttons);

        Content = root;
        this.EnableEscapeToClose();
    }
}
