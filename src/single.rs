use crate::{Callback, ChangeToken, DefaultChangeToken, Registration, State};

/// Represents a [`ChangeToken`](ChangeToken) that changes at most once.
pub struct SingleChangeToken(DefaultChangeToken);

impl SingleChangeToken {
    /// Initializes a new single change token.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notifies any registered callbacks of a change.
    ///
    /// # Remarks
    ///
    /// Registered callbacks will be invoked exactly once. Invoking this function called more than once does nothing.
    #[inline]
    pub fn notify(&self) {
        self.0.notify()
    }
}

impl Default for SingleChangeToken {
    #[inline]
    fn default() -> Self {
        Self(DefaultChangeToken::once())
    }
}

impl ChangeToken for SingleChangeToken {
    #[inline]
    fn changed(&self) -> bool {
        self.0.changed()
    }

    #[inline]
    fn register(&self, callback: Callback, state: State) -> Registration {
        self.0.register(callback, state)
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
    fn single_change_token_should_send_and_sync() {
        // arrange
        let token = SingleChangeToken::default();

        // act

        // assert
        assert_send_and_sync(token);
    }

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
    fn single_change_token_should_not_invoke_callback_more_than_once() {
        // arrange
        let counter = Arc::new(AtomicU8::default());
        let token = SingleChangeToken::default();
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
        assert_eq!(counter.load(Relaxed), 1);
    }
}
