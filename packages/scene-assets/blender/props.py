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
from parts import _bevel
from masonry import *

ARGS = [a for a in script_args() if not a.startswith("--")]
KIND, OUT = ARGS[0], ARGS[1]
reset()
root = empty(KIND)
rng = random.Random(7)


def rough_block(bm, m, jitter):
    """A field stone: a block with its corners knocked out of true by up to `jitter` m,
    so no two stones are the same brick."""
    made = bmesh.ops.create_cube(bm, size=1.0, matrix=m)
    for v in made["verts"]:
        v.co += Vector((rng.uniform(-jitter, jitter), rng.uniform(-jitter, jitter) * 0.5, rng.uniform(-jitter, jitter)))


def wall():
    hx, hy, hz = 2.0, 0.3, 0.8
    stone_m = textured("field_stone", "field_stone", chip=0.4, dirt=0.7, rise=0.6, streak=0.4, mottle=0.35, lichen=0.55)
    mortar_m = textured("mortar", "concrete", colour=(0.11, 0.105, 0.09), chip=0.2, dirt=0.8, rise=0.6)  # deep in shadow between the stones
    # the core, slightly battered (narrower at the top)
    box("wall_core", (2 * hx, 2 * hy - 0.08, 2 * hz - 0.2), (0, 0, hz - 0.1), mortar_m, root, taper=(1.0, 0.8))

    # rubble stones on both faces and round the ends, as a waller lays them: big stones
    # low, smaller ones higher, no two alike, the courses broken, each bedded into the
    # mortar core so only its face stands proud
    def fieldstone(bm, lod, centre, size, turn):
        """One stone: a lumpy block, its corners knocked in and its face bulging (a
        once-subdivided cube, every vertex moved), or, coarser, a plain rough block."""
        m = Matrix.Translation(centre) @ Matrix.Rotation(turn[0], 4, "X") @ Matrix.Rotation(turn[1], 4, "Y") @ \
            Matrix.Rotation(turn[2], 4, "Z")
        if lod == 0:
            stone = bmesh.new()
            bmesh.ops.create_cube(stone, size=1.0)
            bmesh.ops.subdivide_edges(stone, edges=stone.edges[:], cuts=1, use_grid_fill=True)
            for v in sorted(stone.verts, key=lambda v: tuple(round(c, 4) for c in v.co)):
                corner = abs(v.co.x) + abs(v.co.z) > 0.9  # knock the corners in
                k = (0.88 if corner else 1.0) + rng.uniform(-0.14, 0.14)
                v.co = m @ Vector((v.co.x * size[0] * k, v.co.y * size[1] * (1.0 + rng.uniform(0, 0.25)),
                                   v.co.z * size[2] * k))
            me = bpy.data.meshes.new("stone")  # append this stone to the wall's bmesh
            stone.to_mesh(me)
            stone.free()
            bm.from_mesh(me)
            bpy.data.meshes.remove(me)
        else:
            rough_block(bm, m @ Matrix.Diagonal((size[0], size[1], size[2], 1)), 0.02)

    def stones(bm, lod):
        if lod >= 2:
            return False
        top = 2 * hz - 0.22
        for side in (-1, 1):
            z = 0.02
            while z < top - 0.08:
                h0 = min(rng.uniform(0.1, 0.34) * (1.1 - 0.5 * z / top), top - z)
                x = -hx + rng.uniform(0.0, 0.15)
                while x < hx - 0.05:
                    w = min(rng.uniform(0.12, 0.7) * (1.1 - 0.4 * z / top), hx - x)
                    h = h0 * rng.uniform(0.7, 1.0)
                    face = hy - 0.03 - (z / (2 * hz)) * 0.05 + rng.uniform(-0.01, 0.03)
                    fieldstone(bm, lod, (x + w / 2, side * (face - 0.09), z + h / 2 + rng.uniform(-0.03, 0.03)),
                               (w - 0.02, 0.2, h), (rng.uniform(-0.1, 0.1), rng.uniform(-0.15, 0.15), rng.uniform(-0.08, 0.08)))
                    x += w
                z += h0
        for end in (-1, 1):
            z = 0.02
            while z < top - 0.08:
                h0 = min(rng.uniform(0.16, 0.28), top - z)
                y = -hy + 0.06
                while y < hy - 0.08:
                    w = min(rng.uniform(0.2, 0.35), hy - 0.06 - y)
                    fieldstone(bm, lod, (end * (hx - 0.09 + rng.uniform(-0.06, 0.08)), y + w / 2, z + h0 / 2),
                               (0.2, w - 0.02, h0 * rng.uniform(0.75, 1.0)), (0, rng.uniform(-0.1, 0.1), 0))
                    y += w
                z += h0

    mesh_part("wall_stones", stones, stone_m, root, lods=(0, 1))

    def coping(bm, lod):
        # the top course: rounded stones of every height, set on edge and on their beds in
        # turn, so the top line is ragged; the line also sags and rises along the wall
        x = -hx
        while x < hx - 0.05:
            w = min(rng.uniform(0.18, 0.45) if lod == 0 else 0.8, hx - x)
            hh = rng.uniform(0.12, 0.3) if lod == 0 else 0.18
            # the top line sags and rises: two waves that fit the module, so seams meet
            sag = 0.07 * math.sin((x + hx) * math.pi / hx) + 0.03 * math.sin((x + hx) * 2 * math.pi / hx + 1.0)
            fieldstone(bm, lod, (x + w / 2, rng.uniform(-0.06, 0.06), 2 * hz - 0.3 + sag + hh / 2),
                       (w - 0.02, rng.uniform(0.3, 0.5), hh), (rng.uniform(-0.1, 0.1), rng.uniform(-0.2, 0.2), rng.uniform(-0.1, 0.1)))
            x += w

    mesh_part("wall_coping", coping, stone_m, root)
    return [hx, hy, hz]


# Contents stencilled on the crates, in the NATO style.
STENCILS = ("CTG 7.62MM NATO\nBALL M80  840 RDS\nLOT 4-83", "CTG 5.56MM\nBALL M855  1680 RDS\nLOT 11-84",
            "GRENADE HAND\nFRAG DM51  30 EA\nLOT 2-85")


def crate():
    wood_m = textured("crate_wood", "painted_wood", chip=0.5, dirt=0.8, rise=0.6, mottle=0.2)
    pallet_m = textured("pallet", "pallet_wood", chip=0.3, dirt=1.0, rise=0.5, streak=0.0)
    metal_m = textured("crate_latch", "bare_steel", chip=0.6, dirt=0.6)
    stencil_m = textured("stencil", "marking_paint", colour=(0.42, 0.38, 0.2), chip=0.8, dirt=0.4)
    box("pallet_deck", (2.0, 2.0, 0.04), (0, 0, 0.13), pallet_m, root)
    for k, y in enumerate((-0.85, 0.0, 0.85)):
        box(f"pallet_runner_{k}", (2.0, 0.14, 0.11), (0, y, 0.055), pallet_m, root, lods=(0, 1, 2))

    def ammo_box(k, size, x, y, z, yaw):
        """One ammunition box at its real size: a banded body, a lid board, rope
        handles at the ends, and its contents stencilled on the side facing out."""
        sx, sy, sz = size
        c, s = math.cos(yaw), math.sin(yaw)
        box(f"crate_{k}", size, (x, y, z + sz / 2), wood_m, root, bevel=0.012, rot=(0, 0, yaw))
        box(f"crate_{k}_lid", (sx * 0.94, sy * 0.94, 0.018), (x, y, z + sz + 0.004), wood_m, root, rot=(0, 0, yaw),
            lods=(0,))
        for j, d in enumerate((-0.32, 0.32)):
            box(f"crate_{k}_band_{j}", (0.03, sy + 0.008, sz + 0.008), (x + d * sx * c, y + d * sx * s, z + sz / 2),
                metal_m, root, rot=(0, 0, yaw), lods=(0,))
        for j, e in enumerate((-1, 1)):
            box(f"crate_{k}_handle_{j}", (0.03, sy * 0.35, 0.03), (x + e * (sx / 2 + 0.012) * c, y + e * (sx / 2 + 0.012) * s,
                z + sz * 0.7), metal_m, root, rot=(0, 0, yaw), lods=(0,))
        # the long side facing away from the stack's centre carries the stencil
        out = 1 if (-s * x + c * y) > 0 else -1
        nx, ny = -s * out, c * out
        stencil(f"crate_{k}_stencil", STENCILS[k % len(STENCILS)], min(sz * 0.12, 0.045),
                (x + nx * (sy / 2 + 0.001), y + ny * (sy / 2 + 0.001), z + sz * 0.5),
                (math.pi / 2, 0, yaw + (math.pi if out > 0 else 0)), stencil_m, root, lods=(0,))

    # layers of boxes, real ammunition sizes, each layer a little shorter and
    # sparser than the one under it, and every box a little out of line
    layers = [
        ((1.9, 0.46, 0.4), [(0, -0.72), (0, -0.24), (0, 0.24), (0, 0.72)]),
        ((0.62, 0.92, 0.36), [(-0.64, -0.47), (0.0, -0.47), (0.64, -0.47), (-0.64, 0.47), (0.0, 0.47), (0.64, 0.47)]),
        ((0.92, 0.62, 0.36), [(-0.47, -0.64), (0.47, -0.64), (-0.47, 0.0), (0.47, 0.0), (-0.47, 0.64), (0.47, 0.64)]),
        ((0.62, 0.92, 0.34), [(-0.64, -0.47), (0.0, -0.47), (0.64, -0.47), (-0.64, 0.47), (0.0, 0.47)]),
        ((0.62, 0.46, 0.3), [(-0.34, -0.3), (0.34, -0.25), (0.1, 0.3)]),
    ]
    z, k = 0.15, 0
    for size, spots in layers:
        for x, y in spots:
            ammo_box(k, size, x + rng.uniform(-0.03, 0.03), y + rng.uniform(-0.03, 0.03), z, rng.uniform(-0.04, 0.04))
            k += 1
        z += size[2] + 0.02
    return [1.0, 1.0, 1.0]


def bridge_deck():
    hx, hy, hz = 18.0, 5.0, 0.4
    conc = textured("concrete", "concrete", chip=0.5, dirt=0.25, rise=1.2, streak=0.6)
    asphalt = textured("asphalt", "asphalt", chip=0.0, dirt=0.0, streak=0.0)
    steel_m = textured("railing", "bare_steel", colour=(0.1, 0.105, 0.095), chip=0.6, dirt=0.4)
    box("deck_slab", (2 * hx, 2 * hy, 0.4), (0, 0, 0.6), conc, root, bevel=0.03)
    box("deck_surface", (2 * hx, 2 * hy - 1.4, 0.02), (0, 0, 0.81), asphalt, root)
    # a worn centre line, dashed, and the edge lines
    line_m = textured("road_line", "marking_paint", colour=(0.5, 0.49, 0.44), chip=1.0, dirt=0.0, streak=0.0)
    for k in range(9):
        box(f"centre_dash_{k}", (2.2, 0.12, 0.004), (-hx + 2.0 + k * 4.0, 0, 0.8225), line_m, root, lods=(0, 1))
    for s in (-1, 1):
        box(f"edge_line_{'ab'[s > 0]}", (2 * hx - 0.2, 0.1, 0.004), (0, s * (hy - 0.95), 0.8225), line_m, root, lods=(0, 1))
    # the span: the slab rides on four girders that bear on an abutment at each end, the
    # deck's edges cantilevered past the outer girders, dark underneath
    for k, y in enumerate((-3.6, -1.2, 1.2, 3.6)):
        box(f"deck_girder_{k}", (2 * hx - 2.2, 0.45, 0.4), (0, y, 0.2), conc, root, bevel=0.03)
    for e in (-1, 1):
        box(f"abutment_{'ab'[e > 0]}", (1.1, 2 * hy - 0.3, 0.4), (e * (hx - 0.55), 0, 0.2), conc, root, bevel=0.03)
        # the abutments' wing walls, flanking each end of the deck on the banks
        for s in (-1, 1):
            box(f"wing_wall_{'ab'[e > 0]}{'ab'[s > 0]}", (2.6, 0.45, 1.2), (e * (hx - 1.1), s * (hy + 0.2), 0.6), conc, root,
                bevel=0.03, rot=(0, 0, s * e * 0.12))
    # a galvanised W-beam guardrail along the inside of each parapet
    w_beam = [(0.0, 0.0), (0.07, 0.06), (0.025, 0.15), (0.07, 0.24), (0.0, 0.3), (-0.012, 0.3), (0.058, 0.24),
              (0.013, 0.15), (0.058, 0.06), (-0.012, 0.0)]
    rail_m = textured("guardrail", "bare_steel", colour=(0.16, 0.165, 0.16), chip=0.3, dirt=0.3, streak=0.4)
    for s in (-1, 1):
        box(f"deck_kerb_{'ab'[s > 0]}", (2 * hx, 0.7, 0.25), (0, s * (hy - 0.35), 0.925), conc, root, bevel=0.03)
        # the edge beam: the full depth of the deck, its face the bridge's visible side
        box(f"edge_beam_{'ab'[s > 0]}", (2 * hx, 0.3, 0.8), (0, s * (hy - 0.15), 0.4), conc, root, bevel=0.03)
        # a concrete parapet on the deck's edge
        box(f"parapet_{'ab'[s > 0]}", (2 * hx, 0.3, 0.55), (0, s * (hy - 0.15), 1.1), conc, root, bevel=0.02)
        prism(f"guardrail_{'ab'[s > 0]}", [(x * -s, z) for x, z in w_beam], 2 * hx - 0.4, (0, s * (hy - 0.72), 1.1),
              rail_m, root, rot=(0, 0, math.pi / 2), lods=(0, 1, 2))
        for k in range(13):
            x = -hx + 0.5 + k * (2 * hx - 1) / 12
            box(f"rail_post_{'ab'[s > 0]}_{k}", (0.1, 0.06, 0.4), (x, s * (hy - 0.66), 1.25), steel_m, root, lods=(0, 1))
        for k in range(12):
            x = -hx + 1.5 + k * 3.0
            box(f"drain_{'ab'[s > 0]}_{k}", (0.3, 0.08, 0.02), (x, s * (hy - 0.75), 0.82), asphalt, root, lods=(0,))
    return [hx, hy, hz]


box_half = {"wall": wall, "crate": crate, "bridge_deck": bridge_deck}[KIND]()
rest_on_ground()  # a bottom stone bedded below the ground sits on it
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.35 if KIND == "crate" else 0.6, ao_rays=10, paint_scale=1.3 if KIND != "bridge_deck" else 4.0)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT)
