---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "bug-04"
nature: bug
severity: P2
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 04：签章锚点搜索吞掉页面内容流解码错误，最终可能误报 AnchorNotFound

## 速答

`find_anchor_baseline` 在搜索最后一页、指定页和其他页时，只处理 `page_chunks` 的 `Ok` 分支，所有内容流读取/解码错误都会被跳过；如果没有页面匹配锚点，最终统一返回 `AnchorNotFound`。

## 关键证据

- `crates/pdf-sign-core/src/text_scan.rs:60-65` — `page_chunks` 会把页面内容读取或 decode 失败映射为 `ContentStreamRead`。
- `crates/pdf-sign-core/src/anchor.rs:36-40` — 搜索最后一页时仅 `if let Ok(chunks)`，错误被忽略。
- `crates/pdf-sign-core/src/anchor.rs:43-49` — 搜索指定页时同样忽略错误。
- `crates/pdf-sign-core/src/anchor.rs:52-63` — fallback 遍历其他页时也忽略错误。
- `crates/pdf-sign-core/src/anchor.rs:65-70` — 所有页面失败后只返回 `AnchorNotFound`，没有携带任何 decode error。

## 影响

损坏 PDF、加密/压缩流不兼容、或特定页面内容流异常时，用户看到的可能是“锚点不存在”，而不是“页面内容无法读取/解码”。这会误导排障方向。工时提取链路已经在 `worktime.rs:52-70` 保留了 first decode error，签章链路没有同等保护。

## 修复方向

在 `find_anchor_baseline` 中记录首个 `ContentStreamRead`，当没有任何页面能正常证明锚点不存在时返回带 decode 信息的错误；或新增 `AnchorNotFoundWithDecodeErrors`，与工时提取行为保持一致。

## 建议动作

`cs-issue`，因为这是错误语义问题，修复范围小但需要补损坏内容流场景测试。
