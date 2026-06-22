#!/usr/bin/env bash
set -euo pipefail

REPO="subhradeepsarkae-ai/woler"
VERSION="v0.1.0"
BIN_NAME="woler"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

# Colors
BOLD='\033[1m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${BOLD}==> woler installer${NC}"

# Detect OS/arch
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
  Linux)  ;;
  *)      echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_STR="x86_64" ;;
  aarch64|arm64) ARCH_STR="aarch64" ;;
  *)      echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

# Ensure install dir exists
mkdir -p "$INSTALL_DIR"

# Try pre-built binary first
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/$BIN_NAME"
echo -e "  ${CYAN}↓${NC} Downloading $BIN_NAME for $ARCH_STR..."

if command -v curl &>/dev/null; then
  DL_CMD="curl -sL -o"
elif command -v wget &>/dev/null; then
  DL_CMD="wget -q -O"
else
  DL_CMD=""
fi

if [ -n "$DL_CMD" ]; then
  $DL_CMD "$INSTALL_DIR/$BIN_NAME" "$DOWNLOAD_URL" && chmod +x "$INSTALL_DIR/$BIN_NAME"
  echo -e "  ${GREEN}✓${NC} Installed to ${BOLD}$INSTALL_DIR/$BIN_NAME${NC}"
  echo -e "  ${GREEN}✓${NC} Run ${BOLD}woler${NC} to start"
  exit 0
fi

# Fallback to cargo install
echo -e "  ${CYAN}→${NC} No curl/wget found, trying cargo install..."
if ! command -v cargo &>/dev/null; then
  echo "  Need Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  exit 1
fi

cargo install --git "https://github.com/$REPO.git" --root "$INSTALL_DIR/.."
echo -e "  ${GREEN}✓${NC} Installed via cargo"
echo -e "  ${GREEN}✓${NC} Run ${BOLD}woler${NC} to start"
