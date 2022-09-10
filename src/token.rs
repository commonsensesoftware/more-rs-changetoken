/// Propagates notifications that a change has occurred.
pub trait ChangeToken {
    /// Gets a value that indicates if a change has occurred.
    fn changed(&self) -> bool;

    /// Indicates if this token will pro-actively raise callbacks.
    /// If `false`, the token consumer must poll [`changed`](ChangedToken::changed)
    /// to detect changes.
    fn must_poll(&self) -> bool {
        false
    }

    /// Registers for a callback that will be invoked when the entry has changed.
    ///
    /// # Arguments
    ///
    /// * `callback` - the callback to invoke
    fn register(&self, callback: Box<dyn Fn()>);
}
