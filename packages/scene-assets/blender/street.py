"""Street furniture and construction-site bodies, each authored to one simulation box
(the catalog's `footprint_half_m`): the street catalog's rows in
`fixtures/props/city/street.json`. Generic: no brand, logo, livery, sign or plate.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/street.py <kind> <out.glb>

kind (box half extents, metres):
  jersey_barrier  a precast concrete road barrier, a 3 m module; [1.5, 0.3, 0.4]
  bollard         a steel post with a banded cap; [0.1, 0.1, 0.45]
  lamp            a post-top street lamp on a 5 m column; [0.2, 0.2, 2.5]
  bench           a slatted bench on two cast legs; [0.9, 0.3, 0.42]
  bins            two wheeled refuse bins side by side; [0.65, 0.38, 0.55]
  hydrant         a pillar hydrant; [0.2, 0.2, 0.38]
  utility_box     a steel street cabinet on a plinth; [0.7, 0.25, 0.7]
  planter         a concrete planter with a clipped shrub; [1.0, 0.45, 0.5]
  bus_shelter     a glazed shelter: back and end screens under a flat roof; [2.0, 0.75, 1.25]
  heras_fence     a temporary mesh fence panel in two block feet, a 3.5 m module; [1.75, 0.3, 1.0]
  skip_bin        an open steel skip, loaded with rubble; [1.8, 0.85, 0.6]
  pallet_stack    eight timber pallets stacked out of true; [0.6, 0.5, 0.6]
  site_cabin      a closed steel storage cabin, its door and shutter shut; [3.0, 1.2, 1.3]
  traffic_cone    a cone with one reflective band; [0.2, 0.2, 0.37]
  road_barrier    a plastic barrier board on two feet; [1.0, 0.25, 0.5]
  scaffold        one bay of tube scaffold, a boarded lift at 2 m; a 2.5 m module; [1.25, 0.5, 2.0]
  scooter         a parked motor scooter on its stand; [0.9, 0.35, 0.55]

The parked car and its wreck are `street_car.py`. The battle fits each placed box
from the authored one. Origin at the box's centre on the ground, +X along its first
half extent.
"""
import bpy, bmesh, sys, os, math, json, random
from mathutils import Vector, noise

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import *

ARGS = [a for a in script_args() if not a.startswith("--")]
KIND, OUT = ARGS[0], ARGS[1]
reset()
root = empty(KIND)
rng = random.Random(11)
KINDS = {}


def kind(fn):
    KINDS[fn.__name__] = fn
    return fn


def tube(name, a, b, r, mat, lods=TIERS, seg=8):
    """A round bar from `a` to `b`; four-sided on the far tiers."""
    a, b = Vector(a), Vector(b)
    d = b - a
    rot = Vector((0, 0, 1)).rotation_difference(d.normalized()).to_euler()
    return cyl(name, r, d.length, (a + b) / 2, "Z", mat, root, seg=seg, rot=rot, lods=lods, min_seg=4)


def lump(bm, lod, centre, radii, rough, seed):
    """A rounded mass (a shrub's clump, a heap of spoil): a sphere pushed in and out by noise."""
    made = bmesh.ops.create_icosphere(bm, subdivisions=max(1, 2 - lod), radius=1.0)
    off = Vector((seed * 1.7, seed * 0.9, seed * 2.3))
    for v in made["verts"]:
        k = 1.0 + rough * noise.noise(v.co * 1.6 + off)
        v.co = Vector(centre) + Vector((v.co.x * radii[0], v.co.y * radii[1], v.co.z * radii[2])) * k


def steel(name="steel"):
    return textured(name, "galvanised", chip=0.5, dirt=0.7, rise=0.6)


def enamel(name, colour, chip=0.5, dirt=0.7, rise=0.8, streak=0.3):
    return textured(name, "enamel", colour=colour, chip=chip, dirt=dirt, rise=rise, streak=streak)


def plastic(name, colour, dirt=0.7, rise=0.5):
    return textured(name, "hard_plastic", colour=colour, chip=0.15, dirt=dirt, rise=rise, streak=0.2)


# ------------------------------------------------------------------ the street
@kind
def jersey_barrier():
    """A New Jersey profile: a wide foot, a short batter, a steep upper face. The ends are
    square so repeated modules meet."""
    hx, hy, hz = 1.5, 0.3, 0.4
    conc = textured("barrier_concrete", "concrete", chip=0.8, dirt=0.9, rise=0.5, streak=0.6, mottle=0.2)
    side = [(0.3, 0.0), (0.3, 0.08), (0.17, 0.3), (0.085, 2 * hz)]
    profile = side + [(-y, z) for y, z in reversed(side)]
    prism("barrier", profile, 2 * hx, (0, 0, 0), conc, root, bevel=0.015, rot=(0, 0, math.pi / 2))
    iron = textured("lifting_eye", "bare_steel", chip=0.5, dirt=0.5)
    for k, x in enumerate((-0.9, 0.9)):
        box(f"lifting_eye_{k}", (0.14, 0.03, 0.05), (x, 0, 2 * hz + 0.02), iron, root, lods=(0, 1))
    return [hx, hy, hz]


@kind
def bollard():
    hx, hy, hz = 0.1, 0.1, 0.45
    post = enamel("bollard_paint", (0.035, 0.038, 0.04), chip=0.8, dirt=0.8, rise=0.4)
    band = plastic("bollard_band", (0.6, 0.6, 0.56), dirt=0.3)
    cyl("flange", 0.1, 0.025, (0, 0, 0.0125), "Z", post, root, seg=16, lods=(0, 1, 2))
    cyl("post", 0.08, 0.8, (0, 0, 0.4), "Z", post, root, seg=16)
    cyl("band", 0.084, 0.07, (0, 0, 0.68), "Z", band, root, seg=16, lods=(0, 1, 2))
    cyl("cap", 0.088, 0.1, (0, 0, 0.85), "Z", post, root, seg=16, r2=0.05, bevel=0.01)
    return [hx, hy, hz]


@kind
def lamp():
    """A post-top lantern: the column is the body, and nothing overhangs it."""
    hx, hy, hz = 0.2, 0.2, 2.5
    zinc = steel("column")
    dark = enamel("lantern_paint", (0.03, 0.032, 0.034), chip=0.4, dirt=0.3, rise=0.4)
    glow = flat_paint("diffuser", (0.62, 0.6, 0.5), rough=0.3, grime=0.0)
    cyl("base_flange", 0.17, 0.02, (0, 0, 0.01), "Z", zinc, root, seg=12, lods=(0, 1))
    cyl("base", 0.1, 1.0, (0, 0, 0.5), "Z", zinc, root, seg=14, bevel=0.01)
    box("base_door", (0.012, 0.1, 0.4), (0.098, 0, 0.55), zinc, root, lods=(0,))
    cyl("column", 0.062, 3.4, (0, 0, 2.7), "Z", zinc, root, seg=12, r2=0.045)
    cyl("collar", 0.07, 0.12, (0, 0, 4.45), "Z", dark, root, seg=12, lods=(0, 1, 2))
    cyl("diffuser", 0.12, 0.3, (0, 0, 4.66), "Z", glow, root, seg=14, r2=0.19)
    cyl("canopy", 0.2, 0.14, (0, 0, 4.88), "Z", dark, root, seg=14, r2=0.06, bevel=0.01)
    cyl("finial", 0.03, 0.05, (0, 0, 4.975), "Z", dark, root, seg=8, lods=(0, 1))
    return [hx, hy, hz]


@kind
def bench():
    hx, hy, hz = 0.9, 0.3, 0.42
    wood = textured("bench_slat", "pallet_wood", colour=(0.15, 0.1, 0.06), chip=0.3, dirt=0.5, rise=0.5, mottle=0.3)
    iron = textured("bench_leg", "bare_steel", chip=0.5, dirt=0.7)
    for k, x in enumerate((-0.74, 0.74)):
        box(f"leg_front_{k}", (0.05, 0.05, 0.42), (x, -0.24, 0.21), iron, root)
        box(f"leg_back_{k}", (0.05, 0.05, 0.84), (x, 0.26, 0.42), iron, root, rot=(math.radians(-4), 0, 0))
        box(f"leg_rail_{k}", (0.05, 0.52, 0.04), (x, 0.0, 0.4), iron, root, lods=(0, 1, 2))
        box(f"arm_rest_{k}", (0.05, 0.5, 0.03), (x, -0.02, 0.62), iron, root, lods=(0, 1))
    for k in range(4):
        box(f"seat_slat_{k}", (2 * hx, 0.1, 0.035), (0, -0.25 + 0.125 * k, 0.44), wood, root, bevel=0.006,
            lods=(0, 1))
    for k in range(3):
        box(f"back_slat_{k}", (2 * hx, 0.03, 0.09), (0, 0.235 + 0.012 * k, 0.56 + 0.115 * k), wood, root,
            rot=(math.radians(-8), 0, 0), bevel=0.006, lods=(0, 1))
    box("seat_far", (2 * hx, 0.47, 0.035), (0, -0.065, 0.44), wood, root, lods=(2, 3))
    box("back_far", (2 * hx, 0.03, 0.32), (0, 0.25, 0.67), wood, root, rot=(math.radians(-8), 0, 0), lods=(2, 3))
    return [hx, hy, hz]


@kind
def bins():
    hx, hy, hz = 0.65, 0.38, 0.55
    wheel_m = textured("bin_wheel", "rubber", chip=0.0, dirt=0.6, streak=0.0)
    for k, (x, colour) in enumerate(((-0.33, (0.035, 0.075, 0.045)), (0.33, (0.06, 0.062, 0.066)))):
        body = plastic(f"bin_{k}", colour, dirt=0.8, rise=0.7)
        yaw = (-0.04, 0.05)[k]
        box(f"bin_{k}_body", (0.5, 0.58, 0.9), (x, 0.02, 0.53), body, root, bevel=0.03, taper=(1.14, 1.14),
            rot=(0, 0, yaw))
        box(f"bin_{k}_lid", (0.6, 0.72, 0.05), (x, 0.0, 1.065), body, root, bevel=0.015,
            rot=(math.radians(-3), 0, yaw))
        box(f"bin_{k}_rim", (0.6, 0.7, 0.05), (x, 0.0, 0.97), body, root, rot=(0, 0, yaw), lods=(0, 1, 2))
        box(f"bin_{k}_handle", (0.5, 0.04, 0.04), (x, 0.36, 0.99), body, root, rot=(0, 0, yaw), lods=(0, 1))
        for j, s in enumerate((-1, 1)):
            cyl(f"bin_{k}_wheel_{j}", 0.1, 0.05, (x + s * 0.24, 0.26, 0.1), "X", wheel_m, root, seg=12,
                lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def hydrant():
    hx, hy, hz = 0.2, 0.2, 0.38
    red = enamel("hydrant_paint", (0.32, 0.035, 0.022), chip=0.7, dirt=0.8, rise=0.35)
    cap = steel("hydrant_cap")
    cyl("flange", 0.16, 0.05, (0, 0, 0.025), "Z", red, root, seg=16, bevel=0.01)
    cyl("barrel", 0.1, 0.5, (0, 0, 0.3), "Z", red, root, seg=16)
    cyl("collar", 0.125, 0.05, (0, 0, 0.55), "Z", red, root, seg=16, lods=(0, 1, 2))
    cyl("bonnet", 0.115, 0.14, (0, 0, 0.64), "Z", red, root, seg=16, r2=0.05, bevel=0.01)
    cyl("nut", 0.03, 0.05, (0, 0, 0.735), "Z", cap, root, seg=6, lods=(0, 1))
    cyl("side_outlets", 0.05, 0.4, (0, 0, 0.42), "Y", red, root, seg=12)
    for k, s in enumerate((-1, 1)):
        cyl(f"side_cap_{k}", 0.058, 0.03, (0, s * 0.185, 0.42), "Y", cap, root, seg=8, lods=(0, 1))
    cyl("front_outlet", 0.065, 0.2, (0.1, 0, 0.4), "X", red, root, seg=12)
    cyl("front_cap", 0.075, 0.03, (0.185, 0, 0.4), "X", cap, root, seg=8, lods=(0, 1))
    return [hx, hy, hz]


@kind
def utility_box():
    hx, hy, hz = 0.7, 0.25, 0.7
    conc = textured("plinth", "concrete", chip=0.5, dirt=0.9, rise=0.4)
    skin = enamel("cabinet_paint", (0.13, 0.15, 0.13), chip=0.5, dirt=0.8, rise=0.9, streak=0.6)
    dark = textured("cabinet_gap", "rubber", chip=0.0, dirt=0.3, streak=0.0)
    box("plinth", (2 * hx - 0.04, 2 * hy - 0.04, 0.12), (0, 0, 0.06), conc, root, bevel=0.01)
    box("cabinet", (2 * hx, 2 * hy, 1.22), (0, 0, 0.73), skin, root, bevel=0.015)
    box("cap", (2 * hx, 2 * hy, 0.06), (0, 0, 1.37), skin, root, bevel=0.02, taper=(0.96, 0.9))
    # two doors on the street face: the gap between them, a row of louvres and a handle each
    box("door_gap", (0.012, 0.01, 1.12), (0, -hy - 0.002, 0.73), dark, root, lods=(0, 1))
    for k, s in enumerate((-1, 1)):
        for j in range(4):
            box(f"louvre_{k}_{j}", (0.4, 0.012, 0.018), (s * 0.34, -hy - 0.004, 1.08 + j * 0.045), dark, root,
                lods=(0,))
        box(f"door_handle_{k}", (0.03, 0.03, 0.14), (s * 0.07, -hy - 0.012, 0.75), dark, root, lods=(0, 1))
    return [hx, hy, hz]


@kind
def planter():
    hx, hy, hz = 1.0, 0.45, 0.5
    conc = textured("planter_concrete", "concrete", chip=0.7, dirt=0.9, rise=0.5, streak=0.7, lichen=0.3)
    earth = textured("planter_soil", "soil", chip=0.0, dirt=0.0, streak=0.0)
    leaf = flat_paint("shrub", (0.05, 0.085, 0.022), rough=0.85, grime=0.15)
    box("tub", (1.84, 0.76, 0.56), (0, 0, 0.28), conc, root, bevel=0.02, taper=(2 * hx / 1.84, 2 * hy / 0.76))
    box("soil", (1.8, 0.72, 0.04), (0, 0, 0.555), earth, root, lods=(0, 1, 2))

    def shrub(bm, lod):
        for k, x in enumerate((-0.62, -0.3, 0.02, 0.33, 0.62)):  # one clipped mass, not a row of balls
            lump(bm, lod, (x, 0.03 * (-1) ** k, 0.7), (0.34, 0.31, 0.27), 0.2, 3.0 + k)

    mesh_part("shrub", shrub, leaf, root, smooth=True)
    return [hx, hy, hz]


@kind
def bus_shelter():
    """Open to the street on +Y: a glazed back, a half-depth glazed screen at each end,
    a flat roof and a bench. The glass is dark and glossy; nothing here is see-through."""
    hx, hy, hz = 2.0, 0.75, 1.25
    frame = enamel("shelter_frame", (0.045, 0.05, 0.055), chip=0.4, dirt=0.6, rise=0.5)
    glass = flat_paint("glass", (0.055, 0.075, 0.085), rough=0.06, grime=0.3)
    roof_m = enamel("shelter_roof", (0.2, 0.21, 0.21), chip=0.3, dirt=0.4, rise=0.0, streak=0.0)
    wood = textured("shelter_seat", "pallet_wood", colour=(0.15, 0.1, 0.06), chip=0.3, dirt=0.4)
    back = -hy + 0.05
    for k, x in enumerate((-1.95, -0.65, 0.65, 1.95)):
        box(f"post_back_{k}", (0.07, 0.07, 2.4), (x, back, 1.2), frame, root)
    for k, s in enumerate((-1, 1)):
        box(f"post_front_{k}", (0.07, 0.07, 2.4), (s * 1.95, 0.3, 1.2), frame, root)
        box(f"end_glass_{k}", (0.02, 0.98 - 0.07, 1.75), (s * 1.95, (back + 0.3) / 2, 1.225), glass, root)
        for j, z in enumerate((0.32, 2.13)):
            box(f"end_rail_{k}_{j}", (0.05, 0.98, 0.05), (s * 1.95, (back + 0.3) / 2, z), frame, root, lods=(0, 1, 2))
    for k, x in enumerate((-1.3, 0.0, 1.3)):
        box(f"back_glass_{k}", (1.23, 0.02, 1.75), (x, back, 1.225), glass, root)
    for j, z in enumerate((0.32, 2.13)):
        box(f"back_rail_{j}", (3.9, 0.05, 0.05), (0, back, z), frame, root, lods=(0, 1, 2))
    box("roof", (2 * hx, 2 * hy, 0.08), (0, 0, 2.46), roof_m, root, bevel=0.02)
    box("roof_fascia", (2 * hx - 0.06, 2 * hy - 0.06, 0.08), (0, 0, 2.39), frame, root, lods=(0, 1, 2))
    box("seat", (2.4, 0.3, 0.04), (0, back + 0.25, 0.48), wood, root, bevel=0.008)
    for k, x in enumerate((-1.0, 1.0)):
        box(f"seat_leg_{k}", (0.05, 0.26, 0.46), (x, back + 0.25, 0.23), frame, root, lods=(0, 1))
    return [hx, hy, hz]


# ------------------------------------------------------------------ the building site
@kind
def heras_fence():
    """A temporary fence panel: a tube frame filled with welded mesh, stood in two concrete
    blocks. The mesh is wires on the near tiers and gone on the far ones, where it would crawl."""
    hx, hy, hz = 1.75, 0.3, 1.0
    zinc = steel("fence_frame")
    wire = steel("fence_wire")
    conc = textured("fence_block", "concrete", chip=0.6, dirt=0.9, rise=0.3)
    x0, z0, z1 = hx - 0.02, 0.1, 2 * hz - 0.02
    for k, s in enumerate((-1, 1)):
        tube(f"upright_{k}", (s * x0, 0, 0.02), (s * x0, 0, z1), 0.02, zinc)
        box(f"block_{k}", (0.22, 2 * hy, 0.13), (s * (hx - 0.11), 0, 0.065), conc, root, bevel=0.015)
    for k, z in enumerate((z0, z1)):
        tube(f"rail_{k}", (-x0, 0, z), (x0, 0, z), 0.02, zinc)
    tube("mid_upright", (0, 0, z0), (0, 0, z1), 0.012, zinc, lods=(0, 1, 2))
    for lod, across, up in ((0, 30, 7), (1, 14, 5), (2, 6, 0)):
        t = (0.01, 0.014, 0.02)[lod]
        for i in range(1, across):
            x = -x0 + 2 * x0 * i / across
            box(f"wire_v{lod}_{i}", (t, t, z1 - z0), (x, 0, (z0 + z1) / 2), wire, root, lods=(lod,))
        for j in range(1, up):
            z = z0 + (z1 - z0) * j / up
            box(f"wire_h{lod}_{j}", (2 * x0, t, t), (0, 0, z), wire, root, lods=(lod,))
    return [hx, hy, hz]


@kind
def skip_bin():
    """A builder's skip: sloped ends, a rolled rim, lifting lugs, a load of broken spoil."""
    hx, hy, hz = 1.8, 0.85, 0.6
    skin = enamel("skip_paint", (0.42, 0.27, 0.035), chip=1.0, dirt=0.9, rise=0.9, streak=0.7)
    spoil = textured("spoil", "soil", colour=(0.16, 0.15, 0.13), chip=0.0, dirt=0.3, streak=0.0)
    top = 2 * hz
    wide = lambda z: 0.82 + 0.18 * z / top  # the sides lean out to the rim
    prism("skip", [(-1.15, 0.0), (1.15, 0.0), (hx - 0.03, top - 0.04), (-hx + 0.03, top - 0.04)], 2 * hy - 0.06,
          (0, 0, 0), skin, root, bevel=0.02, taper_y=wide)
    for k, s in enumerate((-1, 1)):
        box(f"rim_side_{k}", (2 * hx, 0.07, 0.07), (0, s * (hy - 0.035), top - 0.035), skin, root, bevel=0.012)
        box(f"rim_end_{k}", (0.07, 2 * hy, 0.07), (s * (hx - 0.035), 0, top - 0.035), skin, root, bevel=0.012)
        for j, x in enumerate((-0.75, 0.0, 0.75)):
            box(f"rib_{k}_{j}", (0.07, 0.04, top - 0.16), (x, s * (hy * wide(top / 2) - 0.02), top / 2), skin, root,
                rot=(s * -math.atan2(0.18 * (hy - 0.03), top), 0, 0), lods=(0, 1))
        for j, x in enumerate((-1.2, 1.2)):
            cyl(f"lug_{k}_{j}", 0.04, 0.09, (x, s * (hy - 0.01), top - 0.2), "Y", skin, root, seg=8, lods=(0, 1))

    def load(bm, lod):
        for k, (x, y, r) in enumerate(((-0.9, 0.1, 0.5), (-0.2, -0.15, 0.6), (0.5, 0.12, 0.55), (1.05, -0.05, 0.45))):
            lump(bm, lod, (x, y, top - 0.17), (r, 0.6, 0.2), 0.3, 7.0 + k)

    mesh_part("load", load, spoil, root, lods=(0, 1, 2))
    box("load_bed", (2 * hx - 0.14, 2 * hy - 0.14, 0.04), (0, 0, top - 0.045), spoil, root)  # spoil to the rim
    return [hx, hy, hz]


@kind
def pallet_stack():
    """Eight pallets, 1.2 m by 1.0 m, each a little out of line with the one under it."""
    hx, hy, hz = 0.6, 0.5, 0.6
    wood = textured("pallet", "pallet_wood", chip=0.4, dirt=0.8, rise=0.5, mottle=0.4)
    rise = 2 * hz / 8
    board, block = 0.022, rise - 2 * 0.022
    for k in range(8):
        dx, dy, yaw = rng.uniform(-0.02, 0.02), rng.uniform(-0.02, 0.02), rng.uniform(-0.03, 0.03)
        z = k * rise
        c, s = math.cos(yaw), math.sin(yaw)

        def at(x, y, zz):
            return (dx + x * c - y * s, dy + x * s + y * c, z + zz)

        for j, y in enumerate((-0.45, 0.0, 0.45)):
            box(f"p{k}_foot_{j}", (1.16, 0.1, board), at(0, y, board / 2), wood, root, rot=(0, 0, yaw), lods=(0,))
            box(f"p{k}_bearer_{j}", (1.16, 0.1, block), at(0, y, board + block / 2), wood, root, rot=(0, 0, yaw),
                lods=(0, 1))
        for j in range(7):
            box(f"p{k}_board_{j}", (0.1, 0.96, board), at(-0.53 + j * 0.53 / 3, 0, rise - board / 2), wood, root,
                rot=(0, 0, yaw), lods=(0,))
        box(f"p{k}_deck", (1.16, 0.96, board), at(0, 0, rise - board / 2), wood, root, rot=(0, 0, yaw), lods=(1, 2))
    box("stack_core", (0.9, 0.7, 2 * hz - 0.04), (0, 0, hz), wood, root, lods=(2,))
    box("stack_far", (1.16, 0.96, 2 * hz), (0, 0, hz), wood, root, lods=(3,))
    return [hx, hy, hz]


@kind
def site_cabin():
    """A steel store: profiled walls in a welded frame on four feet, one door and one
    shuttered opening on the long face at -Y, both closed."""
    hx, hy, hz = 3.0, 1.2, 1.3
    wall = textured("cabin_wall", "cladding", colour=(0.07, 0.13, 0.11), chip=0.5, dirt=0.8, rise=0.8, streak=0.5)
    frame = enamel("cabin_frame", (0.055, 0.1, 0.085), chip=0.6, dirt=0.8, rise=0.8)
    door = enamel("cabin_door", (0.085, 0.15, 0.125), chip=0.6, dirt=0.7, rise=0.8, streak=0.5)
    iron = textured("cabin_iron", "bare_steel", chip=0.5, dirt=0.6)
    foot, top = 0.1, 2 * hz
    box("body", (2 * hx - 0.06, 2 * hy - 0.06, top - foot - 0.04), (0, 0, (top + foot - 0.04) / 2), wall, root)
    box("roof", (2 * hx, 2 * hy, 0.06), (0, 0, top - 0.03), frame, root, bevel=0.015)
    box("sole", (2 * hx, 2 * hy, 0.1), (0, 0, foot + 0.05), frame, root, bevel=0.01)
    for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        box(f"corner_{k}", (0.1, 0.1, top - foot), (sx * (hx - 0.05), sy * (hy - 0.05), (top + foot) / 2), frame, root,
            lods=(0, 1, 2))
        box(f"foot_{k}", (0.16, 0.16, foot), (sx * (hx - 0.2), sy * (hy - 0.15), foot / 2), iron, root, lods=(0, 1, 2))
    face = -hy + 0.025
    box("door", (0.95, 0.04, 2.0), (-1.9, face, foot + 0.1 + 1.0), door, root, bevel=0.01)
    box("door_frame", (1.07, 0.03, 2.1), (-1.9, face + 0.012, foot + 0.1 + 1.02), frame, root, lods=(0, 1, 2))
    box("lock_bar", (0.04, 0.03, 1.7), (-1.55, face - 0.03, foot + 1.1), iron, root, lods=(0, 1))
    for j, z in enumerate((0.55, 1.75)):
        box(f"hinge_{j}", (0.05, 0.03, 0.14), (-2.33, face - 0.025, foot + z), iron, root, lods=(0,))
    box("shutter", (1.3, 0.04, 1.0), (0.9, face, 1.55), door, root, bevel=0.01)
    box("shutter_frame", (1.42, 0.03, 1.12), (0.9, face + 0.012, 1.55), frame, root, lods=(0, 1, 2))
    box("shutter_bar", (1.36, 0.03, 0.05), (0.9, face - 0.03, 1.55), iron, root, lods=(0, 1))
    return [hx, hy, hz]


@kind
def traffic_cone():
    hx, hy, hz = 0.2, 0.2, 0.37
    orange = plastic("cone_plastic", (0.55, 0.13, 0.02), dirt=0.6, rise=0.25)
    white = plastic("cone_band", (0.62, 0.62, 0.58), dirt=0.3, rise=0.2)
    r = lambda z: 0.135 - 0.105 * (z - 0.03) / 0.7  # the cone's radius at `z`
    box("base", (2 * hx, 2 * hy, 0.03), (0, 0, 0.015), orange, root, bevel=0.008)
    cyl("cone", r(0.03), 0.7, (0, 0, 0.38), "Z", orange, root, seg=14, r2=r(0.73))
    cyl("band", r(0.4) + 0.004, 0.16, (0, 0, 0.48), "Z", white, root, seg=14, r2=r(0.56) + 0.004, caps=False,
        lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def road_barrier():
    """A plastic works barrier: two posts in flat feet, a banded board at the top and a
    plain one under it, open between."""
    hx, hy, hz = 1.0, 0.25, 0.5
    red = plastic("barrier_red", (0.5, 0.045, 0.03), dirt=0.6, rise=0.5)
    white = plastic("barrier_white", (0.62, 0.62, 0.58), dirt=0.6, rise=0.5)
    foot_m = textured("barrier_foot", "rubber", chip=0.0, dirt=0.7, streak=0.0)
    for k, s in enumerate((-1, 1)):
        box(f"foot_{k}", (0.2, 2 * hy, 0.06), (s * 0.82, 0, 0.03), foot_m, root, bevel=0.01)
        box(f"post_{k}", (0.05, 0.04, 2 * hz - 0.04), (s * 0.82, 0, hz + 0.02), red, root)
    for k in range(5):
        box(f"band_{k}", (0.4, 0.035, 0.2), (-0.8 + 0.4 * k, 0, 2 * hz - 0.1), (red, white)[k % 2], root,
            lods=(0, 1, 2))
    box("board_far", (2 * hx, 0.035, 0.2), (0, 0, 2 * hz - 0.1), red, root, lods=(3,))
    box("board_low", (2 * hx, 0.03, 0.12), (0, 0, 0.5), red, root)
    return [hx, hy, hz]


@kind
def scaffold():
    """One bay of tube-and-fitting scaffold, 2.5 m by 1 m: four standards 4 m tall, ledgers
    and transoms at the foot, the boarded lift and the head, a guard rail and toe board on
    the outer face (-Y) and a brace across it. The standards stand on the module's ends,
    so repeated bays share them."""
    hx, hy, hz = 1.25, 0.5, 2.0
    zinc = steel("scaffold_tube")
    wood = textured("scaffold_board", "pallet_wood", chip=0.4, dirt=0.6, rise=0.4, mottle=0.4)
    r, x0, y0, top = 0.024, hx - 0.026, hy - 0.026, 2 * hz
    for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        tube(f"standard_{k}", (sx * x0, sy * y0, 0.01), (sx * x0, sy * y0, top), r, zinc)
        box(f"base_plate_{k}", (0.15, 0.15, 0.012), (sx * (hx - 0.075), sy * (hy - 0.075), 0.006), zinc, root, lods=(0, 1))
    for j, z in enumerate((0.25, 1.95, top - 0.1)):
        for k, sy in enumerate((-1, 1)):
            tube(f"ledger_{j}_{k}", (-hx, sy * y0, z), (hx, sy * y0, z), r, zinc, lods=(0, 1, 2) if j == 0 else TIERS)
        for k, sx in enumerate((-1, 1)):
            tube(f"transom_{j}_{k}", (sx * x0, -hy, z + 0.05), (sx * x0, hy, z + 0.05), r, zinc,
                 lods=(0, 1, 2) if j == 0 else TIERS)
    for j, z in enumerate((2.5, 3.0)):
        tube(f"guard_rail_{j}", (-hx, -y0, z), (hx, -y0, z), r, zinc, lods=(0, 1, 2))
    tube("brace", (-x0, -y0 - 0.05, 0.3), (x0, -y0 - 0.05, 1.9), r, zinc, lods=(0, 1, 2))
    for k in range(4):
        box(f"board_{k}", (2 * hx, 0.22, 0.04), (0, -0.345 + 0.23 * k, 2.045), wood, root, lods=(0, 1))
    box("deck_far", (2 * hx, 0.92, 0.04), (0, 0, 2.045), wood, root, lods=(2, 3))
    box("toe_board", (2 * hx, 0.03, 0.15), (0, -y0 + 0.03, 2.14), wood, root, lods=(0, 1, 2))
    return [hx, hy, hz]


@kind
def scooter():
    """A step-through motor scooter on its centre stand: leg shield, footboard, a seat
    over the engine cover, small wheels."""
    hx, hy, hz = 0.9, 0.35, 0.55
    skin = enamel("scooter_paint", (0.42, 0.39, 0.3), chip=0.4, dirt=0.7, rise=0.5)
    dark = textured("scooter_trim", "rubber", chip=0.0, dirt=0.5, streak=0.0)
    zinc = steel("scooter_steel")
    lamp_m = flat_paint("lamp", (0.6, 0.6, 0.55), rough=0.2, grime=0.3)
    wr, wx = 0.21, 0.64
    for name, x in (("front", wx), ("rear", -wx)):
        cyl(f"tyre_{name}", wr, 0.11, (x, 0, wr), "Y", dark, root, seg=18, bevel=0.02)
        cyl(f"rim_{name}", 0.12, 0.115, (x, 0, wr), "Y", zinc, root, seg=12, lods=(0, 1, 2))
    box("footboard", (0.5, 0.3, 0.06), (0.1, 0, 0.3), skin, root, bevel=0.02)
    # the engine cover and tail, under the seat
    prism("tail", [(-0.9, 0.5), (-0.86, 0.32), (-0.25, 0.26), (-0.12, 0.34), (-0.12, 0.7), (-0.8, 0.72)], 0.32,
          (0, 0, 0), skin, root, bevel=0.03)
    box("seat", (0.7, 0.28, 0.09), (-0.45, 0, 0.77), dark, root, bevel=0.035)
    box("rack", (0.16, 0.22, 0.02), (-0.82, 0, 0.76), zinc, root, lods=(0, 1))
    # the leg shield, raked back to the headset; the mudguard over the front wheel
    box("leg_shield", (0.05, 0.36, 0.62), (0.4, 0, 0.6), skin, root, rot=(0, math.radians(-14), 0), bevel=0.02)
    prism("mudguard", [(0.4, 0.42), (0.52, 0.47), (0.76, 0.47), (0.9, 0.36), (0.86, 0.33), (0.74, 0.42), (0.54, 0.42),
                       (0.44, 0.38)], 0.16, (0, 0, 0), skin, root, bevel=0.01, lods=(0, 1, 2))
    tube("fork", (0.64, 0, wr), (0.42, 0, 0.92), 0.035, dark, lods=(0, 1, 2))
    box("headset", (0.16, 0.2, 0.14), (0.44, 0, 0.94), skin, root, bevel=0.03)
    cyl("headlamp", 0.06, 0.04, (0.53, 0, 0.94), "X", lamp_m, root, seg=10, lods=(0, 1))
    tube("handlebar", (0.4, -0.31, 1.0), (0.4, 0.31, 1.0), 0.016, dark, lods=(0, 1, 2))
    for k, s in enumerate((-1, 1)):
        tube(f"mirror_stem_{k}", (0.4, s * 0.24, 1.0), (0.38, s * 0.3, 1.08), 0.008, dark, lods=(0,))
        box(f"mirror_{k}", (0.02, 0.09, 0.05), (0.38, s * 0.3, 2 * hz - 0.025), dark, root, lods=(0, 1))
        tube(f"stand_{k}", (-0.2, s * 0.03, 0.3), (-0.1, s * 0.16, 0.0), 0.014, zinc, lods=(0, 1))
    box("tail_lamp", (0.03, 0.12, 0.05), (-0.9, 0, 0.56), flat_paint("tail_lamp", (0.22, 0.02, 0.015), rough=0.25), root,
        lods=(0, 1))
    return [hx, hy, hz]


box_half = KINDS[KIND]()
rest_on_ground()
bpy.context.view_layer.update()
finish(ao_distance=1.0, ao_strength=0.6, ao_rays=10, paint_scale=1.3)
print("PROP", json.dumps(dict(kind=KIND, box=box_half, tris=triangles_by_tier())))
export(OUT)
