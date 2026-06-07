# 修复日志

## 2026-06-07: 修复多页PDF签名失败问题

### 问题描述
- H5R5-六月.pdf（3页）签名失败，错误：`AnchorNotFound`
- 其他2页PDF文件签名正常

### 根本原因
- 代码硬编码在第2页（page_index: 1）查找锚点文本
- H5R5有3页，签名框在第3页（page_index: 2）
- 锚点查找函数只搜索指定页面，不会fallback到其他页面

### 解决方案
修改 `anchor.rs` 中的 `find_anchor_baseline` 函数：
1. 首先在指定页面查找（保持性能和向后兼容）
2. 如果未找到，遍历所有页面查找锚点
3. 所有页面都没找到才返回 `AnchorNotFound` 错误

### 测试验证
✅ 所有PDF文件签名成功（H5R12, H5R30, H5R43, H5R5, H5R54）
✅ 16个单元测试全部通过
✅ 添加回归测试：`sign_pdf_finds_anchor_on_any_page_when_page_index_wrong`

### 影响范围
- 向后兼容：现有2页PDF正常工作
- 新功能：支持任意页数的PDF（锚点在任意页）
- 性能：指定页面优先，未命中时才全页扫描

---

## 2026-06-06: 安全性和健壮性改进

### 提交历史

```
ad5309e test: use tempfile to prevent parallel test collisions
77ae18b feat: preserve page decode errors in worktime extraction
1c593e9 chore: fix clippy warnings in example code
12052a8 fix: replace panic with structured error on malformed PDF Resources
0c62e5a fix: prevent worktime race condition on rapid PDF reselection
8e4331f security: enable restrictive CSP for Tauri WebView
```

### 快速总结

✅ **8 个问题已修复** (3 个 P1 + 5 个 P2)
✅ **6 个原子提交** (可独立回滚)
✅ **17 个测试全部通过** (100% 成功率)
✅ **跨平台兼容** (macOS/Linux/Windows)

## 推送到远程

```bash
git push origin main
```

或创建 PR 分支:

```bash
git checkout -b fix/security-robustness-improvements
git push -u origin fix/security-robustness-improvements
```

然后在 GitHub 创建 PR,模板会自动加载。
