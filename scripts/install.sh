#!/usr/bin/env bash
# ==============================================================================
# ELCODE Forge (eforge) — Standalone POSIX Installer Script
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/elcode-id/elcode-forge/main/scripts/install.sh | bash
#
# Environment variables:
#   EFORGE_VERSION   Specify target version to install (e.g. "0.6.0", default: latest release)
#   BINDIR           Target install directory (default: ~/.local/bin or /usr/local/bin)
# ==============================================================================

set -euo pipefail

REPO_OWNER="elcode-id"
REPO_NAME="elcode-forge"
GITHUB_API="https://api.github.com/repos/${REPO_OWNER}/${REPO_NAME}"

# Color output helpers (respecting NO_COLOR)
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  BOLD="\033[1m"
  GREEN="\033[32m"
  YELLOW="\033[33m"
  RED="\033[31m"
  CYAN="\033[36m"
  RESET="\033[0m"
else
  BOLD=""
  GREEN=""
  YELLOW=""
  RED=""
  CYAN=""
  RESET=""
fi

log_info() {
  printf "${CYAN}info:${RESET} %s\n" "$1"
}

log_ok() {
  printf "${GREEN}✓${RESET} %s\n" "$1"
}

log_warn() {
  printf "${YELLOW}warn:${RESET} %s\n" "$1"
}

log_err() {
  printf "${RED}error:${RESET} %s\n" "$1" >&2
}

# 1. Detect OS and Architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)
    case "$ARCH" in
      x86_64|amd64)
        TARGET="x86_64-unknown-linux-gnu"
        ;;
      *)
        log_err "Unsupported Linux CPU architecture: ${ARCH}. Supported: x86_64."
        exit 1
        ;;
    esac
    ;;
  Darwin)
    case "$ARCH" in
      arm64|aarch64)
        TARGET="aarch64-apple-darwin"
        ;;
      x86_64)
        # Fallback to Apple Silicon binary via Rosetta 2 or warn
        log_warn "macOS x86_64 detected. Attempting to use aarch64 binary with Rosetta 2."
        TARGET="aarch64-apple-darwin"
        ;;
      *)
        log_err "Unsupported macOS CPU architecture: ${ARCH}. Supported: arm64."
        exit 1
        ;;
    esac
    ;;
  *)
    log_err "Unsupported operating system: ${OS}. Supported: Linux, Darwin (macOS)."
    exit 1
    ;;
esac

log_info "Detected system: ${OS} (${ARCH}) -> ${TARGET}"

# 2. Resolve Target Version
if [ -n "${EFORGE_VERSION:-}" ]; then
  VERSION="v${EFORGE_VERSION#v}"
  log_info "Using specified version: ${VERSION}"
else
  log_info "Resolving latest release version from GitHub..."
  LATEST_JSON=$(curl -fsSL "${GITHUB_API}/releases/latest" 2>/dev/null || echo "")
  if [ -n "$LATEST_JSON" ]; then
    VERSION=$(echo "$LATEST_JSON" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
  else
    VERSION="v0.6.0"
  fi
  log_info "Target version: ${VERSION}"
fi

# 3. Determine Installation Directory
if [ -n "${BINDIR:-}" ]; then
  INSTALL_DIR="$BINDIR"
elif [ -w "/usr/local/bin" ]; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
fi

mkdir -p "$INSTALL_DIR"

# 4. Download Release Archive & Checksum
ARCHIVE_NAME="eforge-${VERSION}-${TARGET}.tar.gz"
DOWNLOAD_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/${ARCHIVE_NAME}"
CHECKSUMS_URL="https://github.com/${REPO_OWNER}/${REPO_NAME}/releases/download/${VERSION}/SHA256SUMS.txt"

TMP_DIR="$(mktemp -d 2>/dev/null || mktemp -d -t 'eforge-install')"
trap 'rm -rf "$TMP_DIR"' EXIT

log_info "Downloading ${ARCHIVE_NAME}..."
curl -fsSL "$DOWNLOAD_URL" -o "${TMP_DIR}/${ARCHIVE_NAME}"

log_info "Verifying SHA-256 checksum..."
curl -fsSL "$CHECKSUMS_URL" -o "${TMP_DIR}/SHA256SUMS.txt"

cd "$TMP_DIR"
EXPECTED_HASH=$(grep "${ARCHIVE_NAME}" SHA256SUMS.txt | awk '{print $1}' || echo "")

if [ -z "$EXPECTED_HASH" ]; then
  log_err "Could not find checksum for ${ARCHIVE_NAME} in SHA256SUMS.txt."
  exit 1
fi

if command -v sha256sum >/dev/null 2>&1; then
  ACTUAL_HASH=$(sha256sum "${ARCHIVE_NAME}" | awk '{print $1}')
elif command -v shasum >/dev/null 2>&1; then
  ACTUAL_HASH=$(shasum -a 256 "${ARCHIVE_NAME}" | awk '{print $1}')
else
  log_warn "Neither sha256sum nor shasum is available; skipping local checksum computation."
  ACTUAL_HASH="$EXPECTED_HASH"
fi

if [ "$EXPECTED_HASH" != "$ACTUAL_HASH" ]; then
  log_err "SHA-256 checksum mismatch!"
  log_err "Expected: ${EXPECTED_HASH}"
  log_err "Actual:   ${ACTUAL_HASH}"
  exit 1
fi

log_ok "SHA-256 checksum verified (${EXPECTED_HASH})"

# 5. Extract and Install
log_info "Extracting archive..."
tar -xzf "${ARCHIVE_NAME}"

# Locate extracted binary
EXTRACTED_BIN=$(find . -type f -name "eforge" | head -n 1)

if [ -z "$EXTRACTED_BIN" ]; then
  log_err "Could not find 'eforge' binary inside extracted archive."
  exit 1
fi

cp "$EXTRACTED_BIN" "${INSTALL_DIR}/eforge"
chmod 755 "${INSTALL_DIR}/eforge"

# Install elcode-forge alias if present
EXTRACTED_ALIAS=$(find . -type f -name "elcode-forge" | head -n 1 || echo "")
if [ -n "$EXTRACTED_ALIAS" ]; then
  cp "$EXTRACTED_ALIAS" "${INSTALL_DIR}/elcode-forge"
  chmod 755 "${INSTALL_DIR}/elcode-forge"
else
  ln -sf "${INSTALL_DIR}/eforge" "${INSTALL_DIR}/elcode-forge" 2>/dev/null || true
fi

log_ok "Installed 'eforge' and 'elcode-forge' to ${INSTALL_DIR}"

# 6. Verify Execution
if command -v "${INSTALL_DIR}/eforge" >/dev/null 2>&1; then
  INSTALLED_VER=$("${INSTALL_DIR}/eforge" --version 2>&1 || echo "unknown")
  log_ok "Installation verified: ${INSTALLED_VER}"
fi

# 7. Check PATH
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    ;;
  *)
    log_warn "${INSTALL_DIR} is not currently in your PATH."
    log_info "Add it to your profile with:"
    printf "  export PATH=\"%s:\$PATH\"\n\n" "$INSTALL_DIR"
    ;;
esac

printf "\n${BOLD}${GREEN}ELCODE Forge installation completed successfully!${RESET}\n"
printf "Run ${CYAN}eforge --help${RESET} or ${CYAN}eforge doctor${RESET} to get started.\n"
