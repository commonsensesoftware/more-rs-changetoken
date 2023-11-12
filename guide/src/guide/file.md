# File Change Token

>This type is only available if the **fs** feature is activated

The `FileChangeToken` is a special type of [`ChangeToken`](default.md), which watches for changes to a file and notifies its consumers when a change is observed. The `FileChangeToken` only considers a single change. Once a change has been observed, it will not monitor further changes. The implementation is functionally equivalent to [`SingleChangeToken`](single.md), but for a file change.

>**Important**: `FileChangeToken` callbacks are always invoked on another thread; otherwise, the caller would be blocked waiting for a change.

```rust
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use tokens::FileChangeToken;

fn main() {
    let path = PathBuf::from("./my-app/files/some.txt");
    let state = Arc::new((Mutex::new(false), Condvar::new()));
    let state2 = state.clone();
    let token = FileChangeToken::new(&path);
    let registration = token.register(Box::new(move || {
        let (fired, event) = &*state2;
        *fired.lock().unwrap() = true;
        event.notify_one();
    }));
    let mut file = File::create(&path).unwrap();

    // make a change to the file
    file.write_all("updated".as_bytes()).unwrap();

    let (mutex, event) = &*state;
    let mut fired = mutex.lock().unwrap();

    // the callback happens on another thread so wait
    // here until the callback notifies us
    while !*fired {
        fired = event.wait(fired).unwrap();
    }

    println!("'{}' changed.", path.display());
}
```