---
doc_type: audit-index
audit: 2026-06-07-project-code-audit
scope: "Tauri commands/config, pdf-sign-core signing/worktime pipeline, frontend invoke flow, release/dependency metadata"
created: 2026-06-07
status: active
total_findings: 6
---

# project-code-audit 审计报告

## 范围

本次审计收敛在项目主要交付链路：

- `src-tauri/src/commands.rs`：前端暴露命令、批量签名、工时统计。
- `src-tauri/src/config.rs`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`：模板配置加载、Tauri 权限和发布元数据。
- `crates/pdf-sign-core/src/`：PDF 加载、锚点搜索、签名图片叠加、工时提取。
- `src/main.ts`、`index.html`、`src/styles.css`：前端文件选择、invoke 调用、状态与结果展示。
- `package.json`、`pnpm-lock.yaml`、`.github/workflows/release.yml`、release notes：构建、依赖和发布口径。

已执行验证：

- `cargo test`：18 个 Rust 测试通过。
- `cargo clippy --all-targets --all-features -- -D warnings`：通过。
- `pnpm build`：TypeScript 检查与 Vite build 通过。
- `pnpm audit --audit-level moderate`：发现 2 个 moderate 前端依赖漏洞。

未覆盖：

- `cargo audit` 未安装，本机无法完成 Rust 依赖漏洞扫描。
- 未启动 Tauri GUI 做人工端到端签章视觉复核。

## 总评

整体代码结构清晰，Rust core 与 Tauri shell 分层基本健康，关键签章路径已有单测覆盖，前端也处理了工时统计请求竞态。此次发现 6 条：P1 3 条、P2 3 条；性质分布为 security 1 条、bug 3 条、performance 1 条、arch-drift 1 条。最值得优先处理的是前端 dev 依赖漏洞、发布版本元数据漂移、以及配置解析失败时静默回退默认配置这三项。

## 发现清单

| # | 性质 | 严重度 | 置信度 | 标题 | 文件 |
|---|---|---|---|---|---|
| 1 | security | P1 | high | Vite/esbuild dev 依赖存在已知 moderate 漏洞 | [finding-01.md](finding-01.md) |
| 2 | arch-drift | P1 | high | 发布版本元数据仍是 1.0.2，但当前 release 口径是 1.0.5 | [finding-02.md](finding-02.md) |
| 3 | bug | P1 | high | `templates.toml` 任意解析失败都会静默备份并重置为默认模板 | [finding-03.md](finding-03.md) |
| 4 | bug | P2 | high | 签章锚点搜索吞掉页面内容流解码错误，最终可能误报 AnchorNotFound | [finding-04.md](finding-04.md) |
| 5 | performance | P2 | medium | 签名 PNG 无尺寸/文件大小上限，异常大图片会造成内存和 CPU 放大 | [finding-05.md](finding-05.md) |
| 6 | bug | P2 | medium | 模板签名规格缺少数值校验，异常宽高/偏移会生成不可见或异常 PDF 绘制指令 | [finding-06.md](finding-06.md) |

## 按维度分布

| 性质 | P0 | P1 | P2 | 合计 |
|---|---|---|---|---|
| bug | 0 | 1 | 2 | 3 |
| security | 0 | 1 | 0 | 1 |
| performance | 0 | 0 | 1 | 1 |
| maintainability | 0 | 0 | 0 | 0 |
| arch-drift | 0 | 1 | 0 | 1 |
| **合计** | **0** | **3** | **3** | **6** |

## 下一步建议

- **P1 本迭代修**：finding-01 升级 Vite/esbuild；finding-02 统一版本来源并补发布前检查；finding-03 区分“旧 schema 可迁移”和“用户配置写错”，避免静默丢配置。
- **P2 排期修**：finding-04 保留或汇总内容流解码错误；finding-05 给 PNG 解码设上限；finding-06 对模板规格做边界校验。
- **验证补齐**：安装 `cargo-audit` 后补跑 Rust 依赖审计；修复后跑 `cargo test`、`cargo clippy --all-targets --all-features -- -D warnings`、`pnpm build`、`pnpm audit --audit-level moderate`。
