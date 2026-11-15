"""The armies' uniform prints, `textures` recipes like every other: each
army's camouflage on nylon-cotton ripstop, and `UNIFORMS`, which names the
print an army's uniform, carrier, pouches and helmet cover wear. `textures`
imports this module, which registers them; `textures.UNIFORMS` is this one.
"""
import math

import numpy as np

from textures import SIZE, Baked, fbm, folds, mix, normals_from_height, recipe, smoothstep, warp, weave, worley


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
    return _ripstop(col, 713)


def _ripstop(col, seed):
    """Finish a printed camouflage as nylon-cotton ripstop: faded print, the
    ripstop grid, weave and seam folds in the normal. -> Baked."""
    fade = fbm(4, seed, 4)
    col = col * (0.9 + 0.2 * fade)[..., None]
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float)
    grid = np.maximum(smoothstep(0.8, 1.0, np.cos(xx / SIZE * 64 * math.pi * 2)),
                      smoothstep(0.8, 1.0, np.cos(yy / SIZE * 64 * math.pi * 2)))
    w = weave(128, seed + 2, 0.4)
    h = w * 0.08 + grid * 0.1 + folds(seed + 4, 2) * 2.5
    col = col * (0.97 + 0.05 * grid)[..., None]
    # cloth is matte: its folds and weave occlude the sky, so it takes little sheen
    return Baked(col, 0.25 + 0.75 * fbm(8, seed + 6, 4), normals_from_height(h, 1.0), 0.72 + 0.1 * folds(seed + 4, 2), 0.97)


# An army's uniform print (`UNIFORMS`): the recipe its uniform, carrier,
# pouches and helmet cover are printed in. Prints stay in the dark,
# desaturated range the battle's sun needs, and each reads by its mean at
# battle distance: OCP sandy khaki (it matches the US tan vehicles), MARPAT
# woodland green-brown, Flecktarn yellow-green, EMR grey-green.
UNIFORMS = {
    "us_army_ocp": "ocp_ripstop",
    "usmc_marpat": "marpat_ripstop",
    "german_flecktarn": "flecktarn_ripstop",
    "eastern_emr": "emr_ripstop",
}


def _pixels(block, seed, f_cells, g_cells):
    """A digital print's two clustered fields, in square pixels `block`
    texels across: `f` places the main tones, `g` the accents; each pixel
    jittered so the clusters' edges break up."""
    cell = lambda x: x[::block, ::block].repeat(block, 0).repeat(block, 1)
    jitter = cell(np.random.default_rng(seed).random((SIZE, SIZE)))
    f = cell(fbm(f_cells, seed + 2, 4)) + 0.1 * (jitter - 0.5)
    g = cell(fbm(g_cells, seed + 4, 3)) + 0.1 * (jitter - 0.5)
    return f, g


@recipe("ocp_ripstop", tile=0.42, wear=(0.2, 0.17, 0.12, 1.0))
def ocp_ripstop():
    """US Army OCP (Scorpion W2) on ripstop: a sandy khaki ground under
    smaller, softer blotches than multicam's of pale sand, olive, brown and a
    little dark brown, with no branches."""
    return _ripstop(_camo_print([(0.165, 0.145, 0.1), ((0.235, 0.21, 0.15), 0.68), ((0.08, 0.088, 0.052), 0.6),
                                 ((0.12, 0.088, 0.055), 0.64), ((0.06, 0.048, 0.034), 0.77)], 2701, 8), 2713)


@recipe("emr_ripstop", tile=0.42, wear=(0.15, 0.16, 0.11, 1.0))
def emr_ripstop():
    """Russian EMR (Digital Flora) on ripstop: square pixels about 7 mm
    across, a pale grey-green ground under clustered light-olive, dark-green
    and brown pixels and a few near-black ones."""
    f, g = _pixels(4, 2801, 9, 13)  # 64 pixels across the 0.42 m tile
    tones = np.array(((0.13, 0.14, 0.088), (0.078, 0.098, 0.052), (0.036, 0.052, 0.03), (0.07, 0.052, 0.034),
                      (0.02, 0.022, 0.017)))
    tone = np.where(f < 0.42, 0, np.where(f < 0.55, 1, 2))
    tone = np.where((g > 0.6) & (tone > 0), 3, tone)
    tone = np.where((g < 0.3) & (f > 0.6), 4, tone)
    return _ripstop(tones[tone], 2813)


@recipe("marpat_ripstop", tile=0.42, wear=(0.17, 0.155, 0.11, 1.0))
def marpat_ripstop():
    """USMC MARPAT woodland on ripstop: pixels about 3 mm across, finer than
    EMR's, in broad clustered blotches of olive green and brown over a
    khaki ground, with black pixels along the blotches' cores."""
    f, g = _pixels(2, 2901, 7, 9)
    tones = np.array(((0.15, 0.135, 0.095), (0.068, 0.082, 0.046), (0.078, 0.056, 0.036), (0.022, 0.022, 0.019)))
    tone = np.where(f < 0.36, 0, np.where(g < 0.5, 1, 2))
    tone = np.where((f > 0.66) & (np.abs(g - 0.5) < 0.12), 3, tone)
    return _ripstop(tones[tone], 2913)


@recipe("flecktarn_ripstop", tile=0.42, wear=(0.15, 0.16, 0.1, 1.0))
def flecktarn_ripstop():
    """German Flecktarn on ripstop: a pale yellow-green ground under round
    dots about 1 to 2 cm across, gathered in drifts, of olive, dark green,
    red-brown and a few black, each colour's drifts placed apart."""
    col = np.broadcast_to(np.array((0.125, 0.135, 0.078)), (SIZE, SIZE, 3)).copy()
    for k, (colour, level, cells, radius) in enumerate((
            ((0.075, 0.088, 0.044), 0.52, 26, 0.44), ((0.032, 0.05, 0.024), 0.56, 30, 0.42),
            ((0.078, 0.046, 0.028), 0.62, 34, 0.4), ((0.018, 0.018, 0.015), 0.72, 40, 0.34))):
        seed = 3001 + 11 * k
        drift = warp(fbm(5, seed, 4), 6, seed + 1)
        f1, _, ident = worley(cells, seed + 3)
        chance = np.random.default_rng(seed + 5).random(cells * cells)[ident]
        size = radius * (0.7 + 0.5 * np.random.default_rng(seed + 7).random(cells * cells)[ident])
        present = chance < smoothstep(level - 0.12, level + 0.12, drift)
        col = mix(col, colour, present * smoothstep(size + 0.04, size - 0.04, f1))
    return _ripstop(col, 3053)
