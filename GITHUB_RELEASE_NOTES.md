## ESI PDF Sign v1.0.6

**发布日期**: 2026-06-07

本次版本主要修复代码审计中发现的问题，提升代码质量、安全性和可靠性。

---

## 🔒 安全性改进

- **升级依赖版本**
  - Vite: 5.4.21 → 6.4.3（修复 moderate 级别的 dev server 漏洞）
  - esbuild: 0.21.5 → 0.25.12（通过 Vite 依赖自动升级）
  - ✅ 前端依赖安全审计：无已知漏洞
  - ✅ Rust 依赖安全扫描：无安全漏洞

## 🐛 Bug 修复

1. **配置解析错误处理改进** ([#finding-03](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/finding-03.md))
   - 区分旧 schema 自动迁移和用户配置错误
   - 用户配置错误时返回明确的错误信息，不再静默备份重置
   - 避免用户自定义模板"消失"

2. **锚点搜索错误报告改进** ([#finding-04](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/finding-04.md))
   - 保留页面内容流解码错误信息
   - 未找到锚点时，如果存在解码错误会一并返回
   - 避免误报 `AnchorNotFound` 错误

3. **签名规格数值校验** ([#finding-06](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/finding-06.md))
   - 添加 `SignSpec.validate()` 方法
   - 校验 width/height 必须为正数、有限值且 ≤600 点
   - 校验 dx/dy 必须为有限值且在 ±1000 点范围内
   - 校验 anchor_text 不能为空
   - 阻止异常参数生成无效 PDF

## ⚡ 性能优化

- **PNG 尺寸限制** ([#finding-05](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/finding-05.md))
  - 添加最大尺寸限制：4096×4096 像素
  - 添加最大文件大小限制：10MB
  - 防止异常大图片造成内存和 CPU 放大

## 🧪 测试改进

- **新增 21 个测试用例**
  - ImageTooLarge 错误测试（3个）
  - SignSpec 校验边界测试（18个）
  - 测试总数：19 → 37（+95%）
  - 测试通过率：100% ✅

## 🛠️ 开发工具

- **版本一致性检查脚本**
  - 新增 `scripts/check-version.sh`
  - 自动验证 Cargo.toml、package.json、tauri.conf.json 版本号一致
  - 建议在 CI 流程中使用

- **Rust 依赖安全扫描**
  - 配置 `cargo-audit` 工具
  - 定期扫描依赖安全漏洞

---

## 📥 下载

- **macOS** — `esi-pdf-sign-v1.0.6-macos-universal.dmg`  
  Intel + Apple Silicon 通用 DMG 安装包

- **Windows** — `esi-pdf-sign-v1.0.6-windows-x64-portable.exe`  
  **便携版，双击即用，无需安装**  
  依赖 WebView2 Runtime（Windows 11 默认已带；Windows 10 1803 及以上通常已带）

---

## 🔧 验证结果

```bash
✓ pnpm audit          → 无安全漏洞
✓ pnpm build          → 构建成功
✓ cargo audit         → 0 个安全漏洞
✓ cargo clippy        → 无警告
✓ cargo test          → 37/37 测试通过
✓ 版本一致性检查      → 通过
```

---

## 📄 相关文档

- [完整审计报告](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/index.md)
- [详细修复记录](https://github.com/xpwsgg/esi-pdf-sign/blob/main/codestable/audits/2026-06-07-project-code-audit/FIXES_APPLIED.md)
- [完整变更日志](https://github.com/xpwsgg/esi-pdf-sign/compare/v1.0.5...v1.0.6)

---

**修复了 6 个审计问题，新增 21 个测试用例，代码质量和安全性显著提升！** ✨
