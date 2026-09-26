"""Village props, each authored to one simulation box (the catalog's `footprint_half_m`).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/props.py <kind> <out.glb>

kind:
  wall        a rubble-stone field wall with a mortared coping; box [2, 0.3, 0.8] (4 m module)
  crate       a stack of wooden ammunition crates on a pallet; box [1, 1, 1]
  bridge_deck a concrete road deck on two girders, with kerbs; box [18, 5, 0.4] (geometry lab's bridge)

The battle fits each placed box from the authored one (slice 24). Origin at the
box's centre on the ground, +X along its first half extent.
"""
import bpy, bmesh, sys, os, math, json, random
from mathutils import Vector, Matrix

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *
from masonry import *

ARGS = [a for a in script_args() if not a.startswith("--")]
KIND, OUT = ARGS[0], ARGS[1]
reset()
root = empty(KIND)
rng = random.Random(7)


def wall():
    hx, hy, hz = 2.0, 0.3, 0.8
    stone_m = stone("field_stone", (0.3, 0.28, 0.24), seed=2.0)
    mortar_m = flat_paint("mortar", (0.36, 0.34, 0.3), rough=0.95)
    # the core, slightly battered (narrower at the top)
    box("wall_core", (2 * hx, 2 * hy - 0.08, 2 * hz - 0.2), (0, 0, hz - 0.1), mortar_m, root, taper=(1.0, 0.8))

    # rubble stones laid in rough courses on both faces and round the ends: each an
    # irregular block, some proud of the face, with mortar showing between them
    def stones(bm, lod):
        if lod >= 2:
            return False
        rows = 6
        for r in range(rows):
            z = 0.06 + r * (2 * hz - 0.3) / rows
            h0 = (2 * hz - 0.3) / rows
            for side in (-1, 1):
                x = -hx + (0.1 if r % 2 else 0.0)
                while x < hx - 0.08:
                    w = min(rng.uniform(0.25, 0.48), hx - x)
                    h = h0 * rng.uniform(0.75, 0.95)
                    proud = rng.uniform(0.0, 0.05)
                    inset = hy - 0.05 - (z / (2 * hz)) * 0.05 + proud
                    m = Matrix.Translation((x + w / 2, side * (inset - 0.08), z + h / 2)) @ \
                        Matrix.Rotation(rng.uniform(-0.12, 0.12), 4, "X") @ Matrix.Rotation(rng.uniform(-0.1, 0.1), 4, "Z") @ \
                        Matrix.Diagonal((w - 0.04, 0.2, h, 1))
                    bmesh.ops.create_cube(bm, size=1.0, matrix=m)
                    x += w
            for end in (-1, 1):
                y = -hy + 0.08
                while y < hy - 0.1:
                    w = min(rng.uniform(0.18, 0.3), hy - 0.08 - y)
                    h = h0 * rng.uniform(0.75, 0.95)
                    m = Matrix.Translation((end * (hx - 0.1 + rng.uniform(0, 0.04)), y + w / 2, z + h / 2)) @ \
                        Matrix.Diagonal((0.2, w - 0.03, h, 1))
                    bmesh.ops.create_cube(bm, size=1.0, matrix=m)
                    y += w

    mesh_part("wall_stones", stones, stone_m, root, lods=(0, 1))

    def coping(bm, lod):
        # a rough mortared cap: a ridge of slabs laid on edge
        n = int(2 * hx / (0.12 if lod == 0 else 0.5))
        for i in range(n):
            x = -hx + (i + 0.5) * 2 * hx / n
            hh = 0.16 + (rng.uniform(0, 0.06) if lod == 0 else 0.03)
            m = Matrix.Translation((x, 0, 2 * hz - 0.2 + hh / 2)) @ Matrix.Rotation(rng.uniform(-0.15, 0.15), 4, "Y") @ \
                Matrix.Diagonal((2 * hx / n - 0.01, 2 * hy - 0.1, hh, 1))
            bmesh.ops.create_cube(bm, size=1.0, matrix=m)

    mesh_part("wall_coping", coping, stone_m, root)
    return [hx, hy, hz]


def crate():
    wood_m = timber("crate_wood", (0.12, 0.13, 0.07), seed=4.0)
    pallet_m = timber("pallet", (0.16, 0.12, 0.08), seed=6.0)
    metal_m = flat_paint("crate_latch", (0.07, 0.07, 0.065), rough=0.5, metal=0.6, wear=0.8)
    stencil_m = flat_paint("stencil", (0.5, 0.48, 0.36), rough=0.8)
    box("pallet_deck", (2.0, 2.0, 0.04), (0, 0, 0.13), pallet_m, root)
    for k, y in enumerate((-0.85, 0.0, 0.85)):
        box(f"pallet_runner_{k}", (2.0, 0.14, 0.11), (0, y, 0.055), pallet_m, root, lods=(0, 1, 2))
    layout = [((-0.5, -0.5, 0.15), (0.95, 0.95, 0.55), 0.0), ((0.5, -0.5, 0.15), (0.95, 0.95, 0.55), 0.03),
              ((-0.5, 0.5, 0.15), (0.95, 0.95, 0.55), -0.02), ((0.5, 0.5, 0.15), (0.95, 0.95, 0.55), 0.0),
              ((-0.3, 0.0, 0.7), (1.35, 0.9, 0.6), 0.05), ((0.52, 0.2, 0.7), (0.85, 1.4, 0.6), -0.04),
              ((0.0, -0.05, 1.3), (1.2, 0.8, 0.7), 0.0)]
    for k, ((x, y, z), (sx, sy, sz), yaw) in enumerate(layout):
        box(f"crate_{k}", (sx, sy, sz), (x, y, z + sz / 2), wood_m, root, bevel=0.02, rot=(0, 0, yaw))
        for j, dx in enumerate((-0.3, 0.3)):
            box(f"crate_{k}_band_{j}", (0.04, sy + 0.01, sz + 0.01), (x + dx * sx * math.cos(yaw), y + dx * sx * math.sin(yaw), z + sz / 2),
                metal_m, root, rot=(0, 0, yaw), lods=(0,))
        box(f"crate_{k}_lid", (sx * 0.96, sy * 0.96, 0.02), (x, y, z + sz + 0.005), wood_m, root, rot=(0, 0, yaw), lods=(0,))
        box(f"crate_{k}_stencil", (sx * 0.4, 0.005, sz * 0.25),
            (x - math.sin(yaw) * (sy / 2 + 0.004), y - math.cos(yaw) * (sy / 2 + 0.004), z + sz * 0.55),
            stencil_m, root, rot=(0, 0, yaw), lods=(0,))
    return [1.0, 1.0, 1.0]


def bridge_deck():
    hx, hy, hz = 18.0, 5.0, 0.4
    conc = flat_paint("concrete", (0.36, 0.35, 0.32), rough=0.9, wear=0.3)
    asphalt = flat_paint("asphalt", (0.07, 0.07, 0.068), rough=0.95)
    steel_m = flat_paint("railing", (0.12, 0.13, 0.11), rough=0.5, metal=0.6, wear=0.8)
    box("deck_slab", (2 * hx, 2 * hy, 0.4), (0, 0, 0.6), conc, root, bevel=0.03)
    box("deck_surface", (2 * hx, 2 * hy - 1.4, 0.02), (0, 0, 0.81), asphalt, root)
    for s in (-1, 1):
        box(f"deck_girder_{'ab'[s > 0]}", (2 * hx, 1.0, 0.4), (0, s * (hy - 1.6), 0.2), conc, root, bevel=0.03)
        box(f"deck_kerb_{'ab'[s > 0]}", (2 * hx, 0.7, 0.25), (0, s * (hy - 0.35), 0.925), conc, root, bevel=0.03)
        for k in range(19):
            x = -hx + 0.5 + k * (2 * hx - 1) / 18
            box(f"rail_post_{'ab'[s > 0]}_{k}", (0.1, 0.1, 0.9), (x, s * (hy - 0.2), 1.5), steel_m, root, lods=(0, 1))
        for j, zr in enumerate((1.45, 1.9)):
            box(f"rail_{'ab'[s > 0]}_{j}", (2 * hx - 0.8, 0.09, 0.09), (0, s * (hy - 0.18), zr), steel_m, root, lods=(0, 1, 2))
        for k in range(12):
            x = -hx + 1.5 + k * 3.0
            box(f"drain_{'ab'[s > 0]}_{k}", (0.3, 0.08, 0.02), (x, s * (hy - 0.75), 0.82), asphalt, root, lods=(0,))
    return [hx, hy, hz]


box_half = {"wall": wall, "crate": crate, "bridge_deck": bridge_deck}[KIND]()
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.6, ao_rays=10, paint_scale=1.3 if KIND != "bridge_deck" else 4.0)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT)
