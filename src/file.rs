use crate::{Callback, ChangeToken, Registration, SingleChangeToken};
use notify::{Config, RecommendedWatcher, RecursiveMode::NonRecursive, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::{any::Any, mem::ManuallyDrop};

/// Represents a [`ChangeToken`](crate::ChangeToken) for a file.
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
    /// * `path` - The [path](std::path::Path) of the file to watch for changes
    pub fn new<T: AsRef<Path>>(path: T) -> Self {
        let file = path.as_ref().to_path_buf();
        let path = file.clone();
        let inner = Arc::new(SingleChangeToken::default());
        let handler = inner.clone();
        let (sender, receiver) = channel();
        let mut watcher = RecommendedWatcher::new(sender, Config::default()).unwrap();
        let handle = thread::spawn(move || {
            if let Ok(Ok(event)) = receiver.recv() {
                let changed =
                    event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove();

                if changed || event.need_rescan() {
                    let mut paths = event.paths.iter();
                    let other = path.as_os_str();

                    if paths.any(|p| p.as_os_str().eq_ignore_ascii_case(other)) {
                        handler.notify();
                    }
                }
            }
        });

        if let Some(folder) = file.parent() {
            if folder.exists() {
                watcher.watch(folder, NonRecursive).unwrap();
            }
        }

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
    use std::sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc, Condvar, Mutex,
    };
    use std::time::{Duration, Instant};
    use std::{fs::File, io::Write};
    use tempfile::{NamedTempFile, TempPath};

    #[test]
    fn changed_should_be_false_when_source_file_is_unchanged() {
        // arrange
        let mut file = NamedTempFile::new().expect("new file");

        file.write_all("test".as_bytes()).unwrap();

        let token = FileChangeToken::new(file.path());

        // act
        let changed = token.changed();

        // assert
        assert!(!changed);
    }

    #[test]
    fn changed_should_be_true_when_source_file_changes() {
        // arrange
        let mut file = NamedTempFile::new().expect("new file");

        file.write_all("original".as_bytes()).unwrap();

        let path = file.into_temp_path();
        let token = FileChangeToken::new(&path);
        let mut file = NamedTempFile::from_parts(File::create(&path).expect("valid path"), path);

        file.write_all("updated".as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(250));

        // act
        let changed = token.changed();

        // assert
        assert!(changed);
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_changes() {
        // arrange
        let mut file = NamedTempFile::new().expect("new file");

        file.write_all("original".as_bytes()).unwrap();

        let path = file.into_temp_path();
        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(&path);
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data
                    .downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>()
                    .unwrap();
                value.store(true, Relaxed);
                *fired.lock().unwrap() = true;
                event.notify_one();
            }),
            Some(state.clone()),
        );
        let mut file = NamedTempFile::from_parts(File::create(&path).expect("valid path"), path);

        // act
        file.write_all("updated".as_bytes()).unwrap();

        let time = Instant::now();
        let quarter_second = Duration::from_millis(250);
        let three_seconds = Duration::from_secs(3);
        let (mutex, event, changed) = &*state;
        let mut fired = mutex.lock().unwrap();

        while !*fired && time.elapsed() < three_seconds {
            fired = event.wait_timeout(fired, quarter_second).unwrap().0;
        }

        // assert
        assert!(changed.load(Relaxed));
    }

    #[test]
    fn callback_should_not_be_invoked_after_token_is_dropped() {
        // arrange
        let mut file = NamedTempFile::new().expect("new file");

        file.write_all("original".as_bytes()).unwrap();

        let path = file.into_temp_path();
        let changed = Arc::<AtomicBool>::default();
        let token = FileChangeToken::new(&path);
        let registration = token.register(
            Box::new(|state| {
                state
                    .unwrap()
                    .downcast_ref::<AtomicBool>()
                    .unwrap()
                    .store(true, Relaxed)
            }),
            Some(changed.clone()),
        );
        let mut file = NamedTempFile::from_parts(File::create(&path).expect("valid path"), path);

        // act
        drop(registration);
        drop(token);
        file.write_all("updated".as_bytes()).unwrap();
        thread::sleep(Duration::from_millis(250));

        // assert
        assert_eq!(changed.load(Relaxed), false);
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_is_created() {
        // arrange
        let path = std::env::temp_dir().join("new_file.txt");
        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(&path);
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data
                    .downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>()
                    .unwrap();
                value.store(true, Relaxed);
                *fired.lock().unwrap() = true;
                event.notify_one();
            }),
            Some(state.clone()),
        );
        let mut file = NamedTempFile::from_parts(
            File::create(&path).expect("valid path"),
            TempPath::from_path(path),
        );

        // act
        file.write_all("updated".as_bytes()).unwrap();

        let time = Instant::now();
        let quarter_second = Duration::from_millis(250);
        let three_seconds = Duration::from_secs(3);
        let (mutex, event, changed) = &*state;
        let mut fired = mutex.lock().unwrap();

        while !*fired && time.elapsed() < three_seconds {
            fired = event.wait_timeout(fired, quarter_second).unwrap().0;
        }

        // assert
        assert!(changed.load(Relaxed));
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_is_removed() {
        // arrange
        let mut file = NamedTempFile::new().expect("new file");

        file.write_all("existing".as_bytes()).unwrap();

        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(file.path());
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data
                    .downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>()
                    .unwrap();
                value.store(true, Relaxed);
                *fired.lock().unwrap() = true;
                event.notify_one();
            }),
            Some(state.clone()),
        );

        // act
        drop(file);

        let time = Instant::now();
        let quarter_second = Duration::from_millis(250);
        let three_seconds = Duration::from_secs(3);
        let (mutex, event, changed) = &*state;
        let mut fired = mutex.lock().unwrap();

        while !*fired && time.elapsed() < three_seconds {
            fired = event.wait_timeout(fired, quarter_second).unwrap().0;
        }

        // assert
        assert!(changed.load(Relaxed));
    }
}
