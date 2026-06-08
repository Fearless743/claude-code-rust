pub mod settings;

pub use settings::Settings;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigManager {
    config_dir: PathBuf,
    project_dir: Option<PathBuf>,
    settings: Settings,
    project_settings: Option<Settings>,
}

impl ConfigManager {
    pub fn new() -> eyre::Result<Self> {
        let config_dir = directories::ProjectDirs::from("com", "anthropic", "claude")
            .map(|d| d.config_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".claude"));
        std::fs::create_dir_all(&config_dir)?;

        let settings_path = config_dir.join("claude.json");
        let settings = if settings_path.exists() {
            let data = std::fs::read_to_string(&settings_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Settings::default()
        };

        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let project_settings = Self::load_project_settings(&cwd);

        Ok(Self {
            config_dir,
            project_dir: Some(cwd),
            settings,
            project_settings,
        })
    }

    fn load_project_settings(cwd: &Path) -> Option<Settings> {
        let path = cwd.join(".claude").join("settings.json");
        if path.exists() {
            if let Ok(data) = std::fs::read_to_string(&path) {
                return serde_json::from_str(&data).ok();
            }
        }
        None
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn project_settings(&self) -> Option<&Settings> {
        self.project_settings.as_ref()
    }

    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    pub fn project_dir(&self) -> Option<&PathBuf> {
        self.project_dir.as_ref()
    }

    pub fn merged_settings(&self) -> Settings {
        // Project settings override global settings
        let mut merged = self.settings.clone();
        if let Some(proj) = &self.project_settings {
            if proj.api_key.is_some() {
                merged.api_key = proj.api_key.clone();
            }
            if proj.base_url.is_some() {
                merged.base_url = proj.base_url.clone();
            }
            if proj.model.is_some() {
                merged.model = proj.model.clone();
            }
            if proj.provider.is_some() {
                merged.provider = proj.provider.clone();
            }
            if proj.permission_mode.is_some() {
                merged.permission_mode = proj.permission_mode.clone();
            }
            if proj.mcp_servers.is_some() {
                merged.mcp_servers = proj.mcp_servers.clone();
            }
        }
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_resolve_defaults() {
        let settings = Settings::default();
        assert_eq!(settings.resolve_provider(), "anthropic");
        assert!(settings.resolve_base_url().contains("api.anthropic.com"));
    }
}
