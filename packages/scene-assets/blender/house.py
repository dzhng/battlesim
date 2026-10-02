"""A village farmstead sized to a building prop's box: intact, or its ruin.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/house.py -- <out.glb> <hx> <hy> <hz> <variant> [--ruin RUIN_HEIGHT]

The simulation's building is one box (`map.buildings[].geometry.parts[].half_extents`), which blocks
movers and sight and holds a garrison. The art fills that box as a courtyard
farm, the common village house of the region: a two-storey dwelling along one
long side, a barn along the other, a stable wing joining them, and a yard wall
with a gate closing the square. Every outer wall stands on a face of the box, so
what hides a unit in the simulation is what hides it on screen. Roofs ridge just
under the box's top; chimneys and eaves overhang it slightly (the catalog's
`footprint_m` tolerance).

With --ruin the same plan is a burnt shell: walls broken to a ragged top no
higher than about the rule's `buildings.ruin_height_m`, rubble mounds, charred
beams. Origin at the box's centre on the ground, +X along `hx`.
"""
import bpy, bmesh, sys, os, math, json, random
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *
from masonry import *

ARGS = [a for a in script_args() if a != "--"]
RUIN = None
if "--ruin" in ARGS:
    i = ARGS.index("--ruin")
    RUIN = float(ARGS[i + 1])
    del ARGS[i:i + 2]
POS = ARGS
OUT = POS[0]
HX, HY, HZ = (float(v) for v in POS[1:4])
VARIANT = int(POS[4]) if len(POS) > 4 else 0
H = 2 * HZ

reset()
rng = random.Random(1000 + VARIANT)

PLASTERS = [(0.55, 0.46, 0.31), (0.6, 0.57, 0.5), (0.55, 0.47, 0.43)]
TILES = [(0.3, 0.11, 0.06), (0.24, 0.1, 0.07), (0.33, 0.14, 0.08)]
wall_m = plaster("plaster", PLASTERS[VARIANT % 3], seed=1.0 + VARIANT, burnt_=RUIN is not None)
barn_m = plaster("barn_plaster", tuple(c * 0.92 for c in PLASTERS[(VARIANT + 1) % 3]), seed=3.0 + VARIANT,
                 burnt_=RUIN is not None)
roof_m = tiles("roof_tiles", TILES[VARIANT % 3] if RUIN is None else tuple(c * 0.45 for c in TILES[VARIANT % 3]),
               seed=7.0 + VARIANT)
brick_m = brick("brick", seed=3.0 + VARIANT, burnt_=RUIN is not None)
stone_m = stone("stone")
wood_m = timber("timber", seed=9.0 + VARIANT)
green_m = timber("shutters", (0.06, 0.1, 0.07), seed=12.0 + VARIANT)
frame_m = flat_paint("window_frame", (0.55, 0.53, 0.48), rough=0.7)
glass_m = flat_paint("window_glass", (0.015, 0.02, 0.025), rough=0.1, grime=0.0)
metal_m = flat_paint("gutter", (0.2, 0.2, 0.19), rough=0.5, metal=0.6)
char_m = charred("charred")
rubble_m = rubble_paint("rubble", seed=13.0 + VARIANT)

root = empty("farmstead")

D = min(7.6, HY * 0.62)  # wing depth
DS = min(6.0, HX * 0.4)  # stable wing depth
X0, X1, Y0, Y1 = -HX, HX, -HY, HY
RIDGE = H - 0.25
T = 0.45  # wall thickness (ruins)


def windows_row(prefix, x_from, x_to, y_face, facing_y, z, w, h, spacing, shutters):
    n = max(1, int((x_to - x_from) / spacing))
    for i in range(n):
        x = x_from + (i + 0.5) * (x_to - x_from) / n
        window(f"{prefix}_{i}", (x, y_face + facing_y * 0.02, z), (0, facing_y), w, h, frame_m, glass_m,
               green_m if shutters else None, root, stone_m)


def windows_col(prefix, y_from, y_to, x_face, facing_x, z, w, h, spacing):
    n = max(1, int((y_to - y_from) / spacing))
    for i in range(n):
        y = y_from + (i + 0.5) * (y_to - y_from) / n
        window(f"{prefix}_{i}", (x_face + facing_x * 0.02, y, z), (facing_x, 0), w, h, frame_m, glass_m, None, root,
               stone_m)


def intact():
    # dwelling: south wing, two storeys
    e1 = min(5.8, RIDGE - 2.2)
    wing("dwelling", X0, X1, Y0, Y0 + D, e1, RIDGE, wall_m, roof_m, root, "x")
    box("dwelling_plinth", (X1 - X0 + 0.08, D + 0.08, 0.6), (0, Y0 + D / 2, 0.3), stone_m, root, bevel=0.03, lods=(0, 1, 2))
    box("dwelling_band", (X1 - X0 + 0.06, D + 0.06, 0.12), (0, Y0 + D / 2, 3.05), frame_m, root, lods=(0, 1))
    windows_row("dw_out_lo", X0 + 1.0, X1 - 1.0, Y0, -1, 1.7, 1.0, 1.35, 2.9, True)
    windows_row("dw_out_hi", X0 + 1.0, X1 - 1.0, Y0, -1, 4.3, 1.0, 1.25, 2.9, True)
    windows_row("dw_in_hi", X0 + DS + 1.0, X1 - 1.0, Y0 + D, 1, 4.3, 0.9, 1.2, 3.3, False)
    windows_col("dw_gable", Y0 + 1.2, Y0 + D - 1.2, X1, 1, 4.3, 0.9, 1.2, 2.6)
    windows_col("dw_gable_top", Y0 + D / 2 - 0.6, Y0 + D / 2 + 0.6, X1, 1, e1 + 0.9, 0.6, 0.8, 1.2)
    plank_door("dw_door", (X1 - 5.0, Y0 + D + 0.02, 0.0), (0, 1), 1.1, 2.2, wood_m, root)
    # the street door, between two bays of the ground-floor windows
    bays = max(1, int((X1 - X0 - 2.0) / 2.9))
    xd = X0 + 1.0 + (X1 - X0 - 2.0) / bays * (bays // 2)
    plank_door("dw_street_door", (xd, Y0 - 0.02, 0.0), (0, -1), 1.2, 2.3, wood_m, root)
    box("dw_street_step", (1.8, 0.4, 0.2), (xd, Y0 - 0.2, 0.1), stone_m, root, bevel=0.02, lods=(0, 1))
    box("dw_street_lintel", (1.6, 0.14, 0.2), (xd, Y0 - 0.05, 2.42), stone_m, root, bevel=0.02, lods=(0, 1))
    box("dw_step", (1.6, 0.6, 0.18), (X1 - 5.0, Y0 + D + 0.3, 0.09), stone_m, root, bevel=0.02, lods=(0, 1))
    for k, fx in enumerate((-0.42, 0.38)):
        cx = fx * HX
        box(f"chimney_{k}", (0.7, 0.55, 2.2), (cx, Y0 + D / 2 + 0.3, RIDGE - 0.45), brick_m, root, bevel=0.02)
        box(f"chimney_cap_{k}", (0.85, 0.7, 0.1), (cx, Y0 + D / 2 + 0.3, RIDGE + 0.7), stone_m, root, lods=(0, 1))
    # gutters and downpipes on the dwelling
    for s, y in ((-1, Y0 - 0.3), (1, Y0 + D + 0.3)):
        cyl(f"gutter_{'ab'[s > 0]}", 0.07, X1 - X0 + 0.4, (0, y, e1 - 0.35 * math.tan(math.atan2(RIDGE - e1, D / 2)) - 0.05),
            "X", metal_m, root, seg=10, lods=(0, 1))
        for k, x in enumerate((X0 + 0.25, X1 - 0.25)):
            cyl(f"downpipe_{'ab'[s > 0]}_{k}", 0.05, e1 - 0.2, (x, y - s * 0.2, (e1 - 0.2) / 2), "Z", metal_m, root, seg=8,
                lods=(0,))

    # barn: north wing, one tall storey with a hay loft
    e2 = min(4.6, RIDGE - 3.0)
    wing("barn", X0, X1, Y1 - D, Y1, e2, RIDGE, barn_m, roof_m, root, "x")
    box("barn_plinth", (X1 - X0 + 0.08, D + 0.08, 0.5), (0, Y1 - D / 2, 0.25), stone_m, root, bevel=0.03, lods=(0, 1, 2))
    for k, x in enumerate((-HX * 0.35, HX * 0.3)):
        plank_door(f"barn_door_{k}", (x, Y1 - D - 0.02, 0.0), (0, -1), 3.4, 3.6, wood_m, root)
        box(f"barn_lintel_{k}", (3.8, 0.3, 0.3), (x, Y1 - D - 0.08, 3.75), wood_m, root, bevel=0.02, lods=(0, 1, 2))
    windows_row("barn_out", X0 + 2.0, X1 - 2.0, Y1, 1, 3.2, 0.7, 0.5, 4.5, False)
    # timber framing on the barn's gable ends
    for k, x in enumerate((X0 - 0.03, X1 + 0.03)):
        for j in range(5):
            y = Y1 - D + (j + 0.5) * D / 5
            zt = e2 + (RIDGE - e2) * (1 - abs(y - (Y1 - D / 2)) / (D / 2)) - 0.1
            box(f"barn_post_{k}_{j}", (0.08, 0.18, zt - e2), (x, y, (e2 + zt) / 2), wood_m, root, lods=(0, 1))
        box(f"barn_beam_{k}", (0.1, D - 0.2, 0.2), (x, Y1 - D / 2, e2 + 0.05), wood_m, root, lods=(0, 1))

    # stable: west wing joining them, lower
    e3 = min(3.4, RIDGE - 3.4)
    r3 = min(RIDGE - 1.0, e3 + DS * 0.5)
    wing("stable", X0, X0 + DS, Y0 + D, Y1 - D, e3, r3, wall_m, roof_m, root, "y", gables=(False, False))
    windows_col("st_out", Y0 + D + 0.8, Y1 - D - 0.8, X0, -1, 2.3, 0.6, 0.45, 2.4)
    for k in range(max(1, int((Y1 - Y0 - 2 * D) / 5))):
        y = Y0 + D + (k + 0.5) * (Y1 - Y0 - 2 * D) / max(1, int((Y1 - Y0 - 2 * D) / 5))
        plank_door(f"st_door_{k}", (X0 + DS + 0.02, y, 0.0), (1, 0), 1.2, 2.1, wood_m, root)

    # yard wall with a gate on the east
    ylo, yhi = Y0 + D, Y1 - D
    gate = (ylo + yhi) / 2
    for k, (a, b) in enumerate(((ylo, gate - 1.9), (gate + 1.9, yhi))):
        box(f"yard_wall_{k}", (0.5, b - a, 2.3), (X1 - 0.25, (a + b) / 2, 1.15), wall_m, root, bevel=0.03)
        box(f"yard_cope_{k}", (0.62, b - a, 0.14), (X1 - 0.25, (a + b) / 2, 2.35), roof_m, root, bevel=0.02, lods=(0, 1, 2))
    for k, y in enumerate((gate - 1.9, gate + 1.9)):
        box(f"gate_pier_{k}", (0.7, 0.7, 2.8), (X1 - 0.35, y, 1.4), stone_m, root, bevel=0.04)
        box(f"gate_pier_cap_{k}", (0.8, 0.8, 0.16), (X1 - 0.35, y, 2.88), stone_m, root, bevel=0.03, lods=(0, 1, 2))
    for k, s in enumerate((-1, 1)):
        plank_door(f"gate_leaf_{k}", (X1 - 0.3, gate + s * 0.8, 0.1), (1, 0), 1.5, 2.1, wood_m, root)
    # a few things in the yard
    box("yard_cart_bed", (2.4, 1.3, 0.12), (X1 - 6.0, gate + 2.0, 0.8), wood_m, root, bevel=0.02, rot=(0, 0, 0.3), lods=(0, 1))
    for k, dy in enumerate((-0.7, 0.7)):
        cyl(f"yard_cart_wheel_{k}", 0.55, 0.08, (X1 - 6.0 + 0.2, gate + 2.0 + dy, 0.55), "Y", wood_m, root, seg=16,
            rot=(0, 0, 0.3), lods=(0, 1))
    rubble("woodpile", (X0 + DS + 1.2, Y1 - D - 1.3), 1.0, 0.9, 14, wood_m, 5 + VARIANT, root, lods=(0, 1))


def ragged(name, a0, a1, fixed, along_x, lo, hi, seed, mat, gaps=(), windows=0.0):
    """One of the farmstead's broken walls (`masonry.ragged_wall`), `T` thick."""
    ragged_wall(name, a0, a1, fixed, along_x, lo, hi, seed, mat, root, T, gaps, windows)


def ruin(height):
    top = height + 0.35
    ragged("rw_south", X0, X1, Y0 + T / 2, True, 0.5, top, 1.0, wall_m, [(-HX * 0.5, 1.3), (HX * 0.15, 1.2)], 3.1)
    ragged("rw_north", X0, X1, Y1 - T / 2, True, 0.6, top, 2.0, barn_m, [(HX * 0.3, 3.0)], 4.5)
    ragged("rw_west", Y0 + T, Y1 - T, X0 + T / 2, False, 0.4, top, 3.0, wall_m, [(0.0, 1.4)], 2.6)
    ragged("rw_east", Y0 + T, Y1 - T, X1 - T / 2, False, 0.3, top * 0.95, 4.0, wall_m, [((Y0 + Y1) / 2, 3.6)])
    ragged("rw_dw_inner", X0 + DS, X1 - 1.0, Y0 + D, True, 0.3, height * 0.8, 5.0, wall_m, [(X1 - 5.0, 1.4)])
    ragged("rw_barn_inner", X0 + DS, X1 - 1.0, Y1 - D, True, 0.2, height * 0.7, 6.0, barn_m, [(-HX * 0.35, 3.4)])
    ragged("rw_stable_inner", Y0 + D, Y1 - D, X0 + DS, False, 0.2, height * 0.6, 7.0, wall_m)
    box("rw_plinth_s", (X1 - X0, T + 0.08, 0.5), (0, Y0 + T / 2, 0.25), stone_m, root, lods=(0, 1, 2))
    box("rw_plinth_n", (X1 - X0, T + 0.08, 0.5), (0, Y1 - T / 2, 0.25), stone_m, root, lods=(0, 1, 2))
    # chimney stumps
    for k, fx in enumerate((-0.42, 0.38)):
        box(f"chimney_stump_{k}", (0.7, 0.55, height + 0.1), (fx * HX, Y0 + D / 2 + 0.3, (height + 0.1) / 2), brick_m, root,
            bevel=0.02)
    # rubble filling the wings
    heaps = [((-HX * 0.5, Y0 + D * 0.5), 3.2, 1.3), ((HX * 0.1, Y0 + D * 0.45), 3.6, 1.4), ((HX * 0.65, Y0 + D * 0.5), 2.6, 1.1),
             ((-HX * 0.3, Y1 - D * 0.5), 3.4, 1.2), ((HX * 0.45, Y1 - D * 0.5), 3.0, 1.0),
             ((X0 + DS * 0.5, 0.0), 2.4, 0.9), ((HX * 0.2, (Y0 + Y1) / 2), 2.0, 0.5)]
    for k, (c, r, h) in enumerate(heaps):
        # keep each heap's wobbling rim inside the box
        r = min(r, min(c[0] - X0, X1 - c[0], c[1] - Y0, Y1 - c[1]) / 1.32)
        rubble(f"heap_{k}", c, r, min(h, height * 0.7), 60, rubble_m, 31 * k + VARIANT, root)
    # charred beams fallen across the heaps
    for k in range(10):
        x = rng.uniform(X0 + 3.5, X1 - 3.5)
        y = rng.choice((Y0 + D * rng.uniform(0.35, 0.65), Y1 - D * rng.uniform(0.35, 0.65)))
        box(f"beam_{k}", (rng.uniform(2.5, 5.0), 0.2, 0.22), (x, y, rng.uniform(0.5, min(1.2, height * 0.6))), char_m, root,
            rot=(rng.uniform(-0.2, 0.2), rng.uniform(-0.35, 0.35), rng.uniform(-0.5, 0.5)), bevel=0.02, lods=(0, 1, 2))
    # scorched floors inside the wings
    for k, (x0, x1, y0, y1) in enumerate(((X0, X1, Y0, Y0 + D), (X0, X1, Y1 - D, Y1), (X0, X0 + DS, Y0 + D, Y1 - D))):
        box(f"scorch_{k}", (x1 - x0 - 0.9, y1 - y0 - 0.9, 0.04), ((x0 + x1) / 2, (y0 + y1) / 2, 0.02), char_m, root,
            lods=(0, 1, 2))
    # a fallen roof: tiles slipped over the rubble
    for k in range(4):
        x = rng.uniform(X0 + 3, X1 - 3)
        y = Y0 + D * 0.5 if k % 2 == 0 else Y1 - D * 0.5
        box(f"roof_slab_{k}", (rng.uniform(2.0, 3.5), rng.uniform(1.5, 2.5), 0.12), (x, y, 0.9), char_m if k % 2 else roof_m, root,
            rot=(rng.uniform(-0.4, 0.4), rng.uniform(-0.3, 0.3), rng.uniform(0, 3)), lods=(0, 1))


if RUIN is None:
    intact()
else:
    ruin(RUIN)

rest_on_ground()
finish(ao_distance=3.0, ao_strength=0.55, ao_rays=10, paint_scale=3.0)
info = dict(tris=triangles_by_tier(), box=[HX, HY, HZ], ruin=RUIN)
print("HOUSE", json.dumps(info))
export(OUT)
