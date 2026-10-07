"""Props, each authored to one simulation box (the catalog's `footprint_half_m`).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/props.py <kind> <out.glb>

kind:
  wall        a rubble-stone field wall with a mortared coping; box [2, 0.3, 0.8] (4 m module)
  crate       a stack of wooden ammunition crates on a pallet; box [1, 1, 1]
  bridge_deck a concrete road deck on four girders over two piers, with parapets; box [18, 5, 0.4] (geometry
              lab's bridge). Its piers, abutments and wing walls stand below the box, down to a
              channel's bed (`BED_M`)
  fence       a close-boarded farm fence; box [1.5, 0.1, 0.6] (3 m module)
  sandbags    a sandbag wall two bags thick; box [1, 0.4, 0.5] (2 m module)
  tooth       one dragon's tooth, a concrete truncated pyramid; box [0.6, 0.6, 0.6]

The battle fits each placed box from the authored one. Origin at the
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


SUBSTRUCTURE = ("pier_", "abutment_", "wing_wall_")  # stands below the deck's box, down to the bed
BED_M = 2.0  # how far below the box the substructure reaches: a channel's bed within 2 m of the deck's underside
BANK_M = 12.0  # half the stream's width under the deck: its abutments stand at the banks (the geometry lab's 24 m)


def bridge_deck():
    hx, hy, hz = 18.0, 5.0, 0.4
    conc = textured("concrete", "concrete", chip=0.5, dirt=0.25, rise=1.2, streak=0.6)
    # the substructure: river-washed concrete, stained dark and green below the water's reach
    wet = textured("pier_concrete", "concrete", colour=(0.2, 0.19, 0.16), chip=0.4, dirt=0.9, streak=0.7,
                   dust=(0.07, 0.075, 0.05), lichen=0.5, seed=3.0)
    # the road's dust filmed over the paving, so it meets the country road without a seam
    asphalt = textured("asphalt", "asphalt", chip=0.0, dirt=0.9, rise=1.2, streak=0.0, dust=(0.2, 0.175, 0.14))
    steel_m = textured("railing", "bare_steel", colour=(0.1, 0.105, 0.095), chip=0.6, dirt=0.4)
    box("deck_slab", (2 * hx, 2 * hy, 0.4), (0, 0, 0.6), conc, root, bevel=0.03)
    box("deck_surface", (2 * hx, 2 * hy - 1.4, 0.02), (0, 0, 0.81), asphalt, root)
    # worn edge lines only: a country bridge carries no centre line
    line_m = textured("road_line", "marking_paint", colour=(0.5, 0.49, 0.44), chip=1.0, dirt=0.0, streak=0.0)
    for s in (-1, 1):
        box(f"edge_line_{'ab'[s > 0]}", (2 * hx - 4.0, 0.1, 0.004), (0, s * (hy - 0.95), 0.8225), line_m, root, lods=(0, 1))
    # the span: the slab rides on four girders over three spans, bearing on two piers in the
    # stream and on an abutment at each bank, the deck's edges cantilevered past the outer
    # girders, dark underneath; past the abutments the deck runs on over the approaches
    for k, y in enumerate((-3.6, -1.2, 1.2, 3.6)):
        box(f"deck_girder_{k}", (2 * hx - 2.2, 0.45, 0.4), (0, y, 0.2), conc, root, bevel=0.03)
    for k, x in enumerate((-BANK_M / 3, BANK_M / 3)):
        # a wall pier whose rounded cutwaters stand out past the deck up- and downstream,
        # under a crosshead that carries the girders on bearing pads
        depth = BED_M - 0.3
        box(f"pier_wall_{k}", (0.9, 2 * hy + 0.4, depth), (x, 0, -depth / 2 - 0.3), wet, root, bevel=0.03)
        for s in (-1, 1):
            cyl(f"pier_nose_{k}_{'ab'[s > 0]}", 0.45, depth, (x, s * (hy + 0.2), -depth / 2 - 0.3), "Z", wet, root, seg=20)
        box(f"pier_cap_{k}", (0.95, 2 * hy + 0.4, 0.3), (x, 0, -0.15), wet, root, bevel=0.03)
        for j, y in enumerate((-3.6, -1.2, 1.2, 3.6)):
            box(f"pier_bearing_{k}_{j}", (0.5, 0.5, 0.06), (x, y, 0.0), steel_m, root, lods=(0, 1))
    for e in (-1, 1):
        # the abutment at the bank: a wall from the bed up under the girders across the whole
        # width, its face to the stream
        tall = BED_M + 0.4
        box(f"abutment_{'ab'[e > 0]}", (1.1, 2 * hy - 0.3, tall), (e * (BANK_M + 0.55), 0, 0.4 - tall / 2), wet, root,
            bevel=0.03)
        # wing walls run from it along the deck's sides, retaining the bank under the approach
        for s in (-1, 1):
            tall = BED_M + 0.4
            length = hx - BANK_M - 1.0
            box(f"wing_wall_{'ab'[e > 0]}{'ab'[s > 0]}", (length, 0.4, tall),
                (e * (BANK_M + 0.5 + length / 2), s * (hy + 0.2), 0.4 - tall / 2), wet, root, bevel=0.03)
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


def fence():
    """A close-boarded farm fence, 1.2 m: posts every 1.5 m, three rails, boards on one face.
    The end posts straddle the module's ends, so repeated modules share them."""
    hx, hy, hz = 1.5, 0.1, 0.6
    post_m = textured("fence_post", "pallet_wood", colour=(0.13, 0.11, 0.08), chip=0.3, dirt=1.0, rise=0.5, mottle=0.3)
    board_m = textured("fence_board", "pallet_wood", chip=0.3, dirt=0.9, rise=0.5, streak=0.5, mottle=0.4)
    for k, x in enumerate((-hx, 0.0, hx)):
        box(f"post_{k}", (0.12, 0.12, 2 * hz), (x, -0.02, hz), post_m, root, bevel=0.01, taper=(0.92, 0.92))
    for k, z in enumerate((0.18, 0.62, 1.06)):
        box(f"rail_{k}", (2 * hx, 0.05, 0.09), (0, 0.065, z), post_m, root, bevel=0.008, lods=(0, 1, 2))
    # boards: a little uneven in width, height and lean, with gaps between
    x, k = -hx, 0
    while x < hx - 0.02:
        w = min(rng.uniform(0.14, 0.2), hx - x)
        top = 2 * hz - rng.uniform(0.0, 0.06)
        box(f"board_{k}", (w - 0.02, 0.022, top - 0.05), (x + w / 2, 0.1 - 0.011, 0.05 + (top - 0.05) / 2), board_m,
            root, rot=(0, rng.uniform(-0.015, 0.015), 0), lods=(0, 1))
        x += w
        k += 1
    # the boards as one sheet where boards no longer read
    box("boards_far", (2 * hx, 0.022, 2 * hz - 0.08), (0, 0.089, hz), board_m, root, lods=(2, 3))
    return [hx, hy, hz]


def sandbags():
    """A sandbag wall 1 m high and two bags thick, laid in stretcher bond, battered in
    towards the top; a 2 m module whose end bags are cut to the module so repeats meet."""
    hx, hy, hz = 1.0, 0.4, 0.5
    bag_m = textured("sandbag", "hessian", chip=0.1, dirt=1.0, rise=0.4, streak=0.2, mottle=0.35)
    course, length, depth = 0.143, 0.5, 0.36

    def pillow(bm, lod, centre, size, turn):
        """One filled bag: a cube rounded into a pillow, its ends tucked and squashed."""
        m = Matrix.Translation(centre) @ Matrix.Rotation(turn, 4, "Z")
        cube = bmesh.new()
        bmesh.ops.create_cube(cube, size=1.0)
        if lod == 0:
            bmesh.ops.subdivide_edges(cube, edges=cube.edges[:], cuts=2, use_grid_fill=True)
        for v in cube.verts:
            x, y, z = v.co.x * 2, v.co.y * 2, v.co.z * 2  # -1..1
            if lod == 0:
                # round the cross-section and pinch the ends, where the bag is tied
                bulge = 1.0 - 0.28 * (y * y) * (z * z) - 0.12 * x * x * (y * y + z * z)
                y *= bulge
                z *= 1.0 - 0.25 * x * x * x * x
            v.co = m @ Vector((x * size[0] / 2, y * size[1] / 2, z * size[2] / 2))
        me = bpy.data.meshes.new("bag")
        cube.to_mesh(me)
        cube.free()
        bm.from_mesh(me)
        bpy.data.meshes.remove(me)

    def wall(bm, lod):
        if lod >= 2:
            return False
        rows = int(round(2 * hz / course))
        for r in range(rows):
            z = r * course + course / 2
            inset = r * 0.022  # battered: each course set back a little
            for side in (-1, 1):
                off = (length / 2 if (r + (side > 0)) % 2 else 0.0) - hx
                x = off - length
                while x < hx:
                    a, b = max(x, -hx), min(x + length, hx)
                    if b - a > 0.08:
                        cx = (a + b) / 2 + rng.uniform(-0.01, 0.01)
                        cy = side * (hy - inset - depth / 2 - 0.01) + rng.uniform(-0.015, 0.015)
                        pillow(bm, lod, (cx, cy, z), (b - a - 0.012, depth, course * 1.1), rng.uniform(-0.04, 0.04))
                    x += length

    mesh_part("bags", wall, bag_m, root, lods=(0, 1))
    # far away the wall is its courses, each a battered slab in the bags' shaded tone
    far_m = textured("sandbag_far", "hessian", colour=(0.1, 0.08, 0.052), chip=0.0, dirt=1.0, rise=0.4, streak=0.0)
    rows = int(round(2 * hz / course))
    for r in range(rows):
        inset = r * 0.022
        box(f"bags_far_{r}", (2 * hx, 2 * (hy - inset) - 0.04, course * 0.92), (0, 0, r * course + course / 2), far_m,
            root, lods=(2,))
    box("bags_far_block", (2 * hx, 2 * hy - 0.04, 2 * hz), (0, 0, hz), far_m, root, taper=(1.0, 0.72), lods=(3,))
    return [hx, hy, hz]


def tooth():
    """One dragon's tooth: a cast-concrete truncated pyramid on a footing, its arrises
    knocked about; 1.2 m square and 1.2 m tall."""
    hx, hy, hz = 0.6, 0.6, 0.6
    conc = textured("tooth_concrete", "concrete", chip=0.8, dirt=0.8, rise=0.5, streak=0.6, mottle=0.25, lichen=0.35)

    def pyramid(bm, lod):
        rings = [(0.0, 0.6), (0.14, 0.6), (0.16, 0.54), (1.2, 0.2)]
        vs = []
        for z, r in rings:
            vs.append([bm.verts.new((sx * r, sy * r, z)) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))])
        bm.faces.new(vs[0][::-1])
        bm.faces.new(vs[-1])
        for a, b in zip(vs, vs[1:]):
            for i in range(4):
                j = (i + 1) % 4
                bm.faces.new((a[i], a[j], b[j], b[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        if lod <= 1:
            _bevel(bm, 0.025 if lod == 0 else 0.015, 2 if lod == 0 else 1)
        if lod == 0:  # weathering: the arrises chipped, the faces a little out of true
            bmesh.ops.subdivide_edges(bm, edges=[e for e in bm.edges if e.calc_length() > 0.25], cuts=2)
            for v in bm.verts:
                if 0.02 < v.co.z < 1.19:
                    v.co.x += 0.012 * noise.noise(v.co * 6.0)
                    v.co.y += 0.012 * noise.noise(v.co * 6.0 + Vector((5, 0, 0)))

    mesh_part("tooth", pyramid, conc, root)
    return [hx, hy, hz]


box_half = {"wall": wall, "crate": crate, "bridge_deck": bridge_deck, "fence": fence, "sandbags": sandbags,
            "tooth": tooth}[KIND]()
# a bottom stone bedded below the ground sits on it; a bridge's piers and abutments stand
# below its box, down to the bed
rest_on_ground(keep=SUBSTRUCTURE)
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.35 if KIND == "crate" else 0.6, ao_rays=10, paint_scale=1.3 if KIND != "bridge_deck" else 4.0)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT)
