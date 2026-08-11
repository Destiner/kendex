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

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReportRouteView {
    pub vstack_owned: bool,
    pub repo: Option<String>,
    pub label: Option<String>,
    /// Prefilled new-issue page — only when the report belongs upstream.
    pub issue_url: Option<String>,
}

fn urlencode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Where a problem report about this item belongs: the vstack upstream
/// (with a prefilled issue link) or the user's own repo.
#[tauri::command]
#[specta::specta]
pub fn report_route(
    scope: vstack_core::model::Scope,
    name: String,
    kind: Option<ItemKind>,
) -> Result<ReportRouteView, String> {
    let env = env()?;
    let lock = vstack_core::lock::load(&vstack_core::lock::lock_path(&env, &scope))
        .map_err(|e| e.to_string())?;
    let route = vstack_core::report::route(
        &env,
        &scope,
        &lock,
        &name,
        kind,
        vstack_core::report::DEFAULT_UPSTREAM,
    );
    let issue_url = route.repo.as_ref().map(|repo| {
        let mut url = format!(
            "https://github.com/{repo}/issues/new?title={}",
            urlencode(&format!("{name}: "))
        );
        if let Some(label) = &route.label {
            url.push_str(&format!("&labels={label}"));
        }
        url
    });
    Ok(ReportRouteView {
        vstack_owned: route.vstack_owned,
        repo: route.repo,
        label: route.label,
        issue_url,
    })
}
