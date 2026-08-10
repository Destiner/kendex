use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::env::Env;
use crate::error::{CoreError, Result};
use crate::fs::{atomic_write, read_if_exists};

/// App preferences and the project registry — one settings file, nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub struct AppSettings {
    pub schema: u32,
    #[serde(default)]
    pub projects: Vec<PathBuf>,
    /// Per-harness override of the global root directory, keyed by harness id.
    #[serde(default)]
    pub harness_roots: BTreeMap<String, PathBuf>,
    #[serde(default)]
    pub appearance: Appearance,
}

impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            schema: 1,
            projects: Vec::new(),
            harness_roots: BTreeMap::new(),
            appearance: Appearance::System,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Appearance {
    #[default]
    System,
    Light,
    Dark,
}

pub fn load(env: &Env) -> Result<AppSettings> {
    let path = env.settings_file();
    match read_if_exists(&path)? {
        None => Ok(AppSettings::default()),
        Some(text) => toml::from_str(&text).map_err(|e| CoreError::TomlParse {
            path,
            message: e.to_string(),
        }),
    }
}

pub fn save(env: &Env, settings: &AppSettings) -> Result<()> {
    let text = toml::to_string_pretty(settings).map_err(|e| CoreError::TomlParse {
        path: env.settings_file(),
        message: e.to_string(),
    })?;
    atomic_write(&env.settings_file(), &text)
}

/// Canonicalizes, rejects non-directories and duplicates, persists.
pub fn register_project(env: &Env, path: &Path) -> Result<AppSettings> {
    let canonical = path.canonicalize().map_err(|e| CoreError::io(path, e))?;
    if !canonical.is_dir() {
        return Err(CoreError::NotADirectory { path: canonical });
    }
    let mut settings = load(env)?;
    if settings.projects.contains(&canonical) {
        return Err(CoreError::ProjectAlreadyRegistered { path: canonical });
    }
    settings.projects.push(canonical);
    settings.projects.sort();
    save(env, &settings)?;
    Ok(settings)
}

/// Removes by canonical path when resolvable, else by the recorded path —
/// a registered project whose directory vanished must still be removable.
pub fn unregister_project(env: &Env, path: &Path) -> Result<AppSettings> {
    let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut settings = load(env)?;
    let before = settings.projects.len();
    settings.projects.retain(|p| *p != target);
    if settings.projects.len() == before {
        return Err(CoreError::ProjectNotRegistered { path: target });
    }
    save(env, &settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::FakeOs;

    fn env_in(dir: &Path) -> Env {
        Env::fake(dir, FakeOs::Linux)
    }

    #[test]
    fn missing_settings_file_loads_defaults() {
        let tmp = tempfile::tempdir().unwrap();
        let settings = load(&env_in(tmp.path())).unwrap();
        assert_eq!(settings, AppSettings::default());
    }

    #[test]
    fn settings_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let env = env_in(tmp.path());
        let mut settings = AppSettings {
            appearance: Appearance::Dark,
            ..AppSettings::default()
        };
        settings
            .harness_roots
            .insert("claude".into(), PathBuf::from("/custom/claude"));
        save(&env, &settings).unwrap();
        assert_eq!(load(&env).unwrap(), settings);
    }

    #[test]
    fn register_rejects_duplicates_and_unregister_removes() {
        let tmp = tempfile::tempdir().unwrap();
        let env = env_in(tmp.path());
        let project = tmp.path().join("proj");
        std::fs::create_dir(&project).unwrap();

        let settings = register_project(&env, &project).unwrap();
        assert_eq!(settings.projects.len(), 1);
        assert!(matches!(
            register_project(&env, &project),
            Err(CoreError::ProjectAlreadyRegistered { .. })
        ));

        let settings = unregister_project(&env, &project).unwrap();
        assert!(settings.projects.is_empty());
        assert!(matches!(
            unregister_project(&env, &project),
            Err(CoreError::ProjectNotRegistered { .. })
        ));
    }

    #[test]
    fn vanished_project_is_still_removable() {
        let tmp = tempfile::tempdir().unwrap();
        let env = env_in(tmp.path());
        let project = tmp.path().join("gone");
        std::fs::create_dir(&project).unwrap();
        let registered = register_project(&env, &project).unwrap().projects[0].clone();
        std::fs::remove_dir(&project).unwrap();

        let settings = unregister_project(&env, &registered).unwrap();
        assert!(settings.projects.is_empty());
    }
}
