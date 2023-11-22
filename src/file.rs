use crate::{Callback, ChangeToken, Registration, SingleChangeToken};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::any::Any;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Represents a change token for a file.
/// 
/// # Remarks
/// 
/// Registered notifications always occur on another thread.
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

    fn register(&self, callback: Callback, state: Option<Arc<dyn Any>>) -> Registration {
        self.inner.register(callback, state)
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
    use std::env::temp_dir;
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    #[test]
    fn changed_should_be_false_when_source_file_is_unchanged() {
        // arrange
        let path = temp_dir().join("test.1.txt");
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
        let path = temp_dir().join("test.2.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let token = FileChangeToken::new(&path);
        let mut file = File::create(&path).unwrap();

        file.write_all("updated".as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(250));

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
        let path = temp_dir().join("test.3.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(&path);
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data
                    .downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>()
                    .unwrap();
                value.store(true, Ordering::SeqCst);
                *fired.lock().unwrap() = true;
                event.notify_one();
            }),
            Some(state.clone()),
        );
        let mut file = File::create(&path).unwrap();

        // act
        file.write_all("updated".as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(250));

        let one_second = Duration::from_secs(1);
        let (mutex, event, changed) = &*state;
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
        let path = temp_dir().join("test.4.txt");
        let mut file = File::create(&path).unwrap();

        file.write_all("original".as_bytes()).unwrap();
        drop(file);

        let changed = Arc::<AtomicBool>::default();
        let token = FileChangeToken::new(&path);
        let registration = token.register(
            Box::new(|state| {
                state
                    .unwrap()
                    .downcast_ref::<AtomicBool>()
                    .unwrap()
                    .store(true, Ordering::SeqCst)
            }),
            Some(changed.clone()),
        );
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
