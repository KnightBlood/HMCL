using System;
using Avalonia;

namespace HMCL.NET.Util;

/// <summary>
/// Connects the properties of the controls of the launcher without the
/// reflection used by the bindings of Avalonia, so that the launcher can be
/// published with the native ahead-of-time compiler.
/// </summary>
public static class PropertyLink
{
    /// <summary>
    /// Copies the value of a property into another property, so that every
    /// change of the first property is written into the second one.
    /// </summary>
    /// <param name="source">The control holding the value.</param>
    /// <param name="sourceProperty">The property holding the value.</param>
    /// <param name="target">The control receiving the value.</param>
    /// <param name="targetProperty">The property receiving the value.</param>
    public static void Copy(
        AvaloniaObject source,
        AvaloniaProperty sourceProperty,
        AvaloniaObject target,
        AvaloniaProperty targetProperty)
    {
        target.Bind(targetProperty, source.GetObservable(sourceProperty));
    }

    /// <summary>
    /// Connects two properties, so that the changes made by the user in the
    /// target are written into the source and the changes of the source are
    /// shown in the target.
    /// </summary>
    /// <typeparam name="T">The type of the values of the properties.</typeparam>
    /// <param name="source">The control holding the value.</param>
    /// <param name="sourceProperty">The property holding the value.</param>
    /// <param name="target">The control edited by the user.</param>
    /// <param name="targetProperty">The property edited by the user.</param>
    public static void Link<T>(
        AvaloniaObject source,
        AvaloniaProperty<T> sourceProperty,
        AvaloniaObject target,
        AvaloniaProperty<T> targetProperty)
    {
        // The value of the source is written into the target first, so that
        // the target does not overwrite the value of the source when the
        // connection is made.
        Copy(source, sourceProperty, target, targetProperty);

        // The value edited by the user is written into the source without
        // removing the binding of the source, so that the changes keep being
        // sent to the view model.
        target.GetObservable(targetProperty)
            .Subscribe(new ValueObserver<T>(value => source.SetCurrentValue(sourceProperty, value)));
    }

    /// <summary>
    /// Watches a property and runs an action every time it changes.
    /// </summary>
    /// <typeparam name="T">The type of the value of the property.</typeparam>
    /// <param name="source">The control holding the value.</param>
    /// <param name="sourceProperty">The property to watch.</param>
    /// <param name="action">The action run when the value changes.</param>
    public static void Watch<T>(
        AvaloniaObject source,
        AvaloniaProperty<T> sourceProperty,
        Action<T> action)
    {
        source.GetObservable(sourceProperty).Subscribe(new ValueObserver<T>(action));
    }

    /// <summary>
    /// An observer writing the values it receives into an action.
    /// </summary>
    /// <typeparam name="T">The type of the values.</typeparam>
    private sealed class ValueObserver<T>(Action<T> action) : IObserver<T>
    {
        /// <inheritdoc />
        public void OnCompleted()
        {
            // The observable of a property never completes.
        }

        /// <inheritdoc />
        public void OnError(Exception error)
        {
            // The errors of the observable of a property are ignored.
        }

        /// <inheritdoc />
        public void OnNext(T value) => action(value);
    }
}
