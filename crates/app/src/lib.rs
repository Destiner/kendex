mod audit;
mod commands;

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
        audit::audit_all,
        audit::apply_plan,
        audit::adopt_item,
        audit::toggle_item,
        audit::remove_item,
    ])
}

pub fn run() -> tauri::Result<()> {
    let builder = specta_builder();
    tauri::Builder::default()
        .invoke_handler(builder.invoke_handler())
        .run(tauri::generate_context!())
}
