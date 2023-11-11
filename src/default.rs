use crate::{ChangeCallback, ChangeToken, Registration};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock, Weak,
};

/// Represents a default [`ChangeToken`](trait.ChangeToken.html) that may change zero or more times.
#[derive(Default)]
pub struct DefaultChangeToken {
    once: bool,
    changed: AtomicBool,
    callbacks: RwLock<Vec<Weak<dyn Fn() + Send + Sync>>>,
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
        let result = self
            .changed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

        if let Ok(notified) = result {
            if !notified {
                let mut alive;

                // note: acquire a read-lock and capture any callbacks that are still alive.
                // do NOT invoke the callback with the read-lock held. the callback might
                // register a new callback on the same token which will result in a deadlock.
                // invoking the callbacks after the read-lock is released ensures that won't happen.
                {
                    let callbacks = self.callbacks.read().unwrap();
                    alive = Vec::with_capacity(callbacks.len());

                    for weak in callbacks.iter() {
                        if let Some(callback) = weak.upgrade() {
                            alive.push(callback);
                        }
                    }
                }

                for callback in alive {
                    (callback)();
                }

                self.changed
                    .compare_exchange(true, self.once, Ordering::SeqCst, Ordering::SeqCst)
                    .ok();
            }
        }
    }
}

impl ChangeToken for DefaultChangeToken {
    fn changed(&self) -> bool {
        // note: this is uninteresting and unusable in sync contexts. the value
        // will be true, invoke callbacks, and then likely revert to false
        // before it can be observed. it 'might' be useful in an async context,
        // but a callback is the most practical way a change would be observed
        self.changed.load(Ordering::SeqCst)
    }

    fn register(&self, callback: ChangeCallback) -> Registration {
        let mut callbacks = self.callbacks.write().unwrap();

        // note: writes are much infrequent and we already need to escalate
        // to a write-lock, so do the trimming of any dead callbacks now
        if !callbacks.is_empty() {
            for i in (0..callbacks.len()).rev() {
                if callbacks[i].upgrade().is_none() {
                    callbacks.remove(i);
                }
            }
        }

        let source: Arc<dyn Fn() + Send + Sync> = Arc::from(callback);

        callbacks.push(Arc::downgrade(&source));
        Registration::new(source)
    }
}

unsafe impl Send for DefaultChangeToken {}
unsafe impl Sync for DefaultChangeToken {}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::{
        atomic::{AtomicU8, Ordering},
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
        let clone = counter.clone();
        let token = DefaultChangeToken::default();
        let _registration = token.register(Box::new(move || {
            clone.fetch_add(1, Ordering::SeqCst);
        }));

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn default_change_token_should_invoke_callback_multiple_times() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let clone = counter.clone();
        let token = DefaultChangeToken::default();
        let _registration = token.register(Box::new(move || {
            clone.fetch_add(1, Ordering::SeqCst);
        }));
        token.notify();

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
