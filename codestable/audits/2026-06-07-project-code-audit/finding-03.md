---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "bug-03"
nature: bug
severity: P1
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 03：`templates.toml` 任意解析失败都会静默备份并重置为默认模板

## 速答

配置加载把所有 TOML 解析错误都当作 schema mismatch 处理：重命名旧文件，写入默认 H5P9 配置，然后继续运行。这会在用户手动改错一个字段时静默丢掉自定义模板。

## 关键证据

- `src-tauri/src/config.rs:83-90` — 应用启动/命令执行时调用 `load_or_create_at` 读取配置。
- `src-tauri/src/config.rs:98-119` — `toml::from_str::<AppConfig>(&raw)` 失败后，不返回错误给 UI，而是 `fs::rename(path, &backup)`，`write_default(path)`，再解析默认配置。
- `src-tauri/src/config.rs:101-104` — 注释把失败归因为 schema mismatch，但实际 `toml::from_str` 失败也包含用户 TOML 语法错误、字段类型错误、`page_index` 写成字符串等普通编辑错误。
- `README.md:167-169` — 文档只说明 schema 不兼容会自动备份并写默认值，没有提示普通配置错误也会触发同样行为。

## 影响

用户新增模板或微调坐标时，只要写错一个字段，应用会把整个自定义配置移走并恢复默认 H5P9。后续签名可能找不到新模板，也可能用默认坐标继续处理，形成“配置看起来被吃掉”的体验。对内部工具来说，这类静默恢复比直接报错更难排障。

## 修复方向

区分可识别的旧 schema 迁移与普通解析错误；普通解析错误应返回明确错误并保留原文件原位。若仍要自动恢复，应至少在 UI 中告知备份路径和恢复原因。

## 建议动作

`cs-issue`，因为这是可触发的数据/配置丢失类问题，需要修复并补配置错误测试。
