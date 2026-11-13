"""The military props' recipes (`military.py`), `textures` recipes like every other:
the HESCO liner and the camouflage net. `military_parts` imports this module, which
registers them."""
import numpy as np

from textures import SIZE, Baked, blur, fbm, mix, normals_from_height, recipe, smoothstep, warp, worley


@recipe("hesco_liner", tile=0.6, wear=(0.12, 0.1, 0.075, 1.0))
def hesco_liner():
    """A HESCO cell's face: a sand-coloured non-woven geotextile liner pressed out against
    the galvanised weld mesh round it, 75 mm squares, eight to the tile. The felt pillows a
    little in each square and the wire shades it. The wear is mud."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    n, px, r = 8, 1.0 / SIZE, 0.006  # squares across the tile; a texel; the wire's half width, in tiles
    fx, fy = (xx * n) % 1.0, (yy * n) % 1.0
    d = np.minimum(np.minimum(fx, 1 - fx), np.minimum(fy, 1 - fy)) / n
    wire = smoothstep(r + px, r - px, d)
    pillow = np.sin(np.pi * fx) * np.sin(np.pi * fy)
    shade = smoothstep(0.02, 0.004, d) * (1 - wire)
    fibre = fbm(96, 6401, 3)
    stain = fbm(4, 6403, 4)
    col = np.broadcast_to(np.array((0.36, 0.31, 0.22)), (SIZE, SIZE, 3)) * (0.86 + 0.14 * fibre + 0.08 * pillow)[..., None]
    col = mix(col, (0.2, 0.16, 0.11), smoothstep(0.5, 0.85, stain) * 0.45)
    col = mix(col, (0.13, 0.11, 0.08), shade * 0.5)
    col = mix(col, (0.33, 0.34, 0.34), wire)
    h = pillow * 0.8 + fibre * 0.2 + wire * 1.6
    return Baked(col, 0.25 + 0.75 * fbm(8, 6405, 4), normals_from_height(blur(h), 2.0), 0.85 + 0.15 * pillow - 0.2 * shade,
                 0.9 - 0.45 * wire, 0.6 * wire, 0.0)


@recipe("camo_net", tile=1.2, wear=(0.1, 0.09, 0.06, 1.0))
def camo_net():
    """A woodland camouflage net: a knotted cord net, 10 cm meshes, under garnish of cut
    scrim in three greens and browns, slit into leaves that curl off the net. Its coverage is
    the scrim and the cord and the holes between the leaves: a cutout's recipe. The wear is
    sun-bleached scrim."""
    yy, xx = np.mgrid[0:SIZE, 0:SIZE].astype(float) / SIZE
    px = 1.0 / SIZE
    n = 12  # meshes across the tile
    a = np.abs((xx + yy) * n / 2 % 1.0 - 0.5) / n * 2
    b = np.abs((xx - yy) * n / 2 % 1.0 - 0.5) / n * 2
    cord = smoothstep(0.006 + px, 0.006 - px, np.minimum(a, b))
    f1, f2, cell = worley(14, 6501)
    leaf = smoothstep(0.008, 0.025, f2 - f1)  # each leaf a cell of the scrim, slit at its edges
    holes = smoothstep(0.68, 0.74, warp(fbm(16, 6503, 3), 8, 6505))  # where leaves are cut away
    scrim = leaf * (1 - holes)
    tone = (cell * 7.31) % 1.0
    green, olive, brown = np.array((0.045, 0.06, 0.03)), np.array((0.09, 0.095, 0.05)), np.array((0.08, 0.06, 0.035))
    col = np.where((tone < 0.45)[..., None], green, np.where((tone < 0.78)[..., None], olive, brown))
    col = col * (0.85 + 0.3 * fbm(40, 6507, 2))[..., None]
    col = mix(col, (0.05, 0.045, 0.035), cord * (1 - scrim))
    curl = blur(scrim, 2)
    h = curl * 1.5 + cord * 0.6
    return Baked(col, 0.3 + 0.7 * fbm(6, 6509, 4), normals_from_height(h, 1.0), 0.75 + 0.25 * curl, 0.95, 0.0, 0.0,
                 coverage=np.clip(scrim + cord, 0, 1))
