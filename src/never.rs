use crate::{Callback, ChangeToken, Registration};
use std::{any::Any, sync::Arc};

/// Represents a [`ChangeToken`](crate::ChangeToken) that never changes.
#[derive(Default)]
pub struct NeverChangeToken;

impl NeverChangeToken {
    /// Initializes a new change token.
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChangeToken for NeverChangeToken {
    fn changed(&self) -> bool {
        false
    }

    fn must_poll(&self) -> bool {
        true
    }

    fn register(&self, _callback: Callback, _state: Option<Arc<dyn Any>>) -> Registration {
        Registration::none()
    }
}

unsafe impl Send for NeverChangeToken {}
unsafe impl Sync for NeverChangeToken {}
