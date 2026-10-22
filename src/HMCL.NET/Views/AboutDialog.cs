using System;
using System.Diagnostics;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Views;

/// <summary>
/// The dialog showing the information about the launcher, the links of the
/// project and the ways to give feedback. It mirrors the about page and the
/// feedback page of Java HMCL.
/// </summary>
public sealed class AboutDialog : Window
{
    /// <summary>
    /// The address of the project.
    /// </summary>
    public const string ProjectUrl = "https://github.com/KnightBlood/HMCL";

    /// <summary>
    /// The address of the issue tracker of the project.
    /// </summary>
    public const string FeedbackUrl = "https://github.com/KnightBlood/HMCL/issues";

    /// <summary>
    /// The address of the documentation of the launcher.
    /// </summary>
    public const string DocumentationUrl = "https://docs.hmcl.net/";

    /// <summary>
    /// The address of the terms of the license of the launcher.
    /// </summary>
    public const string LicenseUrl = "https://docs.hmcl.net/terms/";

    /// <summary>
    /// Creates the dialog.
    /// </summary>
    public AboutDialog()
    {
        Title = Lang.Get("hmcl.aboutdialog.about");
        Width = 480;
        SizeToContent = SizeToContent.Height;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        var version = System.Reflection.Assembly.GetExecutingAssembly().GetName().Version;

        var root = new StackPanel { Margin = new Thickness(16), Spacing = 10 };

        root.Children.Add(new TextBlock
        {
            Text = "Hello Minecraft! Launcher",
            FontSize = 18,
            FontWeight = FontWeight.Bold
        });

        root.Children.Add(new TextBlock
        {
            Text = $"HMCL.NET {version?.ToString() ?? "1.0.0"}",
            Foreground = new SolidColorBrush(Color.Parse("#666666"))
        });

        root.Children.Add(CreateSeparator());

        root.Children.Add(new TextBlock { Text = Lang.Get("hmcl.aboutdialog.840_"), FontWeight = FontWeight.Bold });
        root.Children.Add(CreateLinkButton(Lang.Get("hmcl.aboutdialog.842_"), ProjectUrl));
        root.Children.Add(CreateLinkButton(Lang.Get("hmcl.aboutdialog.844_"), FeedbackUrl));
        root.Children.Add(CreateLinkButton(Lang.Get("hmcl.aboutdialog.846_"), DocumentationUrl));

        root.Children.Add(CreateSeparator());

        root.Children.Add(new TextBlock { Text = Lang.Get("hmcl.aboutdialog.848_"), FontWeight = FontWeight.Bold });
        root.Children.Add(CreateLinkButton(Lang.Get("hmcl.aboutdialog.850_"), LicenseUrl));
        root.Children.Add(CreateLinkButton(Lang.Get("hmcl.aboutdialog.852_"), ProjectUrl));

        root.Children.Add(new TextBlock
        {
            Text = Lang.Get("hmcl.aboutdialog.854_"),
            FontSize = 11,
            Foreground = new SolidColorBrush(Color.Parse("#888888")),
            TextWrapping = TextWrapping.Wrap
        });

        var closeButton = new Button
        {
            Content = Lang.Get("hmcl.aboutdialog.856_"),
            Padding = new Thickness(16, 6),
            MinWidth = 72,
            HorizontalAlignment = HorizontalAlignment.Right
        };

        closeButton.Click += (_, _) => Close(true);

        root.Children.Add(closeButton);

        Content = root;

        // The escape key closes the dialog.
        this.EnableEscapeToClose();
    }

    /// <summary>
    /// Creates the separator between two sections of the dialog.
    /// </summary>
    /// <returns>The created separator.</returns>
    private static Control CreateSeparator() => new Rectangle
    {
        Height = 1,
        Fill = new SolidColorBrush(Color.Parse("#DDDDDD"))
    };

    /// <summary>
    /// Creates a button opening the given address in the browser.
    /// </summary>
    /// <param name="title">The title of the button.</param>
    /// <param name="url">The address opened by the button.</param>
    /// <returns>The created button.</returns>
    private static Button CreateLinkButton(string title, string url)
    {
        var button = new Button
        {
            Content = title,
            Padding = new Thickness(10, 4),
            HorizontalAlignment = HorizontalAlignment.Left,
            Background = Brushes.Transparent,
            BorderThickness = new Thickness(0)
        };

        button.Click += (_, _) =>
        {
            try
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = url,
                    UseShellExecute = true
                });
            }
            catch (Exception)
            {
                // The browser cannot be opened.
            }
        };

        return button;
    }
}
