mod audit;
mod commands;
mod editor;
pub mod recovery;
mod sources;

use tauri_specta::{Builder, collect_commands};

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_version,
        commands::scan_machine,
        commands::get_settings,
        commands::update_settings,
        commands::register_project,
        commands::unregister_project,
        commands::discover_projects,
        commands::capability_table,
        commands::report_route,
        audit::audit_all,
        audit::apply_plan,
        audit::adopt_item,
        audit::toggle_item,
        audit::remove_item,
        editor::get_manifest,
        editor::update_manifest,
        editor::editor_inventory,
        sources::sources_overview,
        sources::source_add,
        sources::source_remove,
        sources::source_toggle,
        sources::sources_refresh,
    ])
}

/// WebKitGTK's DMABUF renderer crashes the window outright on several
/// Wayland compositors (GDK protocol error 71 on Hyprland). Disabling it
/// costs GPU-accelerated rendering but always shows a window; a user who
/// has set the variable themselves keeps their choice.
pub fn webview_env(session_type: Option<&str>, current: Option<&str>) -> Option<&'static str> {
    match (session_type, current) {
        (Some("wayland"), None) => Some("1"),
        _ => None,
    }
}

/// Re-exec ourselves with the workaround in the environment — the variable
/// being set is what stops the second pass from re-execing again.
#[cfg(target_os = "linux")]
fn apply_webview_env(value: &str) {
    use std::os::unix::process::CommandExt;
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let error = std::process::Command::new(exe)
        .args(std::env::args_os().skip(1))
        .env("WEBKIT_DISABLE_DMABUF_RENDERER", value)
        .exec();
    // exec only returns on failure; running without the workaround still
    // beats not starting at all.
    let _ = writeln!(std::io::stderr(), "webview workaround skipped: {error}");
}

#[cfg(target_os = "linux")]
use std::io::Write;

pub fn run() -> tauri::Result<()> {
    #[cfg(target_os = "linux")]
    if let Some(value) = webview_env(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER")
            .ok()
            .as_deref(),
    ) {
        apply_webview_env(value);
    }
    use std::io::Write;
    let mut stderr = std::io::stderr();
    match vstack_core::env::Env::detect() {
        Ok(env) => {
            for message in recovery::recover_on_launch(&env) {
                let _ = writeln!(stderr, "recovery: {message}");
            }
        }
        Err(error) => {
            let _ = writeln!(stderr, "recovery skipped: {error}");
        }
    }
    let builder = specta_builder();
    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
}
