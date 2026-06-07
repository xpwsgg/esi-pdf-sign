//! Debug tool to find which page contains a specific anchor text.
//!
//! Usage:
//!   cargo run -p pdf-sign-core --example find_anchor -- <pdf> <anchor_text>

use lopdf::{Document, content::Content, Object};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <pdf> <anchor_text>", args[0]);
        std::process::exit(2);
    }
    let pdf_path = PathBuf::from(&args[1]);
    let anchor_text = &args[2];

    let doc = Document::load(&pdf_path)?;
    let pages = doc.get_pages();
    let total_pages = pages.len();

    println!("PDF: {}", pdf_path.display());
    println!("Total pages: {}", total_pages);
    println!("Searching for anchor: {:?}\n", anchor_text);

    let mut found = false;
    for page_index in 0..total_pages {
        let page_num = (page_index + 1) as u32;
        match pages.get(&page_num) {
            Some(&page_id) => {
                match doc.get_page_content(page_id) {
                    Ok(content_bytes) => {
                        match Content::decode(&content_bytes) {
                            Ok(content) => {
                                let chunks = scan_text(&content);
                                let mut found_in_page = false;
                                for chunk in &chunks {
                                    if chunk.contains(anchor_text) {
                                        println!("✓ FOUND on page {} (index {})", page_num, page_index);
                                        println!("  Text chunk: {:?}", chunk);
                                        found_in_page = true;
                                        found = true;
                                    }
                                }
                                if !found_in_page {
                                    println!("✗ NOT FOUND on page {} (index {})", page_num, page_index);
                                }
                            }
                            Err(e) => {
                                println!("✗ ERROR decoding content on page {}: {}", page_num, e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("✗ ERROR reading content on page {}: {}", page_num, e);
                    }
                }
            }
            None => {
                println!("✗ Page {} not found", page_num);
            }
        }
    }

    if !found {
        println!("\n❌ Anchor text {:?} not found in any page!", anchor_text);
    } else {
        println!("\n✓ Anchor found successfully!");
    }

    Ok(())
}

fn scan_text(content: &Content) -> Vec<String> {
    let mut texts = Vec::new();
    let mut in_text = false;

    for op in &content.operations {
        match op.operator.as_str() {
            "BT" => in_text = true,
            "ET" => in_text = false,
            "Tj" | "TJ" | "'" | "\"" if in_text => {
                for operand in &op.operands {
                    if let Object::String(bytes, _) = operand {
                        if let Some(text) = ascii_filter(bytes) {
                            texts.push(text);
                        }
                    } else if let Object::Array(arr) = operand {
                        for el in arr {
                            if let Object::String(bytes, _) = el {
                                if let Some(text) = ascii_filter(bytes) {
                                    texts.push(text);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    texts
}

fn ascii_filter(bytes: &[u8]) -> Option<String> {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        if (0x20..=0x7E).contains(&b) {
            s.push(b as char);
        } else {
            return None;
        }
    }
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}
