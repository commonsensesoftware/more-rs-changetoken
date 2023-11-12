use crate::{ChangeCallback, ChangeToken, Registration, SingleChangeToken};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::mem::ManuallyDrop;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Represents a change token for a file.
pub struct FileChangeToken {
    watcher: ManuallyDrop<RecommendedWatcher>,
    handle: ManuallyDrop<JoinHandle<()>>,
    inner: Arc<SingleChangeToken>,
}

impl FileChangeToken {
    /// Initializes a new file change token.
    ///
    /// # Arguments
    ///
    /// * `path` - The path of the file to watch for changes
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        let file = path.as_ref().to_path_buf();
        let inner = Arc::new(SingleChangeToken::default());
        let handler = inner.clone();
        let (sender, receiver) = channel();
        let mut watcher = RecommendedWatcher::new(sender, Config::default()).unwrap();
        let handle = thread::spawn(move || {
            if let Ok(Ok(event)) = receiver.recv() {
                if event.kind.is_modify() {
                    handler.notify()
                }
            }
        });

        watcher
            .watch(file.as_ref(), RecursiveMode::NonRecursive)
            .unwrap();

        Self {
            watcher: ManuallyDrop::new(watcher),
            handle: ManuallyDrop::new(handle),
            inner,
        }
    }
}

impl ChangeToken for FileChangeToken {
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    fn register(&self, callback: ChangeCallback) -> Registration {
        self.inner.register(callback)
    }
}

impl Drop for FileChangeToken {
    fn drop(&mut self) {
        // manual drop is necessary to control terminating
        // the channel receiver. if we don't, then we will
        // likely deadlock while waiting to join the
        // receiver's background thread
        let handle = unsafe {
            let _ = ManuallyDrop::take(&mut self.watcher);
            ManuallyDrop::take(&mut self.handle)
        };
        handle.join().ok();
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::env::var;
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    fn new_temp_path(filename: &str) -> PathBuf {
        let temp = var("TEMP")
            .or(var("TMP"))
            .or(var("TMPDIR"))
            .unwrap_or("/tmp".into());
        PathBuf::new().join(temp).join(filename)
    }

    #[test]
    fn changed_should_be_false_when_source_file_is_unchanged() {
        // arrange
        let path = new_temp_path("test.1.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("test".as_bytes()).unwrap();

        let token = FileChangeToken::new(&path);

        // act
        let changed = token.changed();

        // assert
        if path.exists() {
            remove_file(&path).ok();
        }

        assert!(!changed);
    }

    #[test]
    fn changed_should_be_true_when_source_file_changes() {
        // arrange
        let path = new_temp_path("test.2.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let token = FileChangeToken::new(&path);
        let mut file = File::create(&path).unwrap();

        file.write_all("updated".as_bytes()).unwrap();

        // act
        let changed = token.changed();

        // assert
        if path.exists() {
            remove_file(&path).ok();
        }

        assert!(changed);
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_changes() {
        // arrange
        let path = new_temp_path("test.3.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let changed = Arc::<AtomicBool>::default();
        let changed2 = changed.clone();
        let state = Arc::new((Mutex::new(false), Condvar::new()));
        let state2 = state.clone();
        let token = FileChangeToken::new(&path);
        let _unused = token.register(Box::new(move || {
            let (fired, event) = &*state2;
            changed2.store(true, Ordering::SeqCst);
            *fired.lock().unwrap() = true;
            event.notify_one();
        }));
        let mut file = File::create(&path).unwrap();

        // act
        file.write_all("updated".as_bytes()).unwrap();

        let one_second = Duration::from_secs(1);
        let (mutex, event) = &*state;
        let mut fired = mutex.lock().unwrap();

        while !*fired {
            fired = event.wait_timeout(fired, one_second).unwrap().0;
        }

        // assert
        if path.exists() {
            remove_file(&path).ok();
        }

        assert!(changed.load(Ordering::SeqCst));
    }

    #[test]
    fn callback_should_not_be_invoked_after_token_is_dropped() {
        // arrange
        let path = new_temp_path("test.4.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let changed = Arc::<AtomicBool>::default();
        let changed2 = changed.clone();
        let token = FileChangeToken::new(&path);
        let registration = token.register(Box::new(move || changed2.store(true, Ordering::SeqCst)));
        let mut file = File::create(&path).unwrap();

        // act
        drop(registration);
        drop(token);
        file.write_all("updated".as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(250));

        // assert
        if path.exists() {
            remove_file(&path).ok();
        }

        assert_eq!(changed.load(Ordering::SeqCst), false);
    }
}
