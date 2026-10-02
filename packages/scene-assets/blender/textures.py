"""Baked material textures: procedural recipes -> tileable PNGs -> the GLB.

A recipe is a procedural material (camouflage print, weave, rubber, bare
steel, burnt paint, a track link...) evaluated on a periodic lattice, so the
texture tiles seamlessly. It bakes three images at `SIZE` px, the bundle's
texture channels (`scene-assets` `TEXTURE_CHANNELS`):

- albedo (sRGB): the surface colour; alpha is the wear threshold, where the
  surface breaks first (low) as the vertex colour's alpha (how worn: chips on
  edges, mud rising from the ground) climbs past it;
- normal (tangent space, +Y toward +v in the image, OpenGL convention as glTF);
  alpha is the coverage a cutout or blended material reads (`surface`), 1
  for every other;
- ORM: occlusion, roughness, metalness; alpha is the side-tint mask.

A textured material samples its recipe through UVs in metres over the
recipe's tile (`box_uv`, a per-face box projection in the model's rest
space), so the pattern keeps its scale on every part and every tier. The
vertex colour keeps what only the model knows (ambient occlusion, grime,
per-part hue), as a multiplier relative to the recipe's mean albedo
(`macro`), and the wear amount in alpha.

`attach(glb, materials)` writes the images into an exported GLB and points
each named material's glTF texture slots at them, with the wear colour in
the material's extras, and gives a cutout or blended material its glTF alpha
mode. Everything here is numpy on fixed seeds and zlib at a fixed level: the
same recipe writes the same bytes.
"""
import json
import math
import struct
import zlib

import numpy as np

SIZE = 256


# ---------------------------------------------------------------- periodic noise
def _smooth(t):
    return t * t * (3 - 2 * t)


def value_noise(cells, seed, size=SIZE):
    """Smooth value noise in [0, 1] on a periodic lattice of `cells` (int or (cx, cy))."""
    cx, cy = (cells, cells) if isinstance(cells, int) else cells
    rng = np.random.default_rng(seed)
    lat = rng.random((cy, cx))
    x = np.arange(size) * cx / size
    y = np.arange(size) * cy / size
    xi, yi = np.floor(x).astype(int), np.floor(y).astype(int)
    xf, yf = _smooth(x - xi), _smooth(y - yi)
    x0, x1 = xi % cx, (xi + 1) % cx
    y0, y1 = yi % cy, (yi + 1) % cy
    a = lat[np.ix_(y0, x0)]
    b = lat[np.ix_(y0, x1)]
    c = lat[np.ix_(y1, x0)]
    d = lat[np.ix_(y1, x1)]
    top = a + (b - a) * xf[None, :]
    bot = c + (d - c) * xf[None, :]
    return top + (bot - top) * yf[:, None]


def fbm(cells, seed, octaves=4, gain=0.5, size=SIZE):
    """Fractal value noise, rescaled to span [0, 1]."""
    cx, cy = (cells, cells) if isinstance(cells, int) else cells
    out = np.zeros((size, size))
    amp = 1.0
    for o in range(octaves):
        out += amp * value_noise((cx << o, cy << o), seed * 7 + o, size)
        amp *= gain
    lo, hi = out.min(), out.max()
    return (out - lo) / max(hi - lo, 1e-9)


def sample(img, u, v):
    """Bilinear, periodic lookup of `img` at pixel coordinates (u, v) arrays."""
    h, w = img.shape[:2]
    x0 = np.floor(u).astype(int)
    y0 = np.floor(v).astype(int)
    fx, fy = u - x0, v - y0
    x0 %= w
    y0 %= h
    x1, y1 = (x0 + 1) % w, (y0 + 1) % h
    if img.ndim == 3:
        fx, fy = fx[..., None], fy[..., None]
    top = img[y0, x0] * (1 - fx) + img[y0, x1] * fx
    bot = img[y1, x0] * (1 - fx) + img[y1, x1] * fx
    return top * (1 - fy) + bot * fy


def warp(img, amount, seed, cells=6):
    """Domain-warp `img` by `amount` pixels of periodic noise."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float)
    du = (fbm(cells, seed, 3) - 0.5) * 2 * amount
    dv = (fbm(cells, seed + 1, 3) - 0.5) * 2 * amount
    return sample(img, xx + du, yy + dv)


def worley(cells, seed, size=SIZE):
    """Periodic cellular noise: (F1, F2) distances in cell units, and each pixel's cell id."""
    rng = np.random.default_rng(seed)
    pts = rng.random((cells, cells, 2))
    yy, xx = np.mgrid[0:size, 0:size].astype(float) * cells / size
    ci, cj = np.floor(yy).astype(int), np.floor(xx).astype(int)
    f1 = np.full((size, size), 9.0)
    f2 = np.full((size, size), 9.0)
    ident = np.zeros((size, size), dtype=int)
    for di in (-1, 0, 1):
        for dj in (-1, 0, 1):
            ni, nj = ci + di, cj + dj
            p = pts[ni % cells, nj % cells]
            d = np.hypot(ni + p[..., 0] - yy, nj + p[..., 1] - xx)
            closer = d < f1
            f2 = np.where(closer, f1, np.minimum(f2, d))
            ident = np.where(closer, (ni % cells) * cells + (nj % cells), ident)
            f1 = np.where(closer, d, f1)
    return f1, f2, ident


def blur(img, passes=1):
    """A periodic 3x3 box blur, `passes` times."""
    out = img
    for _ in range(passes):
        acc = np.zeros_like(out)
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                acc += np.roll(np.roll(out, dy, 0), dx, 1)
        out = acc / 9
    return out


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


def mix(a, b, t):
    """Mix colours: a, b are (3,) or (H, W, 3); t is (H, W)."""
    a = np.asarray(a, dtype=float)
    b = np.asarray(b, dtype=float)
    return a + (b - a) * np.asarray(t)[..., None]


def normals_from_height(h, strength):
    """Tangent-space normals from a height field in texels; +X toward +u, +Y toward +v."""
    dx = (np.roll(h, -1, 1) - np.roll(h, 1, 1)) * 0.5 * strength
    dy = (np.roll(h, -1, 0) - np.roll(h, 1, 0)) * 0.5 * strength
    n = np.stack([-dx, -dy, np.ones_like(h)], -1)
    return n / np.linalg.norm(n, axis=-1, keepdims=True)


# ---------------------------------------------------------------- encoding
def _srgb(linear):
    c = np.clip(linear, 0, 1)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * np.power(c, 1 / 2.4) - 0.055)


def _u8(x):
    return np.clip(np.round(np.asarray(x) * 255), 0, 255).astype(np.uint8)


def png(rgba):
    """PNG bytes of an (H, W, 4) uint8 image: filter 0 rows, zlib level 9."""
    h, w, _ = rgba.shape
    raw = b"".join(b"\x00" + rgba[y].tobytes() for y in range(h))

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)

    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b""))


class Baked:
    """One recipe's images: linear albedo (H, W, 3) and wear threshold, unit
    normals (H, W, 3), occlusion, roughness, metalness and tint mask, and the
    coverage (how much of the surface is there, 0..1) a cutout or blended
    material reads: it rides the normal image's alpha, apart from wear."""

    def __init__(self, albedo, wear, normal, occlusion, roughness, metal=0.0, tint=1.0, coverage=1.0):
        s = (SIZE, SIZE)
        self.albedo = albedo
        self.wear = np.broadcast_to(wear, s)
        self.normal = normal
        self.orm = [np.broadcast_to(x, s) for x in (occlusion, roughness, metal, tint)]
        self.coverage = np.broadcast_to(coverage, s)

    def mean(self):
        """The linear mean albedo: what the vertex colour's multiplier is relative to."""
        return tuple(float(x) for x in self.albedo.reshape(-1, 3).mean(0))

    def images(self):
        albedo = np.concatenate([_u8(_srgb(self.albedo)), _u8(self.wear)[..., None]], -1)
        normal = np.concatenate([_u8(self.normal * 0.5 + 0.5), _u8(self.coverage)[..., None]], -1)
        orm = np.stack([_u8(x) for x in self.orm], -1)
        return {"albedo": png(albedo), "normal": png(normal), "orm": png(orm)}


# ---------------------------------------------------------------- recipes
# name -> (tile metres, wear colour (linear rgb, roughness), builder() -> Baked)
RECIPES = {}
_CACHE = {}


def recipe(name, tile, wear=(0.13, 0.115, 0.09, 0.95)):
    def wrap(fn):
        RECIPES[name] = (tile, wear, fn)
        return fn

    return wrap


def baked(name):
    if name not in _CACHE:
        _CACHE[name] = RECIPES[name][2]()
    return _CACHE[name]


def tile_of(name):
    return RECIPES[name][0]


def chips(seed, cells=10, bias=0.0):
    """A wear threshold: irregular flakes (cells broken by noise), low where paint lifts first."""
    f1, f2, ident = worley(cells, seed)
    edge = f2 - f1
    rnd = np.random.default_rng(seed + 5).random(cells * cells)[ident]
    breakup = fbm(cells * 2, seed + 9, 4)
    t = 0.45 + 0.25 * rnd + 0.55 * (breakup - 0.5) + 0.1 * smoothstep(0.0, 0.3, edge)
    return np.clip(t + bias, 0.02, 1.0)


@recipe("nato_camo", tile=4.0, wear=(0.15, 0.13, 0.1, 0.95))
def nato_camo():
    """NATO three-colour on armour: green base, brown and black bands with
    crisp, hand-sprayed edges; mottled, sun-faded paint over a fine orange peel."""
    green, brown, black = (0.06, 0.085, 0.045), (0.075, 0.052, 0.032), (0.012, 0.012, 0.011)
    f = warp(fbm(3, 11, 4), 10, 12)
    g = warp(fbm((3, 4), 21, 4), 12, 22)
    edge = 0.012
    col = mix(green, brown, smoothstep(0.52 - edge, 0.52 + edge, f))
    col = mix(col, black, smoothstep(0.6 - edge, 0.6 + edge, g) * (1 - smoothstep(0.62, 0.7, f)))
    mottle = fbm(24, 31, 4)
    col = col * (0.92 + 0.16 * mottle)[..., None]
    dust = smoothstep(0.5, 0.9, fbm(8, 41, 4))
    col = mix(col, (0.12, 0.11, 0.085), dust * 0.25)
    col = mix(col, (0.13, 0.12, 0.1), scratches(81) * 0.6)
    peel = fbm(96, 51, 2)
    h = peel * 0.2 - scratches(81) * 0.6
    rough = 0.44 + 0.3 * dust + 0.08 * (mottle - 0.5)
    return Baked(col, chips(71, 14), normals_from_height(blur(h), 1.0), 0.86 - 0.1 * dust, rough)


def scratches(seed, count=3):
    """Fine scratches and scuffs: thin, broken lines at a few angles."""
    out = np.zeros((SIZE, SIZE))
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float)
    # integer directions keep every line periodic across the tile
    for k, (i, j) in enumerate(((1, 3), (5, -2), (4, 1), (-2, 5))[:count]):
        u = (xx * i + yy * j) / SIZE
        v = (-xx * j + yy * i) / SIZE
        line = np.abs(np.mod(u * 7 + 0.35 * fbm(4, seed + k, 3), 1.0) - 0.5)
        keep = smoothstep(0.68, 0.8, fbm(12, seed + 10 + k, 3))
        out = np.maximum(out, smoothstep(0.012, 0.0, line) * keep * (0.5 + 0.5 * np.cos(v * math.pi * 2 * 5)))
    return np.clip(out, 0, 1)


@recipe("olive_paint", tile=2.0, wear=(0.14, 0.125, 0.1, 0.95))
def olive_paint():
    """Olive drab over steel: a single colour, mottled and dusty, chipped."""
    base = (0.058, 0.066, 0.042)
    mottle = fbm(20, 101, 4)
    col = np.broadcast_to(np.array(base), (SIZE, SIZE, 3)) * (0.85 + 0.3 * mottle)[..., None]
    dust = smoothstep(0.5, 0.95, fbm(6, 103, 4))
    col = mix(col, (0.12, 0.11, 0.085), dust * 0.25)
    h = fbm(96, 105, 2) * 0.2 - scratches(111) * 0.5
    return Baked(col, chips(109, 12), normals_from_height(blur(h), 1.2), 0.86 - 0.1 * dust, 0.66 + 0.2 * dust)


@recipe("rubber", tile=0.6, wear=(0.12, 0.105, 0.085, 1.0))
def rubber():
    """Worn tyre and pad rubber: near black, a little grey where it scuffs, fine grain and cuts."""
    grain = fbm(64, 201, 3)
    col = np.broadcast_to(np.array((0.024, 0.024, 0.023)), (SIZE, SIZE, 3)) * (0.8 + 0.5 * grain)[..., None]
    scuff = smoothstep(0.6, 0.95, fbm(12, 203, 4))
    col = mix(col, (0.06, 0.058, 0.055), scuff * 0.6)
    f1, f2, _ = worley(20, 205)
    cuts = smoothstep(0.04, 0.0, f2 - f1) * (fbm(20, 207, 2) > 0.6)
    h = grain * 0.4 - cuts * 1.5
    return Baked(col, 0.3 + 0.7 * fbm(16, 209, 4), normals_from_height(blur(h), 1.0), 1.0 - 0.3 * cuts, 0.88 - 0.1 * scuff)


@recipe("bare_steel", tile=0.8, wear=(0.08, 0.05, 0.03, 0.9))
def bare_steel():
    """Dark oiled steel: blued and brown, speckled rust, pitted; polished where handled."""
    n = fbm(10, 301, 5)
    col = mix((0.05, 0.05, 0.048), (0.075, 0.065, 0.055), n)
    rust = smoothstep(0.7, 0.85, fbm(14, 303, 5))
    col = mix(col, (0.07, 0.04, 0.025), rust * 0.8)
    pits = smoothstep(0.06, 0.0, worley(40, 305)[0])
    h = n * 0.3 - pits * 0.8 + rust * 0.3
    rough = 0.38 + 0.4 * rust + 0.1 * n
    metal = 0.75 - 0.6 * rust
    return Baked(col, 0.25 + 0.75 * fbm(18, 307, 4), normals_from_height(blur(h), 1.2), 1.0 - 0.2 * pits, rough, metal)


def weave(threads, seed, depth=1.0):
    """A plain weave's height: over-under threads, `threads` per tile, with slub."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE * threads
    wx = np.sin(xx * math.pi * 2) * 0.5 + 0.5
    wy = np.sin(yy * math.pi * 2) * 0.5 + 0.5
    over = (np.floor(xx) + np.floor(yy)) % 2
    h = np.where(over > 0, wx * 0.7 + wy * 0.3, wy * 0.7 + wx * 0.3)
    slub = fbm((4, threads // 4), seed, 3)
    return (h + 0.3 * slub) * depth


def folds(seed, cells=4, aspect=3):
    """Soft cloth folds and creases: stretched, ridged noise."""
    a = fbm((cells, cells * aspect), seed, 3)
    ridge = 1 - np.abs(a - 0.5) * 2
    return blur(ridge ** 3, 2)


@recipe("canvas", tile=0.5, wear=(0.2, 0.18, 0.14, 1.0))
def canvas():
    """Heavy cotton duck: a coarse weave, water stains, dust in the folds."""
    w = weave(64, 401)
    fold = folds(403, 3)
    base = (0.1, 0.098, 0.064)  # olive-drab duck
    stain = fbm(5, 405, 4)
    col = np.broadcast_to(np.array(base), (SIZE, SIZE, 3)) * (0.8 + 0.3 * stain + 0.12 * (w - 0.6))[..., None]
    col = mix(col, (0.12, 0.105, 0.08), smoothstep(0.55, 0.8, 1 - fold) * 0.3)
    h = w * 0.3 + fold * 2.5
    return Baked(col, 0.2 + 0.8 * fbm(10, 407, 4), normals_from_height(h, 1.0), 0.85 + 0.15 * fold, 0.95)


@recipe("track_link", tile=1.0, wear=(0.13, 0.11, 0.085, 1.0))
def track_link():
    """One all-steel track link along u (U = one link pitch), the band's width along v:
    a shoe with a raised grouser bar, hinge pins at both ends, the guide horn's
    slot in the middle, dark gaps between links. Polished on the grouser's top,
    rust and packed mud in the recesses."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    u, v = xx, yy
    gap = smoothstep(0.035, 0.0, np.minimum(u, 1 - u))
    grouser = smoothstep(0.06, 0.03, np.abs(u - 0.62)) * (smoothstep(0.08, 0.12, v) * smoothstep(0.92, 0.88, v))
    pins = smoothstep(0.06, 0.03, np.minimum(np.abs(u - 0.08), np.abs(u - 0.92)))
    slot = smoothstep(0.06, 0.04, np.abs(v - 0.5)) * smoothstep(0.35, 0.3, np.abs(u - 0.5))
    edge = smoothstep(0.08, 0.0, np.minimum(v, 1 - v))
    h = 0.5 + grouser * 1.2 + pins * 0.5 - slot * 1.0 - gap * 1.5 - edge * 0.2 + fbm(40, 501, 3) * 0.15
    grime = fbm(12, 503, 4)
    steel = mix((0.07, 0.066, 0.06), (0.11, 0.1, 0.09), grime)
    col = mix(steel, (0.2, 0.19, 0.17), grouser * 0.8)
    recess = np.clip(slot + gap + (1 - grouser) * 0.3, 0, 1)
    col = mix(col, (0.09, 0.07, 0.05), recess * smoothstep(0.4, 0.8, grime) * 0.7)
    rough = 0.6 - 0.3 * grouser + 0.3 * recess
    metal = 0.7 * (1 - recess * 0.7)
    occ = 1 - 0.7 * gap - 0.4 * slot
    return Baked(col, 0.4 + 0.6 * fbm(8, 505, 3), normals_from_height(blur(h * 3), 1.0), occ, rough, metal)


@recipe("burnt_metal", tile=2.0, wear=(0.085, 0.036, 0.014, 0.95))
def burnt_metal():
    """A burnt-out hull, days after the fire: the paint burnt off to black char and
    dull brown scale in torn patches, pitted and flaking; the vertex colour lays
    soot over the walls (`soot`) and ash on the flats (`ash`), and the wear colour
    is orange rust coming through at edges and low down."""
    grain = fbm(48, 601, 3)
    char = (0.024, 0.022, 0.02)
    scale_ = (0.07, 0.056, 0.044)
    f1, f2, ident = worley(36, 603)
    torn = np.random.default_rng(605).random(36 * 36)[ident]  # small flakes, each its own
    patch = smoothstep(0.45, 0.55, torn + 0.25 * (fbm(24, 607, 3) - 0.5))
    col = mix(char, scale_, 0.45 + patch * 0.2 + 0.25 * (fbm(5, 619, 4) - 0.5))
    col = col * (0.88 + 0.24 * grain)[..., None]
    flake_edge = smoothstep(0.05, 0.0, f2 - f1)
    col = mix(col, (0.006, 0.006, 0.006), flake_edge * 0.25)
    pits = smoothstep(0.1, 0.0, worley(40, 609)[0])
    h = grain * 0.3 + patch * 0.25 - flake_edge * 0.2 - pits * 0.4
    return Baked(col, chips(613, 12, bias=0.1), normals_from_height(blur(h), 1.0), 1.0 - 0.3 * pits, 0.93)


def _camo_print(blobs, seed, scale_cells):
    """A printed multi-colour camouflage: layered, warped blobs with crisp edges."""
    col = np.broadcast_to(np.array(blobs[0]), (SIZE, SIZE, 3)).copy()
    for k, (colour, level) in enumerate(blobs[1:]):
        f = warp(fbm(scale_cells, seed + k * 13, 5), 4, seed + k * 13 + 1, cells=12)
        col = mix(col, colour, smoothstep(level - 0.004, level + 0.004, f))
    return col


@recipe("multicam_ripstop", tile=0.42, wear=(0.075, 0.058, 0.04, 1.0))
def multicam_ripstop():
    """Printed multi-terrain camouflage on nylon-cotton ripstop: khaki ground,
    green and brown blotches, light flecks, dark branches; the ripstop grid,
    faded print and seam folds in the normal."""
    col = _camo_print([(0.14, 0.133, 0.092), ((0.2, 0.188, 0.14), 0.72), ((0.066, 0.086, 0.046), 0.52),
                       ((0.105, 0.075, 0.047), 0.6), ((0.042, 0.034, 0.026), 0.68)], 701, 7)
    branch = np.abs(warp(fbm(6, 711, 4), 8, 712) - 0.5)
    col = mix(col, (0.05, 0.045, 0.034), smoothstep(0.02, 0.01, branch) * 0.5)
    fade = fbm(4, 713, 4)
    col = col * (0.9 + 0.2 * fade)[..., None]
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float)
    grid = np.maximum(smoothstep(0.8, 1.0, np.cos(xx / SIZE * 64 * math.pi * 2)),
                      smoothstep(0.8, 1.0, np.cos(yy / SIZE * 64 * math.pi * 2)))
    w = weave(128, 715, 0.4)
    h = w * 0.08 + grid * 0.1 + folds(717, 2) * 2.5
    col = col * (0.97 + 0.05 * grid)[..., None]
    # cloth is matte: its folds and weave occlude the sky, so it takes little sheen
    return Baked(col, 0.25 + 0.75 * fbm(8, 719, 4), normals_from_height(h, 1.0), 0.72 + 0.1 * folds(717, 2), 0.97)


@recipe("cordura", tile=0.3, wear=(0.2, 0.18, 0.14, 1.0))
def cordura():
    """Coarse nylon (plate carrier, pouches, webbing): a neutral light weave the
    vertex colour tints per part, with PALS webbing rows every 2.5 cm and stitching."""
    w = weave(96, 801)
    yy = np.mgrid[0:SIZE, 0:SIZE][0].astype(float) / SIZE * 12
    pals = smoothstep(0.35, 0.3, np.abs(np.mod(yy, 1.0) - 0.5))
    stitch = (np.mod(np.mgrid[0:SIZE, 0:SIZE][1], 6) < 3) * smoothstep(0.03, 0.0, np.abs(np.mod(yy, 1.0) - 0.15))
    tone = 0.72 + 0.1 * fbm(8, 803, 4) + 0.05 * (w - 0.6) - 0.025 * (1 - pals)
    col = np.broadcast_to(np.array((0.8, 0.8, 0.8)), (SIZE, SIZE, 3)) * tone[..., None]
    h = w * 0.2 + pals * 0.25 + stitch * 0.15
    return Baked(col, 0.3 + 0.7 * fbm(8, 805, 4), normals_from_height(blur(h), 1.2), 0.72 + 0.06 * pals, 0.97 - 0.06 * pals)


@recipe("nylon", tile=0.3, wear=(0.2, 0.18, 0.14, 1.0))
def nylon():
    """Plain coarse nylon (pouches, packs, radio covers): a neutral light weave
    the vertex colour tints per part, soft creases and a little fade."""
    w = weave(96, 811)
    crease = folds(813, 4, aspect=1)
    tone = 0.72 + 0.1 * fbm(6, 815, 4) + 0.05 * (w - 0.6) - 0.04 * crease
    col = np.broadcast_to(np.array((0.8, 0.8, 0.8)), (SIZE, SIZE, 3)) * tone[..., None]
    h = w * 0.2 + crease * 1.5
    return Baked(col, 0.3 + 0.7 * fbm(8, 817, 4), normals_from_height(blur(h), 1.2), 0.75, 0.97)


@recipe("gunmetal", tile=0.25, wear=(0.16, 0.16, 0.16, 0.35))
def gunmetal():
    """Phosphated steel and hard-anodised alloy: near black, a satin sheen, worn
    to bright metal on the edges (the wear colour)."""
    n = fbm(64, 1021, 3)
    col = np.broadcast_to(np.array((0.028, 0.028, 0.03)), (SIZE, SIZE, 3)) * (0.85 + 0.3 * n)[..., None]
    rough = 0.32 + 0.15 * fbm(10, 1023, 3)
    return Baked(col, 0.3 + 0.7 * fbm(12, 1025, 4), normals_from_height(n * 0.3, 0.8), 1.0, rough, 0.55)


@recipe("leather", tile=0.25, wear=(0.13, 0.1, 0.07, 1.0))
def leather():
    """Boot leather and suede: a neutral light grain the vertex colour tints, creased."""
    f1, f2, _ = worley(48, 901)
    grain = smoothstep(0.0, 0.25, f2 - f1)
    crease = folds(903, 3)
    tone = 0.7 + 0.12 * fbm(10, 905, 4) + 0.08 * grain - 0.1 * crease
    col = np.broadcast_to(np.array((0.8, 0.8, 0.8)), (SIZE, SIZE, 3)) * tone[..., None]
    h = grain * 0.4 - crease * 2
    return Baked(col, 0.3 + 0.7 * fbm(6, 907, 4), normals_from_height(blur(h), 1.2), 0.8, 0.86 + 0.1 * crease)


@recipe("polymer", tile=0.25, wear=(0.2, 0.19, 0.17, 0.6))
def polymer():
    """Weapon polymer and phosphated steel: a neutral light stipple the vertex
    colour tints, edges worn to grey metal."""
    stip = fbm(96, 1001, 2)
    tone = 0.72 + 0.12 * stip + 0.06 * fbm(6, 1003, 3)
    col = np.broadcast_to(np.array((0.8, 0.8, 0.8)), (SIZE, SIZE, 3)) * tone[..., None]
    return Baked(col, 0.35 + 0.65 * fbm(10, 1005, 4), normals_from_height(stip, 0.8), 1.0, 0.42 + 0.18 * stip)


@recipe("skin", tile=0.15, wear=(0.1, 0.08, 0.06, 0.8))
def skin():
    """Weathered skin: a neutral light ground the vertex colour tints (face, stubble,
    lips), blotched warm and cool, with pores and a little oily sheen."""
    blotch = fbm(6, 1151, 4)
    pores = smoothstep(0.08, 0.0, worley(64, 1153)[0])
    col = np.stack([0.8 + 0.06 * blotch, 0.78 - 0.02 * blotch, 0.77 - 0.04 * blotch], -1)
    col = col * (1.0 - 0.08 * pores)[..., None]
    h = -pores * 0.6 + fbm(24, 1155, 3) * 0.3
    return Baked(col, 1.0, normals_from_height(h, 0.8), 1.0, 0.52 + 0.12 * blotch, tint=0.0)


@recipe("hard_plastic", tile=0.25, wear=(0.2, 0.18, 0.14, 1.0))
def hard_plastic():
    """Knee pads and helmet fittings: moulded plastic, a light texture, scuffed."""
    n = fbm(40, 1101, 3)
    col = np.broadcast_to(np.array((0.8, 0.8, 0.8)), (SIZE, SIZE, 3)) * (0.72 + 0.12 * n)[..., None]
    scuff = smoothstep(0.7, 0.9, fbm(20, 1103, 4))
    return Baked(col, 0.3 + 0.7 * fbm(8, 1105, 4), normals_from_height(n - scuff, 0.8), 1.0, 0.62 + 0.25 * scuff)


@recipe("ammo_paint", tile=1.0, wear=(0.2, 0.19, 0.17, 0.5))
def ammo_paint():
    """Olive enamel on sheet steel (ammo cans, latches): glossier, chipped to bare metal."""
    base = (0.07, 0.08, 0.045)
    n = fbm(24, 1201, 4)
    col = np.broadcast_to(np.array(base), (SIZE, SIZE, 3)) * (0.85 + 0.3 * n)[..., None]
    return Baked(col, chips(1203, 16), normals_from_height(fbm(96, 1205, 2) * 0.3, 1.0), 1.0, 0.45 + 0.15 * n)


@recipe("painted_wood", tile=1.0, wear=(0.16, 0.11, 0.065, 0.9))
def painted_wood():
    """Olive-painted ammunition crate boards: planks with grain showing through
    thin paint, dark seams between boards; the wear is bare wood."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    boards = 6
    seam = smoothstep(0.02, 0.0, np.abs(np.mod(yy * boards, 1.0) - 0.0) / boards * boards)
    seam = np.maximum(seam, smoothstep(0.012, 0.0, np.minimum(np.mod(yy * boards, 1.0), 1 - np.mod(yy * boards, 1.0))))
    board = np.floor(yy * boards)
    grain = fbm((2, 48), 1301, 4)
    grain = warp(grain, 3, 1303)
    tone = 0.82 + 0.25 * grain + 0.08 * np.sin(board * 3.1)
    col = np.broadcast_to(np.array((0.095, 0.1, 0.058)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.02, 0.02, 0.015), seam)
    h = grain * 0.6 - seam * 2
    return Baked(col, chips(1305, 12, bias=0.08), normals_from_height(blur(h), 1.2), 1.0 - 0.5 * seam, 0.85)


@recipe("pallet_wood", tile=1.0, wear=(0.2, 0.16, 0.1, 1.0))
def pallet_wood():
    """Raw, weathered pallet pine: grey-brown boards, open grain, knots."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    grain = warp(fbm((2, 64), 1401, 4), 4, 1403)
    knots = smoothstep(0.12, 0.0, worley(5, 1405)[0])
    tone = 0.75 + 0.35 * grain - 0.3 * knots
    col = np.broadcast_to(np.array((0.2, 0.16, 0.11)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.16, 0.155, 0.14), smoothstep(0.5, 0.9, fbm(6, 1407, 4)) * 0.5)
    return Baked(col, 0.3 + 0.7 * fbm(6, 1409, 4), normals_from_height(blur(grain * 1.5 - knots), 1.2), 1.0, 0.95)


@recipe("field_stone", tile=1.2, wear=(0.075, 0.072, 0.052, 1.0))
def field_stone():
    """Weathered limestone: speckled grain, pitting, and lichen and moss (the wear
    colour) creeping in from where damp sits."""
    g = fbm(24, 1501, 5)
    spots = smoothstep(0.7, 0.9, fbm(48, 1503, 3))
    col = mix((0.17, 0.165, 0.15), (0.26, 0.25, 0.225), g)
    col = mix(col, (0.18, 0.17, 0.15), spots * 0.6)
    lichen = smoothstep(0.62, 0.72, fbm(10, 1505, 5))
    col = mix(col, (0.19, 0.19, 0.13), lichen * 0.45)
    pits = smoothstep(0.07, 0.0, worley(28, 1507)[0])
    h = g * 1.2 - pits
    return Baked(col, 0.25 + 0.75 * fbm(8, 1509, 5), normals_from_height(blur(h), 2.0), 1.0 - 0.25 * pits, 0.95)


@recipe("concrete", tile=5.0, wear=(0.12, 0.11, 0.09, 1.0))
def concrete():
    """Cast concrete: aggregate speckle, shutter-board lines, water stains; the
    wear is grime."""
    g = fbm(64, 1601, 3)
    agg = smoothstep(0.75, 0.85, fbm(80, 1603, 2))
    stain = fbm(4, 1605, 5)
    col = mix((0.27, 0.27, 0.26), (0.35, 0.35, 0.34), g)
    col = mix(col, (0.22, 0.21, 0.19), agg * 0.5)
    col = col * (0.65 + 0.5 * stain)[..., None]
    yy = np.mgrid[0:SIZE, 0:SIZE][0].astype(float) / SIZE * 8
    board = smoothstep(0.03, 0.0, np.minimum(np.mod(yy, 1.0), 1 - np.mod(yy, 1.0)))
    h = g * 0.5 - agg * 0.4 - board * 0.8
    return Baked(col, 0.2 + 0.8 * fbm(6, 1607, 5), normals_from_height(blur(h), 1.4), 1.0, 0.92)


@recipe("precast", tile=3.0, wear=(0.12, 0.11, 0.09, 1.0))
def precast():
    """A precast concrete wall panel, one 3 m bay by one 3 m floor to the tile: a pale
    neutral exposed-aggregate face the building's tint colours, the joint round its
    edge and rain marks falling from the joint above; the wear is grime."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    f1, _, ident = worley(96, 2601)
    stone = smoothstep(0.5, 0.15, f1)
    rnd = np.random.default_rng(2603).random(96 * 96)[ident]
    stain = fbm(3, 2605, 5)
    joint = smoothstep(0.03, 0.012, np.minimum(np.minimum(xx, 1 - xx), np.minimum(yy, 1 - yy)) * 3.0)
    run = smoothstep(0.3, 0.0, yy) * smoothstep(0.45, 0.8, fbm((24, 2), 2607, 3))  # image rows run down the wall
    tone = 0.84 + 0.2 * stain + 0.14 * (rnd - 0.5) * stone - 0.14 * run
    col = np.broadcast_to(np.array((0.74, 0.73, 0.7)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.12, 0.12, 0.11), joint * 0.8)
    h = stone * 0.5 - joint * 3.0
    return Baked(col, 0.25 + 0.75 * fbm(6, 2609, 5), normals_from_height(blur(h), 1.2), 1.0 - 0.4 * joint, 0.93)


@recipe("mosaic", tile=1.5, wear=(0.16, 0.155, 0.145, 1.0))
def mosaic():
    """Small glazed facing tiles, sixteen to the tile each way: a pale neutral glaze
    the building's tint colours, grey grout, a tile here and there replaced by a
    darker one; the wear is the cement bed where tiles have fallen."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    n = 16
    ix, iy = np.floor(xx * n).astype(int), np.floor(yy * n).astype(int)
    fx, fy = xx * n - ix, yy * n - iy
    grout = np.maximum(smoothstep(0.1, 0.03, np.minimum(fx, 1 - fx)), smoothstep(0.1, 0.03, np.minimum(fy, 1 - fy)))
    fired = np.random.default_rng(2701).random((n, n))[iy, ix]
    odd = np.random.default_rng(2703).random((n, n))[iy, ix] > 0.96
    tone = (0.92 + 0.06 * (fired - 0.5) + 0.14 * (fbm(3, 2705, 4) - 0.5)) * np.where(odd, 0.86, 1.0)
    col = np.broadcast_to(np.array((0.76, 0.76, 0.74)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.3, 0.3, 0.28), grout * 0.7)
    return Baked(col, chips(2707, 8, bias=0.25), normals_from_height(blur(1.0 - grout), 0.8), 1.0 - 0.3 * grout, 0.32 + 0.55 * grout)


@recipe("asphalt", tile=2.0, wear=(0.16, 0.15, 0.13, 1.0))
def asphalt():
    """Old tarmac: black binder, grey chippings, cracks and oil stains."""
    chip = smoothstep(0.6, 0.75, fbm(96, 1701, 2))
    col = mix((0.05, 0.05, 0.048), (0.13, 0.125, 0.115), chip)
    stain = fbm(5, 1703, 5)
    col = col * (0.75 + 0.4 * stain)[..., None]
    f1, f2, _ = worley(8, 1705)
    crack = smoothstep(0.03, 0.0, f2 - f1) * (fbm(8, 1707, 3) > 0.55)
    col = mix(col, (0.02, 0.02, 0.02), crack)
    h = chip * 0.6 - crack * 2
    return Baked(col, 0.3 + 0.7 * fbm(6, 1709, 5), normals_from_height(blur(h), 1.4), 1.0 - 0.4 * crack, 0.9)


@recipe("marking_paint", tile=0.5, wear=(0.06, 0.07, 0.045, 0.7))
def marking_paint():
    """Stencilled white-grey marking paint, patchy and worn back to the camouflage green."""
    n = fbm(20, 1801, 4)
    col = np.broadcast_to(np.array((0.55, 0.54, 0.5)), (SIZE, SIZE, 3)) * (0.8 + 0.25 * n)[..., None]
    return Baked(col, chips(1803, 20, bias=0.1), normals_from_height(fbm(96, 1805, 2) * 0.2, 1.0), 1.0, 0.6, tint=0.0)


@recipe("hessian", tile=0.4, wear=(0.16, 0.14, 0.1, 1.0))
def hessian():
    """Sandbag jute: a loose, hairy weave, sun-bleached tan, earth-stained."""
    w = weave(40, 1901, 1.2)
    hair = fbm(128, 1903, 2)
    stain = fbm(4, 1905, 4)
    col = np.broadcast_to(np.array((0.2, 0.16, 0.1)), (SIZE, SIZE, 3)) * (0.7 + 0.3 * w + 0.15 * hair)[..., None]
    col = mix(col, (0.11, 0.09, 0.065), smoothstep(0.55, 0.8, stain) * 0.6)
    h = w * 0.8 + hair * 0.2
    return Baked(col, 0.2 + 0.8 * fbm(8, 1907, 4), normals_from_height(h, 1.4), 0.8 + 0.2 * w, 0.97, tint=0.0)


@recipe("soil", tile=2.0, wear=(0.07, 0.055, 0.04, 1.0))
def soil():
    """Dug earth: brown loam in clods and crumbs, darker where damp, pale stones."""
    clods = worley(18, 2001)
    lump = smoothstep(0.0, 0.25, clods[1] - clods[0])
    g = fbm(48, 2003, 4)
    damp = fbm(5, 2005, 4)
    col = mix((0.075, 0.058, 0.04), (0.13, 0.1, 0.07), g * 0.6 + lump * 0.4)
    col = col * (0.75 + 0.4 * damp)[..., None]
    stones = smoothstep(0.05, 0.0, worley(30, 2007)[0])
    col = mix(col, (0.2, 0.19, 0.17), stones * 0.7)
    h = lump * 1.0 + g * 0.5 + stones * 0.5
    return Baked(col, 0.3 + 0.7 * fbm(6, 2009, 4), normals_from_height(blur(h), 1.6), 0.8 + 0.2 * lump, 0.97, tint=0.0)


@recipe("plaster", tile=4.0, wear=(0.2, 0.125, 0.09, 0.95))
def plaster():
    """Lime render on a house wall: a pale neutral ground the building's tint
    colours, with trowel marks, damp blotches and hairline cracks; the wear is
    the brick it falls off."""
    blotch = fbm(4, 2101, 5)
    grain = fbm(96, 2103, 3)
    trowel = warp(fbm((6, 24), 2105, 3), 6, 2106)
    f1, f2, _ = worley(7, 2107)
    crack = smoothstep(0.035, 0.0, f2 - f1) * smoothstep(0.5, 0.7, fbm(6, 2109, 3))
    tone = 0.8 + 0.24 * blotch + 0.07 * (grain - 0.5) + 0.06 * (trowel - 0.5)
    col = np.broadcast_to(np.array((0.74, 0.73, 0.7)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.3, 0.29, 0.27), crack * 0.6)
    h = grain * 0.25 + trowel * 0.5 - crack * 1.5
    return Baked(col, chips(2111, 9, bias=0.12), normals_from_height(blur(h), 1.0), 1.0 - 0.3 * crack, 0.9)


@recipe("roughcast", tile=3.0, wear=(0.17, 0.16, 0.15, 1.0))
def roughcast():
    """Pebble-dash render: a pale neutral ground the building's tint colours,
    speckled with stones and stained by rain; the wear is the grey cement under it."""
    f1, _, ident = worley(72, 2201)
    pebble = smoothstep(0.55, 0.1, f1)
    rnd = np.random.default_rng(2203).random(72 * 72)[ident]
    stain = fbm(3, 2205, 5)
    tone = 0.72 + 0.24 * stain + 0.18 * (rnd - 0.5) * pebble - 0.1 * (1 - pebble)
    col = np.broadcast_to(np.array((0.7, 0.69, 0.66)), (SIZE, SIZE, 3)) * tone[..., None]
    return Baked(col, chips(2207, 8, bias=0.2), normals_from_height(blur(pebble * 0.9), 1.6), 0.85 + 0.15 * pebble, 0.96)


@recipe("brick", tile=2.0, wear=(0.2, 0.19, 0.17, 1.0))
def brick():
    """Facing brick in running bond, 26 courses and 8 bricks to the tile: every
    brick its own fired tone, pale recessed mortar; the wear is lime bloom."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    courses, per = 26, 8
    row = np.floor(yy * courses).astype(int)
    u = xx * per + 0.5 * (row % 2)
    fx, fy = u - np.floor(u), yy * courses - row
    mortar = np.maximum(smoothstep(0.13, 0.05, np.minimum(fy, 1 - fy)), smoothstep(0.04, 0.015, np.minimum(fx, 1 - fx)))
    fired = np.random.default_rng(2301).random((courses, per))[row, np.floor(u).astype(int) % per]
    col = mix((0.21, 0.09, 0.062), (0.31, 0.135, 0.082), fired)
    col = mix(col, (0.13, 0.075, 0.062), smoothstep(0.86, 0.9, np.random.default_rng(2303).random((courses, per))[row, np.floor(u).astype(int) % per]))
    grain = fbm(64, 2305, 3)
    col = col * (0.88 + 0.24 * grain)[..., None] * (0.85 + 0.3 * fbm(3, 2307, 4))[..., None]
    col = mix(col, (0.34, 0.32, 0.28), mortar)
    h = (1 - mortar) * 1.0 + grain * 0.2
    return Baked(col, 0.3 + 0.7 * fbm(8, 2309, 4), normals_from_height(blur(h), 1.2), 1.0 - 0.35 * mortar, 0.88 + 0.08 * mortar)


@recipe("roof_tile", tile=2.0, wear=(0.1, 0.11, 0.06, 1.0))
def roof_tile():
    """Clay pantiles, six courses and eight rolls to the tile. A roof's own UVs
    run u along the eave and v up the slope, so each course's butt is at the low
    edge of its row and the course above shades its top. Lichen is the wear."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    courses, pans = 6, 8
    row = np.floor(yy * courses).astype(int)
    fy = yy * courses - row  # 0 at the top of a course (image rows run down the slope)
    pan = np.floor(xx * pans).astype(int)
    roll = 0.5 - 0.5 * np.cos((xx * pans - pan) * 2 * math.pi)
    fired = np.random.default_rng(2401).random((courses, pans))[row, pan]
    shade = smoothstep(0.16, 0.0, fy)
    col = mix((0.2, 0.085, 0.055), (0.31, 0.135, 0.08), 0.3 + 0.4 * fired)
    col = col * ((0.78 + 0.3 * roll) * (1 - 0.5 * shade) * (0.85 + 0.3 * fbm(4, 2403, 4)))[..., None]
    col = mix(col, (0.15, 0.15, 0.1), smoothstep(0.62, 0.8, fbm(7, 2405, 4)) * 0.45)
    h = roll * 1.2 + fy * 1.6
    return Baked(col, 0.3 + 0.7 * fbm(8, 2407, 4), normals_from_height(blur(h * 2), 1.0), 1.0 - 0.45 * shade, 0.82, tint=0.0)


@recipe("joinery", tile=1.0, wear=(0.18, 0.13, 0.085, 0.9))
def joinery():
    """Painted joinery (doors, shutters, shopfronts): a pale neutral paint the
    building's tint colours, over upright boards; the wear is bare wood."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    boards = 8
    fx = xx * boards - np.floor(xx * boards)
    seam = smoothstep(0.06, 0.0, np.minimum(fx, 1 - fx))
    grain = warp(fbm((48, 2), 2501, 4), 3, 2503)
    tone = 0.84 + 0.12 * grain + 0.1 * (fbm(5, 2505, 4) - 0.5)
    col = np.broadcast_to(np.array((0.72, 0.72, 0.7)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.2, 0.2, 0.19), seam * 0.7)
    return Baked(col, chips(2507, 12, bias=0.1), normals_from_height(blur(grain * 0.5 - seam * 2), 1.2), 1.0 - 0.4 * seam, 0.6)


@recipe("rubble_stone", tile=3.0, wear=(0.075, 0.08, 0.05, 1.0))
def rubble_stone():
    """A farm wall of field stone laid as random rubble: flat stones a hand or two
    across, close in tone, in pale recessed lime mortar. A pale ground a
    building's tint can warm or cool; the wear is moss."""
    cells = 9
    rows = (np.arange(SIZE) * 2) % SIZE  # twice the courses up the wall: stones lie flat
    f1, f2, ident = (x[rows] for x in worley(cells, 2601))
    joint = smoothstep(0.16, 0.04, f2 - f1)
    stone = np.random.default_rng(2603).random(cells * cells)[ident]
    grain = fbm(64, 2605, 3)
    col = mix((0.36, 0.345, 0.31), (0.46, 0.44, 0.39), stone)
    col = col * (0.88 + 0.24 * grain)[..., None] * (0.82 + 0.36 * fbm(3, 2607, 4))[..., None]
    col = mix(col, (0.5, 0.48, 0.43), joint)
    h = (1 - joint) * (0.7 + 0.5 * stone) + grain * 0.3
    return Baked(col, 0.3 + 0.7 * fbm(8, 2609, 4), normals_from_height(blur(h), 1.8), 1.0 - 0.4 * joint, 0.94)


# Industrial sheet, block and glass. Image rows run down a wall (and down a roof's slope, by
# its own UVs), so whatever a fixing or a joint sheds trails toward the higher rows.
RUST_STAIN = (0.2, 0.085, 0.035)


def _fixings(xx, yy, ribs, rows, tile, crown_at, seed, reach_m):
    """Screws on a profiled sheet's crowns, in `rows` (v, 0 to 1) across the tile: the
    heads, and the rust that some of them weep down the sheet for `reach_m`."""
    across = np.abs((xx * ribs) % 1.0 - crown_at) / ribs * tile  # metres from the crown's centre line
    rib = np.floor(xx * ribs).astype(int)
    heads, weep = np.zeros_like(xx), np.zeros_like(xx)
    for k, row in enumerate(rows):
        down = ((yy - row) % 1.0) * tile  # metres below the row
        heads = np.maximum(heads, smoothstep(0.03, 0.012, np.hypot(across, np.minimum(down, tile - down))))
        bleeds = np.random.default_rng(seed + k).random(ribs)[rib]
        run = smoothstep(0.035, 0.008, across) * np.exp(-down / (reach_m * (0.4 + bleeds))) * smoothstep(0.5, 0.75, bleeds)
        weep = np.maximum(weep, run * (0.6 + 0.4 * fbm((48, 6), seed + 20 + k, 2)))
    return heads, weep


@recipe("cladding", tile=3.0, wear=(0.105, 0.046, 0.02, 0.9))
def cladding():
    """Painted box-profile steel sheet on a wall, twelve ribs to the tile running up it:
    a pale neutral paint the building's tint colours, chalked and rain-washed, a lap and
    a row of fixings every 3 m and one between, rust weeping from some. The wear is rust."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    ribs = 12
    f = (xx * ribs) % 1.0
    crown = smoothstep(0.1, 0.22, f) * smoothstep(0.62, 0.5, f)
    flank = 4 * crown * (1 - crown)
    sheet = np.random.default_rng(2601).random(3)[np.floor(xx * 3).astype(int)]  # a sheet covers a metre
    fade = fbm(3, 2603, 4)
    wash = fbm((40, 2), 2605, 3)
    lap = smoothstep(0.012, 0.004, np.minimum(yy, 1 - yy))
    heads, weep = _fixings(xx, yy, ribs, (0.03, 0.53), 3.0, 0.36, 2610, 0.55)
    tone = 0.9 + 0.04 * (sheet - 0.5) + 0.16 * (fade - 0.5) + 0.1 * (wash - 0.5) + 0.05 * crown - 0.13 * flank
    col = np.broadcast_to(np.array((0.74, 0.74, 0.72)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.2, 0.2, 0.19), lap * 0.5)
    col = mix(col, RUST_STAIN, weep * 0.75)
    col = mix(col, (0.12, 0.1, 0.09), heads * 0.8)
    h = crown * 2.6 - lap * 0.5 + heads * 0.5
    worn = np.clip(chips(2607, 10, bias=0.15) - 0.4 * weep - 0.3 * lap, 0.02, 1.0)
    return Baked(col, worn, normals_from_height(blur(h), 1.0), 1.0 - 0.22 * flank - 0.3 * lap, 0.5 + 0.2 * fade + 0.3 * weep,
                 0.0, np.clip(1.0 - 0.9 * weep - heads, 0, 1))


@recipe("roof_sheet", tile=6.0, wear=(0.12, 0.052, 0.024, 0.9))
def roof_sheet():
    """Profiled steel roofing, sixteen ribs to the tile running up the slope (a roof's
    own UVs): galvanised grey a material's colour repaints, each sheet a tone of its
    own, a few replaced with new ones and a few gone dull, laps every 3 m, purlin rows
    of fixings weeping rust down the pans, dirt lying in them. The wear is rust."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    ribs = 16
    f = (xx * ribs) % 1.0
    crown = smoothstep(0.1, 0.24, f) * smoothstep(0.54, 0.4, f)
    flank = 4 * crown * (1 - crown)
    cell = (np.floor(yy * 2).astype(int), np.floor(xx * 8).astype(int))  # a sheet is 0.75 m by 3 m
    tone = np.random.default_rng(2701).random((2, 8))[cell]
    age = np.random.default_rng(2703).random((2, 8))[cell]
    fresh, dull = age > 0.86, age < 0.2
    lap = smoothstep(0.007, 0.002, np.minimum((yy * 2) % 1.0, 1 - (yy * 2) % 1.0) / 2)
    side = smoothstep(0.03, 0.0, np.minimum((xx * 8) % 1.0, 1 - (xx * 8) % 1.0)) * 0.5
    heads, weep = _fixings(xx, yy, ribs, (0.02, 0.27, 0.52, 0.77), 6.0, 0.32, 2710, 0.9)
    silt = (1 - crown) * smoothstep(0.35, 0.8, fbm((48, 3), 2705, 3))
    stain = fbm(3, 2707, 4)
    k = 0.9 + 0.07 * (tone - 0.5) + 0.07 * (stain - 0.5) + 0.05 * crown - 0.12 * flank - 0.16 * silt + 0.2 * fresh - 0.14 * dull
    col = np.broadcast_to(np.array((0.5, 0.51, 0.52)), (SIZE, SIZE, 3)) * k[..., None]
    col = mix(col, (0.12, 0.12, 0.12), np.clip(lap + side * 0.5, 0, 1) * 0.6)
    col = mix(col, RUST_STAIN, weep * 0.7)
    col = mix(col, (0.1, 0.09, 0.08), heads * 0.7)
    h = crown * 2.0 - lap * 0.6 + heads * 0.4
    worn = np.clip(chips(2709, 8, bias=0.12) - 0.45 * weep - 0.35 * lap - 0.25 * dull + 0.3 * fresh, 0.02, 1.0)
    rust = np.clip(weep + 0.5 * dull, 0, 1)
    return Baked(col, worn, normals_from_height(blur(h), 1.0), 1.0 - 0.2 * flank - 0.3 * lap, 0.46 + 0.2 * stain + 0.3 * rust,
                 0.12 * (1 - rust), 0.0)


@recipe("flat_roof", tile=8.0, wear=(0.055, 0.068, 0.036, 1.0))
def flat_roof():
    """A flat roof's mineral felt, laid in metre rolls with their laps showing: grey grit
    and pale dust drifted on it. Nothing here is big enough to count across a roof: its
    pools and patches are the building's own. The wear is moss."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    rolls = 8
    roll = np.floor(xx * rolls).astype(int)
    seam = smoothstep(0.05, 0.015, np.minimum((xx * rolls) % 1.0, 1 - (xx * rolls) % 1.0))
    end = np.random.default_rng(2801).random(rolls)[roll]  # where each roll's length ends
    seam = np.maximum(seam, smoothstep(0.006, 0.002, np.abs((yy - end + 0.5) % 1.0 - 0.5)))
    grit = fbm(96, 2803, 2)
    col = np.broadcast_to(np.array((0.2, 0.2, 0.195)), (SIZE, SIZE, 3)) * (0.9 + 0.2 * grit + 0.05 * (end - 0.5))[..., None]
    col = mix(col, (0.3, 0.295, 0.27), smoothstep(0.45, 0.9, fbm(6, 2805, 4)) * 0.4)
    col = mix(col, (0.07, 0.07, 0.07), seam * 0.5)
    h = grit * 0.3 + seam * 0.5
    return Baked(col, np.clip(0.25 + 0.75 * fbm(10, 2809, 4) - 0.25 * seam, 0.02, 1.0), normals_from_height(blur(h), 1.0),
                 1.0, 0.95, 0.0, 0.0)


@recipe("concrete_block", tile=2.4, wear=(0.1, 0.1, 0.085, 1.0))
def concrete_block():
    """Concrete blockwork, twelve courses and six blocks to the tile: a pale neutral
    face the building's tint paints (or leaves grey), each block barely its own tone,
    recessed joints, rain's wash down it. The wear is the damp that darkens its foot."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    courses, per = 12, 6
    row = np.floor(yy * courses).astype(int)
    u = xx * per + 0.5 * (row % 2)
    fx, fy = u - np.floor(u), yy * courses - row
    joint = np.maximum(smoothstep(0.1, 0.04, np.minimum(fy, 1 - fy)), smoothstep(0.05, 0.02, np.minimum(fx, 1 - fx)))
    cast = np.random.default_rng(2901).random((courses, per))[row, np.floor(u).astype(int) % per]
    grain = fbm(80, 2903, 3)
    tone = 0.86 + 0.07 * (cast - 0.5) + 0.12 * (grain - 0.5) + 0.2 * (fbm(3, 2905, 4) - 0.5) + 0.1 * (fbm((36, 2), 2907, 3) - 0.5)
    col = np.broadcast_to(np.array((0.66, 0.66, 0.64)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.42, 0.42, 0.4), joint * 0.7)
    h = (1 - joint) * 0.9 + grain * 0.3
    return Baked(col, 0.25 + 0.75 * fbm(8, 2909, 4), normals_from_height(blur(h), 1.2), 1.0 - 0.3 * joint, 0.93)


@recipe("tilt_slab", tile=6.0, wear=(0.24, 0.235, 0.22, 1.0))
def tilt_slab():
    """Painted precast concrete panels, one 6 m wide to the tile: a sealed joint between
    panels, a reveal groove every 2 m up, the cast-in lifting points, and the dirt each
    groove sheds down the face. A pale neutral paint the building's tint colours; the
    wear is the bare concrete it flakes off."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    joint = smoothstep(0.007, 0.003, np.minimum(xx, 1 - xx))
    below = (yy * 3) % 1.0  # 0 just under a reveal, 1 just above the next
    reveal = smoothstep(0.012, 0.004, np.minimum(below, 1 - below) / 3)
    inserts = smoothstep(0.012, 0.005, np.hypot(np.abs(xx - 0.5) - 0.3, ((yy * 3) % 1.0 - 0.5) / 3))
    shed = smoothstep(0.45, 0.8, fbm((40, 1), 3001, 3)) * np.exp(-below * 2.0 / 0.7) * (1 - reveal)
    grain = fbm(72, 3003, 3)
    tone = 0.9 + 0.2 * (fbm(4, 3005, 4) - 0.5) + 0.06 * (grain - 0.5) - 0.3 * shed
    col = np.broadcast_to(np.array((0.7, 0.7, 0.68)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.1, 0.1, 0.1), np.clip(joint + reveal * 0.7 + inserts * 0.6, 0, 1))
    h = grain * 0.2 - joint * 2.0 - reveal * 1.3 - inserts
    worn = np.clip(chips(3007, 7, bias=0.2) - 0.25 * shed, 0.02, 1.0)
    return Baked(col, worn, normals_from_height(blur(h), 1.2), 1.0 - 0.5 * np.clip(joint + reveal, 0, 1), 0.85 + 0.1 * shed,
                 0.0, 1.0 - np.clip(joint + inserts, 0, 1))


@recipe("roller_slats", tile=1.2, wear=(0.105, 0.046, 0.02, 0.9))
def roller_slats():
    """A roller shutter's curtain, twelve slats to the tile across the opening: a pale
    neutral paint the door's tint colours, dirt in the joints, scuffs down it. The wear is rust."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    g = (yy * 12) % 1.0
    joint = smoothstep(0.14, 0.03, np.minimum(g, 1 - g))
    belly = np.sin(g * math.pi)
    scuff = fbm((30, 2), 3101, 3)
    tone = 0.8 + 0.14 * belly + 0.12 * (scuff - 0.5) + 0.12 * (fbm(4, 3103, 4) - 0.5)
    col = np.broadcast_to(np.array((0.72, 0.72, 0.7)), (SIZE, SIZE, 3)) * tone[..., None]
    col = mix(col, (0.08, 0.08, 0.08), joint * 0.7)
    worn = np.clip(chips(3105, 14, bias=0.12) - 0.25 * joint, 0.02, 1.0)
    return Baked(col, worn, normals_from_height(blur(belly * 1.6 - joint), 1.2), 1.0 - 0.4 * joint, 0.5 + 0.2 * scuff, 0.15,
                 1.0 - 0.6 * joint)


@recipe("factory_glazing", tile=3.0, wear=(0.2, 0.2, 0.19, 1.0))
def factory_glazing():
    """Steel-framed industrial glazing, panes 0.5 m by 0.75 m: dark glossy glass (nothing
    here is see-through), a film of dust thickest at each pane's foot, some panes a
    little duller than their neighbours; grey glazing bars. A window is one module seen
    many times over, so no pane stands out enough to be counted."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    cols, rows = 6, 4
    gx, gy = (xx * cols) % 1.0, (yy * rows) % 1.0
    bar = np.maximum(smoothstep(0.05, 0.028, np.minimum(gx, 1 - gx)), smoothstep(0.034, 0.018, np.minimum(gy, 1 - gy)))
    pane = np.random.default_rng(3201).random((rows, cols))[np.floor(yy * rows).astype(int), np.floor(xx * cols).astype(int)]
    film = np.clip(0.3 * pane + 0.3 * smoothstep(0.5, 1.0, gy) + 0.3 * (fbm(6, 3203, 3) - 0.5), 0, 1)
    col = mix((0.018, 0.024, 0.03), (0.085, 0.09, 0.088), film)
    col = mix(col, (0.27, 0.27, 0.26), bar)
    rough = 0.2 + 0.4 * film
    return Baked(col, 1.0, normals_from_height(blur(bar * 1.2), 1.0), 1.0 - 0.2 * bar, rough + (0.7 - rough) * bar, 0.0, 0.0)


@recipe("grille", tile=1.0, wear=(0.105, 0.046, 0.02, 0.9))
def grille():
    """A steel grille (a window guard, a fence panel): round bars 12.5 cm apart between flat
    rails half a metre apart, in dark paint that rusts where it wears. Its coverage is the
    bars and rails, and nothing between them is there: a cutout's recipe."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    px = 1.0 / SIZE
    bar_r, rail_r = 0.012, 0.02  # half widths, in tiles (metres)
    bx = np.abs((xx * 8) % 1.0 - 0.5) / 8  # to the nearest bar's axis
    ry = np.abs((yy * 2) % 1.0 - 0.5) / 2  # to the nearest rail's
    bar = smoothstep(bar_r + px / 2, bar_r - px / 2, bx)
    rail = smoothstep(rail_r + px / 2, rail_r - px / 2, ry)
    cover = np.maximum(bar, rail)
    round_ = np.sqrt(np.clip(1.0 - (bx / bar_r) ** 2, 0, 1))
    height = np.maximum(round_ * 3.0, rail * 2.0)
    mottle = fbm(12, 3301, 4)
    col = np.broadcast_to(np.array((0.03, 0.036, 0.032)), (SIZE, SIZE, 3)) * (0.85 + 0.3 * mottle)[..., None]
    return Baked(col, chips(3303, 16, bias=0.1), normals_from_height(blur(height), 1.0), 1.0, 0.5 + 0.2 * mottle, 0.0, 0.0,
                 coverage=cover)


@recipe("perforated", tile=0.5, wear=(0.105, 0.046, 0.02, 0.9))
def perforated():
    """Perforated steel sheet: 25 mm round holes staggered 42 mm apart in plate painted a dark
    grey, streaked with dirt. Its coverage is the plate; the holes are not there (a cutout's
    recipe). The plate is flat and darker than what shows through it: a rim lit round each hole
    read as a stud standing off the sheet."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    px = 1.0 / SIZE
    n, hole_r = 12, 0.025  # holes across the tile, and their radius in tiles
    row = np.floor(yy * n)
    gx = (xx * n + 0.5 * (row % 2)) % 1.0 - 0.5
    gy = (yy * n) % 1.0 - 0.5
    d = np.hypot(gx, gy) / n
    cover = smoothstep(hole_r - px / 2, hole_r + px / 2, d)
    streak = fbm((16, 2), 3311, 3)
    col = np.broadcast_to(np.array((0.07, 0.075, 0.08)), (SIZE, SIZE, 3)) * (0.8 + 0.3 * streak)[..., None]
    flat = np.zeros((SIZE, SIZE))
    return Baked(col, chips(3313, 12, bias=0.15), normals_from_height(flat, 1.0), 1.0, 0.5 + 0.2 * streak, 0.0, 0.0,
                 coverage=cover)


# ---------------------------------------------------------------- UVs and the GLB
def box_uv(obj, tile):
    """UVs in metres over `tile` (one number, or one per material slot): each
    face projected along its dominant world axis (the rest pose), so every face
    keeps the recipe's scale on every part and every tier."""
    me = obj.data
    while me.uv_layers:
        me.uv_layers.remove(me.uv_layers[0])
    uv = me.uv_layers.new(name="UVMap")
    mw = obj.matrix_world
    nm = mw.to_3x3().inverted_safe().transposed()
    for poly in me.polygons:
        size = tile if isinstance(tile, (int, float)) else tile.get(poly.material_index, 1.0)
        n = nm @ poly.normal
        ax = max(range(3), key=lambda i: abs(n[i]))
        a, b = {0: (1, 2), 1: (0, 2), 2: (0, 1)}[ax]
        flip = -1.0 if n[ax] < 0 else 1.0
        if ax == 1:  # keep +u running with the right hand on both faces of an axis
            flip = -flip
        for li in poly.loop_indices:
            p = mw @ me.vertices[me.loops[li].vertex_index].co
            uv.data[li].uv = (flip * p[a] / size, p[b] / size)


def material_tiles(obj, textured):
    """Per material slot of `obj`: its recipe's tile ({material name: recipe})."""
    return {i: tile_of(textured[m.name]) for i, m in enumerate(obj.data.materials)
            if m is not None and m.name in textured}


# A textured material's vertex colour is relative to its recipe's mean albedo
# and stored at a third, so it can lighten the texture up to three times (dust, mud, ash,
# rust) as well as shade it; the material says so (extras `colour_scale`).
COLOUR_SCALE = 3.0


def macro(rgb, recipe_name):
    """A vertex colour relative to the recipe's mean albedo, at `COLOUR_SCALE`."""
    mean = baked(recipe_name).mean()
    return tuple(min(1.0, max(0.0, c / max(m, 1e-4) / COLOUR_SCALE)) for c, m in zip(rgb, mean))


# Materials that are not opaque, by name: ("cutout", cutoff) or ("blended",
# opacity). `surface` fills it and `attach` writes it into the export.
COVERAGE = {}
INTERIOR_SHEETS = ("rooms", "shops")


def surface(material, coverage=None, interior=None):
    """Say what a Blender material is beyond an opaque surface with a look of
    its own. Every material helper takes these two and passes them here.

    `coverage` is ("cutout", cutoff): the surface is drawn only where its
    coverage value reaches the cutoff; or ("blended", opacity): it is partly
    there and shows what is behind it. The coverage value is the opacity times
    the recipe's `coverage` image, and never the wear in either alpha.

    `interior` names the interior atlas sheet ("rooms", "shops") the surface
    shows a cell of: a wall of the room box behind a window, opaque and
    untextured, its UVs the cell's (city/README.md, "Interiors")."""
    if coverage is not None:
        kind, value = coverage
        if kind not in ("cutout", "blended") or not 0.0 <= value <= 1.0:
            raise ValueError(f"{material.name}: coverage is ('cutout', cutoff) or ('blended', opacity), 0..1")
        COVERAGE[material.name] = (kind, float(value))
    if interior is not None:
        if interior not in INTERIOR_SHEETS:
            raise ValueError(f"{material.name}: interior sheet is one of {', '.join(INTERIOR_SHEETS)}")
        material["interior"] = interior  # exported as the glTF material's extras
    return material


def attach(path, materials, worn=True):
    """Embed each recipe's images in the GLB at `path` and point the named
    materials' texture slots at them: {material name: recipe name}. The
    material's factors become 1 (the images carry the values); its wear colour
    goes in extras. With `worn` false the surface never wears: the material
    keeps its own factors, so several materials can share one recipe at their
    own base colour, roughness and metalness, and its vertex colour is a plain
    multiplier. A material `surface` marked as a cutout or blended gets its
    glTF alpha mode, with its cutoff or its opacity (the base colour's alpha)."""
    data = open(path, "rb").read()
    jlen = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20:20 + jlen])
    if not any(m.get("name") in materials or m.get("name") in COVERAGE for m in doc.get("materials", [])):
        return []  # an export of plain opaque, untextured materials keeps its bytes
    rest = data[20 + jlen:]
    bin_ = bytearray(rest[8:8 + struct.unpack_from("<I", rest, 0)[0]]) if rest else bytearray()
    views = doc.setdefault("bufferViews", [])
    images = doc.setdefault("images", [])
    textures = doc.setdefault("textures", [])
    doc.setdefault("samplers", [{"magFilter": 9729, "minFilter": 9987, "wrapS": 10497, "wrapT": 10497}])
    slots = {}

    def texture(recipe_name, channel, blob):
        key = (recipe_name, channel)
        if key not in slots:
            while len(bin_) % 4:
                bin_.append(0)
            views.append({"buffer": 0, "byteOffset": len(bin_), "byteLength": len(blob)})
            bin_.extend(blob)
            images.append({"name": f"{recipe_name}_{channel}", "mimeType": "image/png", "bufferView": len(views) - 1})
            textures.append({"sampler": 0, "source": len(images) - 1})
            slots[key] = len(textures) - 1
        return {"index": slots[key]}

    used = set()
    for m in doc.get("materials", []):
        kind, value = COVERAGE.get(m.get("name"), ("opaque", 1.0))
        if kind == "cutout":
            m["alphaMode"] = "MASK"
            m["alphaCutoff"] = value
        elif kind == "blended":
            m["alphaMode"] = "BLEND"
            pbr = m.setdefault("pbrMetallicRoughness", {})
            pbr["baseColorFactor"] = [*pbr.get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])[:3], value]
        name = materials.get(m.get("name"))
        if not name:
            continue
        used.add(m["name"])
        blobs = baked(name).images()
        pbr = m.setdefault("pbrMetallicRoughness", {})
        pbr["baseColorTexture"] = texture(name, "albedo", blobs["albedo"])
        pbr["metallicRoughnessTexture"] = texture(name, "orm", blobs["orm"])
        m["occlusionTexture"] = texture(name, "orm", blobs["orm"])
        m["normalTexture"] = texture(name, "normal", blobs["normal"])
        if worn:
            pbr["metallicFactor"] = 1.0
            pbr["roughnessFactor"] = 1.0
            extras = m.setdefault("extras", {})
            extras["wear"] = [float(x) for x in RECIPES[name][1]]
            extras["colour_scale"] = COLOUR_SCALE
    while len(bin_) % 4:
        bin_.append(0)
    doc["buffers"] = [{"byteLength": len(bin_)}]
    js = json.dumps(doc, separators=(",", ":"), sort_keys=True).encode()
    js += b" " * ((4 - len(js) % 4) % 4)
    total = 12 + 8 + len(js) + 8 + len(bin_)
    out = struct.pack("<III", 0x46546C67, 2, total) + struct.pack("<II", len(js), 0x4E4F534A) + js
    out += struct.pack("<II", len(bin_), 0x004E4942) + bytes(bin_)
    open(path, "wb").write(out)
    return sorted(used)


def swatch(names, path, cols=4):
    """A review sheet of recipes: albedo, normal and ORM side by side (a scratch aid)."""
    rows = []
    for name in names:
        b = baked(name)
        a = _u8(_srgb(b.albedo))
        n = _u8(b.normal * 0.5 + 0.5)
        o = np.stack([_u8(x) for x in b.orm[:3]], -1)
        w = np.repeat(_u8(b.wear)[..., None], 3, -1)
        rows.append(np.concatenate([a, w, n, o], 1))
    img = np.concatenate(rows, 0)
    rgba = np.concatenate([img, np.full(img.shape[:2] + (1,), 255, np.uint8)], -1)
    open(path, "wb").write(png(rgba))
