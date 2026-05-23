//! Quick end-to-end check that the new anchor-relative signing works on a
//! real PDF. Runs `sign_pdf` with the H5P9 default spec and prints the
//! resulting path + computed placement.
//!
//! Usage:
//!   cargo run -p pdf-sign-core --example sign_demo -- <pdf> <signature.png>

use pdf_sign_core::{sign_pdf, SignSpec};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <pdf> <signature.png>", args[0]);
        std::process::exit(2);
    }
    let pdf = PathBuf::from(&args[1]);
    let sig = PathBuf::from(&args[2]);

    let spec = SignSpec {
        page_index: 1,
        anchor_text: "ESI Engineer's Signature".to_string(),
        dx: 0.0,
        dy: 22.634,
        width: 106.7,
        height: 40.0,
    };

    let out = sign_pdf(&pdf, &sig, &spec)?;
    println!("signed -> {}", out.display());
    Ok(())
}
