#!/usr/bin/env bash
# 等待 v1.0.6 构建完成并更新 Release 说明
# 使用方法：./scripts/update-release-notes.sh

set -euo pipefail

VERSION="v1.0.6"
RELEASE_NOTES_FILE="GITHUB_RELEASE_NOTES.md"

echo "⏳ 等待 ${VERSION} 构建完成..."

# 等待 workflow 完成
while true; do
    status=$(gh run list --limit 1 --json status --jq '.[0].status')
    if [ "$status" = "completed" ]; then
        echo "✓ Workflow 已完成"
        break
    fi
    echo "  状态: $status，等待 30 秒..."
    sleep 30
done

# 检查是否成功
conclusion=$(gh run list --limit 1 --json conclusion --jq '.[0].conclusion')
if [ "$conclusion" != "success" ]; then
    echo "✗ Workflow 失败: $conclusion"
    exit 1
fi

echo "✓ Workflow 成功完成"

# 检查 Release 是否已创建
if ! gh release view "$VERSION" > /dev/null 2>&1; then
    echo "✗ Release ${VERSION} 未找到"
    exit 1
fi

echo "✓ Release ${VERSION} 已创建"

# 更新 Release 说明
if [ -f "$RELEASE_NOTES_FILE" ]; then
    echo "📝 更新 Release 说明..."
    gh release edit "$VERSION" --notes-file "$RELEASE_NOTES_FILE"
    echo "✓ Release 说明已更新"
else
    echo "✗ 未找到 ${RELEASE_NOTES_FILE}"
    exit 1
fi

# 显示最终结果
echo ""
echo "🎉 v1.0.6 发布完成！"
echo ""
gh release view "$VERSION" --web
