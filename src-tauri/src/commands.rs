//! Tauri commands exposed to the frontend.
//!
//! `sign_pdfs_cmd` loads the template config, picks the role->png mapping
//! the frontend handed in, then for each PDF it calls `pdf_sign_core::sign_pdf`
//! with the full set of (spec, png) pairs and emits `sign://progress` after
//! each file (per design §2.2 流程级约束).

use crate::config;
use pdf_sign_core::{sign_pdf, SignSpec};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

#[derive(Debug, Serialize, Clone)]
pub struct CmdSignResult {
    pub input: String,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProgressEvent {
    pub done: usize,
    pub total: usize,
    pub last: CmdSignResult,
}

/// Tauri command: batch-sign PDFs.
///
/// `signature_paths` maps a role name (matching `[[template.signature]].role`
/// in templates.toml — "engineer" / "customer" for H5P9) to a PNG file path.
/// Roles that the template defines but the frontend omits are skipped, so
/// the caller can sign just the engineer slot if the customer PNG is absent.
#[tauri::command]
pub async fn sign_pdfs_cmd(
    app: AppHandle,
    pdf_paths: Vec<String>,
    signature_paths: HashMap<String, String>,
    template_name: String,
) -> Result<Vec<CmdSignResult>, String> {
    let cfg = config::load_or_create_default().map_err(|e| e.to_string())?;
    let template = cfg
        .find(&template_name)
        .ok_or_else(|| format!("unknown template: {template_name}"))?;

    // Resolve which template slots the UI handed PNGs for, preserving the
    // order declared in templates.toml.
    let owned: Vec<(&SignSpec, PathBuf)> = template
        .signatures
        .iter()
        .filter_map(|r| {
            signature_paths
                .get(&r.role)
                .map(|p| (&r.spec, PathBuf::from(p)))
        })
        .collect();
    if owned.is_empty() {
        return Err("no signature PNG provided for any role in this template".into());
    }
    let sig_refs: Vec<(&SignSpec, &Path)> = owned.iter().map(|(s, p)| (*s, p.as_path())).collect();

    let inputs: Vec<PathBuf> = pdf_paths.iter().map(PathBuf::from).collect();
    let total = inputs.len();
    let mut results = Vec::with_capacity(total);

    for (i, input) in inputs.iter().enumerate() {
        let cmd_result = match sign_pdf(input, &sig_refs) {
            Ok(output) => CmdSignResult {
                input: input.display().to_string(),
                output: Some(output.display().to_string()),
                error: None,
            },
            Err(error) => CmdSignResult {
                input: input.display().to_string(),
                output: None,
                error: Some(error.to_string()),
            },
        };
        let progress = ProgressEvent {
            done: i + 1,
            total,
            last: cmd_result.clone(),
        };
        let _ = app.emit("sign://progress", &progress);
        results.push(cmd_result);
    }

    Ok(results)
}
