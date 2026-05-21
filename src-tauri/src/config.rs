//! Load `templates.toml` from user config dir; auto-create with default
//! H5P9 template on first launch.
//!
//! Layout (macOS): `~/Library/Application Support/esi-pdf-sign/templates.toml`
//! Layout (Linux): `~/.config/esi-pdf-sign/templates.toml`
//! Layout (Win):   `%APPDATA%\esi-pdf-sign\templates.toml`

use pdf_sign_core::SignSpec;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

const APP_DIR: &str = "esi-pdf-sign";
const TEMPLATES_FILE: &str = "templates.toml";

const DEFAULT_TOML: &str = r#"# ESI PDF Sign — template definitions.
# Each [[template]] maps a PDF template (matched by page count) to where the
# signature image should be drawn. Coordinates are in PDF points; origin is the
# bottom-left of the page, y increases upward.

[[template]]
name = "H5P9"
match_pages = 2

[template.signature]
page_index = 1     # 0-based; H5P9 signs on page 2
x = 466.3
y = 122.8          # vertically centered between "Customer acknowledges" line (PDF y≈179) and "ESI Engineer's Signature" label (PDF y≈106.6); aligns with the cell's "May 20, 2026" baseline
width = 106.7
height = 40.0
"#;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    #[serde(rename = "template", default)]
    pub templates: Vec<TemplateSpec>,
}

#[derive(Debug, Deserialize)]
pub struct TemplateSpec {
    pub name: String,
    /// Reserved for future automatic template detection by page count.
    /// Design §1 explicitly says we do not auto-detect in this release; frontend
    /// passes `template_name` directly.
    #[allow(dead_code)]
    pub match_pages: Option<u32>,
    pub signature: SignSpec,
}

impl AppConfig {
    pub fn find(&self, name: &str) -> Option<&TemplateSpec> {
        self.templates.iter().find(|t| t.name == name)
    }
}

pub fn templates_path() -> Result<PathBuf, ConfigError> {
    let base = dirs::config_dir().ok_or(ConfigError::NoConfigDir)?;
    Ok(base.join(APP_DIR).join(TEMPLATES_FILE))
}

pub fn load_or_create_default() -> Result<AppConfig, ConfigError> {
    let path = templates_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        fs::write(&path, DEFAULT_TOML).map_err(|e| ConfigError::Io {
            path: path.clone(),
            source: e,
        })?;
    }
    let raw = fs::read_to_string(&path).map_err(|e| ConfigError::Io {
        path: path.clone(),
        source: e,
    })?;
    let cfg: AppConfig = toml::from_str(&raw).map_err(|e| ConfigError::Parse {
        path: path.clone(),
        message: e.to_string(),
    })?;
    Ok(cfg)
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("OS provided no user config directory")]
    NoConfigDir,
    #[error("IO failure on {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to parse {path}: {message}")]
    Parse { path: PathBuf, message: String },
}
