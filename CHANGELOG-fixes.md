# 修复日志

## 2026-06-07: 修复签名位置错误（v1.0.4）

### 问题描述
- v1.0.3 修复了 `AnchorNotFound` 错误，但签名位置完全错误
- 签名被放置在错误的页面上

### 根本原因
- 锚点搜索找到了正确的锚点文本，但没有返回找到锚点的页面号
- 签名叠加仍然使用配置中指定的 `page_index: 1`（第2页）
- 但实际锚点在第3页（H5R5）或第2页（其他PDF）

### 解决方案
修改锚点搜索策略，优先搜索**最后一页**（ESI报告签名总是在最后一页）：
1. **优先搜索最后一页**（签名最常见的位置）
2. 尝试指定页面（向后兼容）
3. 搜索所有其他页面（fallback）

### 测试验证
✅ 所有5个PDF文件签名位置正确：
- H5R12-六月.pdf (2页) ✓
- H5R30-六月.pdf (2页) ✓
- H5R43-六月.pdf (2页) ✓
- H5R5-六月.pdf (3页) ✓
- H5R54-六月.pdf (2页) ✓

✅ 16/16单元测试全部通过

### 影响范围
- 向后兼容：现有PDF正常工作
- 修复关键bug：签名现在在正确的页面和位置
- 性能优化：优先搜索最后一页

---

## 2026-06-07: 修复多页PDF签名失败问题（v1.0.3 - 已废弃）

⚠️ **v1.0.3 有严重bug，请使用 v1.0.4**

### 问题描述
- H5R5-六月.pdf（3页）签名失败，错误：`AnchorNotFound`
- 其他2页PDF文件签名正常

### 根本原因
- 代码硬编码在第2页（page_index: 1）查找锚点文本
- H5R5有3页，签名框在第3页（page_index: 2）
- 锚点查找函数只搜索指定页面，不会fallback到其他页面

### 解决方案（有bug）
修改 `find_anchor_baseline` 函数遍历所有页面查找锚点
- ❌ 但签名仍然放置在错误的页面上

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
