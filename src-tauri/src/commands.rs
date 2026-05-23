//! Tauri commands exposed to the frontend.
//!
//! `sign_pdfs_cmd` loads the template config, then for each PDF it calls
//! `pdf_sign_core::sign_pdf` and emits `sign://progress` after each one
//! (per design §2.2 流程级约束: per-file progress event).

use crate::config;
use pdf_sign_core::sign_pdf;
use serde::Serialize;
use std::path::PathBuf;
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
/// Returns one [`CmdSignResult`] per input. Single failures do not abort the
/// batch (design §2.2). Emits `sign://progress` after each PDF.
#[tauri::command]
pub async fn sign_pdfs_cmd(
    app: AppHandle,
    pdf_paths: Vec<String>,
    signature_path: String,
    template_name: String,
) -> Result<Vec<CmdSignResult>, String> {
    let cfg = config::load_or_create_default().map_err(|e| e.to_string())?;
    let template = cfg
        .find(&template_name)
        .ok_or_else(|| format!("unknown template: {template_name}"))?;
    let spec = &template.signature;

    let sig_path = PathBuf::from(&signature_path);
    let inputs: Vec<PathBuf> = pdf_paths.iter().map(PathBuf::from).collect();
    let total = inputs.len();
    let mut results = Vec::with_capacity(total);

    for (i, input) in inputs.iter().enumerate() {
        let cmd_result = match sign_pdf(input, &sig_path, spec) {
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
