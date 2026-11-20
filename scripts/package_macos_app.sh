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
ICON_SRC="$REPO_ROOT/assets/icons/AppIcon.icns"

# Read canonical version directly from Cargo.toml so Info.plist never drifts.
CARGO_VERSION="$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$REPO_ROOT/Cargo.toml")"
CARGO_VERSION="${CARGO_VERSION:-1.1.0}"

# Ensure Retina macOS AppIcon.icns is up to date.
if [ ! -f "$ICON_SRC" ]; then
  echo "Generating Retina AppIcon.icns..."
  python3 "$REPO_ROOT/scripts/generate_app_icon.py"
fi

echo "Building release binary (v${CARGO_VERSION})..."
cargo build --release

rm -rf "$BUNDLE_DIR"
mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

cp "$BIN_SRC" "$MACOS_DIR/sumi-workstation"
chmod +x "$MACOS_DIR/sumi-workstation"
cp "$ICON_SRC" "$RESOURCES_DIR/AppIcon.icns"

# Bundle lightweight real CafeF Index CSV into Contents/Resources so the .app runs
# completely standalone if copied outside the source repository (e.g. /Applications or /tmp).
if [ -f "$REPO_ROOT/data/raw/cafef/CafeF.INDEX.Upto06112026.csv" ]; then
  mkdir -p "$RESOURCES_DIR/data/raw/cafef"
  cp "$REPO_ROOT/data/raw/cafef/CafeF.INDEX.Upto06112026.csv" \
     "$RESOURCES_DIR/data/raw/cafef/CafeF.INDEX.Upto06112026.csv"
fi

cat > "$CONTENTS_DIR/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleDisplayName</key>
    <string>Sumi Workstation</string>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>OHLCV Market Data CSV</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.comma-separated-values-text</string>
                <string>public.tab-separated-values-text</string>
            </array>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>csv</string>
                <string>tsv</string>
            </array>
        </dict>
        <dict>
            <key>CFBundleTypeName</key>
            <string>OHLCV Market Data JSON</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSHandlerRank</key>
            <string>Alternate</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.json</string>
            </array>
            <key>CFBundleTypeExtensions</key>
            <array>
                <string>json</string>
            </array>
        </dict>
    </array>
    <key>CFBundleExecutable</key>
    <string>sumi-workstation</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIdentifier</key>
    <string>vn.sumi.workstation</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Sumi Workstation</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${CARGO_VERSION}</string>
    <key>CFBundleVersion</key>
    <string>20</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.finance</string>
    <key>LSMinimumSystemVersion</key>
    <string>13.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
PLIST

# Validate Info.plist syntax before signing.
/usr/bin/plutil -lint "$CONTENTS_DIR/Info.plist"

# Ad-hoc sign the arm64 Mach-O bundle so Apple Silicon launchd/AMFI executes it cleanly.
codesign --force --deep --sign - "$BUNDLE_DIR"
codesign --verify --deep --strict "$BUNDLE_DIR"

echo "Packaged and signed macOS app bundle at: $BUNDLE_DIR (v${CARGO_VERSION})"
