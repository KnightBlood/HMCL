using Avalonia.Controls;
using Avalonia.Interactivity;
using HMCL.NET.ViewModels;

namespace HMCL.NET.Views;

/// <summary>
/// Page listing the launcher tasks that are running together with their
/// progress, mirroring the Java HMCL task list pane.
/// The data context is a <see cref="TaskListViewModel"/>.
/// </summary>
public partial class TaskListPage : UserControl
{
    /// <summary>
    /// Creates the task list page and wires its toolbar buttons.
    /// </summary>
    public TaskListPage()
    {
        InitializeComponent();

        CancelAllButton.Click += OnCancelAllClick;
        ClearFinishedButton.Click += OnClearFinishedClick;
    }

    /// <summary>
    /// Cancels every running task.
    /// </summary>
    private void OnCancelAllClick(object? sender, RoutedEventArgs e) =>
        (DataContext as TaskListViewModel)?.CancelAll();

    /// <summary>
    /// Removes the finished tasks from the list.
    /// </summary>
    private void OnClearFinishedClick(object? sender, RoutedEventArgs e) =>
        (DataContext as TaskListViewModel)?.ClearFinished();
}
