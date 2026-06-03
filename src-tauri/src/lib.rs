//! Tauri app entry. Step 5: register `sign_pdfs_cmd` command + plugins.

mod commands;
mod config;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::sign_pdfs_cmd,
            commands::extract_worktimes_cmd
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
