use crate::{Callback, ChangeToken, DefaultChangeToken, Registration};
use std::{borrow::Borrow, ops::Deref, sync::Arc, any::Any};

/// Represents a shared [`ChangeToken`](trait.ChangeToken.html).
pub struct SharedChangeToken<T: ChangeToken = DefaultChangeToken> {
    inner: Arc<T>,
}

impl<T: ChangeToken> SharedChangeToken<T> {
    /// Initializes a new shared change token.
    /// 
    /// # Arguments
    /// 
    /// * `token` - The [`ChangeToken`](trait.ChangeToken.html) to be shared.
    pub fn new(token: T) -> Self {
        Self::from(token)
    }
}

impl<T: ChangeToken + Default> Default for SharedChangeToken<T> {
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: ChangeToken> Clone for SharedChangeToken<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ChangeToken> From<T> for SharedChangeToken<T> {
    fn from(token: T) -> Self {
        Self {
            inner: Arc::new(token),
        }
    }
}

impl<T: ChangeToken> ChangeToken for SharedChangeToken<T> {
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    fn must_poll(&self) -> bool {
        self.inner.must_poll()
    }

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        self.inner.register(callback, state)
    }
}

impl<T: ChangeToken> AsRef<T> for SharedChangeToken<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T: ChangeToken> Deref for SharedChangeToken<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: ChangeToken> Borrow<T> for SharedChangeToken<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

unsafe impl<T: ChangeToken> Send for SharedChangeToken<T> {}
unsafe impl<T: ChangeToken> Sync for SharedChangeToken<T> {}
