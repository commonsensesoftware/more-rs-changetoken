# Composite Change Token

Some applications may need to compose or aggregate multiple change tokens. This is the use case for a `CompositeChangeToken`. The `CompositeChangeToken` accepts a sequence of other [`ChangeToken`](default.md) instances and _mediates_ their change notifications. A consumer of a `CompositeChangeToken` will be called back via `CompositeChangeToken::notify` whenever the owner explicitly signals a change or when one of the mediated children signals a change.

```rust
use tokens::*;
use std::sync::{Arc, RwLock};

pub struct Counter {
    id: usize,
    token: RwLock<SharedChangeToken<SingleChangeToken>>,
    value: RwLock<usize>,
}

impl Counter {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            ..Default::default()
        }
    }

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
        format!("[{}] Value: {}", self.id, *self.value.read().unwrap())
    }
}

fn main() {
  let counters = Arc::new(vec![Counter::new(1), Counter::new(2), Counter::new(3)]);
  let counters2 = counters.clone();
  let token = CompositeChangeToken::new(counters.iter().map(|c| Box::new(c.watch())));
  let mut registration = token.register(Box::new(move || {
      for printable in counters2 {
          println!("{}", printable.to_string());
      }
  }));

  // prints '[1] Value 0'
  // prints '[2] Value 0'
  // prints '[3] Value 1'
  counters[2].increment();
}
```