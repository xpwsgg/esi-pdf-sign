---
doc_type: audit-fixes
audit: 2026-06-07-project-code-audit
fixed_date: 2026-06-07
status: completed
---

# 审计问题修复报告

本次修复针对 2026-06-07 代码审计中发现的 6 个问题，按优先级顺序完成。

## 修复摘要

| # | 性质 | 严重度 | 标题 | 状态 |
|---|---|---|---|---|
| 1 | security | P1 | Vite/esbuild dev 依赖存在已知 moderate 漏洞 | ✅ 已修复 |
| 2 | arch-drift | P1 | 发布版本元数据仍是 1.0.2，但当前 release 口径是 1.0.5 | ✅ 已修复 |
| 3 | bug | P1 | `templates.toml` 任意解析失败都会静默备份并重置为默认模板 | ✅ 已修复 |
| 4 | bug | P2 | 签章锚点搜索吞掉页面内容流解码错误，最终可能误报 AnchorNotFound | ✅ 已修复 |
| 5 | performance | P2 | 签名 PNG 无尺寸/文件大小上限，异常大图片会造成内存和 CPU 放大 | ✅ 已修复 |
| 6 | bug | P2 | 模板签名规格缺少数值校验，异常宽高/偏移会生成不可见或异常 PDF 绘制指令 | ✅ 已修复 |

---

## Finding-01: 升级 Vite/esbuild 依赖解决安全漏洞

**修复内容：**
- 升级 `vite` 从 5.4.21 → 6.4.3
- 升级 `esbuild` 从 0.21.5 → 0.25.12（通过 Vite 依赖自动升级）

**修改文件：**
- `package.json`: 更新 vite 版本约束为 `^6.4.3`
- `pnpm-lock.yaml`: 自动更新

**验证：**
```bash
pnpm audit --audit-level moderate
# 结果：No known vulnerabilities found ✓

pnpm build
# 结果：构建成功 ✓
```

---

## Finding-02: 统一版本号到 1.0.5

**修复内容：**
- 统一所有配置文件中的版本号为 `1.0.5`

**修改文件：**
- `package.json`: 1.0.2 → 1.0.5
- `src-tauri/tauri.conf.json`: 1.0.2 → 1.0.5

**验证：**
- Cargo.toml: 1.0.5 ✓
- package.json: 1.0.5 ✓
- tauri.conf.json: 1.0.5 ✓
- Release Notes: v1.0.5 ✓

---

## Finding-03: 改进 templates.toml 解析错误处理

**修复内容：**
- 新增 `is_legacy_schema()` 函数检测旧版本 schema
- 区分"可迁移的旧 schema"和"用户配置错误"
- 用户配置错误时返回明确错误信息，不再静默备份

**修改文件：**
- `src-tauri/src/config.rs`:
  - 添加 `is_legacy_schema()` 辅助函数
  - 修改解析错误处理逻辑
  - 新增测试用例 `user_config_error_returns_clear_message()`

**验证：**
```bash
cargo test --package esi-pdf-sign --lib config::tests
# 结果：3 个测试全部通过 ✓
# - first_launch_writes_default_and_parses
# - legacy_schema_is_backed_up_and_replaced
# - user_config_error_returns_clear_message (新增)
```

---

## Finding-04: 保留锚点搜索的解码错误信息

**修复内容：**
- 新增错误变体 `AnchorNotFoundWithDecodeErrors`
- 在锚点搜索过程中记录首个内容流解码错误
- 未找到锚点时，如果有解码错误则返回带解码信息的错误

**修改文件：**
- `crates/pdf-sign-core/src/error.rs`:
  - 新增 `AnchorNotFoundWithDecodeErrors` 错误变体
- `crates/pdf-sign-core/src/anchor.rs`:
  - 添加 `first_decode_error` 变量跟踪解码错误
  - 修改所有 `page_chunks` 调用的错误处理
  - 返回带解码信息的错误

**验证：**
```bash
cargo test --package pdf-sign-core
# 结果：16 个测试全部通过 ✓
```

---

## Finding-05: 添加签名 PNG 尺寸限制

**修复内容：**
- 添加文件大小检查（最大 10MB）
- 添加像素尺寸检查（最大 4096x4096）
- 新增错误变体 `ImageTooLarge`

**修改文件：**
- `crates/pdf-sign-core/src/error.rs`:
  - 新增 `ImageTooLarge` 错误变体
- `crates/pdf-sign-core/src/overlay.rs`:
  - 添加常量 `MAX_IMAGE_WIDTH`, `MAX_IMAGE_HEIGHT`, `MAX_FILE_SIZE_MB`
  - 修改 `load_png_rgba()` 函数添加尺寸验证

**验证：**
```bash
cargo test --package pdf-sign-core
# 结果：16 个测试全部通过 ✓
```

---

## Finding-06: 添加模板签名规格数值校验

**修复内容：**
- 为 `SignSpec` 添加 `validate()` 方法
- 在批量签名前验证所有签名规格参数
- 校验规则：
  - `anchor_text` 不能为空
  - `width`/`height` 必须为正数、有限值，且 ≤600 点
  - `dx`/`dy` 必须为有限值，且在 ±1000 点范围内

**修改文件：**
- `crates/pdf-sign-core/src/spec.rs`:
  - 添加常量 `MAX_SIGNATURE_DIMENSION`, `MAX_OFFSET`
  - 为 `SignSpec` 实现 `validate()` 方法
- `src-tauri/src/commands.rs`:
  - 在 `sign_pdfs_cmd` 中添加参数校验调用

**验证：**
```bash
cargo test
# 结果：19 个测试全部通过 ✓
# - pdf-sign-core: 16 个测试
# - esi-pdf-sign: 3 个测试
```

---

## 全量验证

所有修复完成后进行全量验证：

```bash
# 前端依赖安全检查
pnpm audit --audit-level moderate
# ✅ No known vulnerabilities found

# 前端构建
pnpm build
# ✅ TypeScript 检查通过，Vite 构建成功

# Rust 代码质量检查
cargo clippy --all-targets --all-features -- -D warnings
# ✅ 无警告

# Rust 单元测试
cargo test
# ✅ 19 个测试全部通过
```

---

## 后续建议

1. **发布新版本 v1.0.6**
   - 所有修复已完成并验证通过
   - 建议创建 release notes 说明本次修复内容

2. **补充测试覆盖**
   - 为 `ImageTooLarge` 错误添加测试用例（大尺寸 PNG）
   - 为 `SignSpec` 校验添加边界测试用例

3. **安装 cargo-audit**
   - 补充 Rust 依赖的安全扫描
   - 建议在 CI 流程中加入 `cargo audit` 检查

4. **发布前检查清单**
   - 添加脚本验证所有配置文件版本号一致
   - 确保 Cargo.toml、package.json、tauri.conf.json、release notes 同步

---

## 修复统计

- **总计问题数**: 6
- **P1 问题**: 3（全部修复）
- **P2 问题**: 3（全部修复）
- **修改文件数**: 7
- **新增测试用例**: 1
- **测试通过率**: 100% (19/19)
