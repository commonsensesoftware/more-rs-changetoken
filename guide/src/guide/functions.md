# Functions

The following are utility functions that are useful when working with change tokens.

## On Change

The `tokens::on_change` function mediates a producer method that returns a [`ChangeToken`](default.md) and a consumer function that responds to a change. Unlike the [`CompositeChangeToken`](composite.md), the `tokens::on_change` function will facilitate calling back to the specified consumer, immediately drop the last [`ChangeToken`](default.md), and request a new [`ChangeToken`](default.md) from the producer.

The return value of the function is an opaque struct that implements the `Subscription` trait representing the perpetual subscription. `tokens::on_change` will continue to signal the consumer with changes and refresh the producer [`ChangeToken`](default.md) until the opaque subscription object has been dropped.

```rust
use std::path::PathBuf;
use tokens::FileChangeToken;

let path = PathBuf::from("./my-app/some.txt");
let subscription = tokens::on_change(
    move || FileChangeToken::new(path.clone()),
    |state| {
        println!("{}", state.unwrap().display());
    },
    Some(path.clone()));
```
