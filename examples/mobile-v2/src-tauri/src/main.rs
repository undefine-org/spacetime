// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_haptics::init())
        .plugin(tauri_plugin_biometric::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_geolocation::init())
        .plugin(tauri_plugin_barcode_scanner::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
