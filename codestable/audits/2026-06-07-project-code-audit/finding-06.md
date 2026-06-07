---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "bug-06"
nature: bug
severity: P2
confidence: medium
suggested_action: cs-issue
status: open
---

# Finding 06：模板签名规格缺少数值校验，异常宽高/偏移会生成不可见或异常 PDF 绘制指令

## 速答

`SignSpec` 从用户配置反序列化后直接进入 PDF 绘制，`width`、`height`、`dx`、`dy` 没有正数、有限数、页面范围或合理区间校验。

## 关键证据

- `crates/pdf-sign-core/src/spec.rs:14-28` — `SignSpec` 只是 `Deserialize` 结构体，没有校验逻辑。
- `src-tauri/src/config.rs:65-70` — `RoleSignSpec` 通过 `#[serde(flatten)]` 直接嵌入 `SignSpec`。
- `src-tauri/src/commands.rs:49-61` — 命令按模板顺序收集 `&SignSpec`，不做业务校验。
- `crates/pdf-sign-core/src/lib.rs:58-64` — placement 直接使用 `spec.dx`、`spec.dy`、`spec.width`、`spec.height`。
- `crates/pdf-sign-core/src/overlay.rs:219-226` — 绘制矩阵直接格式化为 PDF content stream：`{w} 0 0 {h} {x} {y} cm`。

## 影响

用户如果把 `width` 或 `height` 写成 `0`、负数、极大值，应用仍会生成输出 PDF，但签名可能不可见、翻转、覆盖页面大面积内容，或在不同 PDF 查看器中表现异常。由于输出被视为成功，用户可能直到提交报告后才发现签章无效。

## 修复方向

在配置加载后增加 `validate()`：要求 `anchor_text` 非空，`width`/`height` 为有限正数且低于合理上限，偏移为有限数，`page_index` 与模板策略一致；失败时返回明确配置错误并阻止签名。

## 建议动作

`cs-issue`，因为这是输入边界问题，需要明确错误语义和配置校验测试。
