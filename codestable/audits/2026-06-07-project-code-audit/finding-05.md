---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "performance-05"
nature: performance
severity: P2
confidence: medium
suggested_action: cs-refactor
status: open
---

# Finding 05：签名 PNG 无尺寸/文件大小上限，异常大图片会造成内存和 CPU 放大

## 速答

签章流程会把用户选定的 PNG 完整解码为 RGBA，再复制拆分为 RGB 与 alpha 两个 buffer，并分别 zlib 压缩；当前没有任何文件大小或像素尺寸上限。

## 关键证据

- `src/main.ts:238-245` — 前端文件选择只按扩展名过滤 PNG，没有尺寸或大小预检。
- `src-tauri/src/commands.rs:49-61` — 后端把签名路径转成 `PathBuf` 后直接传给核心库，没有做文件元数据检查。
- `crates/pdf-sign-core/src/overlay.rs:61-72` — `ImageReader::open(...).decode()?.into_rgba8()` 会完整解码图片。
- `crates/pdf-sign-core/src/overlay.rs:73-82` — 继续按 `pixel_count` 分配 `rgb` 和 `alpha`，相当于在 RGBA 原始 buffer 之外再持有约 4 字节/像素的额外数据。
- `crates/pdf-sign-core/src/overlay.rs:40-45` — 每个签名位都会调用解码和压缩；工程师/客户双签时同一 PNG 也会重复处理。

## 影响

正常签名图通常很小，风险不高。但如果用户误选超大 PNG，桌面应用可能长时间卡顿、占用大量内存，甚至被系统杀掉。批量 PDF 时，同一签名图会在每份 PDF 处理时重复解码/压缩，放大 CPU 成本。

## 修复方向

给签名 PNG 设置合理文件大小和像素上限；在批量签名前预加载并缓存签名图片 XObject 数据，避免每份 PDF、每个签名位重复解码。

## 建议动作

`cs-refactor`，因为主要是资源保护和重复计算优化，行为应保持一致。
