---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit-recheck
finding_id: "security-01"
nature: security
severity: P2
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 01：RustSec warnings 已暴露但未形成可跟踪策略

## 速答

`cargo audit` 现在可以运行且退出码为 0，说明没有直接 security vulnerability；但输出 17 个 warning，包括 GTK3 bindings unmaintained、`glib` unsound、`unic-*` unmaintained、`proc-macro-error` unmaintained。当前仓库没有 audit 配置说明这些 warning 为什么可接受、何时复核。

## 关键证据

- 命令证据：`cargo audit` 输出 17 个 allowed warnings。
- `Cargo.lock:193` — `atk 0.18.2`，对应 RustSec GTK3 unmaintained warning。
- `Cargo.lock:1423-1424` — `glib 0.18.5`，对应 `RUSTSEC-2024-0429` unsound warning。
- `Cargo.lock:1487` — `gtk 0.18.2`，对应 GTK3 bindings unmaintained warning。
- `Cargo.lock:2891` — `proc-macro-error 1.0.4`，对应 unmaintained warning。
- `Cargo.lock:4465` — `unic-ucd-ident 0.9.0`，对应 `unic-*` unmaintained warning；`cargo tree -i unic-ucd-ident` 显示链路来自 `urlpattern -> tauri-utils -> tauri`。

## 影响

这不是立即可利用漏洞，所以不是 P1/P0。但这些 warning 会长期存在于发布依赖链里，如果没有显式策略，后续审计无法判断它们是已知接受、等待上游、还是应该主动升级。`glib` 的 warning 性质是 unsound，比单纯 unmaintained 更需要复核影响面。

## 修复方向

增加 `audit.toml` 或文档化策略：列出每个 advisory、传递依赖来源、当前可接受原因、复核日期和升级条件；同时跟踪 Tauri/wry/gtk 依赖升级路径。

## 建议动作

`cs-issue`，因为需要把安全扫描结果变成可复核的维护策略，并避免发布流程误读为“完全无风险”。
