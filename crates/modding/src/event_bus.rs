//! Event bus for mod communication.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use mclre_mod_api::{EventData, EventType, ModApiError, ModResult};
use parking_lot::Mutex;
use thiserror::Error;

/// Errors that can occur in the event bus
#[derive(Debug, Error)]
pub enum EventError {
    #[error("Event type not registered: {0:?}")]
    EventTypeNotRegistered(EventType),
    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),
    #[error("Event handler panicked: {0}")]
    HandlerPanicked(String),
    #[error("Mod not loaded: {0}")]
    ModNotLoaded(String),
}

/// Subscription ID for managing event subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionId(pub u64);

/// Event subscription handle
pub struct EventSubscription {
    id: SubscriptionId,
    event_type: EventType,
    mod_id: String,
    callback: Arc<dyn Fn(&dyn EventData) -> ModResult<()> + Send + Sync>,
}

/// Typed event bus for mod communication
pub struct EventBus {
    subscriptions: RwLock<HashMap<EventType, HashMap<SubscriptionId, EventSubscription>>>,
    mod_subscriptions: RwLock<HashMap<String, HashSet<SubscriptionId>>>,
    next_id: Mutex<u64>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
            mod_subscriptions: RwLock::new(HashMap::new()),
            next_id: Mutex::new(1),
        }
    }

    /// Subscribe to an event type
    pub fn subscribe<F>(&self, mod_id: &str, event_type: EventType, callback: F) -> SubscriptionId
    where
        F: Fn(&dyn EventData) -> ModResult<()> + Send + Sync + 'static,
    {
        let id = {
            let mut next = self.next_id.lock();
            let id = *next;
            *next += 1;
            SubscriptionId(id)
        };

        let subscription = EventSubscription {
            id,
            event_type,
            mod_id: mod_id.to_string(),
            callback: Arc::new(callback),
        };

        // Add to event type subscriptions
        self.subscriptions
            .write()
            .expect("RwLock poisoned")
            .entry(event_type)
            .or_default()
            .insert(
                subscription.id,
                EventSubscription {
                    id: subscription.id,
                    event_type: subscription.event_type,
                    mod_id: subscription.mod_id.clone(),
                    callback: subscription.callback.clone(),
                },
            );

        // Track by mod
        self.mod_subscriptions
            .write()
            .expect("RwLock poisoned")
            .entry(mod_id.to_string())
            .or_default()
            .insert(subscription.id);

        subscription.id
    }

    /// Unsubscribe from an event
    pub fn unsubscribe(&self, mod_id: &str, subscription_id: SubscriptionId) -> bool {
        let mut removed = false;

        // Remove from event type subscriptions
        self.subscriptions
            .write()
            .expect("RwLock poisoned")
            .values_mut()
            .for_each(|map| {
                if map.remove(&subscription_id).is_some() {
                    removed = true;
                }
            });

        // Remove from mod tracking
        self.mod_subscriptions
            .write()
            .expect("RwLock poisoned")
            .values_mut()
            .for_each(|set| {
                set.remove(&subscription_id);
            });

        removed
    }

    /// Unsubscribe all subscriptions for a mod
    pub fn unsubscribe_all(&self, mod_id: &str) {
        if let Some(ids) = self
            .mod_subscriptions
            .write()
            .expect("RwLock poisoned")
            .remove(mod_id)
        {
            for id in ids {
                self.subscriptions
                    .write()
                    .expect("RwLock poisoned")
                    .values_mut()
                    .for_each(|map| {
                        map.remove(&id);
                    });
            }
        }
    }

    /// Emit an event to all subscribers
    pub fn emit(&self, event: &dyn EventData) -> Vec<ModResult<()>> {
        let event_type = event.event_type();
        let mut results = Vec::new();

        let subscriptions = self.subscriptions.read().expect("RwLock poisoned");
        if let Some(subs) = subscriptions.get(&event.event_type()) {
            for sub in subs.values() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    (sub.callback)(event)
                }));

                match result {
                    Ok(result) => results.push(result),
                    Err(e) => {
                        tracing::error!(
                            "Event handler for {:?} panicked: {:?}",
                            event.event_type(),
                            e
                        );
                        results.push(Err(ModApiError::Internal("Handler panicked".into())));
                    }
                }
            }
        }

        results
    }

    /// Emit an event asynchronously (non-blocking)
    pub fn emit_async(&self, event: Arc<dyn EventData + Send + Sync>) {
        let event_type = event.event_type();
        let subscriptions = self.subscriptions.read().expect("RwLock poisoned");

        if let Some(subs) = subscriptions.get(&event_type) {
            for sub in subs.values() {
                let event_clone = event.clone();
                let callback = sub.callback.clone();

                // In a real implementation, this would be spawned on a thread pool
                // For now, just call directly
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _ = (callback)(event_clone.as_ref());
                }));
            }
        }
    }

    /// Get all subscriptions for an event type
    pub fn get_subscriptions(&self, event_type: EventType) -> Vec<SubscriptionId> {
        self.subscriptions
            .read()
            .expect("RwLock poisoned")
            .get(&event_type)
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default()
    }

    /// Get all subscriptions for a mod
    pub fn get_mod_subscriptions(&self, mod_id: &str) -> Vec<SubscriptionId> {
        self.mod_subscriptions
            .read()
            .expect("RwLock poisoned")
            .get(mod_id)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get count of subscriptions for an event type
    pub fn subscription_count(&self, event_type: EventType) -> usize {
        self.subscriptions
            .read()
            .expect("RwLock poisoned")
            .get(&event_type)
            .map(|map| map.len())
            .unwrap_or(0)
    }

    /// Clear all subscriptions (e.g., on server shutdown)
    pub fn clear(&self) {
        self.subscriptions.write().expect("RwLock poisoned").clear();
        self.mod_subscriptions
            .write()
            .expect("RwLock poisoned")
            .clear();
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mclre_mod_api::{EventData, EventType, ModApiError, ModResult};
    use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    struct TestEvent {
        pub value: i32,
    }

    impl EventData for TestEvent {
        fn event_type(&self) -> EventType {
            EventType::Tick
        }
    }

    #[test]
    fn test_subscribe_and_emit() {
        let bus = EventBus::new();
        let call_count = Arc::new(AtomicI32::new(0));
        let callback_count = Arc::clone(&call_count);

        let _id = bus.subscribe("test_mod", EventType::Tick, move |event: &dyn EventData| {
            let test_event = (event as &dyn std::any::Any)
                .downcast_ref::<TestEvent>()
                .expect("expected TestEvent");
            callback_count.fetch_add(test_event.value, Ordering::SeqCst);
            Ok(())
        });

        let event = TestEvent { value: 42 };
        let results = bus.emit(&event);

        assert_eq!(call_count.load(Ordering::SeqCst), 42);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }

    #[test]
    fn test_multiple_subscribers() {
        let bus = EventBus::new();
        let count1 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&count1);
        bus.subscribe("mod1", EventType::Tick, move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let c2 = Arc::clone(&count2);
        bus.subscribe("mod2", EventType::Tick, move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        let event = TestEvent { value: 1 };
        let results = bus.emit(&event);

        assert_eq!(count1.load(Ordering::SeqCst), 1);
        assert_eq!(count2.load(Ordering::SeqCst), 1);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_unsubscribe() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        let callback_count = Arc::clone(&count);

        let id = bus.subscribe("mod1", EventType::Tick, move |_| {
            callback_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        bus.emit(&TestEvent { value: 1 });
        assert_eq!(count.load(Ordering::SeqCst), 1);

        bus.unsubscribe("mod1", id);
        bus.emit(&TestEvent { value: 1 });
        assert_eq!(count.load(Ordering::SeqCst), 1); // Should not increment
    }

    #[test]
    fn test_unsubscribe_all() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&count);
        bus.subscribe("mod1", EventType::Tick, move |_| {
            c1.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let c2 = Arc::clone(&count);
        bus.subscribe("mod1", EventType::BlockBreak, move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        bus.emit(&TestEvent { value: 1 });
        assert_eq!(count.load(Ordering::SeqCst), 1);

        bus.unsubscribe_all("mod1");
        bus.emit(&TestEvent { value: 1 });
        assert_eq!(count.load(Ordering::SeqCst), 1); // Should not increment
    }

    #[test]
    fn test_multiple_event_types() {
        let bus = EventBus::new();
        let tick_count = Arc::new(AtomicUsize::new(0));
        let break_count = Arc::new(AtomicUsize::new(0));

        let t = Arc::clone(&tick_count);
        bus.subscribe("mod1", EventType::Tick, move |_| {
            t.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        let b = Arc::clone(&break_count);
        bus.subscribe("mod1", EventType::BlockBreak, move |_| {
            b.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });

        bus.emit(&TestEvent { value: 1 }); // Tick event
        assert_eq!(tick_count.load(Ordering::SeqCst), 1);
        assert_eq!(break_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_handler_panic() {
        let bus = EventBus::new();
        let count = Arc::new(AtomicUsize::new(0));
        let callback_count = Arc::clone(&count);

        bus.subscribe("mod1", EventType::Tick, move |_| {
            callback_count.fetch_add(1, Ordering::SeqCst);
            panic!("Test panic");
        });

        let results = bus.emit(&TestEvent { value: 1 });

        // Should catch panic and return error
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
        assert_eq!(count.load(Ordering::SeqCst), 1); // Should have been called before panic
    }
}
