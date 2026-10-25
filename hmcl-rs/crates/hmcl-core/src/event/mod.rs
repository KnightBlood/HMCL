//! The events of the launcher and the bus that carries them.
//!
//! The launcher tells the parts of it that are apart from each other about
//! the things that happen, for example that the game was started or that the
//! settings were changed. A part of the launcher listens for the events of
//! the kind it is interested in, and the bus delivers them.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// The result of an event.
///
/// The result tells the part of the launcher that fired the event whether it
/// may go on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EventResult {
    /// The action is refused.
    Deny,
    /// The action is neither refused nor allowed.
    #[default]
    Default,
    /// The action is allowed.
    Allow,
}

/// The priority of a listener.
///
/// The listeners with a higher priority are called first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum EventPriority {
    /// The listener is called after the others.
    Lowest,
    /// The listener is called after the others.
    Low,
    /// The listener is called with the others.
    #[default]
    Normal,
    /// The listener is called before the others.
    High,
    /// The listener is called before the others.
    Highest,
    /// The listener is called last and sees the result of the others.
    Monitor,
}

impl EventPriority {
    /// Returns the place of the priority in the order the listeners are
    /// called in.
    ///
    /// The listeners with the higher priority are called first, like in the
    /// Java launcher. The listeners of the priority `Monitor` are called
    /// last, so they see what the other listeners did.
    const fn rank(self) -> u8 {
        match self {
            Self::Highest => 0,
            Self::High => 1,
            Self::Normal => 2,
            Self::Low => 3,
            Self::Lowest => 4,
            Self::Monitor => 5,
        }
    }
}

/// The state of an event.
///
/// The state holds the result of the event and tells whether it was
/// cancelled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventState {
    /// Whether the event was cancelled.
    cancelled: bool,
    /// The result of the event.
    result: EventResult,
}

impl EventState {
    /// Returns whether the event was cancelled.
    pub const fn is_cancelled(&self) -> bool {
        self.cancelled
    }

    /// Cancels the event.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Returns the result of the event.
    pub const fn result(&self) -> EventResult {
        self.result
    }

    /// Sets the result of the event.
    pub fn set_result(&mut self, result: EventResult) {
        self.result = result;
    }
}

/// An event of the launcher.
///
/// The events that may be cancelled or that carry a result tell it with
/// [`Event::is_cancelable`] and [`Event::has_result`]; the state of the
/// event is kept in [`Event::state`].
pub trait Event: 'static {
    /// Returns the name of the event.
    fn name(&self) -> &str;

    /// Returns whether the event may be cancelled.
    fn is_cancelable(&self) -> bool {
        false
    }

    /// Returns whether the event carries a result.
    fn has_result(&self) -> bool {
        false
    }

    /// Returns the state of the event.
    fn state(&self) -> &EventState;

    /// Returns the state of the event, so it can be changed.
    fn state_mut(&mut self) -> &mut EventState;
}

/// Declares an event of the launcher.
///
/// The event holds the state of the event and nothing else; the events that
/// carry more implement [`Event`] themselves.
#[macro_export]
macro_rules! event {
    (
        $(#[$attribute:meta])*
        $name:ident
    ) => {
        $(#[$attribute])*
        #[derive(Debug, Clone, Default)]
        pub struct $name {
            /// The state of the event.
            state: $crate::event::EventState,
        }

        impl $crate::event::Event for $name {
            fn name(&self) -> &str {
                stringify!($name)
            }

            fn state(&self) -> &$crate::event::EventState {
                &self.state
            }

            fn state_mut(&mut self) -> &mut $crate::event::EventState {
                &mut self.state
            }
        }
    };
}

/// A listener of the events of one kind.
type Listener<E> = Arc<Mutex<dyn FnMut(&mut E) + Send>>;

/// The listeners of the events of one kind.
pub struct EventChannel<E: Event> {
    /// The listeners, by the priority they are called with.
    listeners: Mutex<Vec<(EventPriority, Listener<E>)>>,
}

impl<E: Event> EventChannel<E> {
    /// Creates a channel without listeners.
    pub fn new() -> Self {
        Self {
            listeners: Mutex::new(Vec::new()),
        }
    }

    /// Adds a listener that is called with the listeners of the normal
    /// priority.
    pub fn register(&self, listener: impl FnMut(&mut E) + Send + 'static) {
        self.register_with_priority(EventPriority::Normal, listener);
    }

    /// Adds a listener that is called with the listeners of the given
    /// priority.
    pub fn register_with_priority(
        &self,
        priority: EventPriority,
        listener: impl FnMut(&mut E) + Send + 'static,
    ) {
        let mut listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        listeners.push((priority, Arc::new(Mutex::new(listener))));
    }

    /// Returns the number of the listeners of the channel.
    pub fn listener_count(&self) -> usize {
        let listeners = self
            .listeners
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        listeners.len()
    }

    /// Delivers the event to the listeners and returns its result.
    ///
    /// The listeners are called by the order of their priority; the
    /// listeners of the same priority are called in the order they were
    /// added. A listener that cancels the event or that sets its result
    /// changes what the other listeners see.
    pub fn fire(&self, event: &mut E) -> EventResult {
        // The listeners are taken out of the channel, so a listener may add
        // another listener while it is called.
        let mut listeners: Vec<(EventPriority, Listener<E>)> = {
            let mut registered = self
                .listeners
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            std::mem::take(&mut *registered)
        };

        listeners.sort_by_key(|(priority, _)| priority.rank());

        for (_, listener) in &listeners {
            let mut listener = listener.lock().unwrap_or_else(|error| error.into_inner());
            listener(event);
        }

        {
            let mut registered = self
                .listeners
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            registered.append(&mut listeners);
        }

        if event.has_result() {
            event.state().result()
        } else {
            EventResult::Default
        }
    }
}

impl<E: Event> Default for EventChannel<E> {
    fn default() -> Self {
        Self::new()
    }
}

/// The bus that carries the events of the launcher.
///
/// Every kind of event has a channel of its own, so the listeners of one
/// kind of event are not called for another kind.
#[derive(Default)]
pub struct EventBus {
    /// The channels of the events, by the kind of the event.
    channels: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl EventBus {
    /// Creates a bus without channels.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the bus the whole launcher uses.
    pub fn global() -> &'static EventBus {
        static BUS: OnceLock<EventBus> = OnceLock::new();
        BUS.get_or_init(EventBus::new)
    }

    /// Returns the channel of the events of the given kind.
    ///
    /// The channel is created when it is asked for the first time, so every
    /// part of the launcher that listens for the events of a kind listens to
    /// the same channel.
    pub fn channel<E: Event>(&self) -> Arc<EventChannel<E>> {
        let mut channels = self
            .channels
            .lock()
            .unwrap_or_else(|error| error.into_inner());

        let entry = channels
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Arc::new(EventChannel::<E>::new()));

        entry
            .clone()
            .downcast::<EventChannel<E>>()
            .expect("the channel of an event keeps its kind")
    }

    /// Delivers the event to the listeners of its kind and returns its
    /// result.
    pub fn fire<E: Event>(&self, event: &mut E) -> EventResult {
        self.channel::<E>().fire(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    event!(SettingsChanged);
    event!(GameStarted);

    /// An event that may be cancelled and that carries a result.
    #[derive(Debug, Default)]
    struct LaunchRequested {
        state: EventState,
    }

    impl Event for LaunchRequested {
        fn name(&self) -> &str {
            "LaunchRequested"
        }

        fn is_cancelable(&self) -> bool {
            true
        }

        fn has_result(&self) -> bool {
            true
        }

        fn state(&self) -> &EventState {
            &self.state
        }

        fn state_mut(&mut self) -> &mut EventState {
            &mut self.state
        }
    }

    #[test]
    fn the_listeners_are_called() {
        let bus = EventBus::new();
        let channel = bus.channel::<SettingsChanged>();

        let count = Arc::new(Mutex::new(0));
        let counter = Arc::clone(&count);
        channel.register(move |_event| {
            *counter.lock().unwrap() += 1;
        });

        let mut event = SettingsChanged::default();
        assert_eq!(bus.fire(&mut event), EventResult::Default);
        assert_eq!(*count.lock().unwrap(), 1);
    }

    #[test]
    fn the_listeners_are_called_by_their_priority() {
        let bus = EventBus::new();
        let channel = bus.channel::<SettingsChanged>();

        let order = Arc::new(Mutex::new(Vec::new()));

        let first = Arc::clone(&order);
        channel.register_with_priority(EventPriority::Normal, move |_event| {
            first.lock().unwrap().push("normal");
        });
        let second = Arc::clone(&order);
        channel.register_with_priority(EventPriority::High, move |_event| {
            second.lock().unwrap().push("high");
        });
        let third = Arc::clone(&order);
        channel.register_with_priority(EventPriority::Monitor, move |_event| {
            third.lock().unwrap().push("monitor");
        });

        let mut event = SettingsChanged::default();
        bus.fire(&mut event);

        assert_eq!(*order.lock().unwrap(), vec!["high", "normal", "monitor"]);
    }

    #[test]
    fn an_event_can_be_cancelled() {
        let bus = EventBus::new();
        let channel = bus.channel::<LaunchRequested>();

        channel.register(|event| {
            if event.state().is_cancelled() {
                return;
            }
            event.state_mut().cancel();
        });

        let mut event = LaunchRequested::default();
        bus.fire(&mut event);
        assert!(event.state().is_cancelled());
    }

    #[test]
    fn the_result_of_an_event_is_returned() {
        let bus = EventBus::new();
        let channel = bus.channel::<LaunchRequested>();

        channel.register(|event| {
            event.state_mut().set_result(EventResult::Allow);
        });

        let mut event = LaunchRequested::default();
        assert_eq!(bus.fire(&mut event), EventResult::Allow);
        assert_eq!(event.state().result(), EventResult::Allow);
    }

    #[test]
    fn the_listeners_of_one_kind_are_not_called_for_another() {
        let bus = EventBus::new();

        let count = Arc::new(Mutex::new(0));
        let counter = Arc::clone(&count);
        bus.channel::<SettingsChanged>().register(move |_event| {
            *counter.lock().unwrap() += 1;
        });

        let mut event = GameStarted::default();
        bus.fire(&mut event);

        assert_eq!(*count.lock().unwrap(), 0);
    }

    #[test]
    fn every_part_of_the_launcher_uses_the_same_channel() {
        let bus = EventBus::new();

        let first = bus.channel::<SettingsChanged>();
        let second = bus.channel::<SettingsChanged>();
        assert!(Arc::ptr_eq(&first, &second));

        first.register(|_event| {});
        assert_eq!(second.listener_count(), 1);
    }
}
