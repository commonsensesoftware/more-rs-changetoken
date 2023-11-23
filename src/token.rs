use std::{any::Any, sync::Arc, ops::Deref};

pub type Callback = Box<dyn Fn(Option<Arc<dyn Any>>) + Send + Sync>;
type CallbackRef = Arc<dyn Fn(Option<Arc<dyn Any>>) + Send + Sync>;

/// Represents a [`ChangeToken`](crate::ChangeToken) registration.
///
/// # Remarks
///
/// When the registration is dropped, the underlying callback is unregistered.
pub struct Registration(CallbackRef);

impl Registration {
    /// Initializes a new change token registration.
    pub fn new(callback: CallbackRef) -> Self {
        Self(callback)
    }

    /// Initializes a new, empty change token registration.
    pub fn none() -> Self {
        Self::default()
    }
}

impl Default for Registration {
    fn default() -> Self {
        Self(Arc::new(|_| {}))
    }
}

/// Propagates notifications that a change has occurred.
pub trait ChangeToken: Send + Sync {
    /// Gets a value that indicates if a change has occurred.
    fn changed(&self) -> bool;

    /// Indicates if this token will proactively raise callbacks.
    ///
    /// # Remarks
    ///
    /// If `false`, the token consumer should expect for that any callback
    /// specified in [`register`](ChangeToken::register) will be invoked
    /// when a change occurs. If `false`, the token consumer must poll
    /// [`changed`](ChangeToken::changed) to detect changes.
    fn must_poll(&self) -> bool {
        false
    }

    /// Registers for a callback that will be invoked when the token has changed.
    ///
    /// # Arguments
    ///
    /// * `callback` - The callback to invoke
    /// * `state` - The optional state provided to the callback, if any
    /// 
    /// # Returns
    /// 
    /// An opaque change token [registration](Registration). When it
    /// is dropped, the callback function is unregistered.
    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration;
}

// this allows Box<dyn ChangeToken> to be used for T: ChangeToken
impl ChangeToken for Box<dyn ChangeToken> {
    fn changed(&self) -> bool {
        self.deref().changed()
    }
    
    fn must_poll(&self) -> bool {
        self.deref().must_poll()
    }

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        self.deref().register(callback, state)
    }
}