use crate::ChangeToken;

/// Represents a change token that never changes.
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
        false
    }

    fn register(&self, _callback: Box<dyn Fn()>) {}
}
