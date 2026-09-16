//! Example demonstrating the file watcher functionality
//!
//! This example shows how to use the TestWatcher to monitor .st files for changes.

use spacetime::TestWatcher;
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() {
    println!("Starting file watcher demo...");
    println!("This will watch the current directory for .st file changes.");
    println!("Press Ctrl+C to exit.\n");

    // Create a watcher for the current directory
    let watch_paths = vec![PathBuf::from(".")];
    let (mut watcher, mut rx) = TestWatcher::new(watch_paths);

    // Start watching in a background task
    tokio::spawn(async move {
        if let Err(e) = watcher.watch().await {
            eprintln!("Watcher error: {}", e);
        }
    });

    // Listen for file changes
    loop {
        match rx.recv().await {
            Ok(change) => {
                println!(
                    "[{}] File changed: {}",
                    match change.kind {
                        spacetime::ChangeKind::Created => "CREATED",
                        spacetime::ChangeKind::Modified => "MODIFIED",
                        spacetime::ChangeKind::Deleted => "DELETED",
                    },
                    change.path.display()
                );
            }
            Err(e) => {
                eprintln!("Error receiving change: {}", e);
                break;
            }
        }
    }
}
