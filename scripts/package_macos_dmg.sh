#!/usr/bin/env bash
set -euo pipefail

export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

CARGO_VERSION="$(awk -F'"' '/^version[[:space:]]*=/{print $2; exit}' "$REPO_ROOT/Cargo.toml")"
CARGO_VERSION="${CARGO_VERSION:-1.1.0}"

APP_NAME="Sumi Workstation"
BUNDLE_DIR="$REPO_ROOT/dist/${APP_NAME}.app"
DMG_NAME="Sumi-Workstation-v${CARGO_VERSION}-arm64.dmg"
DMG_OUT="$REPO_ROOT/dist/${DMG_NAME}"

# Build & sign the fresh .app bundle first unless SKIP_APP_BUILD=1.
if [ "${SKIP_APP_BUILD:-0}" != "1" ] || [ ! -d "$BUNDLE_DIR" ]; then
  bash "$REPO_ROOT/scripts/package_macos_app.sh"
fi

STAGE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/sumi_dmg_stage.XXXXXX")"
trap 'rm -rf "$STAGE_DIR"' EXIT

cp -R "$BUNDLE_DIR" "$STAGE_DIR/${APP_NAME}.app"
ln -s /Applications "$STAGE_DIR/Applications"

cat > "$STAGE_DIR/README_INSTALL.txt" <<README
SUMI TRADING WORKSTATION — v${CARGO_VERSION} (macOS Apple Silicon arm64)
========================================================================

Cài đặt / Installation:
1. Kéo biểu tượng "Sumi Workstation.app" vào thư mục "Applications" bên cạnh.
   (Drag "Sumi Workstation.app" into the "Applications" folder.)
2. Mở ứng dụng từ Launchpad hoặc Applications.
   Lưu ý: Bản đóng gói nội bộ sử dụng chữ ký ad-hoc. Nếu macOS Gatekeeper hiển thị
   thông báo xác minh, hãy nhấp chuột phải -> Open, hoặc chạy trong Terminal:
     xattr -dr com.apple.quarantine "/Applications/Sumi Workstation.app"

Tính năng nổi bật v${CARGO_VERSION}:
- 100% Offline-First, Zero-Network, GPU Metal 120 FPS (Rust thuần).
- Thanh vẽ dọc TradingView Pro (11 công cụ gồm Kênh song song 3 điểm & Thước đo Giá/Ngày).
- Giao dịch trực tiếp trên biểu đồ: kéo-thả SL/TP/lệnh chờ hít bước giá HOSE/HNX/UPCOM/VN30F.
- Kéo-thả tệp .csv / .json trực tiếp vào cửa sổ Sumi để nạp OHLCV tức thì.
- Xuất báo cáo Strategy Lab dạng HTML tự chứa (biểu đồ SVG vector + In PDF chuẩn A4/Letter).
README

rm -f "$DMG_OUT"
echo "Creating compressed UDZO disk image: $DMG_OUT ..."
/usr/bin/hdiutil create \
  -volname "Sumi Workstation ${CARGO_VERSION}" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  -imagekey zlib-level=9 \
  "$DMG_OUT"

echo "Verifying DMG integrity..."
/usr/bin/hdiutil verify "$DMG_OUT"

ls -lh "$DMG_OUT"
echo "Packaged macOS DMG installer at: $DMG_OUT"
