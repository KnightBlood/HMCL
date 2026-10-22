using System;
using System.Net;
using System.Text.RegularExpressions;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The window showing the description of a modpack, like the web page of the
/// description of Java HMCL.
/// </summary>
/// <remarks>
/// The description of the remote modpacks is written in HTML; the tags are
/// removed, because the launcher has no HTML engine.
/// </remarks>
public sealed class ModpackDescriptionWindow : Window
{
    /// <summary>
    /// The pattern of the HTML tags of the description.
    /// </summary>
    private static readonly Regex TagPattern = new("<[^>]*>", RegexOptions.Compiled);

    /// <summary>
    /// Creates the window showing the given description.
    /// </summary>
    /// <param name="title">The title of the window.</param>
    /// <param name="description">The description of the modpack.</param>
    public ModpackDescriptionWindow(string title, string? description)
    {
        Title = title;
        Width = 620;
        Height = 480;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var text = new TextBlock
        {
            Text = StripHtml(description),
            TextWrapping = TextWrapping.Wrap,
            Margin = new Thickness(16)
        };

        var closeButton = new Button
        {
            Content = Lang.Get("common.close", "关闭"),
            Padding = new Thickness(16, 6),
            HorizontalAlignment = HorizontalAlignment.Right
        };
        closeButton.Click += (_, _) => Close();

        var root = new Grid { RowDefinitions = new RowDefinitions("*,Auto") };
        var scroll = new ScrollViewer { Content = text };
        root.Children.Add(scroll);

        var buttonRow = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            HorizontalAlignment = HorizontalAlignment.Right,
            Margin = new Thickness(16, 0, 16, 16),
            Children = { closeButton }
        };
        Grid.SetRow(buttonRow, 1);
        root.Children.Add(buttonRow);

        Content = root;

        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Removes the HTML tags of a description and decodes the entities.
    /// </summary>
    /// <param name="description">The description of the modpack.</param>
    /// <returns>The text of the description.</returns>
    public static string StripHtml(string? description)
    {
        if (string.IsNullOrWhiteSpace(description))
            return string.Empty;

        var text = TagPattern.Replace(description, string.Empty);
        text = text.Replace("<br>", "\n", StringComparison.OrdinalIgnoreCase)
                   .Replace("</p>", "\n", StringComparison.OrdinalIgnoreCase)
                   .Replace("<br/>", "\n", StringComparison.OrdinalIgnoreCase)
                   .Replace("<br />", "\n", StringComparison.OrdinalIgnoreCase);

        return WebUtility.HtmlDecode(text).Trim();
    }
}
