# Contributing to WireGuard GUI

## 🎯 Quick Start for Contributors

### 1. Fork and Clone
```bash
# Fork the repo on GitHub first, then:
gh repo fork leon3s/wireguard-gui --clone
cd wireguard-gui
```

### 2. Set Up Development Environment

**Prerequisites:**
- Node.js 20+ and npm
- Rust 1.57+
- System dependencies (Tauri)

**Fedora:**
```bash
sudo dnf install -y \
    webkit2gtk4.1-devel \
    openssl-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    gtk3-devel \
    wireguard-tools

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js
curl -fsSL https://rpm.nodesource.com/setup_20.x | sudo bash -
sudo dnf install -y nodejs
```

**Ubuntu/Debian:**
```bash
sudo apt install -y \
    javascriptcoregtk-4.1 \
    libsoup-3.0 \
    webkit2gtk-4.1 \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    wireguard-tools \
    resolvconf
```

### 3. Install Dependencies
```bash
npm install
```

### 4. Run Development Server
```bash
npm run dev
```

This will:
- Start Next.js dev server
- Launch Tauri in development mode
- Enable hot reload

---

## 🐛 Bug Fixes We're Working On

### Priority Bugs:

1. **Snap Package Broken** (Critical)
   - Branch: `fix/snap-build-from-source`
   - Status: In Progress
   - See: `BUGS_FOUND.md`

2. **CSS Class Typo** (Minor)
   - Branch: `fix/css-class-typo`
   - File: `app/page.tsx:96`
   - Easy first contribution!

---

## 📝 Contribution Workflow

### Step 1: Create a Branch
```bash
# For bug fixes:
git checkout -b fix/descriptive-name

# For features:
git checkout -b feature/descriptive-name

# For snap fixes:
git checkout -b fix/snap-build-from-source
```

### Step 2: Make Your Changes

**Example: Fix CSS Typo**
```bash
# Edit app/page.tsx line 96
# Change: className="animate-pulsemb-2 size-16 text-red-500"
# To:     className="animate-pulse mb-2 size-16 text-red-500"

# Test the change
npm run dev
```

### Step 3: Test Your Changes
```bash
# Run linters
npm run lint
npm run clippy

# Build production version
npm run tauri build

# Test the built binary
./src-tauri/target/release/wireguard-gui
```

### Step 4: Commit Your Changes
```bash
git add .
git commit -m "fix: correct CSS class typo in disconnected icon

- Fixed missing space between 'animate-pulse' and 'mb-2'
- Resolves animation not working in disconnected state
- Fixes incorrect bottom margin"
```

**Commit Message Format:**
- `fix:` for bug fixes
- `feat:` for new features
- `docs:` for documentation
- `chore:` for maintenance tasks
- `refactor:` for code refactoring

### Step 5: Push and Create PR
```bash
git push origin fix/css-class-typo
```

Then create PR on GitHub with:
- Clear title describing the fix
- Description of the problem
- How you tested it
- Screenshots if UI changes

---

## 🧪 Testing Your Changes

### Unit Tests
```bash
# Frontend tests
npm test

# Rust tests
cd src-tauri
cargo test
```

### Manual Testing Checklist

**For All Changes:**
- [ ] App starts without errors
- [ ] No console errors in dev tools
- [ ] Tray icon works
- [ ] Can open/close app window

**For Profile Management:**
- [ ] Can create new profile
- [ ] Can edit existing profile
- [ ] Can delete profile
- [ ] Can connect to profile
- [ ] Can disconnect
- [ ] Profile search works

**For UI Changes:**
- [ ] Test in light theme
- [ ] Test in dark theme
- [ ] Test with long profile names
- [ ] Test with many profiles (10+)

---

## 📦 Building Packages

### Build for Your Platform
```bash
npm run tauri build
```

Output will be in:
- `src-tauri/target/release/wireguard-gui` (binary)
- `src-tauri/target/release/bundle/` (platform packages)

### Build Snap Package
```bash
snapcraft clean
snapcraft
sudo snap install --dangerous wireguard-gui_*.snap
```

### Build Debian Package
```bash
npm run tauri build -- --target deb
# Output: src-tauri/target/release/bundle/deb/
```

---

## 🎨 Code Style

### TypeScript/React
- Use functional components with hooks
- Follow existing patterns in codebase
- Use Tailwind CSS for styling
- Keep components small and focused

**Example:**
```tsx
export function MyComponent({ prop }: MyComponentProps) {
  const [state, setState] = useState(false);
  
  return (
    <div className="flex items-center gap-2">
      {/* Component content */}
    </div>
  );
}
```

### Rust
- Follow Rust standard style
- Run `cargo clippy` before committing
- Use descriptive variable names
- Add comments for complex logic

**Example:**
```rust
#[tauri::command]
async fn my_command(
    app_state: State<'_, AppSt>,
    param: String,
) -> Result<(), AppError> {
    // Implementation
    Ok(())
}
```

---

## 🔍 Code Review Process

1. **Automated Checks:**
   - ESLint (TypeScript)
   - Clippy (Rust)
   - Build success
   - All tests pass

2. **Manual Review:**
   - Code quality
   - Follows project patterns
   - Properly tested
   - Documentation updated if needed

3. **Merge:**
   - Squash commits
   - Update CHANGELOG.md
   - Tag release if applicable

---

## 📚 Resources

### Documentation
- [Tauri Docs](https://tauri.app/v1/guides/)
- [Next.js Docs](https://nextjs.org/docs)
- [Tailwind CSS](https://tailwindcss.com/docs)
- [shadcn/ui](https://ui.shadcn.com/)

### Project Structure
```
wireguard-gui/
├── app/                 # Next.js pages
├── components/          # React components
│   └── ui/             # shadcn/ui components
├── lib/                # Utilities and hooks
├── src-tauri/          # Rust backend
│   ├── src/            # Rust source
│   └── scripts/        # Helper scripts
├── snap/               # Snap packaging
└── types/              # TypeScript types
```

---

## ❓ Getting Help

- **Discord:** https://discord.gg/WV4Aac8uZg
- **GitHub Issues:** https://github.com/leon3s/wireguard-gui/issues
- **GitHub Discussions:** For questions and ideas

---

## 🎉 First-Time Contributors

Easy issues to start with:
1. Fix CSS typo (app/page.tsx:96)
2. Add keyboard shortcuts
3. Improve error messages
4. Add tooltips to icons
5. Update documentation

Look for issues tagged with `good first issue` or `help wanted`.

---

## 📜 License

This project is dual-licensed under MIT and Apache 2.0.
By contributing, you agree to license your contributions under the same terms.

---

Thank you for contributing! 🙏

