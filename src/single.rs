use crate::ChangeToken;
use std::rc::{Rc, Weak};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    RwLock,
};

#[derive(Default)]
struct TokenTrigger {
    changed: AtomicBool,
    callbacks: RwLock<Vec<Box<dyn Fn()>>>,
}

impl TokenTrigger {
    fn changed(&self) -> bool {
        self.changed.load(Ordering::SeqCst)
    }

    fn register(&self, callback: Box<dyn Fn()>) {
        self.callbacks.write().unwrap().push(callback)
    }

    fn fire(&self) {
        let result = self
            .changed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);

        if let Ok(signaled) = result {
            if !signaled {
                for callback in self.callbacks.read().unwrap().iter() {
                    (callback)()
                }
            }
        }
    }
}

/// Represents a [`ChangeToken`](trait.ChangeToken.html) that changes at most once.
pub struct SingleChangeToken {
    change: Rc<dyn Fn()>,
    trigger: Rc<TokenTrigger>,
}

impl SingleChangeToken {
    /// Initializes a new single change token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates and returns a trigger callback function for the token.
    pub fn trigger(&self) -> Weak<dyn Fn()> {
        Rc::downgrade(&self.change.clone())
    }
}

impl Default for SingleChangeToken {
    fn default() -> Self {
        let trigger = Rc::new(TokenTrigger::default());
        let clone = trigger.clone();
        let change = Rc::new(move || clone.fire());
        Self { change, trigger }
    }
}

impl ChangeToken for SingleChangeToken {
    fn changed(&self) -> bool {
        self.trigger.changed()
    }

    fn register(&self, callback: Box<dyn Fn()>) {
        self.trigger.register(callback)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::atomic::AtomicU8;

    #[test]
    fn one_time_change_token_should_be_unchanged() {
        // arrange
        let token = SingleChangeToken::default();

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, false);
    }

    #[test]
    fn one_time_change_token_should_be_changed() {
        // arrange
        let token = SingleChangeToken::default();
        let trigger = token.trigger().upgrade().unwrap();

        // act
        (trigger)();

        // assert
        assert_eq!(token.changed(), true);
    }

    #[test]
    fn one_time_change_token_should_invoke_callback() {
        // arrange
        let counter = Rc::new(AtomicU8::default());
        let clone = counter.clone();
        let token = SingleChangeToken::default();
        let trigger = token.trigger().upgrade().unwrap();

        token.register(Box::new(move || {
            clone.fetch_add(1, Ordering::Relaxed);
        }));

        // act
        (trigger)();

        // assert
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn one_time_change_token_should_not_invoke_callback_more_than_once() {
        // arrange
        let counter = Rc::new(AtomicU8::default());
        let clone = counter.clone();
        let token = SingleChangeToken::default();
        let trigger = token.trigger().upgrade().unwrap();

        token.register(Box::new(move || {
            clone.fetch_add(1, Ordering::Relaxed);
        }));
        (trigger)();

        // act
        (trigger)();

        // assert
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
