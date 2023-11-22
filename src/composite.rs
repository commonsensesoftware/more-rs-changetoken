use crate::{Callback, ChangeToken, Registration, SharedChangeToken, SingleChangeToken};
use std::{
    any::Any,
    sync::{Arc, Weak},
};

struct Mediator {
    parent: SharedChangeToken<SingleChangeToken>,
    children: Vec<Box<dyn ChangeToken>>,
    _registrations: Vec<Registration>,
}

impl Mediator {
    fn new(
        parent: SharedChangeToken<SingleChangeToken>,
        tokens: Vec<Box<dyn ChangeToken>>,
    ) -> Arc<Self> {
        Arc::new_cyclic(|me| {
            let registrations = Self::register(me, &tokens);
            Self {
                parent,
                children: tokens,
                _registrations: registrations,
            }
        })
    }

    fn register(me: &Weak<Self>, tokens: &[Box<dyn ChangeToken>]) -> Vec<Registration> {
        let mut registrations = Vec::with_capacity(tokens.len());

        for token in tokens {
            if token.must_poll() {
                continue;
            }

            let registration = token.register(
                Box::new(|state| {
                    let weak = state.unwrap();
                    if let Some(this) = weak.downcast_ref::<Weak<Self>>().unwrap().upgrade() {
                        this.parent.notify();
                    }
                }),
                Some(Arc::new(me.clone())),
            );

            registrations.push(registration);
        }

        registrations
    }
}

/// Represents a composition of one or more [`ChangeToken`](trait.ChangeToken.html) instances.
pub struct CompositeChangeToken {
    inner: SharedChangeToken<SingleChangeToken>,
    mediator: Arc<Mediator>,
}

impl CompositeChangeToken {
    /// Initializes a new composite change token.
    ///
    /// # Arguments
    ///
    /// * `tokens` - A sequence of [`ChangeToken`](trait.ChangeToken.html) instances.
    pub fn new(tokens: impl Iterator<Item = Box<dyn ChangeToken>>) -> Self {
        let inner = SharedChangeToken::<SingleChangeToken>::default();
        let shared: SharedChangeToken<SingleChangeToken> = inner.clone();
        Self {
            inner,
            mediator: Mediator::new(shared, tokens.collect()),
        }
    }

    /// Notifies any registered callbacks of a change.
    pub fn notify(&self) {
        self.inner.notify()
    }
}

impl ChangeToken for CompositeChangeToken {
    fn changed(&self) -> bool {
        self.inner.changed() || self.mediator.children.iter().any(|t| t.changed())
    }

    fn must_poll(&self) -> bool {
        self.mediator.children.iter().all(|t| t.must_poll())
    }

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        self.inner.register(callback, state)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::*;
    use std::iter::empty;
    use std::sync::{
        atomic::{AtomicU8, Ordering},
        Arc,
    };

    #[test]
    fn changed_should_be_false_when_no_changes_have_occurred() {
        // arrange
        let token = CompositeChangeToken::new(empty());

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, false);
    }

    #[test]
    fn changed_should_be_true_if_child_has_changed() {
        // arrange
        let child = SingleChangeToken::new();

        child.notify();

        let child: Box<dyn ChangeToken> = Box::new(child);
        let tokens = vec![child];
        let token = CompositeChangeToken::new(tokens.into_iter());

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, true);
    }

    #[test]
    fn changed_should_be_true_if_ever_notified() {
        // arrange
        let child: Box<dyn ChangeToken> = Box::new(SingleChangeToken::new());
        let tokens = vec![child];
        let token = CompositeChangeToken::new(tokens.into_iter());

        token.notify();

        // act
        let changed = token.changed();

        // assert
        assert_eq!(changed, true);
    }

    #[test]
    fn must_poll_should_be_true_all_tokens_do_not_support_callbacks() {
        // arrange
        let child: Box<dyn ChangeToken> = Box::new(NeverChangeToken::new());
        let tokens = vec![child];
        let token = CompositeChangeToken::new(tokens.into_iter());

        // act
        let poll_required = token.must_poll();

        // assert
        assert_eq!(poll_required, true);
    }

    #[test]
    fn must_poll_should_be_false_if_at_least_one_token_supports_callbacks() {
        // arrange
        let tokens: Vec<Box<dyn ChangeToken>> = vec![
            Box::new(NeverChangeToken::new()),
            Box::new(SingleChangeToken::new()),
        ];
        let token = CompositeChangeToken::new(tokens.into_iter());

        // act
        let poll_required = token.must_poll();

        // assert
        assert_eq!(poll_required, false);
    }

    #[test]
    fn child_should_trigger_parent_callbacks() {
        // arrange
        let child = SharedChangeToken::<DefaultChangeToken>::default();
        let tokens: Vec<Box<dyn ChangeToken>> = vec![Box::new(child.clone())];
        let token = CompositeChangeToken::new(tokens.into_iter());
        let counter = Arc::new(AtomicU8::default());
        let _registration = token.register(
            Box::new(|state| {
                state
                    .unwrap()
                    .downcast_ref::<AtomicU8>()
                    .unwrap()
                    .fetch_add(1, Ordering::SeqCst);
            }),
            Some(counter.clone()),
        );

        // act
        child.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn child_should_not_trigger_callbacks_multiple_times() {
        // arrange
        let child = SharedChangeToken::<DefaultChangeToken>::default();
        let tokens: Vec<Box<dyn ChangeToken>> = vec![Box::new(child.clone())];
        let token = CompositeChangeToken::new(tokens.into_iter());
        let counter = Arc::new(AtomicU8::default());
        let _registration = token.register(
            Box::new(|state| {
                state
                    .unwrap()
                    .downcast_ref::<AtomicU8>()
                    .unwrap()
                    .fetch_add(1, Ordering::SeqCst);
            }),
            Some(counter.clone()),
        );

        child.notify();

        // act
        child.notify();

        // assert
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn notify_should_not_trigger_callbacks_multiple_times() {
        // arrange
        let child: Box<dyn ChangeToken> = Box::new(NeverChangeToken::new());
        let tokens = vec![child];
        let token = CompositeChangeToken::new(tokens.into_iter());
        let counter = Arc::new(AtomicU8::default());
        let _registration = token.register(
            Box::new(|state| {
                state
                    .unwrap()
                    .downcast_ref::<AtomicU8>()
                    .unwrap()
                    .fetch_add(1, Ordering::SeqCst);
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
