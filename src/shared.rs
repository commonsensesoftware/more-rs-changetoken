use crate::{Callback, ChangeToken, DefaultChangeToken, Registration, State};
use std::{borrow::Borrow, ops::Deref, sync::Arc};

/// Represents a shared [`ChangeToken`](ChangeToken).
pub struct SharedChangeToken<T: ChangeToken = DefaultChangeToken>(Arc<T>);

impl<T: ChangeToken> SharedChangeToken<T> {
    /// Initializes a new shared change token.
    ///
    /// # Arguments
    ///
    /// * `token` - The [`ChangeToken`](ChangeToken) to be shared
    #[inline]
    pub fn new(token: T) -> Self {
        Self::from(token)
    }
}

impl<T: ChangeToken + Default> Default for SharedChangeToken<T> {
    #[inline]
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: ChangeToken> Clone for SharedChangeToken<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ChangeToken> From<T> for SharedChangeToken<T> {
    #[inline]
    fn from(token: T) -> Self {
        Self(Arc::new(token))
    }
}

impl<T: ChangeToken> ChangeToken for SharedChangeToken<T> {
    #[inline]
    fn changed(&self) -> bool {
        self.0.changed()
    }

    #[inline]
    fn must_poll(&self) -> bool {
        self.0.must_poll()
    }

    #[inline]
    fn register(&self, callback: Callback, state: State) -> Registration {
        self.0.register(callback, state)
    }
}

impl<T: ChangeToken> AsRef<T> for SharedChangeToken<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        &self.0
    }
}

impl<T: ChangeToken> Deref for SharedChangeToken<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ChangeToken> Borrow<T> for SharedChangeToken<T> {
    #[inline]
    fn borrow(&self) -> &T {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assert_send_and_sync;

    #[test]
    fn shared_change_token_should_send_and_sync() {
        // arrange
        let token = SharedChangeToken::<DefaultChangeToken>::default();

        // act

        // assert
        assert_send_and_sync(token);
    }
}
