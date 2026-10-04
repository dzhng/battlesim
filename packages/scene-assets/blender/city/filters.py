"""Texture filters the graph sets burn into ambientCG sets (`ambientcg.bake`'s `grime`), and the
cutout bars their railings and cages are sheets of (`graphset.GraphSet.sheeted`)."""
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import textures  # noqa: E402


def grime(streak, blotch, colour=(0.2, 0.18, 0.15)):
    """Dirt burnt into a wall texture: damp blotches, and rain streaks down it."""

    def burn(albedo, roughness):
        blotches = textures.smoothstep(0.45, 0.85, textures.fbm(3, 911, 4))
        runs = textures.smoothstep(0.55, 0.9, textures.fbm((40, 2), 913, 3)) * (0.5 + 0.5 * textures.fbm(5, 915, 3))
        dirt = np.clip(blotch * blotches + streak * runs, 0.0, 0.9)
        return textures.mix(albedo, np.asarray(colour) * albedo.mean((0, 1)) / max(albedo.mean(), 1e-6), dirt), \
            np.clip(roughness + 0.25 * dirt, 0.0, 1.0)

    return burn


def toned(mean, keep, level_cycles=0):
    """A set brought to a colour of our own: its hue kept by `keep` (0 is grey) and its
    mean albedo set to `mean` (linear). `level_cycles` levels its brightness over
    anything longer than that many cycles to the tile: a set lighter at one corner
    than another, repeated, is a chequer from the air. Nothing is added that would
    repeat: a surface this recipe covers by the hundred square metres gets its dirt
    from its mesh."""

    def burn(albedo, roughness):
        grey = (albedo @ np.array([0.2126, 0.7152, 0.0722]))[..., None]
        if level_cycles:
            spectrum = np.fft.fft2(grey[..., 0])
            fy, fx = np.meshgrid(np.fft.fftfreq(grey.shape[0]) * grey.shape[0], np.fft.fftfreq(grey.shape[1]) * grey.shape[1], indexing="ij")
            slow = np.hypot(fx, fy) <= level_cycles
            slow[0, 0] = False
            level = np.fft.ifft2(np.where(slow, spectrum, 0)).real[..., None]
            albedo = albedo * np.clip((grey - level) / np.maximum(grey, 1e-6), 0.0, 4.0)
            grey = (albedo @ np.array([0.2126, 0.7152, 0.0722]))[..., None]
        out = grey + (albedo - grey) * keep
        return np.clip(out * (np.asarray(mean) / out.mean((0, 1))), 0.0, 1.0), roughness

    return burn


def scorched(albedo, roughness):
    """A wall after a fire: smoked grey all over, a little uneven, with a few patches
    where the face has spalled to the render under it. The marks that say fire are the
    fans above the openings, which are geometry: clouds of soot in a texture repeat as
    camouflage, and runs down a brown wall are the grain of a plank. Only the colour
    changes, so the burnt wall shares the intact wall's normal and roughness images."""
    soot = textures.smoothstep(0.25, 0.8, textures.fbm(1, 931, 3))
    spall = textures.smoothstep(0.86, 0.9, textures.fbm(5, 941, 4))
    out = textures.mix(albedo, (0.03, 0.028, 0.026), np.clip(0.52 + 0.18 * soot, 0.0, 0.94))
    render = np.array((0.085, 0.08, 0.075)) * (0.7 + 0.5 * textures.fbm(40, 943, 2))[..., None]
    return textures.mix(out, render, spall * 0.8), roughness


def bars():
    """A cage's or a railing's bars: round bars 12.5 cm apart between flat rails half a
    metre apart, as the shared `grille` recipe has them, but in no colour of its own
    (white: each shows its own paint or metal) and over half a metre, so a bar is twelve
    texels across. At the shared recipe's one metre a bar seen from 30 m had a saw's edge."""
    size = textures.SIZE
    yy, xx = np.mgrid[0:size, 0:size].astype(float) / size
    px = 1.0 / size
    bar_r, rail_r = 0.012 / 0.5, 0.02 / 0.5  # half widths, in tiles
    bx = np.abs((xx * 4) % 1.0 - 0.5) / 4  # to the nearest bar's axis
    ry = np.abs(yy % 1.0 - 0.5)  # to the rail's
    bar = textures.smoothstep(bar_r + px / 2, bar_r - px / 2, bx)
    rail = textures.smoothstep(rail_r + px / 2, rail_r - px / 2, ry)
    height = np.maximum(np.sqrt(np.clip(1.0 - (bx / bar_r) ** 2, 0, 1)) * 3.0, rail * 2.0)
    ones = np.ones((size, size))
    return textures.Baked(np.ones((size, size, 3)), 1.0, textures.normals_from_height(textures.blur(height), 1.0), 1.0, ones, ones,
                          coverage=np.maximum(bar, rail))
