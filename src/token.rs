use std::sync::Arc;

pub type ChangeCallback = Box<dyn Fn() + Send + Sync>;

/// Represents a [change token](trait.ChangeToken.html) registration.
///
/// # Remarks
///
/// When the registration is dropped, the underlying callback is unregistered.
pub struct Registration(Arc<dyn Fn() + Send + Sync>);

impl Registration {
    /// Initializes a new change token registration.
    pub fn new(callback: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self(callback)
    }

    /// Initializes a new, empty change token registration.
    pub fn none() -> Self {
        Self::default()
    }
}

impl Default for Registration {
    fn default() -> Self {
        Self(Arc::new(|| {}))
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
    /// * `callback` - the callback to invoke
    fn register(&self, callback: ChangeCallback) -> Registration;
}
