#!/usr/bin/env bash
# ==============================================================================
# Sumi Supercharts v3.0 — macOS .app / .dmg Release Packager
# ==============================================================================
# Mirrors Yumeko's macOS packaging conventions:
#   - Builds UI (`npm --prefix ui run build`) + Rust Tauri v2 release binary
#   - Packages `Sumi Supercharts.app` with `CFBundleURLTypes` (`sumi://` URI scheme)
#   - Applies ad-hoc `codesign --force --deep --sign -` (no Apple Developer ID needed)
#   - Installs to `~/Applications/Sumi Supercharts.app` & registers Spotlight / LaunchServices
#   - Optional `--dmg` creates a compressed distributable `.dmg` in `dist-macos/`
#   - Zero hardcoded user paths; runs on any Apple Silicon or Intel Mac.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_NAME="Sumi Supercharts"
BUNDLE_ID="com.sumi.supercharts"
INSTALL_DIR="${HOME}/Applications"
DIST_DIR="${REPO_ROOT}/dist-macos"
MAKE_DMG=0
SKIP_INSTALL=0

for arg in "$@"; do
  case "$arg" in
    --dmg) MAKE_DMG=1 ;;
    --no-install) SKIP_INSTALL=1 ;;
    --help|-h)
      echo "Usage: $0 [--dmg] [--no-install]"
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

export PATH="/opt/homebrew/opt/rustup/bin:${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:${PATH}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "ERROR: cargo not found on PATH. Install Rust via rustup first." >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "ERROR: npm not found on PATH. Install Node.js >= 20 first." >&2
  exit 1
fi

VERSION="$(awk -F'"' '/^version = / { print $2; exit }' "${REPO_ROOT}/Cargo.toml")"
VERSION="${VERSION:-3.0.0}"
echo "==> Building ${APP_NAME} v${VERSION} for macOS ($(uname -m))"

echo "==> [1/4] Building React UI bundle..."
npm --prefix "${REPO_ROOT}/ui" ci --no-audit --no-fund >/dev/null 2>&1 || npm --prefix "${REPO_ROOT}/ui" install --no-audit --no-fund
npm --prefix "${REPO_ROOT}/ui" run build

echo "==> [2/4] Building Rust Tauri v2 desktop binary (release)..."
(cd "${REPO_ROOT}" && cargo build --release --package sumi-desktop)

BIN_PATH="${REPO_ROOT}/target/release/sumi-desktop"
if [[ ! -x "${BIN_PATH}" ]]; then
  echo "ERROR: Expected release binary at ${BIN_PATH}" >&2
  exit 1
fi

echo "==> [3/4] Assembling ${APP_NAME}.app bundle..."
mkdir -p "${DIST_DIR}"
STAGE_APP="${DIST_DIR}/${APP_NAME}.app"
rm -rf "${STAGE_APP}"
mkdir -p "${STAGE_APP}/Contents/MacOS" "${STAGE_APP}/Contents/Resources"

cp "${BIN_PATH}" "${STAGE_APP}/Contents/MacOS/sumi-desktop"
chmod +x "${STAGE_APP}/Contents/MacOS/sumi-desktop"

# Also copy `sumi` CLI binary if built in this workspace (Wave 2+)
if [[ -x "${REPO_ROOT}/target/release/sumi" ]]; then
  cp "${REPO_ROOT}/target/release/sumi" "${STAGE_APP}/Contents/MacOS/sumi"
  chmod +x "${STAGE_APP}/Contents/MacOS/sumi"
fi

# Copy app icon if present
ICON_SRC="${REPO_ROOT}/crates/sumi-desktop/icons/icon.icns"
if [[ -f "${ICON_SRC}" ]]; then
  cp "${ICON_SRC}" "${STAGE_APP}/Contents/Resources/icon.icns"
fi

cat > "${STAGE_APP}/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>vi</string>
  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>
  <key>CFBundleExecutable</key>
  <string>sumi-desktop</string>
  <key>CFBundleIconFile</key>
  <string>icon.icns</string>
  <key>CFBundleIdentifier</key>
  <string>${BUNDLE_ID}</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>${VERSION}</string>
  <key>CFBundleVersion</key>
  <string>${VERSION}</string>
  <key>LSApplicationCategoryType</key>
  <string>public.app-category.finance</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>CFBundleURLTypes</key>
  <array>
    <dict>
      <key>CFBundleURLName</key>
      <string>${BUNDLE_ID}.deeplink</string>
      <key>CFBundleURLSchemes</key>
      <array>
        <string>sumi</string>
      </array>
    </dict>
  </array>
</dict>
</plist>
EOF

echo "==> [4/4] Applying ad-hoc codesign..."
codesign --force --deep --sign - "${STAGE_APP}"

if [[ "${SKIP_INSTALL}" -eq 0 ]]; then
  mkdir -p "${INSTALL_DIR}"
  rm -rf "${INSTALL_DIR}/${APP_NAME}.app"
  cp -R "${STAGE_APP}" "${INSTALL_DIR}/${APP_NAME}.app"
  # Register URI handler (`sumi://`) with macOS LaunchServices
  LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
  if [[ -x "${LSREGISTER}" ]]; then
    "${LSREGISTER}" -f "${INSTALL_DIR}/${APP_NAME}.app" || true
  fi
  echo "==> Installed to ${INSTALL_DIR}/${APP_NAME}.app"
fi

if [[ "${MAKE_DMG}" -eq 1 ]]; then
  DMG_PATH="${DIST_DIR}/Sumi-Supercharts-${VERSION}-macos-$(uname -m).dmg"
  rm -f "${DMG_PATH}"
  hdiutil create -volname "${APP_NAME}" -srcfolder "${STAGE_APP}" -ov -format UDZO "${DMG_PATH}"
  echo "==> Created DMG: ${DMG_PATH}"
fi

echo "==> Done! Launch with: open \"${INSTALL_DIR}/${APP_NAME}.app\""
