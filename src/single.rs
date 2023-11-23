use crate::{Callback, ChangeToken, DefaultChangeToken, Registration};
use std::{any::Any, sync::Arc};

/// Represents a [`ChangeToken`](crate::ChangeToken) that changes at most once.
pub struct SingleChangeToken {
    inner: DefaultChangeToken,
}

impl SingleChangeToken {
    /// Initializes a new single change token.
    pub fn new() -> Self {
        Self::default()
    }

    /// Notifies any registered callbacks of a change.
    ///
    /// # Remarks
    ///
    /// Registered callbacks will be invoked exactly once. If this function is called more
    /// than once, no action is performed.
    pub fn notify(&self) {
        self.inner.notify()
    }
}

impl Default for SingleChangeToken {
    fn default() -> Self {
        Self {
            inner: DefaultChangeToken::once(),
        }
    }
}

impl ChangeToken for SingleChangeToken {
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        self.inner.register(callback, state)
    }
}

unsafe impl Send for SingleChangeToken {}
unsafe impl Sync for SingleChangeToken {}

#[cfg(test)]
mod tests {

    use super::*;
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    };

    #[test]
    fn single_change_token_should_be_unchanged() {
        // arrange
        let token = SingleChangeToken::default();

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, false);
    }

    #[test]
    fn single_change_token_should_be_changed() {
        // arrange
        let token = SingleChangeToken::default();

        // act
        token.notify();

        // assert
        assert_eq!(token.changed(), true);
    }

    #[test]
    fn single_change_token_should_invoke_callback() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let token = SingleChangeToken::default();
        let _registration = token.register(
            Box::new(|state| {
                state.unwrap().downcast_ref::<AtomicU8>().unwrap().fetch_add(1, Ordering::SeqCst);
            }),
            Some(counter.clone()),
        );

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn single_change_token_should_not_invoke_callback_more_than_once() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let token = SingleChangeToken::default();
        let _registration = token.register(
            Box::new(|state| {
                state.unwrap().downcast_ref::<AtomicU8>().unwrap().fetch_add(1, Ordering::SeqCst);
            }),
            Some(counter.clone()),
        );
        token.notify();

        // act
        token.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
