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

pub fn run() -> tauri::Result<()> {
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
