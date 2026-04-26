"""
Generate wallpapers for Achronyme web (dark theme).

Two families:
  • VM cards (TriArchitecture): akron, artik, lysis — each in its semantic
    accent color (proof / valid / info), dark base.
  • Purple epicentral (PageGrid feat-circuit, feat-prove, comparison): strong
    central focal points — replaces the existing diagonal-stripe gradient.

All output: 2400×1200 PNG, no border, written to public/wallpapers/.
"""
import numpy as np
from PIL import Image
from dataclasses import dataclass, field
from typing import Callable, List, Tuple

# ----------------------------------------------------------------------
# Palettes — all dark-theme, matching design tokens in src/styles/global.css
# ----------------------------------------------------------------------
# Format: (background, negative_accent, positive_accent, extreme_accent)
#
# void          = #121217 (18,18,23)
# surface       = #19191E (25,25,30)
# proof         = #A855F7 (168,85,247)   purple-500
# proof_light   = #C084FC (192,132,252)  purple-400
# proof_dim     = #7C3AED (124,58,237)   violet-600
# valid         = #34D399 (52,211,153)   green-400
# info          = #60A5FA (96,165,250)   blue-400

PALETTES = {
    # -------- VM cards (subtle, card-bg friendly) --------
    "akron_dark": [
        (20, 20, 26),       # bg
        (60, 30, 80),       # neg → deep violet-shadow
        (168, 85, 247),     # pos → proof
        (216, 180, 254),    # accent → purple-200
    ],
    "artik_dark": [
        (20, 22, 26),
        (20, 55, 50),       # neg → deep teal
        (52, 211, 153),     # pos → valid
        (167, 243, 208),    # accent → green-200
    ],
    "lysis_dark": [
        (20, 22, 28),
        (28, 50, 95),       # neg → deep navy
        (96, 165, 250),     # pos → info
        (191, 219, 254),    # accent → blue-200
    ],
    # -------- Epicentral purple — matched to announcement banner --------
    # Banner uses bg-gradient-to-r from-proof-dim via-proof to-proof-dim:
    #   proof-dim = #7C3AED (124, 58, 237)
    #   proof     = #A855F7 (168, 85, 247)
    # Wallpapers stay in this hue family so they read as a richer, textured
    # extension of the banner.
    "purple_deep": [
        (90, 48, 180),      # bg → proof-dim slightly muted
        (124, 58, 237),     # neg → proof-dim
        (168, 85, 247),     # pos → proof
        (240, 220, 255),    # accent → near-white highlight
    ],
    "purple_warm": [
        (100, 52, 195),     # bg → warm violet near proof-dim
        (140, 70, 240),     # neg → bright violet
        (192, 132, 252),    # pos → proof-light
        (245, 230, 255),
    ],
    "purple_intense": [
        (95, 50, 200),      # bg → vibrant violet
        (140, 70, 245),     # neg → very bright violet
        (180, 105, 252),    # pos → proof-bright
        (250, 240, 255),
    ],
}


@dataclass
class Scene:
    conformal: Callable[[np.ndarray], np.ndarray] = lambda z: z
    grid_freq: float = 10.0
    grid_weight: float = 0.0
    chirps: List[Tuple[complex, float, float]] = field(default_factory=list)
    epicenter: complex = 0 + 0j
    slit_offset: complex = 0 + 0j
    slit_freq: float = 22.0
    slit_weight: float = 0.0
    polar_grid: Tuple[float, float, float] = (0.0, 0.0, 0.0)
    bias: Tuple[float, float] = (0.0, 0.0)
    domain: float = 1.5
    palette: str = "akron_dark"
    grain: float = 0.05
    gamma: float = 0.90
    field_dampen: float = 1.0
    accent_weight: float = 0.45


def srgb_to_linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c):
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(c, 1 / 2.4) - 0.055)


def build_grid(W, H, domain):
    aspect = H / W
    x = np.linspace(-domain, domain, W)
    y = np.linspace(-domain * aspect, domain * aspect, H)
    X, Y = np.meshgrid(x, y)
    return X + 1j * Y


def compute_field(z, s: Scene):
    w = s.conformal(z)
    H = s.grid_weight * np.sin(s.grid_freq * w.real) * np.sin(s.grid_freq * w.imag)
    for z0, k, weight in s.chirps:
        H = H + weight * np.cos(k * np.abs(z - z0) ** 2)
    if s.slit_weight > 0 and s.slit_offset != 0:
        z1, z2 = s.epicenter + s.slit_offset, s.epicenter - s.slit_offset
        dr = np.abs(z - z1) - np.abs(z - z2)
        H = H + s.slit_weight * np.cos(s.slit_freq * dr)
    rf, af, pw = s.polar_grid
    if pw > 0:
        dz = z - s.epicenter
        H = H + pw * np.sin(rf * np.abs(dz)) * np.sin(af * np.angle(dz))
    H = H + s.bias[0] * z.real + s.bias[1] * z.imag
    H = np.tanh(H * 0.7)
    H = np.sign(H) * np.power(np.abs(H), s.gamma)
    H = H * s.field_dampen
    return H


def colorize(H, palette_name, accent_weight=0.45):
    bg, neg, pos, accent = [np.array(c, np.float32) / 255.0 for c in PALETTES[palette_name]]
    bg_l, neg_l, pos_l, acc_l = (srgb_to_linear(c) for c in (bg, neg, pos, accent))
    H = H[..., None]
    pos_mask = np.clip(H, 0, 1)
    neg_mask = np.clip(-H, 0, 1)
    # Smooth accent ramp — quadratic on |H|, no hard threshold band.
    abs_H = np.abs(H)
    extreme = abs_H * abs_H
    out = bg_l + pos_mask * (pos_l - bg_l) + neg_mask * (neg_l - bg_l)
    out = out + extreme * (acc_l - out) * accent_weight
    out = linear_to_srgb(np.clip(out, 0, 1))
    return (np.clip(out, 0, 1) * 255).astype(np.uint8)


def add_grain(rgb, sigma, seed):
    rng = np.random.default_rng(seed)
    arr = rgb.astype(np.float32) / 255.0
    luma = rng.normal(0, sigma, arr.shape[:2])[..., None]
    chroma = rng.normal(0, sigma * 0.4, arr.shape)
    return (np.clip(arr + luma + chroma, 0, 1) * 255).astype(np.uint8)


def render(scene: Scene, width: int, height: int, seed: int) -> Image.Image:
    np.random.seed(seed)
    z = build_grid(width, height, scene.domain)
    H = compute_field(z, scene)
    rgb = colorize(H, scene.palette, scene.accent_weight)
    rgb = add_grain(rgb, scene.grain, seed + 1)
    return Image.fromarray(rgb, "RGB")


# ======================================================================
# VM CARDS — TriArchitecture.astro
# ======================================================================

# Akron — Vasarely-style grid (proof / purple), subtle for card bg
akron = Scene(
    conformal=lambda z: z ** 2,
    grid_freq=11.0,
    grid_weight=1.0,
    chirps=[(0.5 + 0.3j, 7.0, 0.20)],
    bias=(0.04, 0.0),
    palette="akron_dark",
    grain=0.035,
    gamma=1.30,
    field_dampen=0.40,
    accent_weight=0.20,
    domain=1.7,
)

# Artik — radial chirp (valid / green), off-frame epicenter so center stays calm
artik = Scene(
    conformal=lambda z: z,
    grid_freq=6.0,
    grid_weight=0.20,
    chirps=[(0.0 + 1.8j, 28.0, 0.95)],   # higher k → thinner rings, less overwhelming
    bias=(0.0, -0.04),
    palette="artik_dark",
    grain=0.035,
    gamma=1.25,
    field_dampen=0.42,
    accent_weight=0.18,
    domain=1.4,
)

# Lysis — Young double-slit (info / blue), off-center
lysis = Scene(
    conformal=lambda z: z,
    slit_offset=0.32 + 0.05j,
    slit_freq=22.0,
    slit_weight=0.85,
    epicenter=-0.4 + 0.15j,
    chirps=[(1.2 - 0.6j, 14.0, 0.30)],
    bias=(0.06, 0.03),
    palette="lysis_dark",
    grain=0.035,
    gamma=1.20,
    field_dampen=0.45,
    accent_weight=0.20,
    domain=1.5,
)

# ======================================================================
# EPICENTRAL PURPLE — PageGrid landing
# ======================================================================

# purple-banner — bright epicentral, smooth chirp
# Lower field_dampen so most pixels stay at the saturated bg (vivid violet),
# rings ride on top as subtle hue shifts toward proof / proof-light.
purple_banner = Scene(
    conformal=lambda z: z,
    chirps=[
        (0.0 + 0.0j, 22.0, 0.95),
        (1.1 - 0.5j, 12.0, 0.30),
    ],
    bias=(0.04, 0.02),
    palette="purple_deep",
    grain=0.035,
    gamma=1.05,
    field_dampen=0.70,
    accent_weight=0.55,
    domain=1.4,
)

# purple-feature — off-center burst (lives on feat-vm / VM Mode)
purple_feature = Scene(
    conformal=lambda z: z,
    chirps=[
        (-0.7 + 0.3j, 20.0, 0.95),
        (1.2 - 0.4j, 10.0, 0.30),
    ],
    bias=(0.07, 0.03),
    palette="purple_warm",
    grain=0.035,
    gamma=1.05,
    field_dampen=0.70,
    accent_weight=0.55,
    domain=1.6,
)

# purple-comparison — "achronyme way" highlight, dense central chirp
purple_comparison = Scene(
    conformal=lambda z: z,
    chirps=[
        (0.15 - 0.1j, 22.0, 1.0),
        (-0.9 + 0.5j, 9.0, 0.30),
    ],
    bias=(0.04, 0.04),
    palette="purple_intense",
    grain=0.035,
    gamma=1.0,
    field_dampen=0.72,
    accent_weight=0.58,
    domain=1.4,
)


import os
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.join(SCRIPT_DIR, "..", "public", "wallpapers")
W, H = 2400, 1200

if __name__ == "__main__":
    targets = [
        ("akron",             akron,             11),
        ("artik",             artik,             23),
        ("lysis",             lysis,             47),
        ("purple-banner",     purple_banner,     71),
        ("purple-feature",    purple_feature,    97),
        ("purple-comparison", purple_comparison, 131),
    ]
    os.makedirs(OUT_DIR, exist_ok=True)
    for name, scene, seed in targets:
        img = render(scene, W, H, seed)
        path = f"{OUT_DIR}/{name}.webp"
        img.save(path, "WEBP", quality=82, method=6)
        size_kb = os.path.getsize(path) / 1024
        print(f"wrote {path}  ({W}x{H}, {size_kb:.0f} KB)")
