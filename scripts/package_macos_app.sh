#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APP_NAME="Sumi Workstation"
BUNDLE_DIR="$REPO_ROOT/dist/${APP_NAME}.app"
CONTENTS_DIR="$BUNDLE_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
BIN_SRC="$REPO_ROOT/target/release/sumi-workstation"

if [ ! -f "$BIN_SRC" ]; then
  echo "Building release binary..."
  cargo build --release
fi

rm -rf "$BUNDLE_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$BIN_SRC" "$MACOS_DIR/sumi-workstation"
chmod +x "$MACOS_DIR/sumi-workstation"

cat > "$CONTENTS_DIR/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Sumi Workstation</string>
    <key>CFBundleExecutable</key>
    <string>sumi-workstation</string>
    <key>CFBundleIdentifier</key>
    <string>vn.sumi.workstation</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Sumi Workstation</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>19</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Ad-hoc sign the arm64 Mach-O bundle so Apple Silicon launchd/AMFI executes it cleanly.
codesign --force --deep --sign - "$BUNDLE_DIR"

echo "Packaged macOS app bundle at: $BUNDLE_DIR"
