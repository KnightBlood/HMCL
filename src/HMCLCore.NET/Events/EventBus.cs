using System;
using System.Collections.Generic;
using System.Linq;

namespace HMCLCore.NET.Events;

/// <summary>
/// The event bus of the launcher, used to publish and to subscribe to the
/// events of the launcher.
/// </summary>
public sealed class EventBus
{
    /// <summary>
    /// The event bus shared by the whole launcher.
    /// </summary>
    public static EventBus Instance { get; } = new();

    private readonly Dictionary<Type, List<Delegate>> _handlers = new();

    /// <summary>
    /// Subscribes to the events of the given type.
    /// </summary>
    /// <typeparam name="T">The type of the event.</typeparam>
    /// <param name="handler">The handler called when the event is published.</param>
    public void Subscribe<T>(Action<T> handler) where T : EventBase
    {
        ArgumentNullException.ThrowIfNull(handler);

        var eventType = typeof(T);
        if (!_handlers.TryGetValue(eventType, out var handlers))
        {
            handlers = new List<Delegate>();
            _handlers[eventType] = handlers;
        }

        handlers.Add(handler);
    }

    /// <summary>
    /// Unsubscribes from the events of the given type.
    /// </summary>
    /// <typeparam name="T">The type of the event.</typeparam>
    /// <param name="handler">The handler to remove.</param>
    public void Unsubscribe<T>(Action<T> handler) where T : EventBase
    {
        if (handler != null && _handlers.TryGetValue(typeof(T), out var handlers))
            handlers.Remove(handler);
    }

    /// <summary>
    /// Publishes an event to the handlers subscribed to its type.
    /// </summary>
    /// <typeparam name="T">The type of the event.</typeparam>
    /// <param name="event">The published event.</param>
    public void Publish<T>(T @event) where T : EventBase
    {
        if (_handlers.TryGetValue(typeof(T), out var handlers))
        {
            foreach (var handler in handlers.ToArray())
            {
                try
                {
                    ((Action<T>)handler)(@event);
                }
                catch (Exception)
                {
                    // A failing handler must not break the other ones.
                }
            }
        }
    }
}
