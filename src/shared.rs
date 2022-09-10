use crate::{ChangeToken, SingleChangeToken};
use std::{rc::Rc, sync::Arc, ops::Deref};

/// Represents a shared change token.
pub struct SharedChangeToken<T: ChangeToken = SingleChangeToken> {
    inner: Rc<T>,
}

impl<T: ChangeToken> Deref for SharedChangeToken<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner    
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
            inner: Rc::new(token),
        }
    }
}

impl<T: ChangeToken + Default> Default for SharedChangeToken<T> {
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: ChangeToken> ChangeToken for SharedChangeToken<T> {
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    fn must_poll(&self) -> bool {
        self.inner.must_poll()
    }

    fn register(&self, callback: Box<dyn Fn()>) {
        self.inner.register(callback)
    }
}

/// Represents an asynchronously shared change token.
pub struct AsyncSharedChangeToken<T: ChangeToken = SingleChangeToken> {
    inner: Arc<T>,
}

impl<T: ChangeToken> From<T> for AsyncSharedChangeToken<T> {
    fn from(token: T) -> Self {
        Self {
            inner: Arc::new(token),
        }
    }
}

impl<T: ChangeToken + Default> Default for AsyncSharedChangeToken<T> {
    fn default() -> Self {
        Self::from(T::default())
    }
}

impl<T: ChangeToken> ChangeToken for AsyncSharedChangeToken<T> {
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    fn must_poll(&self) -> bool {
        self.inner.must_poll()
    }

    fn register(&self, callback: Box<dyn Fn()>) {
        self.inner.register(callback)
    }
}

impl<T: ChangeToken> Clone for AsyncSharedChangeToken<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: ChangeToken> Deref for AsyncSharedChangeToken<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner    
    }
}

unsafe impl<T: ChangeToken> Send for AsyncSharedChangeToken<T> {}
unsafe impl<T: ChangeToken> Sync for AsyncSharedChangeToken<T> {}
