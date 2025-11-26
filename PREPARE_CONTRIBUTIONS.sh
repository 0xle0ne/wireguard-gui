#!/bin/bash
# Prepare contributions for WireGuard GUI

set -e

echo "🔧 Preparing bug fixes for contribution..."
echo ""

# Check if gh CLI is installed
if ! command -v gh &> /dev/null; then
    echo "❌ GitHub CLI (gh) not found"
    echo "Install with: sudo dnf install gh"
    echo "Then authenticate: gh auth login"
    exit 1
fi

# Fork the repo if not already forked
echo "📦 Checking fork status..."
if ! gh repo view jbyrd/wireguard-gui &> /dev/null; then
    echo "Forking repository..."
    gh repo fork leon3s/wireguard-gui --clone=false
fi

# Add your fork as remote if not already added
if ! git remote | grep -q "^fork$"; then
    USER=$(gh api user --jq '.login')
    git remote add fork https://github.com/$USER/wireguard-gui.git
fi

echo "✅ Fork configured!"
echo ""

# Create branches for each fix
echo "🌿 Creating fix branches..."

# Fix 1: CSS Class Typo (Easy first contribution!)
git checkout main
git pull origin main
git checkout -b fix/css-class-typo 2>/dev/null || git checkout fix/css-class-typo

echo "📝 Applying CSS fix..."
sed -i 's/animate-pulsemb-2/animate-pulse mb-2/g' app/page.tsx

git add app/page.tsx
git commit -m "fix: correct CSS class typo in disconnected icon

- Fixed missing space between 'animate-pulse' and 'mb-2' classes
- Resolves animation not working properly in disconnected state
- Fixes incorrect bottom margin spacing

Closes #XX" || echo "Already committed"

echo "✅ CSS fix branch ready!"
echo ""

# Fix 2: Update Snap Configuration
git checkout main
git checkout -b fix/snap-build-from-source 2>/dev/null || git checkout fix/snap-build-from-source

echo "📝 Creating updated snap configuration..."

cat > snap/snapcraft.yaml << 'EOF'
name: wireguard-gui
base: core22
version: '0.1.8'
summary: Wireguard client GUI made with nextauri
icon: src-tauri/icons/128x128.png
description: |
  Provide a Wireguard client GUI for easy profile management
  
  This may have bugs, please report them on the repository
  https://github.com/leon3s/wireguard-gui
grade: stable
source-code: https://github.com/leon3s/wireguard-gui
confinement: strict
architectures:
  - build-on: amd64

apps:
  wireguard-gui:
    command: usr/bin/wireguard-gui
    common-id: com.wireguard-gui.gg
    extensions: [gnome]
    desktop: usr/share/applications/wireguard-gui.desktop
    plugs:
      - home
      - network
      - network-manager
      - network-control
      - modem-manager
      - network-setup-observe
      - firewall-control
      - hardware-observe
      - network-setup-control
      - login-session-observe
      - network-observe
      - desktop
      - desktop-legacy

parts:
  wireguard-gui:
    plugin: npm
    source: .
    npm-include-node: true
    npm-node-version: "20.11.0"
    override-build: |
      # Install Rust
      curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
      source $HOME/.cargo/env
      
      # Install dependencies
      npm install
      
      # Build Next.js
      npm run next-build
      
      # Build Tauri app
      npm run tauri build
      
      # Install files
      mkdir -p $SNAPCRAFT_PART_INSTALL/usr/bin
      mkdir -p $SNAPCRAFT_PART_INSTALL/usr/share/applications
      mkdir -p $SNAPCRAFT_PART_INSTALL/usr/share/icons/hicolor/128x128/apps
      
      # Copy binary
      cp src-tauri/target/release/wireguard-gui $SNAPCRAFT_PART_INSTALL/usr/bin/
      
      # Copy icon
      cp src-tauri/icons/128x128.png $SNAPCRAFT_PART_INSTALL/usr/share/icons/hicolor/128x128/apps/wireguard-gui.png
      
      # Create desktop file
      cat > $SNAPCRAFT_PART_INSTALL/usr/share/applications/wireguard-gui.desktop << DESKTOP
[Desktop Entry]
Version=1.0
Type=Application
Name=WireGuard GUI
Comment=Manage WireGuard VPN profiles
Exec=wireguard-gui
Icon=\${SNAP}/usr/share/icons/hicolor/128x128/apps/wireguard-gui.png
Terminal=false
Categories=Network;System;
DESKTOP

    build-packages:
      - curl
      - wget
      - file
      - pkg-config
      - libssl-dev
      - libgtk-3-dev
      - libwebkit2gtk-4.1-dev
      - libayatana-appindicator3-dev
      - librsvg2-dev
      - patchelf
      - libjavascriptcoregtk-4.1-dev
      - libsoup-3.0-dev
      
    stage-packages:
      - wireguard-tools
      - libwebkit2gtk-4.1-0
      - libjavascriptcoregtk-4.1-0
      - libayatana-appindicator3-1
      - libsoup-3.0-0
      
    prime:
      - -usr/share/doc
      - -usr/share/man

layout:
  /usr/lib/$SNAPCRAFT_ARCH_TRIPLET/webkit2gtk-4.1:
    bind: $SNAP/usr/lib/$SNAPCRAFT_ARCH_TRIPLET/webkit2gtk-4.1
EOF

git add snap/snapcraft.yaml
git commit -m "fix: update snap to build from source instead of downloading old .deb

- Snap now builds from source using current code
- Updated version from 0.1.1 to 0.1.8
- Removed dependency on non-existent 0.1.0 .deb file
- Added proper build dependencies for Tauri
- Fixed snap confinement issues
- Added desktop-legacy plug for better compatibility

This fixes the broken snap package that users reported.
The snap was failing with Mesa driver mismatches and missing files.

Resolves: 'The snap version is currently not working' issue
Closes #XX" || echo "Already committed"

echo "✅ Snap fix branch ready!"
echo ""

echo "═══════════════════════════════════════════════════"
echo "✅ All fixes prepared!"
echo "═══════════════════════════════════════════════════"
echo ""
echo "📤 Next steps to contribute:"
echo ""
echo "1. Push the CSS fix:"
echo "   git checkout fix/css-class-typo"
echo "   git push fork fix/css-class-typo"
echo "   gh pr create --base leon3s:main --head jbyrd:fix/css-class-typo \\"
echo "     --title 'fix: correct CSS class typo in disconnected icon' \\"
echo "     --body-file PR_DESCRIPTION_CSS.md"
echo ""
echo "2. Push the snap fix:"
echo "   git checkout fix/snap-build-from-source"
echo "   git push fork fix/snap-build-from-source"
echo "   gh pr create --base leon3s:main --head jbyrd:fix/snap-build-from-source \\"
echo "     --title 'fix: update snap to build from source' \\"
echo "     --body-file PR_DESCRIPTION_SNAP.md"
echo ""
echo "3. Or use the GitHub web interface to create PRs"
echo ""

