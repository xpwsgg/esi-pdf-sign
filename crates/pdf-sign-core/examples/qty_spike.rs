//! 临时 spike: 验证能否从服务报告 PDF 的 QTY 列提取工时数字。
//! 复用 anchor_spike 的 content-stream 扫描逻辑（追踪 text matrix），
//! dump 每页带坐标的 ASCII chunk，并自动定位 "QTY" 表头与其下方数字。
//!
//! 用法: cargo run -p pdf-sign-core --example qty_spike -- <pdf-path>

use lopdf::{content::Content, Document, Object};
use std::env;

#[derive(Clone)]
struct Mat([f64; 6]);
impl Mat {
    fn identity() -> Self {
        Mat([1., 0., 0., 1., 0., 0.])
    }
    fn translate(&mut self, tx: f64, ty: f64) {
        let m = self.0;
        self.0[4] = m[4] + tx * m[0] + ty * m[2];
        self.0[5] = m[5] + tx * m[1] + ty * m[3];
    }
}

#[derive(Clone)]
struct Chunk {
    x: f64,
    y: f64,
    text: String,
}

fn num(o: &Object) -> f64 {
    match o {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => *r as f64,
        _ => 0.,
    }
}

fn ascii(bytes: &[u8]) -> Option<String> {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        if (0x20..=0x7e).contains(&b) {
            s.push(b as char);
        } else {
            return None;
        }
    }
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

fn push_str(operands: &[Object], tm: &Mat, out: &mut Vec<Chunk>) {
    if let Some(Object::String(b, _)) = operands.first() {
        if let Some(text) = ascii(b) {
            out.push(Chunk {
                x: tm.0[4],
                y: tm.0[5],
                text,
            });
        }
    }
}

fn push_arr(operands: &[Object], tm: &Mat, out: &mut Vec<Chunk>) {
    let Some(Object::Array(arr)) = operands.first() else {
        return;
    };
    let mut buf = Vec::new();
    for el in arr {
        if let Object::String(b, _) = el {
            buf.extend_from_slice(b);
        }
    }
    if let Some(text) = ascii(&buf) {
        out.push(Chunk {
            x: tm.0[4],
            y: tm.0[5],
            text,
        });
    }
}

fn scan(content: &Content) -> Vec<Chunk> {
    let mut tm = Mat::identity();
    let mut tlm = Mat::identity();
    let mut leading = 0.0_f64;
    let mut in_text = false;
    let mut out = Vec::new();
    for op in &content.operations {
        match op.operator.as_str() {
            "BT" => {
                tm = Mat::identity();
                tlm = Mat::identity();
                in_text = true;
            }
            "ET" => in_text = false,
            "Tm" if op.operands.len() == 6 => {
                let m = [
                    num(&op.operands[0]),
                    num(&op.operands[1]),
                    num(&op.operands[2]),
                    num(&op.operands[3]),
                    num(&op.operands[4]),
                    num(&op.operands[5]),
                ];
                tm = Mat(m);
                tlm = Mat(m);
            }
            "Td" if op.operands.len() == 2 => {
                tlm.translate(num(&op.operands[0]), num(&op.operands[1]));
                tm = tlm.clone();
            }
            "TD" if op.operands.len() == 2 => {
                let ty = num(&op.operands[1]);
                leading = -ty;
                tlm.translate(num(&op.operands[0]), ty);
                tm = tlm.clone();
            }
            "T*" => {
                tlm.translate(0., -leading);
                tm = tlm.clone();
            }
            "TL" if op.operands.len() == 1 => leading = num(&op.operands[0]),
            "Tj" if in_text => push_str(&op.operands, &tm, &mut out),
            "TJ" if in_text => push_arr(&op.operands, &tm, &mut out),
            "'" if in_text => {
                tlm.translate(0., -leading);
                tm = tlm.clone();
                push_str(&op.operands, &tm, &mut out);
            }
            "\"" if in_text && op.operands.len() == 3 => {
                tlm.translate(0., -leading);
                tm = tlm.clone();
                push_str(&op.operands[2..], &tm, &mut out);
            }
            _ => {}
        }
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <pdf-path>", args[0]);
        std::process::exit(2);
    }
    let pdf_path = &args[1];
    let doc = Document::load(pdf_path)?;
    let pages = doc.get_pages();
    println!("PDF: {pdf_path}");
    println!("pages: {}", pages.len());

    for (page_num, page_id) in pages {
        let bytes = doc.get_page_content(page_id)?;
        let content = Content::decode(&bytes)?;
        let mut chunks = scan(&content);
        // 按 y 降序 (页面从上到下), 再按 x 升序
        chunks.sort_by(|a, b| {
            b.y.partial_cmp(&a.y)
                .unwrap()
                .then(a.x.partial_cmp(&b.x).unwrap())
        });

        println!("\n========== PAGE {page_num} | {} chunks ==========", chunks.len());
        for c in &chunks {
            println!("  ({:>7.2}, {:>7.2})  {:?}", c.x, c.y, c.text);
        }

        // 自动定位 QTY 表头
        if let Some(qty) = chunks.iter().find(|c| c.text.trim() == "QTY" || c.text.contains("QTY")) {
            println!("\n  --- QTY header at (x={:.2}, y={:.2}) ---", qty.x, qty.y);
            let tol = 40.0;
            let mut sum = 0.0;
            let mut n = 0;
            for c in &chunks {
                if c.y < qty.y - 1.0 && (c.x - qty.x).abs() < tol {
                    let t = c.text.trim();
                    if let Ok(v) = t.parse::<f64>() {
                        println!("    QTY value candidate: {v}  at (x={:.2}, y={:.2})", c.x, c.y);
                        sum += v;
                        n += 1;
                    }
                }
            }
            println!("    >>> {n} values, SUM = {sum}");
        } else {
            println!("\n  --- no QTY header found on this page ---");
        }
    }
    Ok(())
}
