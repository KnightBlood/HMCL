using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;

namespace HMCL.NET.Views.Wizard;

/// <summary>
/// The summary of the choices made by the user in a wizard, shown as the last
/// page of the wizard like the summary of Java HMCL.
/// </summary>
public sealed class WizardSummary
{
    /// <summary>
    /// Creates the summary of a wizard.
    /// </summary>
    /// <param name="title">The title of the summary.</param>
    /// <param name="items">
    /// The items of the summary, each one holding the name of a choice and the
    /// value chosen by the user.
    /// </param>
    public WizardSummary(string title, IEnumerable<(string Name, string Value)> items)
    {
        Title = title ?? throw new ArgumentNullException(nameof(title));
        Items = items?.Select(item => (item.Name, item.Value)).ToList()
                  ?? new List<(string, string)>();
    }

    /// <summary>
    /// The title of the summary.
    /// </summary>
    public string Title { get; }

    /// <summary>
    /// The items of the summary.
    /// </summary>
    public IReadOnlyList<(string Name, string Value)> Items { get; }
}

/// <summary>
/// Implemented by the wizard steps which show a summary of the choices made by
/// the user before the wizard is completed.
/// </summary>
public interface IWizardSummaryProvider
{
    /// <summary>
    /// Returns the summary of the wizard, or null when the step does not show
    /// one.
    /// </summary>
    /// <returns>The summary of the wizard.</returns>
    WizardSummary? GetSummary();
}

/// <summary>
/// The last page of a wizard, showing the summary of the choices made by the
/// user.
/// </summary>
public sealed class SummaryWizardStep : IWizardStep
{
    private readonly WizardSummary _summary;
    private readonly Func<Task>? _onFinish;

    /// <summary>
    /// Creates the summary step.
    /// </summary>
    /// <param name="summary">The summary shown by the step.</param>
    /// <param name="onFinish">
    /// The action run when the user confirms the summary, for example the
    /// export of the modpack.
    /// </param>
    public SummaryWizardStep(WizardSummary summary, Func<Task>? onFinish = null)
    {
        _summary = summary ?? throw new ArgumentNullException(nameof(summary));
        _onFinish = onFinish;

        var list = new StackPanel { Spacing = 6 };
        foreach (var (name, value) in summary.Items)
        {
            var row = new Grid { ColumnDefinitions = new ColumnDefinitions("160,*") };
            row.Children.Add(new TextBlock
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
            row.Children.Add(valueText);
            list.Children.Add(row);
        }

        Content = new ScrollViewer { Content = list };
    }

    /// <summary>
    /// The title of the step.
    /// </summary>
    public string Title => _summary.Title;

    /// <summary>
    /// The content of the step.
    /// </summary>
    public Control Content { get; }

    /// <summary>
    /// The wizard may always be completed from the summary.
    /// </summary>
    public bool CanGoNext => true;

    /// <summary>
    /// The summary does not need any preparation.
    /// </summary>
    /// <returns>A completed task.</returns>
    public Task OnEnterAsync() => Task.CompletedTask;

    /// <summary>
    /// Runs the action of the summary when the user leaves the step.
    /// </summary>
    /// <returns>A task that completes when the action is done.</returns>
    public async Task OnLeaveAsync()
    {
        if (_onFinish is not null)
            await _onFinish();
    }
}
