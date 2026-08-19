pub mod audit;
mod commands;
pub mod decisions;
mod editor;
mod native;
mod packages;
mod paths;
pub mod recovery;
mod sources;
mod window;

use tauri_specta::{Builder, collect_commands};

pub fn specta_builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_version,
        commands::scan_machine,
        commands::get_settings,
        commands::update_settings,
        commands::register_project,
        commands::unregister_project,
        commands::install_drift_hook,
        commands::discover_projects,
        commands::capability_table,
        commands::report_route,
        audit::audit_all,
        audit::apply_plan,
        audit::adopt_item,
        audit::toggle_item,
        audit::remove_item,
        decisions::list_decisions,
        decisions::dismiss_findings,
        decisions::revoke_dismissal,
        decisions::revoke_safety_override,
        editor::get_manifest,
        editor::update_manifest,
        editor::editor_inventory,
        editor::custom_hook_deliveries,
        editor::item_source,
        native::pick_folder,
        native::reveal_path,
        native::open_in_editor,
        sources::sources_overview,
        sources::source_add,
        sources::source_remove,
        sources::source_toggle,
        sources::sources_refresh,
        sources::bundles_overview,
        sources::bundle_install,
        packages::package_versions,
        packages::updates_overview,
        packages::updates_refresh,
        packages::update_set_ignored,
        packages::package_set_rev,
        packages::package_diff,
        packages::package_fork,
        packages::fork_rename,
        packages::apply_discard_edits,
        packages::package_files,
        packages::package_file,
        packages::package_readme,
        packages::package_meta,
        window::window_minimize,
        window::window_toggle_maximize,
        window::window_close,
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

/// Everything that must settle before the window opens: the old-name dirs
/// move first so recovery — and everything after it — reads the new ones.
/// A failed move is fatal to the launch, same as in the CLI: opening
/// anyway would write fresh state beside the stranded old files, and the
/// next launch would find both generations of the global scope and refuse
/// it forever.
pub fn prepare_launch(
    env: &kendex_core::env::Env,
) -> Result<Vec<String>, kendex_core::error::CoreError> {
    let moved = kendex_core::rename::migrate_global_dirs(env)?;
    let mut messages = moved.leftovers;
    messages.extend(recovery::recover_on_launch(env));
    Ok(messages)
}

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
    match kendex_core::env::Env::detect() {
        Ok(env) => match prepare_launch(&env) {
            Ok(messages) => {
                for message in messages {
                    let _ = writeln!(stderr, "launch: {message}");
                }
            }
            // Opening with the move half-done would fork the library in
            // two, so the launch stops loudly instead of showing an app
            // whose state is about to become unrecoverable.
            Err(error) => panic!(
                "kendex cannot start: moving your data from vstack2 to kendex failed ({error}). \
                 Starting anyway would split your library between the old and new locations. \
                 Fix the reported problem and launch again."
            ),
        },
        Err(error) => {
            let _ = writeln!(stderr, "recovery skipped: {error}");
        }
    }
    let builder = specta_builder();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
}
