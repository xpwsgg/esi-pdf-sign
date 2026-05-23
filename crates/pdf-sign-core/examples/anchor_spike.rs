//! Spike v2: 直接用 lopdf 解析 page 内容流找 "ESI Engineer's Signature" 的 (x, y)
//! 不依赖 pdf-extract（pdf-extract 对中文 CMap 直接 panic）
//! 策略：扫描 Tj/TJ 操作,跟踪 text matrix tm; 把可见 ASCII 字节拼成字符串,匹配子串。
//! 中文字体字节大多落在 0x80+ 或控制字符范围,会被自然过滤。
//!
//! 用法: cargo run -p pdf-sign-core --example anchor_spike -- <pdf> <page 1-based>

use lopdf::{content::Content, Document, Object};
use std::env;

#[derive(Debug, Clone)]
struct Mat([f64; 6]);

impl Mat {
    fn identity() -> Self {
        Mat([1., 0., 0., 1., 0., 0.])
    }
    /// translate (tx, ty): self = T(tx,ty) * self
    fn translate(&mut self, tx: f64, ty: f64) {
        let m = self.0;
        // result = [a b c d (e + tx*a + ty*c) (f + tx*b + ty*d)]
        self.0[4] = m[4] + tx * m[0] + ty * m[2];
        self.0[5] = m[5] + tx * m[1] + ty * m[3];
    }
}

#[derive(Debug, Clone)]
struct AsciiChunk {
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

fn ascii_visible(b: u8) -> Option<char> {
    if (0x20..=0x7e).contains(&b) {
        Some(b as char)
    } else {
        None
    }
}

fn extract_ascii(bytes: &[u8]) -> Option<String> {
    let mut s = String::with_capacity(bytes.len());
    for &b in bytes {
        if let Some(c) = ascii_visible(b) {
            s.push(c);
        } else {
            return None; // 非 ASCII 字体 (中文 hex 等), 整个 chunk 丢弃
        }
    }
    Some(s)
}

fn collect_tj_string(operands: &[Object], chunks: &mut Vec<AsciiChunk>, tm: &Mat) {
    if let Some(Object::String(bytes, _)) = operands.first() {
        if let Some(text) = extract_ascii(bytes) {
            if !text.is_empty() {
                chunks.push(AsciiChunk {
                    x: tm.0[4],
                    y: tm.0[5],
                    text,
                });
            }
        }
    }
}

fn collect_capital_tj(operands: &[Object], chunks: &mut Vec<AsciiChunk>, tm: &Mat) {
    let Some(Object::Array(arr)) = operands.first() else {
        return;
    };
    let mut buf = Vec::new();
    for el in arr {
        if let Object::String(b, _) = el {
            buf.extend_from_slice(b);
        }
    }
    if let Some(text) = extract_ascii(&buf) {
        if !text.is_empty() {
            chunks.push(AsciiChunk {
                x: tm.0[4],
                y: tm.0[5],
                text,
            });
        }
    }
}

fn find_anchor(chunks: &[AsciiChunk], needle: &str) -> Option<(f64, f64, String)> {
    for c in chunks {
        if c.text.find(needle).is_some() {
            // 起始字符就在这个 chunk 内: 简化起见, 先返回 chunk 起点(idx=0 时精确;
            // idx>0 时近似——但根据实际PDF排版,标签通常是一次 Tj 完成的,idx=0)
            return Some((c.x, c.y, c.text.clone()));
        }
    }
    // 退一步: 跨 chunk 拼接 (考虑标签可能拆成多个 Tj)
    // 这里简化: 把所有 chunks 顺序连接,找子串后定位回原始 chunk
    let mut joined = String::new();
    let mut chunk_offsets: Vec<(usize, &AsciiChunk)> = Vec::new();
    for c in chunks {
        chunk_offsets.push((joined.len(), c));
        joined.push_str(&c.text);
        joined.push('\x1f'); // 分隔符防止跨 chunk 误拼
    }
    if let Some(pos) = joined.find(needle) {
        // 找到起始 chunk
        for (off, c) in chunk_offsets.iter().rev() {
            if *off <= pos {
                return Some((c.x, c.y, format!("(joined) anchor chunk: {}", c.text)));
            }
        }
    }
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <pdf-path> <page-num 1-based>", args[0]);
        std::process::exit(2);
    }
    let pdf_path = &args[1];
    let page_num: u32 = args[2].parse()?;

    let doc = Document::load(pdf_path)?;
    let pages = doc.get_pages();
    let page_id = *pages.get(&page_num).expect("page");
    let content_bytes = doc.get_page_content(page_id)?;
    let content = Content::decode(&content_bytes)?;

    let mut tm = Mat::identity();
    let mut tlm = Mat::identity();
    let mut leading = 0.0_f64;
    let mut chunks: Vec<AsciiChunk> = Vec::new();
    let mut in_text = false;

    for op in &content.operations {
        let opn = op.operator.as_str();
        match opn {
            "BT" => {
                tm = Mat::identity();
                tlm = Mat::identity();
                in_text = true;
            }
            "ET" => {
                in_text = false;
            }
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
                let tx = num(&op.operands[0]);
                let ty = num(&op.operands[1]);
                tlm.translate(tx, ty);
                tm = tlm.clone();
            }
            "TD" if op.operands.len() == 2 => {
                let tx = num(&op.operands[0]);
                let ty = num(&op.operands[1]);
                leading = -ty;
                tlm.translate(tx, ty);
                tm = tlm.clone();
            }
            "T*" => {
                tlm.translate(0., -leading);
                tm = tlm.clone();
            }
            "TL" if op.operands.len() == 1 => {
                leading = num(&op.operands[0]);
            }
            "Tj" if in_text => collect_tj_string(&op.operands, &mut chunks, &tm),
            "TJ" if in_text => collect_capital_tj(&op.operands, &mut chunks, &tm),
            "'" if in_text => {
                // ' = T* + Tj
                tlm.translate(0., -leading);
                tm = tlm.clone();
                collect_tj_string(&op.operands, &mut chunks, &tm);
            }
            "\"" if in_text && op.operands.len() == 3 => {
                tlm.translate(0., -leading);
                tm = tlm.clone();
                collect_tj_string(&op.operands[2..], &mut chunks, &tm);
            }
            _ => {}
        }
    }

    println!("collected {} ascii chunks (page {})", chunks.len(), page_num);
    let needle = "ESI Engineer's Signature";
    match find_anchor(&chunks, needle) {
        Some((x, y, debug)) => {
            println!("FOUND `{}`", needle);
            println!("  baseline (x, y) = ({:.3}, {:.3})", x, y);
            println!("  chunk text: {}", debug);
        }
        None => {
            println!("NOT FOUND. Last 10 chunks:");
            for c in chunks.iter().rev().take(10).rev() {
                println!("  ({:.1},{:.1}) {}", c.x, c.y, c.text);
            }
        }
    }
    Ok(())
}
