use crate::{ChangeToken, Registration};
use std::sync::{Arc, Mutex, Weak};

/// Registers a consumer action to be invoked whenever the token produced changes.
///
/// # Arguments
///
/// * `producer` - The function that produces the change token
/// * `consumer` - The function that is called when the change token changes
pub fn on_change<TToken, TProducer, TConsumer>(
    producer: TProducer,
    consumer: TConsumer,
) -> impl Drop
where
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn() + Send + Sync + 'static,
{
    ChangeTokenRegistration::new(producer, consumer)
}

struct ChangeTokenRegistration<TToken, TProducer, TConsumer>
where
    TToken: ChangeToken,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn() + Send + Sync + 'static,
{
    me: Weak<Self>,
    producer: TProducer,
    consumer: TConsumer,

    // we are mediating between the producer and consumer so we need to hold
    // onto the current ChangeToken and Registration for the callback function.
    // these are both dropped when this mediated registration is itself dropped.
    registration: Mutex<(Option<TToken>, Registration)>,
}

impl<TToken, TProducer, TConsumer> ChangeTokenRegistration<TToken, TProducer, TConsumer>
where
    TToken: ChangeToken + 'static,
    TProducer: Fn() -> TToken + Send + Sync + 'static,
    TConsumer: Fn() + Send + Sync + 'static,
{
    fn new(producer: TProducer, consumer: TConsumer) -> Arc<Self> {
        Arc::new_cyclic(|me| {
            let token = (producer)();
            let instance = Self {
                me: me.clone(),
                producer,
                consumer,
                registration: Default::default(),
            };
            instance.register(token);
            instance
        })
    }

    fn register(&self, token: TToken) {
        let this = self.me.clone();
        let registration = token.register(Box::new(move || this.upgrade().unwrap().on_notified()));

        // only update the registration if the token hasn't
        // already changed and it doesn't require polling.
        // the old token and registration are immediately dropped
        if !token.changed() || token.must_poll() {
            *self.registration.lock().unwrap() = (Some(token), registration);
        }
    }

    fn on_notified(&self) {
        let token = (self.producer)();
        (self.consumer)();
        self.register(token);
    }
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
        let consumer = fired.clone();
        let _unused = on_change(
            move || producer.clone(),
            move || consumer.store(true, Ordering::SeqCst),
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
        let consumer = fired.clone();
        let registration = ManuallyDrop::new(on_change(
            move || producer.clone(),
            move || consumer.store(true, Ordering::SeqCst),
        ));

        // act
        let _ = ManuallyDrop::into_inner(registration);
        token.notify();

        // assert
        assert!(!fired.load(Ordering::SeqCst));
    }
}
