{{#include guide/links.md}}

# Getting Started

The simplest way to get started is to install the crate using the default features.

```bash
cargo install more-changetoken
```

## Example

A change token provides a way to signal a consumer that a change has occurred. This can commonly occur when using _Interior Mutability_ or when changes happen asynchronously. The most common usage scenario is sharing a change token between a producer and one or more consumers.

```rust
use tokens::*;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Counter {
    token: SharedChangeToken,
    value: RwLock<usize>,
}

impl Counter {
    pub fn increment(&self) {
        *self.value.write().unwrap() += 1;
        self.token.notify();
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
    let registration = counter.watch().register(
        Box::new(|state| {
            let printable = state.unwrap().downcast_ref::<Counter>().unwrap();
            println!("{}", printable.to_string());
        }),
        Some(counter.clone()));

    counter.increment(); // prints 'Value 1'
    counter.increment(); // prints 'Value 2'
    counter.increment(); // prints 'Value 3'
}
```

Since the registered callback might occur on another thread, the specified function must implement `Send` and `Sync` to ensure it is safe to invoke. Although the backing implementation - [`SharedChangeToken`] - can be shared, the caller is intentionally unaware of that capability because only an implementation of [`ChangeToken`] is returned, which does not implement `Clone`. This behavior ensures that a consumer cannot unintentionally propagate copies of the provided to change token to others.

[`register`] returns a [`Registration`] that represents the registration of a callback. When the [`Registration`] is dropped, the callback is also dropped. A change token will not leak callbacks over time, which means it is important to hold onto the registration for as long as it is needed. Using a discard (e.g. `_`) will immediately drop the registration and callback reference.