#!/bin/bash
# Local build script for Discord PS5 homebrew package
# Run this on Linux/WSL with PS5SDK installed

set -e

# Configuration
TITLE_ID="DISCORDPS5"
TITLE="Discord PS5"
VERSION="0.1.0"
APP_VER="01.00"
CONTENT_ID="DISCORDPS5_001"
CATEGORY="media"  # "media" makes it appear in Media tab
SYSTEM_VER="5.00"

# Paths
BINARY_PATH="target/aarch64-unknown-freebsd/release/discord-app"
OUTPUT_PKG="discord-ps5.pkg"
WORK_DIR="build/pkg"
PS5SDK_DIR="${PS5_SDK_PATH:-/opt/ps5sdk}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Discord PS5 Package Builder ===${NC}"
echo "Title ID: $TITLE_ID"
echo "Title: $TITLE"
echo "Version: $VERSION"
echo "Category: $CATEGORY (appears in Media tab)"
echo ""

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo -e "${RED}Error: Binary not found at $BINARY_PATH${NC}"
    echo "Please build the project first with:"
    echo "  cargo build --release --target aarch64-unknown-freebsd --bin discord-app"
    exit 1
fi

# Check if PS5SDK exists
if [ ! -d "$PS5SDK_DIR" ]; then
    echo -e "${YELLOW}Warning: PS5SDK not found at $PS5SDK_DIR${NC}"
    echo "Packaging tools may not be available."
    echo "Set PS5_SDK_PATH environment variable if installed elsewhere."
fi

# Check if Python and PIL are available
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}Error: python3 not found${NC}"
    exit 1
fi

if ! python3 -c "from PIL import Image" 2>/dev/null; then
    echo -e "${YELLOW}Warning: PIL not installed, will create minimal icon${NC}"
    echo "Install with: pip3 install pillow"
fi

# Run the packaging script
echo -e "${GREEN}Building package...${NC}"
python3 packaging/build_pkg.py \
    "$BINARY_PATH" \
    -o "$OUTPUT_PKG" \
    --title-id "$TITLE_ID" \
    --title "$TITLE" \
    --version "$VERSION" \
    --app-ver "$APP_VER" \
    --content-id "$CONTENT_ID" \
    --category "$CATEGORY" \
    --system-ver "$SYSTEM_VER" \
    --ps5sdk "$PS5SDK_DIR" \
    --work-dir "$WORK_DIR"

if [ -f "$OUTPUT_PKG" ]; then
    echo -e "${GREEN}Success! Package created: $OUTPUT_PKG${NC}"
    echo "Size: $(ls -lh $OUTPUT_PKG | awk '{print $5}')"
    echo ""
    echo "To install on PS5:"
    echo "  1. Copy $OUTPUT_PKG to a USB drive"
    echo "  2. On PS5, go to Settings > System > Package Installer"
    echo "  3. Install from USB"
    echo "  4. The app will appear in the Media tab"
else
    echo -e "${YELLOW}Package structure created at $WORK_DIR/package${NC}"
    echo "Use orbis-pub-gen or pkg-python to create final .pkg"
fi