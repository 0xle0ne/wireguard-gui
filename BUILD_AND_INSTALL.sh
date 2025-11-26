#!/bin/bash
# Build and install WireGuard GUI on Fedora from source

set -e

echo "🔧 Installing build dependencies for Fedora..."

# Install Tauri prerequisites for Fedora
sudo dnf install -y \
    webkit2gtk4.1-devel \
    openssl-devel \
    curl \
    wget \
    file \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    wireguard-tools \
    gtk3-devel \
    cairo-devel \
    pango-devel \
    gdk-pixbuf2-devel \
    atk-devel \
    libsoup3-devel

# Install Rust if not present
if ! command -v cargo &> /dev/null; then
    echo "📦 Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Install Node.js if not present
if ! command -v node &> /dev/null; then
    echo "📦 Installing Node.js..."
    curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
    sudo dnf install -y nodejs
fi

echo "✅ Dependencies installed!"
echo ""
echo "🔨 Building WireGuard GUI..."

# Build the application
npm install
npm run tauri build

echo ""
echo "📦 Installing WireGuard GUI..."

# The built binary will be in src-tauri/target/release/
BINARY="src-tauri/target/release/wireguard-gui"

if [ -f "$BINARY" ]; then
    sudo cp "$BINARY" /usr/local/bin/wireguard-gui
    sudo chmod +x /usr/local/bin/wireguard-gui
    
    # Create desktop entry
    sudo tee /usr/share/applications/wireguard-gui.desktop > /dev/null <<EOF
[Desktop Entry]
Version=1.0
Type=Application
Name=WireGuard GUI
Comment=Manage WireGuard VPN profiles
Exec=/usr/local/bin/wireguard-gui
Icon=network-vpn
Terminal=false
Categories=Network;System;
StartupNotify=false
EOF
    
    echo "✅ WireGuard GUI installed successfully!"
    echo ""
    echo "🚀 You can now run: wireguard-gui"
    echo "   Or search for 'WireGuard GUI' in your applications menu"
else
    echo "❌ Build failed - binary not found at $BINARY"
    exit 1
fi

