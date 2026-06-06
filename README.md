# ESI 服务报告批量签名

> 给 ESI 服务月报 PDF 批量叠加 PNG 视觉签名，并自动统计工时（QTY 列）的桌面工具。

一个面向 ESI 服务工程师的团队内部桌面应用：选定若干份月报 PDF 与一张签名图片，一键产出全部签名后的 PDF，同时汇总每份报告及合计的工时。

- **跨平台桌面应用**：基于 Tauri 2，构建产物为 macOS DMG 与 Windows 便携 EXE。
- **纯 Rust PDF 处理**：核心库使用 [`lopdf`](https://crates.io/crates/lopdf)，不依赖任何 C 动态库，打包零额外步骤。
- **视觉签章**：在 PDF 页面内容流上叠加签名图，不改动 XFA 表单字段。

---

## 功能特性

| 功能 | 说明 |
|---|---|
| 📄 批量签名 | 一次选多份 PDF，串行处理，单份失败不影响其余，最后汇总成功/失败列表 |
| ✍️ 双签名位 | 支持「工程师签名」（必选）与「客户签名」（可选）两个落点 |
| ⏱️ 工时统计 | 自动解析报告中的 `QTY` 列，按文件展示明细 + 合计总工时 |
| 📌 锚点定位 | 按锚点文本动态定位签名落点，不硬编码坐标，适配同模板内的页面浮动 |
| 💾 记住签名 | 通过 `tauri-plugin-store` 持久化上次使用的签名图路径，重启自动回填 |
| 🔄 实时进度 | 处理过程中通过事件推送进度（`3/12 …`），完成后可一键在文件夹中显示输出 |

---

## 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vanilla TypeScript + [Vite](https://vitejs.dev/) 5（无框架运行时） |
| 桌面框架 | [Tauri](https://tauri.app/) 2（`store` / `dialog` / `opener` 插件） |
| 后端 / 核心库 | Rust 2021（MSRV 1.77.2） |
| PDF 操作 | [`lopdf`](https://crates.io/crates/lopdf) 0.38（纯 Rust） |
| 图像解码 | [`image`](https://crates.io/crates/image) 0.25（PNG） |
| 配置 | [`toml`](https://crates.io/crates/toml) 0.8 |
| 包管理 | [pnpm](https://pnpm.io/) 10.30.3 / Node 20 |

---

## 项目架构

采用 Cargo workspace 的三层结构，核心库与桌面应用解耦，便于独立测试与未来扩展（如增加 CLI 子项目）。

```
esi-pdf-sign/
├── crates/
│   └── pdf-sign-core/        # 纯 Rust 核心库（无 Tauri 依赖，可独立测试）
│       └── src/
│           ├── lib.rs        # 对外 API：sign_pdf / sign_pdfs
│           ├── spec.rs       # SignSpec：锚点相对定位规格
│           ├── anchor.rs     # 在内容流中定位锚点文本基线坐标
│           ├── overlay.rs    # 嵌入 PNG XObject + 追加绘制指令
│           ├── text_scan.rs  # PDF 内容流文本块（Chunk）扫描
│           ├── worktime.rs   # 提取 QTY 工时列
│           └── error.rs      # SignError 错误枚举
├── src-tauri/                # Tauri 后端（Rust）
│   └── src/
│       ├── lib.rs            # 应用入口：注册插件与命令
│       ├── commands.rs       # sign_pdfs_cmd / extract_worktimes_cmd
│       └── config.rs         # 加载 / 自动创建 templates.toml
├── src/                      # 前端（TypeScript）
│   ├── main.ts               # UI 逻辑与命令调用
│   └── styles.css
├── index.html                # 应用界面
└── codestable/               # 设计文档（feature design / brainstorm）
```

**数据流**：前端 `invoke` → Tauri 命令 → 加载 `templates.toml` 找到模板 → 调用 `pdf-sign-core` 逐份处理 → 通过 `sign://progress` 事件回推进度 → 返回结果汇总。

---

## 快速开始

### 环境要求

- [Node.js](https://nodejs.org/) 20+ 与 [pnpm](https://pnpm.io/) 10.30.3
- [Rust](https://www.rust-lang.org/tools/install) 工具链（stable，≥ 1.77.2）
- Tauri 系统依赖：参见 [Tauri Prerequisites](https://tauri.app/start/prerequisites/)
  - macOS：Xcode Command Line Tools
  - Windows：[WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)（Win11 默认已带）
  - Linux：`webkit2gtk` 等系统库

### 安装依赖

```bash
pnpm install
```

### 本地开发

```bash
# 启动桌面应用（开发模式，自动热重载）
pnpm tauri dev
```

> 仅调试前端 UI（不启动 Tauri 窗口）可用 `pnpm dev`，访问 http://localhost:1420。

### 构建打包

```bash
# 打包当前平台的桌面应用
pnpm tauri build
```

产物位于 `src-tauri/target/release/bundle/`。

---

## 使用说明

1. **选择 PDF 文件**：点击「浏览 PDF…」，可多选 H5P9 月报。选定后会立即统计并展示工时。
2. **选择工程师签名图片**：点击「浏览图片…」，选一张**透明背景的 PNG**（必选）。
3. **选择客户签名图片**（可选）：如需客户签名位，再选一张 PNG。
4. **开始签名**：点击「开始签名」，进度实时更新。
5. **查看结果**：完成后每份 PDF 显示 ✓/✗，成功项可「在文件夹中显示」定位输出文件。

**输出位置**：签名后的 PDF 写入源文件所在目录下的 `signed/` 子目录，**保留原文件名**。
例如 `/reports/H5P9-五月.pdf` → `/reports/signed/H5P9-五月.pdf`。

> ⚠️ 重复对同一份 PDF 签名会**覆盖** `signed/` 中的同名文件（无版本号策略）。

---

## 配置：templates.toml

模板定义了「哪份 PDF、在哪个锚点、贴多大签名」。首次启动时会在用户配置目录自动生成默认的 **H5P9** 模板：

| 平台 | 路径 |
|---|---|
| macOS | `~/Library/Application Support/esi-pdf-sign/templates.toml` |
| Linux | `~/.config/esi-pdf-sign/templates.toml` |
| Windows | `%APPDATA%\esi-pdf-sign\templates.toml` |

默认内容：

```toml
[[template]]
name = "H5P9"
match_pages = 2

[[template.signature]]
role = "engineer"
page_index = 1                       # 0-based：H5P9 签在第 2 页
anchor_text = "ESI Engineer's Signature"
dx = 0.0
dy = 22.634
width = 106.7
height = 40.0

[[template.signature]]
role = "customer"
page_index = 1
anchor_text = "Authorised Customer's Signature"
dx = 0.0
dy = 22.634
width = 106.7
height = 40.0
```

字段含义：

- `page_index`：锚点所在页（**0-based**）。
- `anchor_text`：在该页内容流中**逐字搜索**的子串（大小写敏感）。
- `dx` / `dy`：相对锚点首字符基线的偏移（PDF 坐标系，原点在左下角，y 向上，单位 pt）。
- `width` / `height`：签名图绘制尺寸（pt）。

> 配置在应用启动时**一次性加载**，修改后需重启应用。若文件 schema 与新版本不兼容，旧文件会被自动备份为 `templates.toml.bak.<时间戳>` 并写入新默认值，应用不会崩溃。
>
> 新增其他模板：复制一段 `[[template]]` 并调整 `name` / `anchor_text` / 偏移即可。

---

## 工作原理

### 锚点相对定位

不依赖硬编码坐标：签名时在目标页内容流中找到 `anchor_text` 首次出现的位置，取其首字符的文本基线坐标 `(x, y)`，再把签名图的左下角绘制在 `(x + dx, y + dy)`。这样同一模板内即使表格上下浮动，签名也能贴在正确的相对位置。

### 工时（QTY）提取

ESI 报告的工时位于 `QTY` 列，其表头横坐标固定但纵坐标在不同报告间浮动，因此**动态定位**表头：

1. 在文本块中找到 `QTY` 表头，记录其 `(x, y)`。
2. 表头下方、横坐标落在同一列（容差 20pt）内的数字块即为工时值。
3. 同一基线 y（容差 3pt）、位于 QTY 列左侧的文本块即该行的 `part_number` 与 `description`。
4. 第一个含有效 QTY 表格的页面胜出，行按页面 y 从上到下排序，求和得总工时。

---

## 测试

核心逻辑（签名叠加、错误分支、工时解析、配置加载）均有单元测试：

```bash
cargo test            # 运行全部 Rust 测试
cargo clippy          # 静态检查
```

> 部分集成测试依赖真实样本 `H5P9-*.pdf` 与 `fixtures/*.png`（含业务隐私，**不纳入版本库**）。缺失时相关测试会自动跳过。

---

## 发布

向仓库推送 `v*` 形式的 tag（或手动触发 `workflow_dispatch`）即可启动 GitHub Actions 自动构建并发布：

```bash
git tag v1.0.2
git push origin v1.0.2
```

产出物：

- **macOS** — `*-macos-universal.dmg`：Intel + Apple Silicon 通用安装包。
- **Windows** — `*-windows-x64-portable.exe`：便携版，双击即用，无需安装（依赖 WebView2 Runtime）。

CI 定义见 [`.github/workflows/release.yml`](.github/workflows/release.yml)。

---

## 设计边界（本工具明确不做）

为保持简单可靠，以下能力**有意不实现**：

- ❌ **数字签名**：仅做视觉签章，不提供 PKI / PAdES / eIDAS 等密码学完整性保护。
- ❌ **改动表单字段**：只在页面内容流上叠图，不触碰 PDF 的 XFA / AcroForm 字段。
- ❌ **UI 框选位置**：签名落点走配置文件，界面上不做拖拽/框选。
- ❌ **模板自动识别**：当前仅内置 H5P9，新模板靠手动加配置。
- ❌ **签名图编辑器**：用户自备透明背景 PNG。
- ❌ **并发批量**：串行处理（单批通常 < 12 份，串行足够快）。

---

## 许可证

[MIT](https://opensource.org/licenses/MIT)
