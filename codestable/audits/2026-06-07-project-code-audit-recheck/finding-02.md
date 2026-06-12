---
doc_type: audit-finding
audit: 2026-06-07-project-code-audit-recheck
finding_id: "maintainability-02"
nature: maintainability
severity: P2
confidence: high
suggested_action: cs-refactor
status: open
---

# Finding 02：发布质量门禁脚本未接入 CI，release notes 更新脚本可能操作错 workflow

## 速答

项目新增了 `scripts/check-version.sh`，但 `.github/workflows/release.yml` 还没有调用它，也没有调用 `pnpm audit`/`cargo test`/`cargo clippy`/`cargo audit`。另外未跟踪的 `scripts/update-release-notes.sh` 通过 `gh run list --limit 1` 读取最新 workflow，可能在仓库有其他 workflow 时更新错 release。

## 关键证据

- `scripts/check-version.sh:20-47` — 脚本能比较 `Cargo.toml`、`package.json`、`src-tauri/tauri.conf.json` 的版本号；本地运行已通过。
- `.github/workflows/release.yml:62-69` — release workflow 只安装依赖、生成 icon、执行 `pnpm tauri build`，没有版本一致性检查、audit、clippy、test 步骤。
- `scripts/update-release-notes.sh:13-24` — 使用 `gh run list --limit 1` 获取最新 run 状态和结论，没有按 workflow 名称、branch/tag、event 或 commit SHA 过滤。
- `scripts/update-release-notes.sh:7-8` — 版本号和 notes 文件硬编码为 `v1.0.6` 与 `GITHUB_RELEASE_NOTES.md`。
- `git status --short` — `scripts/update-release-notes.sh` 当前是未跟踪文件，发布辅助流程没有进入受控版本。

## 影响

版本漂移 finding 已通过脚本本地修复，但脚本没有进入 release workflow，下一次 tag 发布仍可能绕过检查。`update-release-notes.sh` 如果被使用，在多个 workflow 并发或最近一次 run 不是目标 release run 时，可能编辑错误 release 或误判构建状态。

## 修复方向

把版本检查、前端 audit、Rust test/clippy、必要的 cargo audit 策略接入 release workflow 的 build 前置步骤；若保留 release notes 更新脚本，应按 `workflow name + tag/ref + commit SHA` 定位 run，并避免硬编码版本。

## 建议动作

`cs-refactor`，因为主要是发布流程可靠性和脚本健壮性改进，不改变应用行为。
