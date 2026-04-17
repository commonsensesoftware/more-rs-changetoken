use crate::{Callback, ChangeToken, Registration, State};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, RwLock, Weak,
    },
    vec::IntoIter,
};

type StatefulCallback = dyn Fn(State) + Send + Sync;
type CallbackWithState = (Weak<StatefulCallback>, State);

struct Ready {
    fired: bool,
    callbacks: Vec<(Arc<StatefulCallback>, State)>,
}

impl IntoIterator for Ready {
    type Item = (Arc<StatefulCallback>, State);
    type IntoIter = IntoIter<Self::Item>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.callbacks.into_iter()
    }
}

#[derive(Default)]
struct Notification {
    fired: AtomicBool,
    callbacks: Vec<CallbackWithState>,
}

impl Notification {
    fn fire(&self, once: bool) -> Ready {
        let fired = match self.fired.compare_exchange(false, once, Relaxed, Relaxed) {
            Ok(value) => value,
            Err(value) => value,
        };

        Ready {
            fired,
            callbacks: self
                .callbacks
                .iter()
                .filter_map(|r| r.0.upgrade().map(|c| (c, r.1.clone())))
                .collect(),
        }
    }

    fn register(&mut self, callback: Callback, state: State) -> Arc<StatefulCallback> {
        for i in (0..self.callbacks.len()).rev() {
            if self.callbacks[i].0.upgrade().is_none() {
                self.callbacks.remove(i);
            }
        }

        let source: Arc<StatefulCallback> = Arc::from(callback);

        self.callbacks.push((Arc::downgrade(&source), state));
        source
    }
}

/// Represents a default [`ChangeToken`](ChangeToken) that may change zero or more times.
#[derive(Default)]
pub struct DefaultChangeToken {
    once: bool,
    notification: RwLock<Notification>,
}

impl DefaultChangeToken {
    pub(crate) fn once() -> Self {
        Self {
            once: true,
            ..Default::default()
        }
    }

    /// Initializes a new default change token.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notifies any registered callbacks of a change.
    pub fn notify(&self) {
        let notification = self.notification.read().unwrap().fire(self.once);

        if !notification.fired {
            for (callback, state) in notification {
                callback(state);
            }
        }
    }
}

impl ChangeToken for DefaultChangeToken {
    #[inline]
    fn changed(&self) -> bool {
        // always false unless self.once = true, which is used by SingleChangeToken
        self.notification.read().unwrap().fired.load(Relaxed)
    }

    fn register(&self, callback: Callback, state: State) -> Registration {
        Registration::new(self.notification.write().unwrap().register(callback, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_send_and_sync;
    use std::sync::{
        atomic::{AtomicU8, Ordering::Relaxed},
        Arc,
    };

    #[test]
    fn default_change_token_should_send_and_sync() {
        // arrange
        let token = DefaultChangeToken::default();

        // act

        // assert
        assert_send_and_sync(token);
    }

    #[test]
    fn default_change_token_should_be_unchanged() {
        // arrange
        let token = DefaultChangeToken::default();

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, false);
    }

    #[test]
    fn default_change_token_should_invoke_callback() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let token = DefaultChangeToken::default();
        let _registration = token.register(
            Box::new(|state| {
                state.unwrap().downcast_ref::<AtomicU8>().unwrap().fetch_add(1, Relaxed);
            }),
            Some(counter.clone()),
        );

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Relaxed), 1);
    }

    #[test]
    fn default_change_token_should_invoke_callback_multiple_times() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let token = DefaultChangeToken::default();
        let _registration = token.register(
            Box::new(|state| {
                state.unwrap().downcast_ref::<AtomicU8>().unwrap().fetch_add(1, Relaxed);
            }),
            Some(counter.clone()),
        );
        token.notify();

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Relaxed), 2);
    }
}
