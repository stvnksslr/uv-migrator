#!/usr/bin/env bash
set -euo pipefail

# Verify curl is available
if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required but not installed. Please install curl first."
    exit 1
fi

REPO="stvnksslr/uv-migrator"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="uv-migrator"

# Create install directory if it doesn't exist
mkdir -p "${INSTALL_DIR}"

# Ensure .local/bin is in PATH
if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    echo "Adding ${INSTALL_DIR} to PATH in your shell profile..."
    SHELL_PROFILE=""
    if [[ -f "${HOME}/.zshrc" ]]; then
        SHELL_PROFILE="${HOME}/.zshrc"
    elif [[ -f "${HOME}/.bashrc" ]]; then
        SHELL_PROFILE="${HOME}/.bashrc"
    elif [[ -f "${HOME}/.profile" ]]; then
        SHELL_PROFILE="${HOME}/.profile"
    fi

    if [[ -n "${SHELL_PROFILE}" ]]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >>"${SHELL_PROFILE}"
        echo "Added ${INSTALL_DIR} to PATH in ${SHELL_PROFILE}"
        echo "Please restart your shell or run: source ${SHELL_PROFILE}"
    else
        echo "Warning: Could not find shell profile to update PATH"
        echo "Please manually add ${INSTALL_DIR} to your PATH"
    fi
fi

# Determine system architecture
ARCH=$(uname -m)
case ${ARCH} in
x86_64) ARCH="x86_64" ;;
aarch64 | arm64) ARCH="aarch64" ;;
*)
    echo "Unsupported architecture: ${ARCH}"
    exit 1
    ;;
esac

# Determine OS
OS=$(uname -s)
case ${OS} in
Linux) OS="unknown-linux-gnu" ;;
Darwin) OS="apple-darwin" ;;
*)
    echo "Unsupported OS: ${OS}"
    exit 1
    ;;
esac

echo "Fetching latest release..."
LATEST_RELEASE=$(curl -s https://api.github.com/repos/${REPO}/releases/latest)
VERSION=$(echo "${LATEST_RELEASE}" | grep -o '"tag_name": "[^"]*' | cut -d'"' -f4)
ASSET_NAME="${BINARY_NAME}-${ARCH}-${OS}.tar.gz"

# Get download URL
DOWNLOAD_URL=$(echo "${LATEST_RELEASE}" | grep -o "\"browser_download_url\": \"[^\"]*${ASSET_NAME}\"" | cut -d'"' -f4)

if [ -z "${DOWNLOAD_URL}" ]; then
    echo "Could not find download URL for ${ASSET_NAME}"
    exit 1
fi

# Create temporary directory
TMP_DIR=$(mktemp -d)
trap 'rm -rf ${TMP_DIR}' EXIT

# Download and extract
echo "Downloading ${ASSET_NAME}..."
curl -sL "${DOWNLOAD_URL}" -o "${TMP_DIR}/${ASSET_NAME}"
tar -xzf "${TMP_DIR}/${ASSET_NAME}" -C "${TMP_DIR}"

# Install binary
echo "Installing to ${INSTALL_DIR}..."
mv "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

echo "Successfully installed ${BINARY_NAME} ${VERSION} to ${INSTALL_DIR}"
echo "Run 'uv-migrator --help' to get started"
