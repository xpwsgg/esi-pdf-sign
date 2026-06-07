# 🔧 修复: 安全、健壮性与测试可靠性全面提升

## 📋 概述

本 PR 修复了 8 个已确认的问题,涵盖安全漏洞、运行时稳定性、测试可靠性和代码质量:

- **3 个 P1 级别问题** (安全 + 严重 bug)
- **5 个 P2 级别问题** (质量 + 可维护性)

所有修复已通过完整验证: clippy、测试套件(17 个测试)、前端构建、跨平台兼容性。

---

## 🎯 修复的问题

### P1 — 安全与严重 Bug

#### 1. 🔐 Tauri CSP 安全漏洞
**问题**: WebView 的 CSP 设置为 `null`,无内容安全策略  
**修复**: 启用限制性 CSP — `script-src 'self'` 阻止外部/内联脚本  
**影响**: 防止未来意外引入 XSS 或外部内容风险

```json
// src-tauri/tauri.conf.json
"csp": "default-src 'self'; img-src 'self' asset: https://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self'"
```

#### 2. ⚡ Worktime 竞态条件
**问题**: 快速重新选择 PDF 时,慢速的旧请求可能覆盖新数据  
**修复**: 引入单调递增的 `worktimeReqId`,仅渲染最新请求的结果  
**影响**: UI 始终显示当前选中 PDF 的正确工时数据

```typescript
let worktimeReqId = 0
async function loadWorktimes() {
  const reqId = ++worktimeReqId
  const batch = await invoke("extract_worktimes_cmd", ...)
  if (reqId !== worktimeReqId) return // superseded
  renderWorktime(batch)
}
```

#### 3. 💥 PDF 结构异常导致 panic
**问题**: 损坏的 PDF Resources 引用触发 `expect()` panic,违反"批量签名永不 panic"契约  
**修复**: 新增 `SignError::PdfStructureError`,替换 4 处 `expect()` 为 `map_err()`  
**影响**: 损坏的 PDF 现返回结构化错误而非崩溃,批量处理继续进行

```rust
// 修复前
let obj = doc.get_object_mut(dict_id).expect("resources exists");

// 修复后
let obj = doc.get_object_mut(dict_id).map_err(|_| SignError::PdfStructureError {
    path: pdf_path.to_path_buf(),
    detail: format!("Referenced Resources object {:?} does not exist", dict_id),
})?;
```

---

### P2 — 质量与可维护性

#### 4. 🧹 Clippy 警告阻止 CI
**问题**: `clippy --all-targets` 失败,阻止严格的质量门  
**修复**: 移除冗余 `clone()`,`find().is_some()` → `contains()`  
**影响**: `cargo clippy -D warnings` 通过

#### 5. 🔍 Worktime 解码错误被静默丢弃
**问题**: 页面解码失败时无法诊断真实原因  
**修复**: 新增 `WorktimeTableNotFoundWithDecodeErrors`,保留第一个解码错误  
**影响**: UI 错误消息区分"无工时表"与"内容流损坏"

#### 6. 📍 错误消息显示路径为 "unknown"
**问题**: PDF 结构错误使用占位符路径,批量失败难以诊断  
**修复**: 传递真实 `pdf_path` 到所有错误处理函数  
**影响**: 前端显示实际文件路径,批量失败易于定位

#### 7. 🧪 测试临时路径冲突
**问题**: 固定或时间戳路径在并行测试时可能冲突  
**修复**: 使用 `tempfile::TempDir`,自动随机路径 + RAII 清理  
**影响**: 并行测试稳定,连续 3 次运行 100% 通过

#### 8. 🪟 Windows 编译失败
**问题**: `std::os::unix::fs::PermissionsExt` 在 Windows 不可用  
**修复**: 为 Unix 特定测试添加 `#[cfg(unix)]` 守护  
**影响**: 跨平台编译通过

---

## 🧪 测试

所有修复已通过完整验证:

```bash
✅ cargo clippy --all-targets --workspace -- -D warnings
✅ cargo test --workspace (17 tests, 100% pass rate)
✅ cargo test -- --test-threads=8 (并行测试无冲突)
✅ pnpm build (TypeScript + Vite)
✅ 跨平台兼容 (cfg(unix) guard)
```

**测试覆盖**:
- 15 个 `pdf-sign-core` 单元测试(包括 1 个 Unix 特定测试)
- 2 个 `config` 测试(tempfile 迁移)
- 连续 3 次运行,零失败

---

## 📦 变更范围

### 修改的文件 (12 个)

```
src-tauri/
├── tauri.conf.json              # CSP 配置
├── Cargo.toml                   # tempfile dev 依赖
└── src/config.rs                # tempfile 迁移

src/
└── main.ts                      # 竞态条件修复

crates/pdf-sign-core/
├── Cargo.toml                   # tempfile dev 依赖
├── src/
│   ├── error.rs                 # 新增错误变体 (2)
│   ├── overlay.rs               # panic → 错误 + 路径传递
│   ├── worktime.rs              # 保留解码错误
│   └── lib.rs                   # tempfile + cfg(unix)
└── examples/
    ├── qty_spike.rs             # clippy 修复
    └── anchor_spike.rs          # clippy 修复

Cargo.lock                       # 锁定 tempfile 依赖
```

### 代码统计

- **新增**: ~120 行
- **删除**: ~50 行
- **净增加**: ~70 行

---

## 🔄 向后兼容性

✅ **无破坏性变更**
- 所有 API 保持不变
- 新增的错误变体是附加的,不影响现有错误处理
- CSP 仅限制 WebView,不影响核心功能
- 测试改进对生产代码零影响

---

## 🚀 部署建议

1. **立即合并** — P1 安全和稳定性问题已修复
2. **回归测试** — 虽然已验证,建议在 staging 环境测试批量签名场景
3. **监控** — 关注生产环境是否有新的 `PdfStructureError` 报告,表明遇到了之前会 panic 的损坏 PDF

---

## 🎖️ 质量保证

- ✅ 所有 commit 均为原子性,可独立回滚
- ✅ 遵循 Conventional Commits 规范
- ✅ 每个修复都有明确的问题陈述和验证证据
- ✅ 代码遵循项目现有风格和模式

---

## 📝 相关 Issue

(如果有 issue tracker,在此链接相关 issue)

- Closes #XXX (CSP 安全)
- Closes #XXX (竞态条件)
- Closes #XXX (PDF panic)
- Closes #XXX (测试可靠性)

---

## 👥 审查清单

请审查者重点关注:

- [ ] CSP 策略是否满足 Tauri 应用的安全需求
- [ ] `worktimeReqId` 机制是否正确处理所有竞态场景
- [ ] `PdfStructureError` 的错误消息是否足够可操作
- [ ] tempfile 清理是否在所有测试路径都能正常工作
- [ ] 错误处理是否覆盖所有 panic 点

---

## 🙏 致谢

感谢代码审查和测试反馈！
