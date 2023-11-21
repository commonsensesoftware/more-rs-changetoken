# Default Change Token

A `ChangeToken` has the following capabilities.

```rust
pub type ChangeCallback = Box<dyn Fn(Option<Arc<dyn Any>>) + Send + Sync>;

pub trait ChangeToken: Send + Sync {
    fn changed(&self) -> bool;
    fn must_poll(&self) -> bool;
    fn register(
        &self,
        callback: ChangeCallback,
        state: Option<Arc<dyn Any>>) -> Registration;
}
```

All of the out-of-the-box change tokens use callbacks to signal a change, but `ChangeToken::must_poll` can return `true` to indicate that a consumer should poll `ChangeToken::changed`. `ChangeToken::changed` is expected to return `true` when a change has been observed. The result may vary between invocations depending on the implementations.

When `ChangeToken::register` is called, a `Registration` is returned. A `Registration` is an opaque struct that is used to terminate the registration. When the `Registration` struct is dropped, the callback will be removed from the change token's callback list. The caller owns the `Registration`, which ensures that a memory leak never occurs from the `ChangeToken` holding onto a callback longer than it should.

The `DefaultChangeToken` is the default implementation from which all other `ChangeToken` implementations are based on. This simple `ChangeToken` manages a list of callbacks and invokes them whenever `DefaultChangeToken::notify` is called by the producer. This `ChangeToken` supports triggering callbacks multiple times.

Since the token may be signaled multiple times, `ChangeToken::changed` only reports `true` while it is actively invoking callbacks. When used in a synchronous context, this means the return value will always be `false`. When used in an asynchronous context, the return value _may_ be `true` and potentially useful to a caller. For most usage scenarios, the act of invoking a callback signals a change and the value of `ChangeToken::changed` is uninteresting.