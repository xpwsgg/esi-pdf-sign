---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit
finding_id: "security-01"
nature: security
severity: P1
confidence: high
suggested_action: cs-issue
status: open
---

# Finding 01：Vite/esbuild dev 依赖存在已知 moderate 漏洞

## 速答

`pnpm audit --audit-level moderate` 报出 2 个 moderate 漏洞，当前锁文件使用 `vite@5.4.21` 和 `esbuild@0.21.5`，主要风险在开发服务器暴露和 optimized deps sourcemap 路径处理。

## 关键证据

- `package.json:20-21` — dev 依赖声明为 `vite: ^5.4.0`。
- `pnpm-lock.yaml:30-32` — 实际锁定 `vite@5.4.21`。
- `pnpm-lock.yaml:36-168`、`pnpm-lock.yaml:403`、`pnpm-lock.yaml:679-755` — Vite 链路锁定 `esbuild@0.21.5`。
- 命令证据：`pnpm audit --audit-level moderate` 返回 2 个漏洞：
  - `esbuild <=0.24.2`：开发服务器请求读取问题，patched `>=0.25.0`。
  - `vite <=6.4.1`：optimized deps `.map` path traversal，patched `>=6.4.2`。

## 影响

这些漏洞主要影响本地 dev server，不直接表示 Tauri 生产包可被远程利用。但仓库 README 明确允许 `pnpm dev` 调试前端，且 Tauri dev 也会启动 Vite；在共享网络、浏览器访问 dev server、或开发机打开不可信网页时，风险真实存在。

## 修复方向

升级 Vite 到带修复的版本线，同时刷新 lockfile，确认 Tauri/Vite 构建仍通过；如果选择升级到 Vite 6/7，需要一并确认 Node 版本和插件兼容性。

## 建议动作

`cs-issue`，因为这是明确的依赖安全问题，修复后需要用 audit/build 做闭环验证。
