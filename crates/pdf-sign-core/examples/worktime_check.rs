//! Dev check: run worktime extraction against a real service-report PDF.
//!
//! Usage: cargo run -p pdf-sign-core --example worktime_check -- <pdf-path>

use pdf_sign_core::extract_worktime;
use std::env;
use std::path::PathBuf;

fn main() {
    let path = match env::args().nth(1) {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("usage: worktime_check <pdf-path>");
            std::process::exit(2);
        }
    };
    match extract_worktime(&path) {
        Ok(wt) => {
            println!("total = {}", wt.total);
            for r in &wt.rows {
                println!("  {:>6} | {:<18} | {}", r.qty, r.part_number, r.description);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
