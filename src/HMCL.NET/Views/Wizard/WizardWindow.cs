using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Layout;
using Avalonia.Media;
using HMCL.NET.Util;
using HMCL.NET.Views.Dialogs;

namespace HMCL.NET.Views.Wizard;

/// <summary>
/// A reusable multi-step dialog hosting a sequence of <see cref="IWizardStep"/>
/// pages. It shows the current step title, the step content and the
/// navigation buttons, mirroring Java HMCL's wizard dialogs.
/// </summary>
public sealed class WizardWindow : Window
{
    private readonly List<IWizardStep> _steps;
    private readonly TextBlock _titleText = new() { FontSize = 18, FontWeight = FontWeight.Bold };
    private readonly TextBlock _progressText = new() { Foreground = new SolidColorBrush(Color.Parse("#888")) };
    private readonly ContentControl _content = new();
    private readonly Button _backButton = new() { Content = Lang.Get("hmcl.wizardwindow.1284_"), Padding = new Thickness(16, 6) };
    private readonly Button _nextButton = new()
    {
        Content = Lang.Get("hmcl.wizardwindow.1286_"),
        Padding = new Thickness(16, 6),
        Background = new SolidColorBrush(Color.Parse("#2B579A")),
        Foreground = Brushes.White
    };
    private readonly Button _cancelButton = new() { Content = Lang.Get("hmcl.wizardwindow.button_cancel"), Padding = new Thickness(16, 6) };
    private readonly Button _refreshButton = new()
    {
        Content = Lang.Get("button.refresh", "刷新"),
        Padding = new Thickness(12, 6),
        IsVisible = false
    };

    private int _index;

    private IWizardStep? _currentStep;

    /// <summary>
    /// Refreshes the navigation buttons when a step reports that it cannot be
    /// continued any more.
    /// </summary>
    /// <param name="sender">The step that changed.</param>
    /// <param name="e">The event arguments.</param>
    private void OnCanGoNextChanged(object? sender, EventArgs e) => UpdateButtons();

    /// <summary>
    /// Creates a wizard window hosting the given steps.
    /// </summary>
    /// <param name="title">The window title.</param>
    /// <param name="steps">The steps of the wizard, in order.</param>
    public WizardWindow(string title, IEnumerable<IWizardStep> steps)
    {
        _steps = new List<IWizardStep>(steps);

        Title = title;
        Width = 720;
        Height = 520;
        MinWidth = 560;
        MinHeight = 420;
        CanResize = false;
        WindowStartupLocation = WindowStartupLocation.CenterOwner;

        _backButton.Click += OnBackClick;
        _nextButton.Click += OnNextClick;
        _cancelButton.Click += (_, _) => Close(false);

        var header = new StackPanel { Spacing = 2, Margin = new Thickness(0, 0, 0, 12) };
        header.Children.Add(_titleText);
        header.Children.Add(_progressText);

        // The steps that can be refreshed show a refresh button.
        var headerRow = new Grid { ColumnDefinitions = new ColumnDefinitions("*,Auto") };
        Grid.SetColumn(headerRow, 0);
        header.Children.Add(headerRow);
        Grid.SetColumn(_refreshButton, 1);
        headerRow.Children.Add(_refreshButton);

        _refreshButton.Click += async (_, _) => await RefreshCurrentStepAsync();

        var buttons = new StackPanel
        {
            Orientation = Orientation.Horizontal,
            Spacing = 8,
            HorizontalAlignment = HorizontalAlignment.Right
        };
        buttons.Children.Add(_backButton);
        buttons.Children.Add(_nextButton);
        buttons.Children.Add(_cancelButton);

        var root = new Grid { RowDefinitions = new RowDefinitions("Auto,*,Auto"), Margin = new Thickness(16) };
        Grid.SetRow(header, 0);
        Grid.SetRow(_content, 1);
        Grid.SetRow(buttons, 2);

        root.Children.Add(header);
        root.Children.Add(_content);
        root.Children.Add(buttons);

        Content = root;
        this.EnableEscapeToClose();

        ShowStep(0);
    }

    /// <summary>
    /// Raised when the user confirms the last step. The wizard closes itself
    /// right after the event is raised.
    /// </summary>
    public event Action? Completed;

    /// <summary>
    /// The step currently shown.
    /// </summary>
    public IWizardStep? CurrentStep => _index >= 0 && _index < _steps.Count ? _steps[_index] : null;

    /// <summary>
    /// Shows the step at the given index and updates the navigation buttons.
    /// </summary>
    /// <param name="index">The index of the step to show.</param>
    private async void ShowStep(int index)
    {
        if (index < 0 || index >= _steps.Count)
            return;

        _index = index;
        var step = _steps[index];

        if (_currentStep is IWizardStepNotifier notifier)
            notifier.CanGoNextChanged -= OnCanGoNextChanged;

        _currentStep = step;
        if (step is IWizardStepNotifier stepNotifier)
            stepNotifier.CanGoNextChanged += OnCanGoNextChanged;

        _titleText.Text = step.Title;
        _progressText.Text = $"第 {index + 1} / {_steps.Count} 步";
        _content.Content = step.Content;

        _backButton.IsEnabled = index > 0;
        _nextButton.IsEnabled = step.CanGoNext;
        _nextButton.Content = index == _steps.Count - 1 ? Lang.Get("hmcl.wizardwindow.wizard_finish") : Lang.Get("hmcl.wizardwindow.1292_");

        // The refresh button is only shown by the steps that can be refreshed.
        _refreshButton.IsVisible = step is IWizardRefreshable { CanRefresh: true };

        try
        {
            await step.OnEnterAsync();
        }
        catch (Exception)
        {
            // A failing step must not break the wizard.
        }

        UpdateButtons();
    }

    /// <summary>
    /// Refreshes the enabled state of the navigation buttons.
    /// </summary>
    private void UpdateButtons() =>
        _nextButton.IsEnabled = CurrentStep?.CanGoNext ?? false;

    /// <summary>
    /// Refreshes the content of the current step, when it can be refreshed.
    /// </summary>
    private async Task RefreshCurrentStepAsync()
    {
        if (CurrentStep is not IWizardRefreshable refreshable || !refreshable.CanRefresh)
            return;

        try
        {
            await refreshable.RefreshAsync();
        }
        catch (Exception)
        {
            // A failing refresh must not break the wizard.
        }
    }

    /// <summary>
    /// Goes back to the previous step.
    /// </summary>
    private async void OnBackClick(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (_index <= 0)
            return;

        if (CurrentStep != null)
        {
            try
            {
                await CurrentStep.OnLeaveAsync();
            }
            catch (Exception)
            {
                // Ignore failures while leaving a step.
            }
        }

        ShowStep(_index - 1);
    }

    /// <summary>
    /// Goes to the next step, or completes the wizard on the last step.
    /// </summary>
    private async void OnNextClick(object? sender, Avalonia.Interactivity.RoutedEventArgs e)
    {
        if (CurrentStep == null)
            return;

        try
        {
            await CurrentStep.OnLeaveAsync();
        }
        catch (Exception)
        {
            // Ignore failures while leaving a step.
        }

        // The step may show a summary of the choices made by the user; the
        // summary becomes the last page of the wizard.
        if (CurrentStep is IWizardSummaryProvider provider && provider.GetSummary() is { } summary &&
            !_summaryShown)
        {
            _summaryShown = true;
            _steps.Add(new SummaryWizardStep(summary));
            ShowStep(_index + 1);
            return;
        }

        if (_index >= _steps.Count - 1)
        {
            // The last step may run a long operation, which is shown in a
            // dialog like the task executor of Java HMCL.
            if (CurrentStep is IWizardTaskStep taskStep)
            {
                var dialog = new TaskDialog(taskStep.TaskMessage, taskStep.TaskTitle);
                var shown = dialog.ShowDialog(this);
                var succeeded = await dialog.RunAsync(taskStep.RunTaskAsync);

                if (!succeeded)
                    return;

                await shown;
                Completed?.Invoke();
                Close(true);
                return;
            }

            Completed?.Invoke();
            Close(true);
            return;
        }

        ShowStep(_index + 1);
    }

    /// <summary>
    /// Whether the summary of the wizard was already shown.
    /// </summary>
    private bool _summaryShown;
}
