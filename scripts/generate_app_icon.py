#!/usr/bin/env python3
"""
Zero-dependency Retina macOS AppIcon.icns generator for Sumi Trading Workstation (v1.1).

Renders a 1024x1024 anti-aliased RGBA PNG (Apple Sequoia/Sonoma squircle superellipse,
deep midnight gradient #0D111A, subtle institutional grid, 3 bold candlesticks in Emerald
#26A69A & Crimson #EF5350, and a luminous Electric Blue #2962FF trend arc), exports all
10 Apple .iconset sizes via `/usr/bin/sips`, and compiles `assets/icons/AppIcon.icns`
via `/usr/bin/iconutil -c icns`.
"""

import math
import os
import shutil
import struct
import subprocess
import sys
import zlib
from pathlib import Path


def write_rgba_png(path: Path, width: int, height: int, rgba_bytes: bytes) -> None:
    def chunk(tag: bytes, data: bytes) -> bytes:
        crc = zlib.crc32(tag + data) & 0xFFFFFFFF
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

    raw_rows = bytearray()
    stride = width * 4
    for y in range(height):
        raw_rows.append(0)  # Filter type 0 (None)
        start = y * stride
        raw_rows.extend(rgba_bytes[start : start + stride])

    ihdr = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", ihdr)
        + chunk(b"IDAT", zlib.compress(bytes(raw_rows), level=6))
        + chunk(b"IEND", b"")
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(png)


def clamp(v: float, lo: float, hi: float) -> float:
    return lo if v < lo else (hi if v > hi else v)


def smoothstep(edge0: float, edge1: float, x: float) -> float:
    t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def blend_over(dst: list, sr: float, sg: float, sb: float, sa: float) -> None:
    if sa <= 0.0:
        return
    if sa >= 1.0:
        dst[0], dst[1], dst[2], dst[3] = sr, sg, sb, 1.0
        return
    da = dst[3]
    out_a = sa + da * (1.0 - sa)
    if out_a <= 1e-6:
        return
    inv = 1.0 / out_a
    dst[0] = (sr * sa + dst[0] * da * (1.0 - sa)) * inv
    dst[1] = (sg * sa + dst[1] * da * (1.0 - sa)) * inv
    dst[2] = (sb * sa + dst[2] * da * (1.0 - sa)) * inv
    dst[3] = out_a


def sd_rounded_box(px: float, py: float, cx: float, cy: float, hx: float, hy: float, r: float) -> float:
    qx = abs(px - cx) - (hx - r)
    qy = abs(py - cy) - (hy - r)
    return math.hypot(max(qx, 0.0), max(qy, 0.0)) + min(max(qx, qy), 0.0) - r


def sd_segment(px: float, py: float, ax: float, ay: float, bx: float, by: float) -> float:
    pax, pay = px - ax, py - ay
    bax, bay = bx - ax, by - ay
    denom = bax * bax + bay * bay
    h = clamp((pax * bax + pay * bay) / denom, 0.0, 1.0) if denom > 1e-9 else 0.0
    return math.hypot(pax - bax * h, pay - bay * h)


def render_master_1024() -> bytes:
    size = 1024
    buf = bytearray(size * size * 4)

    # Apple macOS squircle bounds inside 1024x1024 (standard ~10% inset = 824x824)
    half = 412.0
    center = 512.0
    squircle_p = 4.6

    # Trend curve polyline points (in 1024 space)
    curve_pts = [
        (195.0, 690.0),
        (345.0, 648.0),
        (480.0, 540.0),
        (615.0, 442.0),
        (788.0, 312.0),
    ]

    # 3 bold institutional candlesticks: (cx, wick_top, wick_bot, body_top, body_bot, r, g, b, glow_r, glow_g, glow_b)
    candles = [
        # Candle 1: Bearish Crimson pullback
        (322.0, 445.0, 725.0, 505.0, 650.0, 239 / 255, 83 / 255, 80 / 255, 255 / 255, 118 / 255, 114 / 255),
        # Candle 2: Bullish Emerald breakout
        (512.0, 325.0, 650.0, 392.0, 578.0, 38 / 255, 166 / 255, 154 / 255, 76 / 255, 214 / 255, 200 / 255),
        # Candle 3: Tall Bullish Emerald continuation
        (702.0, 240.0, 535.0, 296.0, 466.0, 42 / 255, 210 / 255, 192 / 255, 102 / 255, 236 / 255, 222 / 255),
    ]

    body_half_w = 52.0
    wick_half_w = 10.5

    for y in range(size):
        dy = (y + 0.5 - center) / half
        ady = abs(dy)
        dy_p = ady ** squircle_p
        v_grad = clamp((y - 100.0) / 824.0, 0.0, 1.0)

        # Subtle background gradient (#141D2F top -> #0A0E17 bottom)
        bg_r = (20.0 * (1.0 - v_grad) + 10.0 * v_grad) / 255.0
        bg_g = (29.0 * (1.0 - v_grad) + 14.0 * v_grad) / 255.0
        bg_b = (47.0 * (1.0 - v_grad) + 23.0 * v_grad) / 255.0

        # Grid line check (horizontal)
        h_grid = min(abs(y - 280.0), abs(y - 435.0), abs(y - 590.0), abs(y - 745.0))
        h_grid_a = 0.055 * (1.0 - smoothstep(0.0, 2.0, h_grid))

        row_off = y * size * 4
        for x in range(size):
            px = x + 0.5
            py = y + 0.5
            dx = (px - center) / half
            adx = abs(dx)
            sq_val = adx ** squircle_p + dy_p
            # Approximate Euclidean signed distance from squircle boundary
            sq_r = sq_val ** (1.0 / squircle_p) if sq_val > 0.0 else 0.0
            sd_sq = (sq_r - 1.0) * half

            col = [0.0, 0.0, 0.0, 0.0]

            # Soft macOS drop shadow below squircle
            dy_sh = (py - 18.0 - center) / half
            sh_r = (adx ** squircle_p + abs(dy_sh) ** squircle_p) ** (1.0 / squircle_p)
            sd_sh = (sh_r - 1.0) * half
            if sd_sh < 42.0 and sd_sq > -2.0:
                sh_a = 0.34 * (1.0 - smoothstep(-8.0, 42.0, sd_sh))
                blend_over(col, 0.0, 0.0, 0.0, sh_a)

            # Inside squircle mask
            sq_alpha = 1.0 - smoothstep(-1.4, 1.4, sd_sq)
            if sq_alpha > 0.0:
                # Radial luminous blue ambient aura near top-right
                dist_aura = math.hypot(px - 690.0, py - 310.0)
                aura = 0.20 * (1.0 - smoothstep(40.0, 460.0, dist_aura))
                inner = [
                    clamp(bg_r + aura * 0.16, 0.0, 1.0),
                    clamp(bg_g + aura * 0.38, 0.0, 1.0),
                    clamp(bg_b + aura * 0.92, 0.0, 1.0),
                    1.0,
                ]

                # Subtle institutional chart grid
                v_grid = min(abs(px - 260.0), abs(px - 428.0), abs(px - 596.0), abs(px - 764.0))
                v_grid_a = 0.045 * (1.0 - smoothstep(0.0, 2.0, v_grid))
                grid_a = max(h_grid_a, v_grid_a)
                if grid_a > 0.0:
                    blend_over(inner, 0.55, 0.68, 0.88, grid_a)

                # Luminous Electric Blue trend curve behind candles
                d_curve = 1e9
                for i in range(len(curve_pts) - 1):
                    ax, ay = curve_pts[i]
                    bx, by = curve_pts[i + 1]
                    d_curve = min(d_curve, sd_segment(px, py, ax, ay, bx, by))
                # Soft blue glow halo around trend line
                if d_curve < 38.0:
                    glow_a = 0.28 * (1.0 - smoothstep(4.0, 38.0, d_curve))
                    blend_over(inner, 41 / 255, 118 / 255, 255 / 255, glow_a)
                if d_curve < 12.0:
                    line_a = 1.0 - smoothstep(8.5, 11.2, d_curve)
                    t_x = clamp((px - 195.0) / 595.0, 0.0, 1.0)
                    lr = (41.0 * (1.0 - t_x) + 64.0 * t_x) / 255.0
                    lg = (98.0 * (1.0 - t_x) + 196.0 * t_x) / 255.0
                    lb = 1.0
                    blend_over(inner, lr, lg, lb, line_a * 0.96)

                # Glowing terminal dot at top-right of trend line
                d_node = math.hypot(px - 788.0, py - 312.0)
                if d_node < 48.0:
                    blend_over(inner, 64 / 255, 196 / 255, 255 / 255, 0.35 * (1.0 - smoothstep(10.0, 48.0, d_node)))
                if d_node < 19.0:
                    blend_over(inner, 130 / 255, 224 / 255, 1.0, 1.0 - smoothstep(15.5, 18.5, d_node))

                # 3 Bold Candlesticks
                for (cx, w_top, w_bot, b_top, b_bot, cr, cg, cb, hr, hg, hb) in candles:
                    if abs(px - cx) > 74.0 or py < w_top - 24.0 or py > w_bot + 24.0:
                        continue
                    # Wick
                    w_cy = 0.5 * (w_top + w_bot)
                    w_hy = 0.5 * (w_bot - w_top)
                    sd_w = sd_rounded_box(px, py, cx, w_cy, wick_half_w, w_hy, 6.0)
                    if sd_w < 2.0:
                        wa = 1.0 - smoothstep(-1.2, 1.2, sd_w)
                        blend_over(inner, hr, hg, hb, wa * 0.92)
                    # Body
                    b_cy = 0.5 * (b_top + b_bot)
                    b_hy = 0.5 * (b_bot - b_top)
                    sd_b = sd_rounded_box(px, py, cx, b_cy, body_half_w, b_hy, 15.0)
                    if sd_b < 24.0:
                        # Subtle ambient candle glow
                        blend_over(inner, cr, cg, cb, 0.14 * (1.0 - smoothstep(0.0, 24.0, sd_b)))
                    if sd_b < 2.0:
                        ba = 1.0 - smoothstep(-1.3, 1.3, sd_b)
                        # Top-to-bottom subtle bevel shading on candle body
                        bt = clamp((py - b_top) / max(b_bot - b_top, 1.0), 0.0, 1.0)
                        shade = 1.08 * (1.0 - bt) + 0.88 * bt
                        blend_over(
                            inner,
                            clamp(cr * shade, 0.0, 1.0),
                            clamp(cg * shade, 0.0, 1.0),
                            clamp(cb * shade, 0.0, 1.0),
                            ba,
                        )
                        # Crisp top highlight specular strip
                        if sd_b < -2.0 and (py - b_top) < 16.0:
                            spec = 0.16 * (1.0 - (py - b_top) / 16.0)
                            blend_over(inner, 1.0, 1.0, 1.0, spec * ba)

                # Inner macOS glass rim highlight along squircle edge (smooth top-to-bottom transition)
                rim_dist = abs(sd_sq + 4.0)
                if rim_dist < 4.5:
                    rim_t = smoothstep(-0.45, 0.45, dy)
                    rim_strength = 0.28 * (1.0 - rim_t) + 0.10 * rim_t
                    rim_a = rim_strength * (1.0 - smoothstep(0.0, 4.5, rim_dist))
                    blend_over(inner, 0.72, 0.82, 1.0, rim_a)

                blend_over(col, inner[0], inner[1], inner[2], sq_alpha)

            idx = row_off + x * 4
            buf[idx] = int(round(clamp(col[0], 0.0, 1.0) * 255.0))
            buf[idx + 1] = int(round(clamp(col[1], 0.0, 1.0) * 255.0))
            buf[idx + 2] = int(round(clamp(col[2], 0.0, 1.0) * 255.0))
            buf[idx + 3] = int(round(clamp(col[3], 0.0, 1.0) * 255.0))

    return bytes(buf)


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    icons_dir = repo_root / "assets" / "icons"
    master_png = icons_dir / "AppIcon_1024.png"
    iconset_dir = icons_dir / "AppIcon.iconset"
    icns_out = icons_dir / "AppIcon.icns"

    print("Rendering 1024x1024 Sumi AppIcon master PNG...")
    rgba = render_master_1024()
    write_rgba_png(master_png, 1024, 1024, rgba)
    print(f"Wrote {master_png} ({master_png.stat().st_size} bytes)")

    if iconset_dir.exists():
        shutil.rmtree(iconset_dir)
    iconset_dir.mkdir(parents=True, exist_ok=True)

    iconset_specs = [
        ("icon_16x16.png", 16),
        ("icon_16x16@2x.png", 32),
        ("icon_32x32.png", 32),
        ("icon_32x32@2x.png", 64),
        ("icon_128x128.png", 128),
        ("icon_128x128@2x.png", 256),
        ("icon_256x256.png", 256),
        ("icon_256x256@2x.png", 512),
        ("icon_512x512.png", 512),
        ("icon_512x512@2x.png", 1024),
    ]

    for fname, px in iconset_specs:
        dst = iconset_dir / fname
        if px == 1024:
            shutil.copy2(master_png, dst)
        else:
            subprocess.run(
                ["/usr/bin/sips", "-z", str(px), str(px), str(master_png), "--out", str(dst)],
                check=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )

    subprocess.run(
        ["/usr/bin/iconutil", "-c", "icns", str(iconset_dir), "-o", str(icns_out)],
        check=True,
    )
    shutil.rmtree(iconset_dir)
    print(f"Compiled Retina macOS icon: {icns_out} ({icns_out.stat().st_size} bytes)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
