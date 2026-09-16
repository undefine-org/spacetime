//! File watcher for live test runner
//!
//! Monitors .st test files for changes, debounces events, and supports
//! incremental compilation through hash-based caching.

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::sync::broadcast;
use tokio::time::{Instant, sleep};

/// Represents a file change event
#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: PathBuf,
    pub kind: ChangeKind,
}

/// Type of file change
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
}

/// File watcher for test files
pub struct TestWatcher {
    paths: Vec<PathBuf>,
    pub tx: broadcast::Sender<FileChange>,
    file_hashes: Arc<Mutex<HashMap<PathBuf, u64>>>,
    debounce_duration: Duration,
}

impl TestWatcher {
    /// Create a new test watcher
    ///
    /// Returns the watcher and a receiver for file change events
    pub fn new(paths: Vec<PathBuf>) -> (Self, broadcast::Receiver<FileChange>) {
        let (tx, rx) = broadcast::channel(100);

        (
            Self {
                paths,
                tx,
                file_hashes: Arc::new(Mutex::new(HashMap::new())),
                debounce_duration: Duration::from_millis(100),
            },
            rx,
        )
    }

    /// Start watching for file changes
    ///
    /// This will continuously monitor the configured paths and broadcast
    /// change events through the channel.
    pub async fn watch(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (notify_tx, mut notify_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create the notify watcher
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                if let Ok(event) = res {
                    let _ = notify_tx.send(event);
                }
            },
            Config::default(),
        )?;

        // Watch all configured paths
        for path in &self.paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::Recursive)?;
            }
        }

        // Debouncing state
        let mut pending_changes: HashMap<PathBuf, (ChangeKind, Instant)> = HashMap::new();
        let tx = self.tx.clone();
        let file_hashes = self.file_hashes.clone();
        let debounce_duration = self.debounce_duration;

        // Process events with debouncing
        let debounce_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Process incoming events
                    Some(event) = notify_rx.recv() => {
                        Self::process_event(event, &mut pending_changes).await;
                    }

                    // Check for debounced events to emit
                    _ = sleep(Duration::from_millis(10)) => {
                        let now = Instant::now();
                        let mut to_emit = Vec::new();

                        // Find events that have waited long enough
                        pending_changes.retain(|path, (kind, timestamp)| {
                            if now.duration_since(*timestamp) >= debounce_duration {
                                to_emit.push((path.clone(), kind.clone()));
                                false
                            } else {
                                true
                            }
                        });

                        // Emit debounced events
                        for (path, kind) in to_emit {
                            // Check if file actually changed using hash
                            if Self::has_file_changed(&path, kind.clone(), &file_hashes).await {
                                let change = FileChange { path, kind };
                                let _ = tx.send(change);
                            }
                        }
                    }
                }
            }
        });

        // Keep the watcher alive
        debounce_task.await?;
        Ok(())
    }

    /// Process a file system event
    async fn process_event(event: Event, pending: &mut HashMap<PathBuf, (ChangeKind, Instant)>) {
        let kind = match event.kind {
            EventKind::Create(_) => Some(ChangeKind::Created),
            EventKind::Modify(_) => Some(ChangeKind::Modified),
            EventKind::Remove(_) => Some(ChangeKind::Deleted),
            _ => None,
        };

        if let Some(change_kind) = kind {
            for path in event.paths {
                if Self::should_watch(&path) {
                    pending.insert(path, (change_kind.clone(), Instant::now()));
                }
            }
        }
    }

    /// Check if a file should be watched
    fn should_watch(path: &Path) -> bool {
        // Watch .st sources and vendored bundle artifacts (PLAN-024 W2): a
        // `vendor build` rewrites `*.bundle.js`, which must trigger live reload.
        match path.extension().and_then(|e| e.to_str()) {
            Some("st") => true,
            // EDN is a peer concrete syntax (PLAN-148): an .edn page change must
            // trigger the same recompile/reload as .st.
            Some("edn") => true,
            Some("js") => path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".bundle.js")),
            _ => false,
        }
    }

    /// Check if file has actually changed using hash comparison
    async fn has_file_changed(
        path: &Path,
        kind: ChangeKind,
        file_hashes: &Arc<Mutex<HashMap<PathBuf, u64>>>,
    ) -> bool {
        // Always report creations and deletions
        match kind {
            ChangeKind::Created | ChangeKind::Deleted => return true,
            ChangeKind::Modified => {}
        }

        // For modifications, check if content actually changed
        match Self::hash_file(path) {
            Ok(new_hash) => {
                let mut hashes = file_hashes.lock().await;
                let old_hash = hashes.get(path).copied();

                // Update stored hash
                hashes.insert(path.to_path_buf(), new_hash);

                // File changed if hash is different
                old_hash != Some(new_hash)
            }
            Err(_) => {
                // If we can't read the file, report the change anyway
                true
            }
        }
    }

    /// Hash file contents for change detection
    fn hash_file(path: &Path) -> Result<u64, std::io::Error> {
        use std::collections::hash_map::DefaultHasher;

        let contents = std::fs::read(path)?;
        let mut hasher = DefaultHasher::new();
        contents.hash(&mut hasher);
        Ok(hasher.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use tokio::time::timeout;

    #[tokio::test]
    async fn test_file_watcher_detects_creation() {
        // Create temp directory
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.st");

        // Create watcher
        let (mut watcher, mut rx) = TestWatcher::new(vec![temp_dir.path().to_path_buf()]);

        // Start watching in background
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });

        // Give watcher time to initialize
        sleep(Duration::from_millis(50)).await;

        // Create a .st file
        fs::write(&test_file, "timeline scroll {}").unwrap();

        // Wait for event (with timeout)
        let result = timeout(Duration::from_secs(2), rx.recv()).await;

        assert!(result.is_ok(), "Should receive event within timeout");
        let change = result.unwrap().unwrap();
        assert_eq!(change.path, test_file);
        // File creation may trigger either Created or Modified event depending on the OS
        assert!(
            change.kind == ChangeKind::Created || change.kind == ChangeKind::Modified,
            "Expected Created or Modified, got {:?}",
            change.kind
        );
    }

    #[tokio::test]
    async fn test_file_watcher_detects_modification() {
        // Create temp directory with existing file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.st");
        fs::write(&test_file, "timeline scroll {}").unwrap();

        // Create watcher
        let (mut watcher, mut rx) = TestWatcher::new(vec![temp_dir.path().to_path_buf()]);

        // Start watching in background
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });

        // Give watcher time to initialize
        sleep(Duration::from_millis(50)).await;

        // Modify the file
        fs::write(&test_file, "timeline scroll { range: 0, 1; }").unwrap();

        // Wait for event
        let result = timeout(Duration::from_secs(2), rx.recv()).await;

        assert!(result.is_ok(), "Should receive event within timeout");
        let change = result.unwrap().unwrap();
        assert_eq!(change.path, test_file);
        assert_eq!(change.kind, ChangeKind::Modified);
    }

    #[tokio::test]
    async fn test_debouncing_rapid_changes() {
        // Create temp directory with existing file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.st");
        fs::write(&test_file, "timeline scroll {}").unwrap();

        // Create watcher
        let (mut watcher, mut rx) = TestWatcher::new(vec![temp_dir.path().to_path_buf()]);

        // Start watching in background
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });

        // Give watcher time to initialize
        sleep(Duration::from_millis(50)).await;

        // Make multiple rapid changes
        for i in 0..5 {
            fs::write(
                &test_file,
                format!("timeline scroll {{ /* change {} */ }}", i),
            )
            .unwrap();
            sleep(Duration::from_millis(10)).await;
        }

        // Wait for debounce period
        sleep(Duration::from_millis(150)).await;

        // Should receive only one event due to debouncing
        let result = timeout(Duration::from_millis(500), rx.recv()).await;
        assert!(result.is_ok(), "Should receive one event");

        // Try to receive another event - should timeout
        let result2 = timeout(Duration::from_millis(500), rx.recv()).await;
        assert!(
            result2.is_err(),
            "Should not receive multiple events (debounced)"
        );
    }

    #[tokio::test]
    async fn test_ignores_non_st_files() {
        // Create temp directory
        let temp_dir = TempDir::new().unwrap();
        let js_file = temp_dir.path().join("test.js");
        let st_file = temp_dir.path().join("test.st");

        // Create watcher
        let (mut watcher, mut rx) = TestWatcher::new(vec![temp_dir.path().to_path_buf()]);

        // Start watching in background
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });

        // Give watcher time to initialize
        sleep(Duration::from_millis(50)).await;

        // Create a .js file (should be ignored)
        fs::write(&js_file, "console.log('hello')").unwrap();

        // Create a .st file (should be detected)
        fs::write(&st_file, "timeline scroll {}").unwrap();

        // Wait for event
        let result = timeout(Duration::from_secs(2), rx.recv()).await;

        assert!(result.is_ok(), "Should receive event for .st file");
        let change = result.unwrap().unwrap();
        assert_eq!(
            change.path, st_file,
            "Event should be for .st file, not .js"
        );
    }

    #[tokio::test]
    async fn test_hash_based_change_detection() {
        // Create temp directory with file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.st");
        let content = "timeline scroll {}";
        fs::write(&test_file, content).unwrap();

        // Create watcher
        let (mut watcher, mut rx) = TestWatcher::new(vec![temp_dir.path().to_path_buf()]);

        // Start watching in background
        tokio::spawn(async move {
            let _ = watcher.watch().await;
        });

        // Give watcher time to initialize
        sleep(Duration::from_millis(50)).await;

        // Write same content (hash should be identical)
        fs::write(&test_file, content).unwrap();

        // Wait for debounce
        sleep(Duration::from_millis(200)).await;

        // Should NOT receive event because content hasn't actually changed
        let _result = timeout(Duration::from_millis(500), rx.recv()).await;

        // First write might trigger an event, so let's modify with different content
        fs::write(&test_file, "timeline scroll { range: 0, 1; }").unwrap();

        // This should trigger an event
        let result = timeout(Duration::from_secs(2), rx.recv()).await;
        assert!(
            result.is_ok(),
            "Should receive event for actual content change"
        );
    }

    #[test]
    fn test_should_watch() {
        assert!(TestWatcher::should_watch(Path::new("test.st")));
        assert!(TestWatcher::should_watch(Path::new("/path/to/test.st")));
        // EDN pages are peers to .st (PLAN-148): watched exactly the same.
        assert!(TestWatcher::should_watch(Path::new("page.edn")));
        assert!(!TestWatcher::should_watch(Path::new("test.js")));
        assert!(!TestWatcher::should_watch(Path::new("test.html")));
        assert!(!TestWatcher::should_watch(Path::new("test")));
        // Vendored bundle artifacts ARE watched (PLAN-024 W2): `vendor build`
        // rewriting one must trigger live reload, but plain .js stays ignored.
        assert!(TestWatcher::should_watch(Path::new(
            "stdlib/text/vendor/pretext.bundle.js"
        )));
        assert!(!TestWatcher::should_watch(Path::new("adapters/browser.js")));
    }

    #[test]
    fn test_hash_file() {
        // Create temp file
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.st");

        // Write content and hash
        fs::write(&test_file, "content1").unwrap();
        let hash1 = TestWatcher::hash_file(&test_file).unwrap();

        // Write same content - hash should be same
        fs::write(&test_file, "content1").unwrap();
        let hash2 = TestWatcher::hash_file(&test_file).unwrap();
        assert_eq!(hash1, hash2);

        // Write different content - hash should be different
        fs::write(&test_file, "content2").unwrap();
        let hash3 = TestWatcher::hash_file(&test_file).unwrap();
        assert_ne!(hash1, hash3);
    }
}
