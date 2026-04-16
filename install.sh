#!/bin/bash
set -e

detect_platform() {
  PLATFORM="unknown"
  
  if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
  elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
  elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "win32" ]]; then
    OS="windows"
    echo "Windows detected. Please use the Windows installation instructions instead."
    echo "Visit: https://github.com/tensara/tensara-cli#windows-installation"
    exit 1
  else
    echo "Unsupported operating system: $OSTYPE"
    exit 1
  fi
  
  ARCH="amd64"
  
  PLATFORM="${OS}-${ARCH}"
  echo "Detected platform: $PLATFORM"
}

download_binary() {
  VERSION=${1:-"v1.1.0"}
  
  if [[ "$OS" == "windows" ]]; then
    ASSET_NAME="tensara-${OS}-${ARCH}.exe"
  else
    ASSET_NAME="tensara-${OS}-${ARCH}"
  fi
  
  DOWNLOAD_URL="https://github.com/tensara/tensara-cli/releases/download/${VERSION}/${ASSET_NAME}"
  
  echo "Downloading from: $DOWNLOAD_URL"
  
  TMP_DIR=$(mktemp -d)
  cd "$TMP_DIR"
  
  if command -v curl &> /dev/null; then
    curl -L "$DOWNLOAD_URL" -o "tensara"
  elif command -v wget &> /dev/null; then
    wget -q "$DOWNLOAD_URL" -O "tensara"
  else
    echo "Neither curl nor wget found. Please install one of them and try again."
    exit 1
  fi
  
  chmod +x tensara
}

install_binary() {
  INSTALL_DIR="/usr/local/bin"
  if [[ ! -w "$INSTALL_DIR" ]]; then
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
    
    if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
      echo "Adding $INSTALL_DIR to your PATH"
      SHELL_TYPE=$(basename "$SHELL")
      
      if [[ "$SHELL_TYPE" == "zsh" ]]; then
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$HOME/.zshrc"
        echo "Please run 'source ~/.zshrc' to update your PATH"
      elif [[ "$SHELL_TYPE" == "bash" ]]; then
        echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$HOME/.bashrc"
        echo "Please run 'source ~/.bashrc' to update your PATH"
      else
        echo "Please add $INSTALL_DIR to your PATH manually"
      fi
    fi
  fi
  
  echo "Installing to $INSTALL_DIR"
  mv tensara "$INSTALL_DIR/"
  
  if command -v tensara &> /dev/null; then
    echo "✅ Installation successful! You can now use 'tensara' from your terminal."
  else
    echo "Installation completed, but 'tensara' command not found in PATH."
    echo "Please restart your terminal or run 'tensara' from $INSTALL_DIR/tensara"
  fi
}

cleanup() {
  if [[ -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
}

main() {
  echo "Tensara CLI installer"
  echo "====================="
  
  # Check for version argument
  VERSION=${1:-"v1.1.0"}
  
  detect_platform
  download_binary "$VERSION"
  install_binary
  cleanup
  
  echo ""
  echo "Thank you for installing Tensara CLI!"
  echo "Run 'tensara --help' to get started."
}

# Run the installer
main "$@"
