"""ambientCG material sets baked into our three texture channels.

A set arrives as 1K JPEGs in the local pack cache (`packs.py`, pinned by hash,
CC0). `bake` box-filters it down to the size every texture in the game has
(`textures.SIZE`), burns the grime in, and registers it as a `textures.py`
recipe, so `textures.attach` embeds it like any other: albedo with nothing worn,
the normal map, and occlusion, roughness and metalness in one image.

Nothing from a set is committed; the baked PNGs live inside the exported GLB.
"""
import os
import sys

import bpy
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import packs  # noqa: E402
import textures  # noqa: E402


def _pixels(path):
    """An image's bytes as floats, top row first, undisturbed by colour management."""
    image = bpy.data.images.load(path, check_existing=False)
    image.colorspace_settings.name = "Non-Color"
    w, h = image.size
    raw = np.empty(w * h * 4, dtype=np.float32)
    image.pixels.foreach_get(raw)
    bpy.data.images.remove(image)
    return raw.reshape(h, w, 4)[::-1, :, :3].astype(np.float64)


def _fit(image, repeat):
    """Box-filter to `SIZE / repeat` and tile it `repeat` times each way. A set that
    is not square (a plank floor twice as wide as it is long) is first repeated along
    its short side until it is."""
    h, w = image.shape[:2]
    if max(h, w) % min(h, w):
        raise SystemExit(f"an ambientCG image is {w}x{h}: its sides are not a whole multiple")
    image = np.tile(image, (max(h, w) // h, max(h, w) // w, 1))
    edge = textures.SIZE // repeat
    k = image.shape[0] // edge
    if k * edge != image.shape[0]:
        raise SystemExit(f"an ambientCG image is {w}x{h}; expected a multiple of {edge}")
    small = image.reshape(edge, k, edge, k, -1).mean((1, 3))
    return np.tile(small, (repeat, repeat, 1))


def _linear(srgb):
    return np.where(srgb <= 0.04045, srgb / 12.92, ((srgb + 0.055) / 1.055) ** 2.4)


def bake(name, material, tile_m, repeat=1, tint=(1.0, 1.0, 1.0), rough=(1.0, 0.0), normal=1.0, grime=None):
    """Register recipe `name`: ambientCG set `material`, whose image spans `tile_m`
    metres, repeated `repeat` times so the recipe's tile is `repeat * tile_m`.
    `tint` multiplies the colour, `rough` is (scale, offset) on its roughness and
    `normal` scales its bumps. `grime(albedo, roughness)` returns both with the
    dirt burnt in; it sees the whole tile, so its marks repeat only at the tile."""

    def build():
        channel = lambda key: packs.ambientcg(material, key)
        albedo = _fit(_linear(_pixels(channel("Color"))), repeat) * np.asarray(tint)
        roughness = np.clip(_fit(_pixels(channel("Roughness")), repeat)[..., 0] * rough[0] + rough[1], 0.0, 1.0)
        n = _fit(_pixels(channel("NormalGL")), repeat) * 2.0 - 1.0
        n[..., :2] *= normal
        n /= np.linalg.norm(n, axis=-1, keepdims=True)
        ao, metal = channel("AmbientOcclusion"), channel("Metalness")
        occlusion = _fit(_pixels(ao), repeat)[..., 0] if ao else 1.0
        metalness = _fit(_pixels(metal), repeat)[..., 0] if metal else 0.0
        if grime:
            albedo, roughness = grime(albedo, roughness)
        return textures.Baked(albedo, 1.0, n, occlusion, roughness, metalness)

    textures.recipe(name, tile=tile_m * repeat)(build)
