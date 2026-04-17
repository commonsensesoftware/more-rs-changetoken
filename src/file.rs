use crate::{Callback, ChangeToken, Registration, SingleChangeToken, State};
use notify::{Config, RecommendedWatcher, RecursiveMode::NonRecursive, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};

/// Represents a [`ChangeToken`](ChangeToken) for a file.
///
/// # Remarks
///
/// Registered notifications always occur on another thread.
pub struct FileChangeToken {
    watcher: Option<RecommendedWatcher>,
    handle: Option<JoinHandle<()>>,
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
        let mut watcher = RecommendedWatcher::new(sender, Config::default()).unwrap_or_else(|e| panic!("{}", e));
        let handle = spawn(move || {
            while let Ok(Ok(event)) = receiver.recv() {
                let changed = event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove();

                if changed || event.need_rescan() {
                    let mut paths = event.paths.iter();
                    let other = path.as_os_str();

                    if paths.any(|p| p.as_os_str().eq_ignore_ascii_case(other)) {
                        handler.notify();
                        break;
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
            watcher: Some(watcher),
            handle: Some(handle),
            inner,
        }
    }
}

impl ChangeToken for FileChangeToken {
    #[inline]
    fn changed(&self) -> bool {
        self.inner.changed()
    }

    #[inline]
    fn register(&self, callback: Callback, state: State) -> Registration {
        self.inner.register(callback, state)
    }
}

impl Drop for FileChangeToken {
    fn drop(&mut self) {
        let _ = self.watcher.take();

        if let Some(handle) = self.handle.take() {
            handle.join().ok();
        }
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
    use std::{fs::File, io::Write, thread::sleep};
    use tempfile::{tempdir, NamedTempFile, TempPath};
        use crate::assert_send_and_sync;

    #[test]
    fn file_change_token_should_send_and_sync() {
        // arrange
        let dir = tempdir().expect("temp dir");
        let file = NamedTempFile::new_in(&dir).expect("new file");
        let token = FileChangeToken::new(file.path());

        // act

        // assert
        assert_send_and_sync(token);
    }

    #[test]
    fn changed_should_be_false_when_source_file_is_unchanged() {
        // arrange
        let dir = tempdir().expect("temp dir");
        let mut file = NamedTempFile::new_in(&dir).expect("new file");

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
        let dir = tempdir().expect("temp dir");
        let mut file = NamedTempFile::new_in(&dir).expect("new file");

        file.write_all("original".as_bytes()).unwrap();

        let path = file.into_temp_path();
        let token = FileChangeToken::new(&path);
        let mut file = NamedTempFile::from_parts(File::create(&path).expect("valid path"), path);

        file.write_all("updated".as_bytes()).unwrap();
        sleep(Duration::from_millis(250));

        // act
        let changed = token.changed();

        // assert
        assert!(changed);
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_changes() {
        // arrange
        let dir = tempdir().expect("temp dir");
        let mut file = NamedTempFile::new_in(&dir).expect("new file");

        file.write_all("original".as_bytes()).unwrap();

        let path = file.into_temp_path();
        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(&path);
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data.downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>().unwrap();
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
        let dir = tempdir().expect("temp dir");
        let mut file = NamedTempFile::new_in(&dir).expect("new file");

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

        // act
        drop(registration);
        drop(token);
        
        let mut file = NamedTempFile::from_parts(File::create(&path).expect("valid path"), path);
        
        file.write_all("updated".as_bytes()).unwrap();
        sleep(Duration::from_millis(250));

        // assert
        assert_eq!(changed.load(Relaxed), false);
    }

    #[test]
    fn callback_should_be_invoked_when_source_file_is_created() {
        // arrange
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("new_file.txt");
        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(&path);
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data.downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>().unwrap();
                value.store(true, Relaxed);
                *fired.lock().unwrap() = true;
                event.notify_one();
            }),
            Some(state.clone()),
        );
        let mut file = NamedTempFile::from_parts(
            File::create(&path).expect("valid path"),
            TempPath::try_from_path(path).unwrap(),
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
        let dir = tempdir().expect("temp dir");
        let mut file = NamedTempFile::new_in(&dir).expect("new file");

        file.write_all("existing".as_bytes()).unwrap();

        let state = Arc::new((Mutex::new(false), Condvar::new(), AtomicBool::default()));
        let token = FileChangeToken::new(file.path());
        let _unused = token.register(
            Box::new(|state| {
                let data = state.unwrap();
                let (fired, event, value) = data.downcast_ref::<(Mutex<bool>, Condvar, AtomicBool)>().unwrap();
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
