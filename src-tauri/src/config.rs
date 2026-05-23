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
# Each [[template]] maps a PDF template (matched by page count) to a list of
# named signature slots. At sign time we search each slot's `anchor_text`
# verbatim in the target page's content stream; the signature image's
# bottom-left corner is drawn at (anchor_x + dx, anchor_baseline_y + dy),
# with PDF origin at the page's bottom-left and y increasing upward.
# The frontend picks which slots get a PNG; missing slots are skipped.

[[template]]
name = "H5P9"
match_pages = 2

[[template.signature]]
role = "engineer"
page_index = 1                       # 0-based: H5P9 signs on page 2
anchor_text = "ESI Engineer's Signature"
dx = 0.0
dy = 22.634
width = 106.7
height = 40.0

[[template.signature]]
role = "customer"
page_index = 1
anchor_text = "Authorised Customer's Signature"
dx = 0.0
dy = 22.634
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
    #[serde(rename = "signature", default)]
    pub signatures: Vec<RoleSignSpec>,
}

#[derive(Debug, Deserialize)]
pub struct RoleSignSpec {
    pub role: String,
    #[serde(flatten)]
    pub spec: SignSpec,
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
    load_or_create_at(&path)
}

/// Internal entry point so tests can point at a tmp dir instead of the real
/// user config directory.
pub(crate) fn load_or_create_at(path: &std::path::Path) -> Result<AppConfig, ConfigError> {
    if !path.exists() {
        write_default(path)?;
    }
    let raw = fs::read_to_string(path).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        source: e,
    })?;
    match toml::from_str::<AppConfig>(&raw) {
        Ok(cfg) => Ok(cfg),
        Err(_parse_err) => {
            // Schema mismatch — usually from an upgrade. Move the old file
            // aside (so any user-tuned dx/dy can still be recovered) and
            // rewrite with the current default TOML so the app keeps
            // working without a manual `rm`.
            let backup = backup_path(path);
            fs::rename(path, &backup).map_err(|e| ConfigError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
            write_default(path)?;
            let raw = fs::read_to_string(path).map_err(|e| ConfigError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
            toml::from_str(&raw).map_err(|e| ConfigError::Parse {
                path: path.to_path_buf(),
                message: format!("bundled default TOML is invalid (bug): {e}"),
            })
        }
    }
}

fn write_default(path: &std::path::Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    fs::write(path, DEFAULT_TOML).map_err(|e| ConfigError::Io {
        path: path.to_path_buf(),
        source: e,
    })
}

fn backup_path(path: &std::path::Path) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    path.with_file_name(format!("{file_name}.bak.{ts}"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_tmp() -> PathBuf {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("esi-pdf-sign-cfg-{ts}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("templates.toml")
    }

    #[test]
    fn first_launch_writes_default_and_parses() {
        let path = unique_tmp();
        let cfg = load_or_create_at(&path).expect("first launch ok");
        assert!(path.exists(), "default file written");
        assert_eq!(cfg.templates.len(), 1);
        assert_eq!(cfg.templates[0].name, "H5P9");
        assert_eq!(cfg.templates[0].signatures.len(), 2);
        std::fs::remove_dir_all(path.parent().unwrap()).ok();
    }

    #[test]
    fn legacy_schema_is_backed_up_and_replaced() {
        let path = unique_tmp();
        // Pre-existing file with the v0.0.1 single-signature schema.
        std::fs::write(
            &path,
            r#"
[[template]]
name = "H5P9"
match_pages = 2

[template.signature]
page_index = 1
x = 466.3
y = 122.8
width = 106.7
height = 40.0
"#,
        )
        .unwrap();

        let cfg = load_or_create_at(&path).expect("recovers from schema mismatch");

        // New default written in place.
        assert_eq!(cfg.templates.len(), 1);
        assert_eq!(cfg.templates[0].signatures.len(), 2);

        // Old file preserved as .bak.<timestamp>.
        let parent = path.parent().unwrap();
        let saw_bak = std::fs::read_dir(parent)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| e.file_name().to_string_lossy().contains(".bak."));
        assert!(saw_bak, "legacy file should be renamed to .bak.<ts>");

        std::fs::remove_dir_all(parent).ok();
    }
}
