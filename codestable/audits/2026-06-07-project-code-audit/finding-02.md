---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "arch-drift-02"
nature: arch-drift
severity: P1
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 02：发布版本元数据仍是 1.0.2，但当前 release 口径是 1.0.5

## 速答

Rust workspace 和 release notes 已是 `1.0.5`，但 `package.json` 与 `src-tauri/tauri.conf.json` 仍是 `1.0.2`，会导致前端构建日志、Tauri bundle 元数据和发布说明不一致。

## 关键证据

- `Cargo.toml:8-11` — workspace package version 为 `1.0.5`。
- `RELEASE_NOTES_v1.0.5.md:1-13` — 当前发布说明为 `Release v1.0.5`，并声明 v1.0.5 是关键修复版本。
- `package.json:2-4` — npm package version 仍为 `1.0.2`；`pnpm build` 输出也显示 `esi-pdf-sign@1.0.2 build`。
- `src-tauri/tauri.conf.json:3-5` — Tauri app `version` 仍为 `1.0.2`。
- `.github/workflows/release.yml:73-86` — release artifact 名称来自 tag，但 app metadata 仍由 Tauri 配置参与打包。

## 影响

用户拿到 `v1.0.5` tag 的安装包时，系统或应用元数据可能仍显示 `1.0.2`。这会直接影响排障、升级判断、回滚判断和用户确认是否已安装“签名位置修复版”。考虑到 release notes 明确要求用户从 v1.0.3/v1.0.4 升级到 v1.0.5，该漂移会放大支持成本。

## 修复方向

统一版本来源，至少同步 `package.json`、`src-tauri/tauri.conf.json`、workspace 版本；发布流程中增加一个检查脚本，阻止 tag 版本与元数据不一致。

## 建议动作

`cs-issue`，因为这是发布正确性问题，影响用户能否确认关键修复是否真正安装。
