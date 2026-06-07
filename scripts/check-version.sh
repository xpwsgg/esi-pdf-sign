#!/usr/bin/env bash
# 检查所有配置文件中的版本号是否一致
# 用于 CI 流程和发布前验证

set -euo pipefail

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 获取脚本所在目录的父目录（项目根目录）
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

echo "🔍 检查版本号一致性..."
echo ""

# 提取各文件中的版本号
CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
PACKAGE_VERSION=$(grep '"version":' package.json | sed 's/.*"version": "\(.*\)".*/\1/')
TAURI_VERSION=$(grep '"version":' src-tauri/tauri.conf.json | sed 's/.*"version": "\(.*\)".*/\1/')

echo "📦 发现的版本号："
echo "  Cargo.toml:              $CARGO_VERSION"
echo "  package.json:            $PACKAGE_VERSION"
echo "  tauri.conf.json:         $TAURI_VERSION"
echo ""

# 检查一致性
if [ "$CARGO_VERSION" = "$PACKAGE_VERSION" ] && [ "$CARGO_VERSION" = "$TAURI_VERSION" ]; then
    echo -e "${GREEN}✓ 所有版本号一致: v$CARGO_VERSION${NC}"
    echo ""
    exit 0
else
    echo -e "${RED}✗ 版本号不一致！${NC}"
    echo ""
    echo "请确保以下文件中的版本号相同："
    echo "  - Cargo.toml"
    echo "  - package.json"
    echo "  - src-tauri/tauri.conf.json"
    echo ""
    echo "期望版本: v$CARGO_VERSION (从 Cargo.toml)"
    echo ""
    exit 1
fi
