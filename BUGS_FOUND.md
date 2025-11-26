# 🐛 Bugs Found in WireGuard GUI

## Bug Report for Contributing Back to Project

### 1. **Snap Package is Outdated and Broken** (CRITICAL)
**Location:** `snap/snapcraft.yaml`

**Problem:**
- Snap version stuck at 0.1.1 (Feb 2024)
- Latest release is 0.1.8 (Oct 2025)
- Snap tries to download 0.1.0 .deb which doesn't exist anymore (line 37)
- Results in runtime errors:
  - `libEGL fatal: DRI driver not from this Mesa build`
  - Missing GTK color scheme files
  - Mesa driver version mismatch

**Current Code (lines 3 and 37):**
```yaml
version: '0.1.1'
...
wget https://github.com/leon3s/wireguard-gui/releases/download/0.1.0-stable/wireguard-gui_0.1.0_amd64.deb
```

**Fix Required:**
Change snap to build from source instead of downloading non-existent .deb file.

**Impact:** Users cannot use snap version at all - it's completely broken.

---

### 2. **CSS Class Typo in UI** (MINOR)
**Location:** `app/page.tsx` line 96

**Problem:**
Missing space between CSS classes causes animation to not work properly.

**Current Code:**
```tsx
<Unlock className="animate-pulsemb-2 size-16 text-red-500" />
```

**Should Be:**
```tsx
<Unlock className="animate-pulse mb-2 size-16 text-red-500" />
```

**Impact:** 
- Animation doesn't work for disconnected state icon
- Incorrect bottom margin

---

### 3. **Snap Confinement Issues**
**Location:** `snap/snapcraft.yaml`

**Problem:**
- Strict confinement causes file access issues
- Cannot access `/home/user/.config/gtk-3.0/colors.css`
- PolicyKit (pkexec) doesn't work properly in snap confinement

**Recommendation:**
- Consider using `classic` confinement for this app
- Or add proper interface connections for PolicyKit

---

### 4. **README Warning Not Prominent Enough**
**Location:** `README.md` line 25-27

**Current:**
```markdown
## IMPORTANT

The snap version is currently not working, please use the .deb package instead.
```

**Problem:**
- Users install snap anyway because it's the easiest method
- No .deb packages available in recent releases
- Should provide alternative installation methods

**Recommendation:**
Add prominent warning at top of README with build instructions.

---

## Proposed Fixes

### Fix 1: Update Snap to Build from Source

**File:** `snap/snapcraft.yaml`

```yaml
name: wireguard-gui
base: core22
version: '0.1.8'  # Updated version
summary: Wireguard client GUI made with nextauri
icon: src-tauri/icons/128x128.png
description: |
  Provide a Wireguard client GUI for easy profile management
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
  rust-deps:
    plugin: rust
    source: .
    rust-channel: stable
    override-pull: |
      craftctl default
      # Install Rust if not present
      if ! command -v rustup &> /dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
      fi

  node-deps:
    plugin: npm
    source: .
    npm-include-node: true
    npm-node-version: "20.11.0"
    override-build: |
      npm install
      npm run next-build
      craftctl default

  wireguard-gui:
    after: [rust-deps, node-deps]
    plugin: nil
    source: .
    override-build: |
      # Build with Tauri
      cd $SNAPCRAFT_PART_SRC
      npm run tauri build
      
      # Install binary and desktop file
      mkdir -p $SNAPCRAFT_PART_INSTALL/usr/bin
      mkdir -p $SNAPCRAFT_PART_INSTALL/usr/share/applications
      
      cp src-tauri/target/release/wireguard-gui $SNAPCRAFT_PART_INSTALL/usr/bin/
      
      # Create desktop file
      cat > $SNAPCRAFT_PART_INSTALL/usr/share/applications/wireguard-gui.desktop << EOF
      [Desktop Entry]
      Version=1.0
      Type=Application
      Name=WireGuard GUI
      Comment=Manage WireGuard VPN profiles
      Exec=wireguard-gui
      Icon=${SNAP}/usr/share/icons/hicolor/128x128/apps/wireguard-gui.png
      Terminal=false
      Categories=Network;System;
      EOF
      
    build-packages:
      - wget
      - curl
      - file
      - libssl-dev
      - libgtk-3-dev
      - libwebkit2gtk-4.1-dev
      - libayatana-appindicator3-dev
      - librsvg2-dev
      - libjavascriptcoregtk-4.1-dev
      - libsoup-3.0-dev
      
    stage-packages:
      - wireguard-tools
      - libwebkit2gtk-4.1-0
      - libjavascriptcoregtk-4.1-0
      - libayatana-appindicator3-1
      
    prime:
      - -usr/share/doc
      - -usr/share/man

layout:
  /usr/lib/$SNAPCRAFT_ARCH_TRIPLET/webkit2gtk-4.1:
    bind: $SNAP/usr/lib/$SNAPCRAFT_ARCH_TRIPLET/webkit2gtk-4.1
```

---

### Fix 2: CSS Class Typo

**File:** `app/page.tsx` line 96

```tsx
// Before:
<Unlock className="animate-pulsemb-2 size-16 text-red-500" />

// After:
<Unlock className="animate-pulse mb-2 size-16 text-red-500" />
```

---

## Testing Plan

1. **Snap Build Test:**
   ```bash
   cd wireguard-gui
   snapcraft clean
   snapcraft
   sudo snap install --dangerous wireguard-gui_*.snap
   wireguard-gui
   ```

2. **CSS Fix Test:**
   - Run app in disconnected state
   - Verify unlock icon animates with pulse
   - Verify proper spacing below icon

3. **Fedora DNF Install Test:**
   - Create RPM spec file
   - Test installation on Fedora 40, 41, 42

---

## How to Contribute

1. **Fork the repository:**
   ```bash
   gh repo fork leon3s/wireguard-gui --clone
   cd wireguard-gui
   ```

2. **Create a branch for each fix:**
   ```bash
   git checkout -b fix/snap-build-from-source
   git checkout -b fix/css-class-typo
   ```

3. **Make changes and test**

4. **Submit pull requests:**
   - One PR for snap fixes
   - One PR for CSS fix
   - Clear description of problem and solution

---

## Additional Improvements

### Future Enhancements:
1. Add Fedora/RHEL RPM packaging
2. Add AppImage support
3. Add Flatpak support
4. Improve error messages when pkexec fails
5. Add connection status notifications
6. Add keyboard shortcuts
7. Add dark theme improvements

---

## Contact Maintainer

- **GitHub:** https://github.com/leon3s/wireguard-gui
- **Issues:** https://github.com/leon3s/wireguard-gui/issues
- **Discord:** https://discord.gg/WV4Aac8uZg

