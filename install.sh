#!/bin/sh
# Stint installer — detects platform and installs the best way available.
# Usage: curl -fsSL https://daltonr121.github.io/stint/install.sh | sudo sh
set -e

echo "Installing Stint..."

# Detect OS and architecture
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux)
        # Check if apt is available (Debian/Ubuntu)
        if command -v apt-get > /dev/null 2>&1; then
            echo "Detected Debian/Ubuntu — installing via apt..."
            curl -fsSL https://daltonr121.github.io/stint/stint.gpg | gpg --dearmor -o /usr/share/keyrings/stint.gpg
            echo "deb [signed-by=/usr/share/keyrings/stint.gpg] https://daltonr121.github.io/stint stable main" > /etc/apt/sources.list.d/stint.list
            apt-get update -qq
            apt-get install -y stint
            echo "Done! Run 'stint --version' to verify."
            exit 0
        fi

        # Fallback to tarball
        case "$ARCH" in
            x86_64|amd64) TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64|arm64) TARGET="aarch64-unknown-linux-gnu" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac

        echo "Downloading binary for Linux $ARCH..."
        LATEST=$(curl -fsSL https://api.github.com/repos/DaltonR121/stint/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
        curl -fsSL "https://github.com/DaltonR121/stint/releases/download/${LATEST}/stint-${TARGET}.tar.gz" | tar xz -C /usr/local/bin
        echo "Done! Run 'stint --version' to verify."
        ;;

    Darwin)
        case "$ARCH" in
            x86_64) TARGET="x86_64-apple-darwin" ;;
            arm64) TARGET="aarch64-apple-darwin" ;;
            *) echo "Unsupported architecture: $ARCH"; exit 1 ;;
        esac

        echo "Downloading binary for macOS $ARCH..."
        LATEST=$(curl -fsSL https://api.github.com/repos/DaltonR121/stint/releases/latest | grep '"tag_name"' | cut -d'"' -f4)
        curl -fsSL "https://github.com/DaltonR121/stint/releases/download/${LATEST}/stint-${TARGET}.tar.gz" | tar xz -C /usr/local/bin
        echo "Done! Run 'stint --version' to verify."
        ;;

    *)
        echo "Unsupported OS: $OS"
        echo "Try: cargo install stint-cli"
        exit 1
        ;;
esac
