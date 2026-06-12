---
doc_type: audit-index
audit: 2026-06-07-project-code-audit-recheck
scope: "Recheck fixes for 2026-06-07-project-code-audit findings plus release/dependency residual risk"
created: 2026-06-07
status: active
total_findings: 2
---

# project-code-audit-recheck 复审报告

## 范围

本次复审只针对上一轮 `2026-06-07-project-code-audit` 的 6 条发现做闭环确认，并额外检查修复后引入或暴露的发布/依赖残余风险。

复核文件：

- `package.json`、`pnpm-lock.yaml`：Vite/esbuild 升级。
- `Cargo.toml`、`src-tauri/tauri.conf.json`、`scripts/check-version.sh`：版本一致性。
- `src-tauri/src/config.rs`：配置解析错误与旧 schema 处理。
- `crates/pdf-sign-core/src/anchor.rs`、`error.rs`：锚点搜索 decode error 保留。
- `crates/pdf-sign-core/src/overlay.rs`、`spec.rs`、`src-tauri/src/commands.rs`：PNG 限制与签名规格校验。
- `.github/workflows/release.yml`、`scripts/update-release-notes.sh`、release notes：发布门禁和发布脚本。

## 原 6 条 finding 复核

| 原 finding | 原严重度 | 复核状态 | 证据 |
|---|---:|---|---|
| finding-01 Vite/esbuild 漏洞 | P1 | 已关闭 | `package.json:21` 为 `vite ^6.4.3`，`pnpm-lock.yaml:30-32` 锁定 `vite 6.4.3`，`pnpm-lock.yaml:36-90` 显示 esbuild 平台包为 `0.25.12`；`pnpm audit --audit-level moderate` 通过。 |
| finding-02 版本元数据漂移 | P1 | 已关闭 | `Cargo.toml:8-11`、`package.json:2-4`、`src-tauri/tauri.conf.json:3-5` 均为 `1.0.6`；`bash scripts/check-version.sh` 通过。 |
| finding-03 配置解析错误静默回默认 | P1 | 已关闭 | `src-tauri/src/config.rs:98-135` 区分 legacy schema 与用户配置错误；`src-tauri/src/config.rs:243-286` 增加用户配置错误不备份测试；`cargo test` 通过。 |
| finding-04 锚点 decode error 被吞 | P2 | 已关闭 | `crates/pdf-sign-core/src/error.rs:64-70` 新增 `AnchorNotFoundWithDecodeErrors`；`crates/pdf-sign-core/src/anchor.rs:35-87` 记录首个 decode error 并在未找到锚点时返回。 |
| finding-05 PNG 无尺寸/文件大小上限 | P2 | 已关闭 | `crates/pdf-sign-core/src/overlay.rs:22-29` 定义 4096x4096 与 10MB 上限；`overlay.rs:70-116` 执行文件大小和尺寸检查；`overlay.rs:303-378` 有对应测试。 |
| finding-06 SignSpec 缺少数值校验 | P2 | 已关闭，带残余建议 | `crates/pdf-sign-core/src/spec.rs:38-83` 增加 `validate()`；`src-tauri/src/commands.rs:62-70` 在 Tauri 签名前调用。残余：`pdf_sign_core::sign_pdf` 公共 API 本身仍不主动调用 `validate()`，若未来被 CLI/其他库直接调用可能绕过校验。 |

## 本轮验证

已执行并通过：

- `cargo test`：Tauri config 测试 3 个通过，`pdf-sign-core` 37 个测试通过，doc tests 通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：通过。
- `pnpm build`：TypeScript 检查与 Vite 6.4.3 production build 通过。
- `pnpm audit --audit-level moderate`：`No known vulnerabilities found`。
- `bash scripts/check-version.sh`：`Cargo.toml`、`package.json`、`tauri.conf.json` 均为 `1.0.6`。
- `cargo audit`：命令退出码 0，0 个 security vulnerability；输出 17 个 RustSec warning。

验证中遇到的限制：

- `cargo tree -i gtk --target x86_64-unknown-linux-gnu` 与 `cargo tree -i glib --target x86_64-unknown-linux-gnu` 因下载/锁等待超时未完成；没有阻塞 `cargo audit` 本身。
- 未做 Tauri GUI 人工端到端视觉签章复核。

## 新发现/残余风险

| # | 性质 | 严重度 | 置信度 | 标题 | 文件 |
|---|---|---|---|---|---|
| 1 | security | P2 | high | RustSec warnings 已暴露但未形成可跟踪策略 | [finding-01.md](finding-01.md) |
| 2 | maintainability | P2 | high | 发布质量门禁脚本未接入 CI，release notes 更新脚本可能操作错 workflow | [finding-02.md](finding-02.md) |

## 下一步建议

- 原 6 条 finding 可视为关闭。
- 建议把 `scripts/check-version.sh`、`pnpm audit --audit-level moderate`、`cargo clippy`、`cargo test` 接入 `.github/workflows/release.yml` 的 Build 前置步骤。
- 对 `cargo audit` 的 17 个 warning 建立策略：哪些 ignore、到期复核日期、哪些等待 Tauri 上游升级；避免“0 个漏洞”掩盖长期维护风险。
- 若计划复用 `pdf-sign-core` 给 CLI 或其他调用方，建议把 `SignSpec::validate()` 移入 `sign_pdf()`/`sign_pdfs()` 公共 API 边界，或新增 validated spec 类型。
