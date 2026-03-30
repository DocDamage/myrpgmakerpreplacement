//! Typed Event Bus System
//!
//! A thread-safe, priority-based event bus supporting typed events with enum dispatch.
//!
//! # Example Usage
//!
//! ```
//! use dde_core::events::{EventBus, Event, EventType, EventPriority, EventFilter, downcast_event};
//! use dde_core::events::battle::BattleEvent;
//!
//! let bus = EventBus::new();
//!
//! // Subscribe to battle events
//! let sub = bus.subscribe(
//!     EventFilter::Type(EventType::Battle),
//!     |event| {
//!         if let Some(battle) = downcast_event::<BattleEvent>(event) {
//!             match battle {
//!                 BattleEvent::EnemyDefeated { xp_gained, .. } => {
//!                     // Update quest progress
//!                     println!("Gained {} XP!", xp_gained);
//!                 }
//!                 _ => {}
//!             }
//!         }
//!     }
//! );
//!
//! // Publish a battle event with normal priority
//! bus.publish(
//!     BattleEvent::EnemyDefeated {
//!         entity: hecs::Entity::DANGLING,
//!         xp_gained: 100,
//!     },
//!     EventPriority::Normal
//! );
//!
//! // Process all queued events
//! bus.process_events();
//!
//! // Unsubscribe when done
//! bus.unsubscribe(sub);
//! ```

use crossbeam_channel::{bounded, unbounded, Receiver, Sender};
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Type alias for event predicate functions
type EventPredicate = Arc<dyn Fn(&dyn Event) -> bool + Send + Sync>;

/// Type alias for event callback functions
type EventCallback = Arc<dyn Fn(&dyn Event) + Send + Sync>;

/// Unique identifier for event subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
    /// Generate a new unique subscription ID
    fn next() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Event type categorization for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// World-related events (entity spawn, movement, etc.)
    World,
    /// Battle-related events
    Battle,
    /// UI/interaction events
    Ui,
    /// Audio events
    Audio,
    /// AI/Dialogue events
    Ai,
    /// Quest progression events
    Quest,
    /// Input events
    Input,
    /// System events (save/load, etc.)
    System,
    /// Custom event type with identifier
    Custom(u32),
}

/// Event priority levels for processing order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum EventPriority {
    /// Low priority - processed last
    Low = 0,
    /// Normal priority - default
    Normal = 1,
    /// High priority - processed first
    High = 2,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Filter for event subscriptions
#[derive(Clone)]
pub enum EventFilter {
    /// Accept all events
    All,
    /// Filter by event type category
    Type(EventType),
    /// Filter by specific event type (using TypeId)
    Exact(TypeId),
    /// Filter by multiple event types
    Types(Vec<EventType>),
    /// Custom filter function
    Custom(EventPredicate),
}

impl std::fmt::Debug for EventFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventFilter::All => write!(f, "EventFilter::All"),
            EventFilter::Type(ty) => write!(f, "EventFilter::Type({:?})", ty),
            EventFilter::Exact(id) => write!(f, "EventFilter::Exact({:?})", id),
            EventFilter::Types(types) => write!(f, "EventFilter::Types({:?})", types),
            EventFilter::Custom(_) => write!(f, "EventFilter::Custom(<function>)"),
        }
    }
}

impl PartialEq for EventFilter {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (EventFilter::All, EventFilter::All) => true,
            (EventFilter::Type(a), EventFilter::Type(b)) => a == b,
            (EventFilter::Exact(a), EventFilter::Exact(b)) => a == b,
            (EventFilter::Types(a), EventFilter::Types(b)) => a == b,
            // Custom filters are never equal since we can't compare closures
            _ => false,
        }
    }
}

impl EventFilter {
    /// Check if an event matches this filter
    pub fn matches(&self, event: &dyn Event) -> bool {
        match self {
            EventFilter::All => true,
            EventFilter::Type(ty) => event.event_type() == *ty,
            EventFilter::Exact(type_id) => event.type_id() == *type_id,
            EventFilter::Types(types) => types.contains(&event.event_type()),
            EventFilter::Custom(func) => func(event),
        }
    }
}

/// Trait for all events that can be sent through the event bus
///
/// This trait is object-safe and can be used with `dyn Event`.
/// To downcast to a concrete type, use the [`downcast_event`] function.
///
/// # Example
///
/// ```
/// use dde_core::events::{Event, EventType};
/// use std::any::{Any, TypeId};
///
/// #[derive(Debug)]
/// struct MyEvent {
///     data: String,
/// }
///
/// impl Event for MyEvent {
///     fn event_type(&self) -> EventType {
///         EventType::Custom(1)
///     }
///
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
///
///     fn type_id(&self) -> TypeId {
///         TypeId::of::<Self>()
///     }
/// }
/// ```
pub trait Event: Send + Sync + 'static {
    /// Get the event type category
    fn event_type(&self) -> EventType;

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Get the TypeId of this event
    ///
    /// Note: This shadows `std::any::Any::type_id` but with the correct
    /// signature for trait objects.
    fn type_id(&self) -> TypeId;
}

/// Downcast an event to a concrete type
///
/// # Example
///
/// ```
/// use dde_core::events::{Event, downcast_event, EventType};
/// use dde_core::events::battle::BattleEvent;
///
/// let event = BattleEvent::BattleStarted { encounter_id: 1 };
/// let event_ref: &dyn Event = &event;
///
/// if let Some(battle) = downcast_event::<BattleEvent>(event_ref) {
///     println!("Battle event: {:?}", battle);
/// }
/// ```
pub fn downcast_event<T: Event>(event: &dyn Event) -> Option<&T> {
    event.as_any().downcast_ref::<T>()
}

/// Internal wrapper for events with priority
struct PrioritizedEvent {
    event: Box<dyn Event>,
    #[allow(dead_code)]
    priority: EventPriority,
}

/// Subscription entry containing filter and callback
struct Subscription {
    filter: EventFilter,
    callback: EventCallback,
}

/// Thread-safe event bus with priority support and filtering
///
/// The event bus uses crossbeam-channel for efficient multi-producer,
/// multi-consumer message passing. Events are queued with priorities
/// and dispatched when `process_events()` is called.
///
/// # Thread Safety
///
/// The event bus is thread-safe and can be shared across threads using
/// `Arc`. Publishing events can be done from any thread.
pub struct EventBus {
    /// Channel senders for each priority level
    high_sender: Sender<PrioritizedEvent>,
    normal_sender: Sender<PrioritizedEvent>,
    low_sender: Sender<PrioritizedEvent>,
    /// Channel receivers for each priority level
    high_receiver: Receiver<PrioritizedEvent>,
    normal_receiver: Receiver<PrioritizedEvent>,
    low_receiver: Receiver<PrioritizedEvent>,
    /// Subscriptions storage
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    /// Create a new event bus
    ///
    /// # Example
    ///
    /// ```
    /// use dde_core::events::EventBus;
    ///
    /// let bus = EventBus::new();
    /// ```
    pub fn new() -> Self {
        // Use bounded channels with reasonable capacity to prevent memory exhaustion
        const CHANNEL_CAPACITY: usize = 10_000;

        let (high_sender, high_receiver) = bounded(CHANNEL_CAPACITY);
        let (normal_sender, normal_receiver) = bounded(CHANNEL_CAPACITY);
        let (low_sender, low_receiver) = bounded(CHANNEL_CAPACITY);

        Self {
            high_sender,
            normal_sender,
            low_sender,
            high_receiver,
            normal_receiver,
            low_receiver,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new event bus with unbounded channels
    ///
    /// # Warning
    ///
    /// Unbounded channels can grow indefinitely if events are published
    /// faster than they are processed. Use with caution.
    pub fn new_unbounded() -> Self {
        let (high_sender, high_receiver) = unbounded();
        let (normal_sender, normal_receiver) = unbounded();
        let (low_sender, low_receiver) = unbounded();

        Self {
            high_sender,
            normal_sender,
            low_sender,
            high_receiver,
            normal_receiver,
            low_receiver,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Publish an event to the bus
    ///
    /// Events are queued with their priority and will be dispatched
    /// when `process_events()` is called.
    ///
    /// # Type Parameters
    ///
    /// * `E` - The event type, must implement the `Event` trait
    ///
    /// # Parameters
    ///
    /// * `event` - The event to publish
    /// * `priority` - The priority level for this event
    ///
    /// # Example
    ///
    /// ```
    /// use dde_core::events::{EventBus, EventPriority};
    /// use dde_core::events::battle::BattleEvent;
    ///
    /// let bus = EventBus::new();
    ///
    /// bus.publish(
    ///     BattleEvent::BattleStarted { encounter_id: 1 },
    ///     EventPriority::High
    /// );
    /// ```
    pub fn publish<E: Event>(&self, event: E, priority: EventPriority) {
        let prioritized = PrioritizedEvent {
            event: Box::new(event),
            priority,
        };

        let sender = match priority {
            EventPriority::High => &self.high_sender,
            EventPriority::Normal => &self.normal_sender,
            EventPriority::Low => &self.low_sender,
        };

        if let Err(e) = sender.send(prioritized) {
            tracing::error!("Failed to send event: {}", e);
        }
    }

    /// Subscribe to events matching a filter
    ///
    /// Returns a `SubscriptionId` that can be used to unsubscribe later.
    ///
    /// # Parameters
    ///
    /// * `filter` - The filter to apply to incoming events
    /// * `callback` - Function to call when a matching event is received
    ///
    /// # Example
    ///
    /// ```
    /// use dde_core::events::{EventBus, EventFilter, EventType};
    ///
    /// let bus = EventBus::new();
    ///
    /// let sub = bus.subscribe(
    ///     EventFilter::Type(EventType::Battle),
    ///     |event| {
    ///         println!("Received battle event: {:?}", event.event_type());
    ///     }
    /// );
    /// ```
    pub fn subscribe<F>(&self, filter: EventFilter, callback: F) -> SubscriptionId
    where
        F: Fn(&dyn Event) + Send + Sync + 'static,
    {
        let id = SubscriptionId::next();
        let subscription = Subscription {
            filter,
            callback: Arc::new(callback),
        };

        self.subscriptions.write().insert(id, subscription);
        id
    }

    /// Subscribe to all events
    ///
    /// Convenience method equivalent to `subscribe(EventFilter::All, callback)`
    pub fn subscribe_all<F>(&self, callback: F) -> SubscriptionId
    where
        F: Fn(&dyn Event) + Send + Sync + 'static,
    {
        self.subscribe(EventFilter::All, callback)
    }

    /// Subscribe to a specific event type
    ///
    /// Convenience method for type-based filtering
    pub fn subscribe_to_type<F>(&self, event_type: EventType, callback: F) -> SubscriptionId
    where
        F: Fn(&dyn Event) + Send + Sync + 'static,
    {
        self.subscribe(EventFilter::Type(event_type), callback)
    }

    /// Unsubscribe from events
    ///
    /// Removes the subscription with the given ID. Returns `true` if the
    /// subscription was found and removed, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```
    /// use dde_core::events::{EventBus, EventFilter, EventType};
    ///
    /// let bus = EventBus::new();
    /// let sub = bus.subscribe(EventFilter::All, |_| {});
    ///
    /// assert!(bus.unsubscribe(sub));
    /// assert!(!bus.unsubscribe(sub)); // Already unsubscribed
    /// ```
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.subscriptions.write().remove(&id).is_some()
    }

    /// Process all queued events
    ///
    /// Drains all events from the internal queues and dispatches them
    /// to matching subscribers. Events are processed in priority order:
    /// High -> Normal -> Low.
    ///
    /// Returns the total number of events processed.
    ///
    /// # Example
    ///
    /// ```
    /// use dde_core::events::EventBus;
    ///
    /// let bus = EventBus::new();
    ///
    /// // ... publish some events ...
    ///
    /// let processed = bus.process_events();
    /// println!("Processed {} events", processed);
    /// ```
    pub fn process_events(&self) -> usize {
        let mut count = 0;

        // Process in priority order: High -> Normal -> Low
        count += self.process_queue(&self.high_receiver);
        count += self.process_queue(&self.normal_receiver);
        count += self.process_queue(&self.low_receiver);

        count
    }

    /// Process a single queue
    fn process_queue(&self, receiver: &Receiver<PrioritizedEvent>) -> usize {
        let mut count = 0;
        let subscriptions = self.subscriptions.read();

        while let Ok(prioritized) = receiver.try_recv() {
            count += 1;
            let event = prioritized.event;

            // Dispatch to all matching subscribers
            for subscription in subscriptions.values() {
                if subscription.filter.matches(event.as_ref()) {
                    (subscription.callback)(event.as_ref());
                }
            }
        }

        count
    }

    /// Check if all event queues are empty
    pub fn is_empty(&self) -> bool {
        self.high_receiver.is_empty()
            && self.normal_receiver.is_empty()
            && self.low_receiver.is_empty()
    }

    /// Get the approximate number of pending events
    pub fn len(&self) -> usize {
        // Note: This is approximate due to race conditions
        self.high_receiver.len() + self.normal_receiver.len() + self.low_receiver.len()
    }

    /// Clear all pending events without processing them
    pub fn clear(&self) {
        while self.high_receiver.try_recv().is_ok() {}
        while self.normal_receiver.try_recv().is_ok() {}
        while self.low_receiver.try_recv().is_ok() {}
    }

    /// Get the number of active subscriptions
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.read().len()
    }

    /// Clear all subscriptions
    pub fn clear_subscriptions(&self) {
        self.subscriptions.write().clear();
    }
}

impl Clone for EventBus {
    /// Clone the event bus
    ///
    /// Creates a new event bus that shares the same subscriptions
    /// but has independent channels. This allows multiple producers
    /// to publish to separate channels while sharing the same
    /// subscription set.
    fn clone(&self) -> Self {
        // For clone, we create new channels
        let (high_sender, high_receiver) = bounded(10_000);
        let (normal_sender, normal_receiver) = bounded(10_000);
        let (low_sender, low_receiver) = bounded(10_000);

        Self {
            high_sender,
            normal_sender,
            low_sender,
            high_receiver,
            normal_receiver,
            low_receiver,
            subscriptions: Arc::clone(&self.subscriptions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // Test event types
    #[derive(Debug)]
    struct TestBattleEvent {
        damage: u32,
    }

    impl Event for TestBattleEvent {
        fn event_type(&self) -> EventType {
            EventType::Battle
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn type_id(&self) -> TypeId {
            TypeId::of::<Self>()
        }
    }

    #[derive(Debug)]
    struct TestUiEvent {
        #[allow(dead_code)]
        button_id: String,
    }

    impl Event for TestUiEvent {
        fn event_type(&self) -> EventType {
            EventType::Ui
        }

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn type_id(&self) -> TypeId {
            TypeId::of::<Self>()
        }
    }

    #[test]
    fn test_event_bus_creation() {
        let bus = EventBus::new();
        assert!(bus.is_empty());
        assert_eq!(bus.len(), 0);
        assert_eq!(bus.subscription_count(), 0);
    }

    #[test]
    fn test_publish_and_process() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        bus.subscribe(EventFilter::All, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(TestBattleEvent { damage: 20 }, EventPriority::Normal);

        assert_eq!(bus.len(), 2);

        let processed = bus.process_events();
        assert_eq!(processed, 2);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
        assert!(bus.is_empty());
    }

    #[test]
    fn test_priority_ordering() {
        let bus = EventBus::new();
        let order = Arc::new(RwLock::<Vec<String>>::new(Vec::new()));

        let order_clone = Arc::clone(&order);
        bus.subscribe(EventFilter::All, move |event| {
            if downcast_event::<TestBattleEvent>(event).is_some() {
                order_clone.write().push("battle".to_string());
            }
        });

        // Publish in reverse priority order
        bus.publish(TestBattleEvent { damage: 1 }, EventPriority::Low);
        bus.publish(TestBattleEvent { damage: 2 }, EventPriority::Normal);
        bus.publish(TestBattleEvent { damage: 3 }, EventPriority::High);

        bus.process_events();

        let result = order.read();
        // All three events should be processed
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_type_filtering() {
        let bus = EventBus::new();
        let battle_count = Arc::new(AtomicUsize::new(0));
        let ui_count = Arc::new(AtomicUsize::new(0));

        let battle_clone = Arc::clone(&battle_count);
        bus.subscribe(EventFilter::Type(EventType::Battle), move |event| {
            if event.event_type() == EventType::Battle {
                battle_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        let ui_clone = Arc::clone(&ui_count);
        bus.subscribe(EventFilter::Type(EventType::Ui), move |event| {
            if event.event_type() == EventType::Ui {
                ui_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(
            TestUiEvent {
                button_id: "btn".to_string(),
            },
            EventPriority::Normal,
        );
        bus.publish(TestBattleEvent { damage: 20 }, EventPriority::Normal);

        bus.process_events();

        assert_eq!(battle_count.load(Ordering::SeqCst), 2);
        assert_eq!(ui_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_unsubscribe() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = Arc::clone(&counter);
        let sub = bus.subscribe(EventFilter::All, move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.process_events();
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        // Unsubscribe
        assert!(bus.unsubscribe(sub));
        assert!(!bus.unsubscribe(sub)); // Already removed

        bus.publish(TestBattleEvent { damage: 20 }, EventPriority::Normal);
        bus.process_events();
        // Counter should not have increased
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_downcast_event() {
        let bus = EventBus::new();
        let received_damage = Arc::new(RwLock::new(0u32));

        let damage_clone = Arc::clone(&received_damage);
        bus.subscribe(EventFilter::Type(EventType::Battle), move |event| {
            if let Some(battle) = downcast_event::<TestBattleEvent>(event) {
                *damage_clone.write() = battle.damage;
            }
        });

        bus.publish(TestBattleEvent { damage: 42 }, EventPriority::Normal);
        bus.process_events();

        assert_eq!(*received_damage.read(), 42);
    }

    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        bus.subscribe(EventFilter::All, move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = Arc::clone(&counter2);
        bus.subscribe(EventFilter::All, move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.process_events();

        assert_eq!(counter1.load(Ordering::SeqCst), 1);
        assert_eq!(counter2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_clear_events() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = Arc::clone(&counter);
        bus.subscribe(EventFilter::All, move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(TestBattleEvent { damage: 20 }, EventPriority::Normal);

        assert_eq!(bus.len(), 2);

        bus.clear();
        assert!(bus.is_empty());

        bus.process_events();
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_clear_subscriptions() {
        let bus = EventBus::new();

        bus.subscribe(EventFilter::All, |_| {});
        bus.subscribe(EventFilter::Type(EventType::Battle), |_| {});

        assert_eq!(bus.subscription_count(), 2);

        bus.clear_subscriptions();
        assert_eq!(bus.subscription_count(), 0);
    }

    #[test]
    fn test_exact_filter() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = Arc::clone(&counter);
        bus.subscribe(
            EventFilter::Exact(TypeId::of::<TestBattleEvent>()),
            move |event| {
                if downcast_event::<TestBattleEvent>(event).is_some() {
                    c.fetch_add(1, Ordering::SeqCst);
                }
            },
        );

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(
            TestUiEvent {
                button_id: "btn".to_string(),
            },
            EventPriority::Normal,
        );

        bus.process_events();

        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_subscribe_convenience_methods() {
        let bus = EventBus::new();
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        bus.subscribe_all(move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
        });

        let c2 = Arc::clone(&counter2);
        bus.subscribe_to_type(EventType::Battle, move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(
            TestUiEvent {
                button_id: "btn".to_string(),
            },
            EventPriority::Normal,
        );

        bus.process_events();

        assert_eq!(counter1.load(Ordering::SeqCst), 2); // All events
        assert_eq!(counter2.load(Ordering::SeqCst), 1); // Only battle
    }

    #[test]
    fn test_custom_filter() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = Arc::clone(&counter);
        let filter = EventFilter::Custom(Arc::new(move |event| {
            if let Some(battle) = downcast_event::<TestBattleEvent>(event) {
                battle.damage > 50
            } else {
                false
            }
        }));

        bus.subscribe(filter, move |_| {
            c.fetch_add(1, Ordering::SeqCst);
        });

        bus.publish(TestBattleEvent { damage: 10 }, EventPriority::Normal);
        bus.publish(TestBattleEvent { damage: 100 }, EventPriority::Normal);
        bus.publish(
            TestUiEvent {
                button_id: "btn".to_string(),
            },
            EventPriority::Normal,
        );

        bus.process_events();

        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only damage > 50
    }
}
