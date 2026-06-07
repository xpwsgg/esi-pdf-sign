# Release Notes - v1.0.6

**发布日期**: 2026-06-07

本次版本主要修复代码审计中发现的问题，提升代码质量、安全性和可靠性。

## 🔒 安全性改进

- **升级依赖版本**
  - Vite: 5.4.21 → 6.4.3（修复 moderate 级别的 dev server 漏洞）
  - esbuild: 0.21.5 → 0.25.12（通过 Vite 依赖自动升级）
  - 前端依赖安全审计：无已知漏洞 ✓
  - Rust 依赖安全扫描：无安全漏洞 ✓

## 🐛 Bug 修复

- **配置解析错误处理** (#3)
  - 区分旧 schema 自动迁移和用户配置错误
  - 用户配置错误时返回明确的错误信息，不再静默备份重置
  - 避免用户自定义模板"消失"

- **锚点搜索错误报告** (#4)
  - 保留页面内容流解码错误信息
  - 未找到锚点时，如果存在解码错误会一并返回
  - 避免误报 `AnchorNotFound` 错误

- **签名规格数值校验** (#6)
  - 添加 `SignSpec.validate()` 方法
  - 校验 width/height 必须为正数、有限值且 ≤600 点
  - 校验 dx/dy 必须为有限值且在 ±1000 点范围内
  - 校验 anchor_text 不能为空
  - 阻止异常参数生成无效 PDF

## ⚡ 性能优化

- **PNG 尺寸限制** (#5)
  - 添加最大尺寸限制：4096×4096 像素
  - 添加最大文件大小限制：10MB
  - 防止异常大图片造成内存和 CPU 放大

## 🧪 测试改进

- **新增 21 个测试用例**
  - ImageTooLarge 错误测试（3个）
  - SignSpec 校验边界测试（18个）
  - 测试总数：19 → 37（+95%）
  - 测试通过率：100% ✓

## 🛠️ 开发工具

- **版本一致性检查脚本**
  - 新增 `scripts/check-version.sh`
  - 自动验证 Cargo.toml、package.json、tauri.conf.json 版本号一致
  - 建议在 CI 流程中使用

- **Rust 依赖安全扫描**
  - 安装 `cargo-audit` 工具
  - 定期扫描依赖安全漏洞

## 📝 文档

- **审计报告**
  - 完整审计报告：`codestable/audits/2026-06-07-project-code-audit/`
  - 修复记录：`codestable/audits/2026-06-07-project-code-audit/FIXES_APPLIED.md`

## 🔧 技术细节

**修改的文件**:
- `Cargo.toml`, `package.json`, `src-tauri/tauri.conf.json`: 版本号统一
- `package.json`: 依赖版本升级
- `src-tauri/src/config.rs`: 配置解析错误处理改进
- `crates/pdf-sign-core/src/anchor.rs`: 锚点搜索错误跟踪
- `crates/pdf-sign-core/src/overlay.rs`: PNG 尺寸限制
- `crates/pdf-sign-core/src/spec.rs`: SignSpec 校验
- `crates/pdf-sign-core/src/error.rs`: 新增错误变体
- `src-tauri/src/commands.rs`: 添加签名规格校验调用
- `scripts/check-version.sh`: 新增版本检查脚本

**验证结果**:
```bash
✓ pnpm audit          → 无安全漏洞
✓ pnpm build          → 构建成功
✓ cargo audit         → 0 个安全漏洞，17 个维护警告
✓ cargo clippy        → 无警告
✓ cargo test          → 37/37 测试通过
✓ ./scripts/check-version.sh → 版本号一致
```

## 🎯 下一步计划

- 考虑升级 Tauri 到最新版本（以解决 GTK3 维护警告）
- 补充更多集成测试
- 添加 CI/CD 流程自动化

---

**完整变更日志**: [v1.0.5...v1.0.6](https://github.com/your-repo/compare/v1.0.5...v1.0.6)
