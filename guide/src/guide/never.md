{{#include links.md}}

# Never Change Token

There may be edge cases where a [`ChangeToken`](default.md) is required, but there will never been any changes. This is the usage scenario for [`NeverChangeToken`]. This [`ChangeToken`](default.md) will **never** register any callbacks, [`changed`] will always return `false`, and [`must_poll`] will always return `true`. [`NeverChangeToken`] effectively implements the _Null Object_ pattern for the [`ChangeToken`](default.md) trait.

```rust
use tokens::*;
use std::sync::{Arc, RwLock};

#[derive(Default)]
pub struct Counter {
    value: RwLock<usize>,
}

impl Counter {
    pub fn increment(&self) {
        *self.value.write().unwrap() += 1;
    }

    pub fn watch(&self) -> impl ChangeToken {
        // TODO: placeholder until change support is implemented
        NeverChangeToken::new()
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

    counter.increment(); // prints nothing; callback not invoked
}
```