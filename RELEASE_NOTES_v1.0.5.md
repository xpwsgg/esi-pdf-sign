# Release v1.0.5

## 🎯 正式修复：签名位置完全正确

**这是最终正确的版本**。v1.0.3 和 v1.0.4 都有严重bug，请直接升级到 v1.0.5。

## 🐛 关键修复

### 问题历史
- **v1.0.2**: 多页PDF签名失败（`AnchorNotFound`）
- **v1.0.3**: 修复了错误查找，但签名位置错误（放在配置的页面而非找到锚点的页面）
- **v1.0.4**: 优化了搜索顺序，但仍然使用错误的页面索引
- **v1.0.5**: ✅ **完全修复** - 签名现在放置在实际找到锚点的页面上

### 根本原因
v1.0.3-v1.0.4 的 `find_anchor_baseline` 函数只返回坐标 `(x, y)`，但签名叠加时仍然使用配置文件中的 `page_index`，导致即使锚点在第3页，签名也被错误地放在第2页。

### 解决方案
1. **修改返回值**：`find_anchor_baseline` 现在返回 `(page_index, x, y)` 三元组
2. **使用实际页面**：签名叠加使用 `found_page_index`，而不是 `spec.page_index`
3. **添加验证**：如果 `page_index` 超出PDF总页数，提前返回 `PageOutOfRange` 错误
4. **智能搜索策略**：最后一页 → 指定页面 → 所有页面（详见README）

## ✅ 验证结果

所有5个PDF签名位置完全正确：
- ✅ H5R12-六月.pdf (2页) - 签名在第2页
- ✅ H5R30-六月.pdf (2页) - 签名在第2页
- ✅ H5R43-六月.pdf (2页) - 签名在第2页
- ✅ H5R5-六月.pdf (3页) - **签名在第3页** ✨
- ✅ H5R54-六月.pdf (2页) - 签名在第2页

✅ 工程师签名和客户签名位置都正确  
✅ 16/16 单元测试通过

## 📝 代码变更

- `crates/pdf-sign-core/src/anchor.rs`: 
  - 返回 `(usize, f64, f64)` 而不是 `(f64, f64)`
  - 添加 `PageOutOfRange` 验证
- `crates/pdf-sign-core/src/lib.rs`: 
  - 使用 `found_page_index` 进行签名叠加
- `README.md`: 
  - 新增"智能页面搜索策略"章节
- `CHANGELOG-fixes.md`: 
  - 完整的修复历史

## 📦 提交历史

```
41e27c6 fix: use actual found page for signature placement, not config page_index
89b2bff chore(release): bump to v1.0.4
b821a0b fix: prioritize last page for anchor search (signatures are always on last page)
e45a1a8 chore(release): bump to v1.0.3
54df5da fix: auto-search all pages when anchor not found on specified page
```

## ⚠️ 升级建议

**使用 v1.0.3 或 v1.0.4 的用户请立即升级到 v1.0.5**，并重新签名所有PDF文件。

---

**Full Changelog**: https://github.com/xpwsgg/esi-pdf-sign/compare/v1.0.4...v1.0.5
