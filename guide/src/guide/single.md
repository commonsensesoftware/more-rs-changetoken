# Single Change Token

The `SingleChangeToken` behaves the same as the [`DefaultChangeToken`](default.md) with a single exception. It changes exactly once. Once `SingleChangeToken::notify` has been invoked, `ChangeToken::changed` will **always** return `true`. Any registered callbacks will be invoked, at most, once. If a callback is registered after `SingleChangeToken::notify` has been called, it will **never** be invoked.

The design of a [`ChangeToken`](default.md) does not indicate whether it supports multiple notifications. As a result, consumers are likely to create new change tokens from producers often. `SingleChangeToken` tends to be the most commonly used change token. It guarantees at-most once execution and prevents change tokens from living longer than they need to.

```rust
use tokens::*;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Counter {
    token: RwLock<SharedChangeToken<SingleChangeToken>>,
    value: RwLock<usize>,
}

impl Counter {
    pub fn increment(&self) {
        *self.value.write().unwrap() += 1;
        let token = std::mem.replace(
            &mut *self.token.write().unwrap(),
            Default::default());
        token.notify();
    }

    pub fn watch(&self) -> impl ChangeToken {
        self.token.clone()
    }
}

impl ToString for Counter {
    fn to_string(&self) -> String {
        format!("Value: {}", *self.value.read().unwrap())
    }
}

fn main() {
    let counter = Arc::new(Counter::default());
    let printable = counter.clone();
    let mut registration = counter.watch().register(Box::new(move || {
        println!("{}", printable.to_string());
    }));

    counter.increment(); // prints 'Value 1'
    counter.increment(); // doesn't print because token already fired

    let printable = counter.clone();
    registration = counter.watch().register(Box::new(move || {
        println!("{}", printable.to_string());
    }));

    counter.increment(); // prints 'Value 3'
}
```