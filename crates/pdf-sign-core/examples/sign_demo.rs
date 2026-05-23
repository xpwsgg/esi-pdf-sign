//! End-to-end check for anchor-relative dual signing.
//!
//! Usage:
//!   cargo run -p pdf-sign-core --example sign_demo -- <pdf> <engineer.png> [customer.png]
//!
//! If `customer.png` is supplied a second image is stamped on the
//! "Authorised Customer's Signature" anchor; otherwise only the engineer
//! slot is signed.

use pdf_sign_core::{sign_pdf, SignSpec};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: {} <pdf> <engineer.png> [customer.png]",
            args[0]
        );
        std::process::exit(2);
    }
    let pdf = PathBuf::from(&args[1]);
    let engineer_png = PathBuf::from(&args[2]);
    let customer_png = args.get(3).map(PathBuf::from);

    let engineer = SignSpec {
        page_index: 1,
        anchor_text: "ESI Engineer's Signature".to_string(),
        dx: 0.0,
        dy: 22.634,
        width: 106.7,
        height: 40.0,
    };
    let customer = SignSpec {
        anchor_text: "Authorised Customer's Signature".to_string(),
        ..engineer.clone()
    };

    let mut pairs: Vec<(&SignSpec, &std::path::Path)> = vec![(&engineer, &engineer_png)];
    if let Some(c) = &customer_png {
        pairs.push((&customer, c.as_path()));
    }

    let out = sign_pdf(&pdf, &pairs)?;
    println!(
        "signed [{}] -> {}",
        if customer_png.is_some() { "engineer+customer" } else { "engineer" },
        out.display()
    );
    Ok(())
}
