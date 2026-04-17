use crate::{Callback, ChangeToken, Registration, State};

/// Represents a [`ChangeToken`](ChangeToken) that never changes.
#[derive(Default)]
pub struct NeverChangeToken;

impl NeverChangeToken {
    /// Initializes a new change token.
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

impl ChangeToken for NeverChangeToken {
    #[inline]
    fn changed(&self) -> bool {
        false
    }

    #[inline]
    fn must_poll(&self) -> bool {
        true
    }

    #[inline]
    fn register(&self, _callback: Callback, _state: State) -> Registration {
        Registration::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_send_and_sync;

    #[test]
    fn never_change_token_should_send_and_sync() {
        // arrange
        let token = NeverChangeToken::default();

        // act

        // assert
        assert_send_and_sync(token);
    }
}
