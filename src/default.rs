use crate::{Callback, ChangeToken, Registration};
use std::{
    any::Any,
    mem,
    sync::{Arc, Mutex, Weak},
};

type State = Option<Arc<dyn Any>>;
type StatefulCallback = dyn Fn(State) + Send + Sync;
type CallbackWithState = (Weak<StatefulCallback>, State);

struct Ready {
    fired: bool,
    callbacks: Vec<(Arc<StatefulCallback>, State)>,
}

impl IntoIterator for Ready {
    type Item = (Arc<StatefulCallback>, State);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.callbacks.into_iter()
    }
}

#[derive(Default)]
struct Notification {
    fired: bool,
    callbacks: Vec<CallbackWithState>,
}

impl Notification {
    fn fire(&mut self, once: bool) -> Ready {
        Ready {
            fired: mem::replace(&mut self.fired, once),
            callbacks: self
                .callbacks
                .iter()
                .filter_map(|r| r.0.upgrade().map(|c| (c, r.1.clone())))
                .collect(),
        }
    }

    fn register(
        &mut self,
        callback: Callback,
        state: Option<Arc<dyn Any>>,
    ) -> Arc<StatefulCallback> {
        // writes are much infrequent, so do the trimming of any dead callbacks now
        if !self.callbacks.is_empty() {
            for i in (0..self.callbacks.len()).rev() {
                if self.callbacks[i].0.upgrade().is_none() {
                    self.callbacks.remove(i);
                }
            }
        }

        let source: Arc<StatefulCallback> = Arc::from(callback);

        self.callbacks.push((Arc::downgrade(&source), state));
        source
    }
}

/// Represents a default [`ChangeToken`](crate::ChangeToken) that may change zero or more times.
#[derive(Default)]
pub struct DefaultChangeToken {
    once: bool,
    notification: Mutex<Notification>,
}

impl DefaultChangeToken {
    pub(crate) fn once() -> Self {
        Self {
            once: true,
            ..Default::default()
        }
    }

    /// Initializes a new default change token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Notifies any registered callbacks of a change.
    pub fn notify(&self) {
        let notification = self.notification.lock().unwrap().fire(self.once);

        if !notification.fired {
            for (callback, state) in notification {
                callback(state);
            }
        }
    }
}

impl ChangeToken for DefaultChangeToken {
    fn changed(&self) -> bool {
        // this is uninteresting and unusable in sync contexts. the value
        // will be true, invoke callbacks, and then likely revert to false
        // before it can be observed. it 'might' be useful in an async context,
        // but a callback is the most practical way a change would be observed
        self.notification.lock().unwrap().fired
    }

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        Registration::new(self.notification.lock().unwrap().register(callback, state))
    }
}

unsafe impl Send for DefaultChangeToken {}
unsafe impl Sync for DefaultChangeToken {}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::{
        atomic::{AtomicU8, Ordering::Relaxed},
        Arc,
    };

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
                state
                    .unwrap()
                    .downcast_ref::<AtomicU8>()
                    .unwrap()
                    .fetch_add(1, Relaxed);
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
                state
                    .unwrap()
                    .downcast_ref::<AtomicU8>()
                    .unwrap()
                    .fetch_add(1, Relaxed);
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
