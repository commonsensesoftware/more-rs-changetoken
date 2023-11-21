use crate::{ChangeToken, Registration, Subscription};
use std::{
    any::Any,
    sync::{Arc, Mutex, Weak},
};

/// Registers a consumer action to be invoked whenever the token produced changes.
///
/// # Arguments
///
/// * `producer` - The function that produces the change token
/// * `consumer` - The function that is called when the change token changes
/// * `state` - The optional state supplied to the consumer, if any
pub fn on_change<TToken, TProducer, TConsumer, TState>(
    producer: TProducer,
    consumer: TConsumer,
    state: Option<Arc<TState>>,
) -> impl Subscription
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
    SubscriptionImpl(ChangeTokenRegistration::new(producer, consumer, state))
}

struct ChangeTokenRegistration<TToken, TProducer, TConsumer, TState>
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
    me: Weak<Self>,
    producer: TProducer,
    consumer: TConsumer,
    state: Option<Arc<TState>>,

    // we are mediating between the producer and consumer so we need to hold
    // onto the current ChangeToken and Registration for the callback function.
    // these are both dropped when this mediated registration is itself dropped.
    registration: Mutex<(Option<TToken>, Registration)>,
}

impl<TToken, TProducer, TConsumer, TState>
    ChangeTokenRegistration<TToken, TProducer, TConsumer, TState>
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
    fn new(producer: TProducer, consumer: TConsumer, state: Option<Arc<TState>>) -> Arc<Self> {
        let token = (producer)();
        let instance = Arc::new_cyclic(|me| Self {
            me: me.clone(),
            producer,
            consumer,
            state,
            registration: Default::default(),
        });

        instance.register(token);
        instance
    }

    fn register(&self, token: TToken) {
        let this = Arc::new(self.me.clone());
        let registration = token.register(Box::new(Self::on_changed), Some(this));

        // only update the registration if the token hasn't
        // already changed and it doesn't require polling.
        // the old token and registration are immediately dropped
        if !token.changed() || token.must_poll() {
            *self.registration.lock().unwrap() = (Some(token), registration);
        }
    }

    fn on_changed(state: Option<Arc<dyn Any>>) {
        state
            .unwrap()
            .downcast_ref::<Weak<Self>>()
            .unwrap()
            .upgrade()
            .unwrap()
            .on_notified()
    }

    fn on_notified(&self) {
        let token = (self.producer)();
        (self.consumer)(self.state.clone());
        self.register(token);
    }
}

struct SubscriptionImpl<TToken, TProducer, TConsumer, TState>(
    Arc<ChangeTokenRegistration<TToken, TProducer, TConsumer, TState>>,
)
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static;

impl<TToken, TProducer, TConsumer, TState> Subscription
    for SubscriptionImpl<TToken, TProducer, TConsumer, TState>
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
}

unsafe impl<TToken, TProducer, TConsumer, TState> Send
    for ChangeTokenRegistration<TToken, TProducer, TConsumer, TState>
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
}

unsafe impl<TToken, TProducer, TConsumer, TState> Sync
    for ChangeTokenRegistration<TToken, TProducer, TConsumer, TState>
where
    TState: 'static,
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn(Option<Arc<TState>>) + Send + Sync + 'static,
{
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::*;
    use std::{
        mem::ManuallyDrop,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    #[test]
    fn changed_should_signal_consumer() {
        // arrange
        let token = SharedChangeToken::<DefaultChangeToken>::default();
        let fired = Arc::new(AtomicBool::default());
        let producer = token.clone();
        let _unused = on_change(
            move || producer.clone(),
            |state| state.unwrap().store(true, Ordering::SeqCst),
            Some(fired.clone()),
        );

        // act
        token.notify();

        // assert
        assert!(fired.load(Ordering::SeqCst));
    }

    #[test]
    fn changed_should_not_signal_consumer_after_registration_is_dropped() {
        // arrange
        let token = SharedChangeToken::<SingleChangeToken>::default();
        let fired = Arc::new(AtomicBool::default());
        let producer = token.clone();
        let subscription = ManuallyDrop::new(on_change(
            move || producer.clone(),
            |state| state.unwrap().store(true, Ordering::SeqCst),
            Some(fired.clone()),
        ));

        // act
        let _ = ManuallyDrop::into_inner(subscription);
        token.notify();

        // assert
        assert!(!fired.load(Ordering::SeqCst));
    }
}
