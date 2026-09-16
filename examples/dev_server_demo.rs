//! Demo of the WebSocket dev server for live test running
//!
//! This example shows how to set up a complete development server
//! with file watching and WebSocket-based live reloading.
//!
//! Run with: cargo run --example dev_server_demo

use spacetime::{dev_server, watcher};
use std::sync::{Arc, RwLock};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Spacetime Dev Server with Live Reload...\n");

    // 1. Create file watcher for current directory
    let watch_path = std::env::current_dir()?;
    println!("✓ Watching directory: {}", watch_path.display());

    let (mut test_watcher, _change_rx) = watcher::TestWatcher::new(vec![watch_path.clone()]);
    println!("✓ File watcher initialized");

    // 2. Get the broadcast sender from the watcher
    let file_changes_tx = test_watcher.tx.clone();
    println!("✓ Created file change broadcast channel");

    // 3. Create WebSocket server state
    let compilation_cache = Arc::new(RwLock::new(None));
    let server_state = Arc::new(dev_server::TestServerState::new(
        file_changes_tx.clone(),
        compilation_cache,
    ));
    println!("✓ WebSocket server state created");

    // 4. Build router with WebSocket endpoint
    let app = dev_server::create_test_runner_routes(server_state);
    println!("✓ WebSocket routes configured at /ws");

    // 5. Start the watcher in background
    let watcher_handle = tokio::spawn(async move {
        if let Err(e) = test_watcher.watch().await {
            eprintln!("Watcher error: {}", e);
        }
    });

    // 6. Start the server
    let addr = "127.0.0.1:3030";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("\n🚀 Dev server running at: http://{}", addr);
    println!("   WebSocket endpoint: ws://{}/ws", addr);
    println!("\nWatching for file changes...");
    println!("Press Ctrl+C to stop\n");

    // Spawn a task to monitor and print file changes
    let mut change_rx2 = file_changes_tx.subscribe();
    tokio::spawn(async move {
        while let Ok(change) = change_rx2.recv().await {
            println!("📝 File {:?}: {}", change.kind, change.path.display());
        }
    });

    // 7. Serve the application
    axum::serve(listener, app).await?;

    Ok(())
}
