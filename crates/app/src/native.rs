//! Thin wrappers over OS-native pickers and file browsers. Neither plugin's
//! own IPC commands are exposed to the frontend — these wrap the plugins'
//! Rust APIs behind vstack's own typed commands instead, the same way
//! `window.rs` wraps the frameless titlebar's OS calls.

use std::path::Path;

use tauri_plugin_dialog::DialogExt;

/// Native folder picker. Blocking, so this must not run on the main thread
/// — an async command already runs off it, which is what the plugin's own
/// docs call for.
#[tauri::command]
#[specta::specta]
pub async fn pick_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let picked = app.dialog().file().blocking_pick_folder();
    let Some(path) = picked else {
        return Ok(None);
    };
    path.into_path()
        .map(|p| Some(p.display().to_string()))
        .map_err(|e| e.to_string())
}

/// Shows `path` in the system file browser. Only ever reveals a path that
/// is actually there — the plain-word error is the fix, not a stack trace
/// from the OS call that would have failed instead.
#[tauri::command]
#[specta::specta]
pub fn reveal_path(path: String) -> Result<(), String> {
    if !Path::new(&path).exists() {
        return Err(format!("{path} does not exist"));
    }
    tauri_plugin_opener::reveal_item_in_dir(&path).map_err(|e| e.to_string())
}
