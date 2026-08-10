use serde::Serialize;
use specta::Type;
use vstack_core::env::Env;
use vstack_core::harness::{KindCaps, capabilities};
use vstack_core::model::{HarnessId, ItemKind};
use vstack_core::scan::ScanResult;
use vstack_core::settings::{self, AppSettings};
use vstack_core::{discover, scan};

fn env() -> Result<Env, String> {
    Env::detect().map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[tauri::command]
#[specta::specta]
pub fn scan_machine() -> Result<ScanResult, String> {
    let env = env()?;
    let app_settings = settings::load(&env).map_err(|e| e.to_string())?;
    Ok(scan::scan(&env, &app_settings))
}

#[tauri::command]
#[specta::specta]
pub fn get_settings() -> Result<AppSettings, String> {
    settings::load(&env()?).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn update_settings(settings: AppSettings) -> Result<AppSettings, String> {
    let env = env()?;
    settings::save(&env, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
#[specta::specta]
pub fn register_project(path: String) -> Result<AppSettings, String> {
    settings::register_project(&env()?, path.as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn unregister_project(path: String) -> Result<AppSettings, String> {
    settings::unregister_project(&env()?, path.as_ref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn discover_projects(root: String) -> Result<Vec<String>, String> {
    Ok(discover::discover_projects(root.as_ref())
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|p| p.display().to_string())
        .collect())
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityRow {
    pub harness: HarnessId,
    pub kind: ItemKind,
    pub caps: KindCaps,
}

/// The full harness × kind capability matrix — the UI gates every action on
/// this, never on its own assumptions.
#[tauri::command]
#[specta::specta]
pub fn capability_table() -> Vec<CapabilityRow> {
    let mut rows = Vec::new();
    for harness in HarnessId::ALL {
        for kind in ItemKind::ALL {
            rows.push(CapabilityRow {
                harness,
                kind,
                caps: capabilities(harness, kind),
            });
        }
    }
    rows
}
