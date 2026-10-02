"""Warehouses and light industry: a town's sheds, works and depots, as one kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/industry.py

writes `assets/source/city/industry/kit.glb` and `templates.json` (the format is in
this folder's readme). Five buildings, each its own structure: a steel workshop
shed on a block dado, a precast concrete warehouse with a flat roof and a row of
loading docks, a twin-span steel distribution warehouse, a brick works under a
sawtooth roof with a rendered office block joined to it, and a blockwork depot
with a roof monitor. Plain and regional-neutral; they carry the first shipping
family's name until a family has industry of its own.

The physical box is the authority. Every outer wall stands on a face of a part.
A pitched or sawtooth roof ridges at its part's top and a flat roof's parapet is
the top: the box is what stops rounds and sight. Canopies, gutters, dock faces
and bollards reach past a face by at most the set's side fit; vents, stacks and
roof plant rise above the top by at most its top fit. Doors and windows sit in
the bays of their edge's 3 m lattice.

Long walls are rows: a bay of dado, a window, a pier, a canopy or a gutter is
modelled once, 3 m wide, and placed along the wall. Each template's shell (its
closed walls and its roof) is a module of its own, and the roof is where the eye
lands from the game's camera: its sheets, laps, rooflights, mends and rust are in
the shell's own texture, faces and paint, so they hold at every tier. At the two
coarse tiers a building is its shell's one row: `fold` copies what is left of the
other rows into it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from masonry import panel_door  # noqa: E402

FAMILY = "china"
UP, SOUTH = Vector((0, 0, 1)), (0, -1, 0)
OVER, VERGE = 0.25, 0.12  # how far a sheet roof's eaves and verges overhang
DROP = 0.02  # a wall stops this far under its roof

kit = Kit("industry", "industry.py", fit_side_m=1.2, fit_top_m=1.2)


# ---------------------------------------------------------------- materials
FAR = {}  # a weathered material's name -> its twin for the coarsest tier


def weather(mat, fn, far):
    """Lay `fn(colour, wear, p, n) -> (colour, wear)` over a textured material's paint: marks
    some metres across, placed by where a vertex is. The coarsest tier has a vertex at each
    corner of a wall or a roof and no more, so there the surface wears `far(colour, wear)`,
    the same marks averaged, in a twin of the material."""
    base = PAINTS[mat.name]
    twin = mat.copy()
    twin.name = mat.name + "_far"
    PAINTS[twin.name] = lambda p, n, edge: far(*base(p, n, edge))
    TEXTURED[twin.name] = TEXTURED[mat.name]
    FAR[mat.name] = twin
    PAINTS[mat.name] = lambda p, n, edge: fn(*base(p, n, edge), p, n)
    return mat


def sheet(name, colour=None, rust=0.5, soil=0.3, seed=0.0):
    """Sheet roofing in `colour`. Rust comes through in patches some metres across (the
    recipe's fixings and laps first, as the wear rises), and dirt lies in drifts."""
    mat = textured(name, "roof_sheet", colour=colour, dirt=0.0, chip=0.5, streak=0.0, seed=seed)

    def fn(c, wear, p, n):
        if n.z < 0.3:
            return c, wear
        patch = smoothstep(-0.2, 0.45, fbm(p, 0.11, 3, 71.0 + seed))
        drift = 0.5 + 0.5 * fbm(p, 0.3, 2, 83.0 + seed)
        fade = 1.0 + 0.16 * fbm(p, 0.06, 2, 61.0 + seed)  # whole stretches bleached or dulled
        c = tuple(x * fade * (1.0 - 0.45 * soil * (0.3 + 0.7 * smoothstep(0.35, 0.8, drift))) for x in c)
        c = lerp3(c, RUST, min(0.5, 0.3 * rust * patch))
        return c, max(wear, min(0.9, rust * patch * (0.55 + 0.45 * drift)))

    return weather(mat, fn, lambda c, wear: (lerp3(tuple(x * (1.0 - 0.25 * soil) for x in c), RUST, min(0.25, 0.12 * rust)),
                                             max(wear, min(0.5, 0.3 * rust))))


def rusting(mat, amount, seed=0.0):
    """A wall material whose wear (rust on sheet, bare concrete under paint) comes in patches."""

    def fn(c, wear, p, n):
        if abs(n.z) > 0.5:
            return c, wear
        patch = smoothstep(0.0, 0.6, fbm(p, 0.16, 3, 91.0 + seed))
        return c, max(wear, min(0.62, amount * patch))

    return weather(mat, fn, lambda c, wear: (c, max(wear, 0.3 * amount)))


def felt(name, seed=0.0):
    """A flat roof's felt: damp darkens it in wide pools and moss takes the wettest of them."""
    mat = textured(name, "flat_roof", dirt=0.0, chip=0.0, streak=0.0, seed=seed)

    def fn(c, wear, p, n):
        level = fbm(p, 0.085, 3, 97.0 + seed)
        damp = smoothstep(0.1, 0.4, level)
        silt = math.exp(-((level - 0.08) / 0.07) ** 2)  # the pale tide mark a pool leaves as it dries
        c = lerp3(tuple(x * (1.0 - 0.5 * damp) for x in c), (0.3, 0.29, 0.26), 0.45 * silt)
        return c, max(wear, 0.6 * damp * smoothstep(0.0, 0.5, fbm(p, 0.35, 2, 99.0 + seed)))

    return weather(mat, fn, lambda c, wear: (tuple(x * 0.8 for x in c), wear))


# Walls. The pale ones are tint-masked: a row's tint is the building's paint.
clad_m = rusting(textured("wall_cladding", "cladding", tint=1.0, dirt=0.5, chip=0.4, streak=0.3, rise=1.2), 0.5)
panel_m = textured("panel_cladding", "cladding", tint=1.0, dirt=0.0, chip=0.3, streak=0.0, seed=4.0)
block_m = rusting(textured("wall_block", "concrete_block", tint=1.0, dirt=0.7, chip=0.25, streak=0.4, rise=1.1), 0.35, 3.0)
slab_m = rusting(textured("wall_tilt_slab", "tilt_slab", tint=1.0, dirt=0.6, chip=0.3, streak=0.45, rise=1.0), 0.5, 5.0)
brick_m = textured("wall_brick", "brick", colour=(0.2, 0.09, 0.066), dirt=0.5, chip=0.12, streak=0.45, rise=0.8, mottle=0.08)
brick_high_m = textured("gable_brick", "brick", colour=(0.2, 0.09, 0.066), dirt=0.0, chip=0.12, streak=0.45, mottle=0.08)
render_m = textured("wall_render", "plaster", tint=1.0, dirt=0.55, chip=0.5, streak=0.4, rise=0.8)
dado_m = textured("dado_block", "concrete_block", colour=(0.3, 0.3, 0.29), dirt=0.8, chip=0.3, streak=0.3, rise=0.7)
plinth_m = textured("plinth_concrete", "concrete", colour=(0.2, 0.2, 0.19), dirt=0.8, chip=0.4, rise=0.5)
concrete_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.3)
pier_m = textured("pier_concrete", "concrete", colour=(0.27, 0.27, 0.255), dirt=0.7, chip=0.4, streak=0.4, rise=0.9)
# Roofs take no tint: a row has one, and it is the walls'.
ROOFS = {
    "rusty": sheet("roof_rusty", (0.27, 0.25, 0.23), rust=1.35, soil=0.5, seed=1.0),  # galvanised, long unpainted
    "pale": sheet("roof_pale", (0.6, 0.6, 0.57), rust=0.4, soil=0.7, seed=2.0),
    "green": sheet("roof_green", (0.1, 0.15, 0.115), rust=0.75, soil=0.6, seed=3.0),  # paint long faded
    "oxide": sheet("roof_oxide", (0.2, 0.082, 0.058), rust=0.6, soil=0.7, seed=4.0),
    # what a roof is mended with: new galvanised sheets, and sheets tarred over
    "new": sheet("roof_new", (0.36, 0.375, 0.39), rust=0.0, soil=0.15, seed=5.0),
    "tarred": sheet("roof_tarred", (0.05, 0.05, 0.055), rust=0.0, soil=0.3, seed=6.0),
}
felt_m = felt("roof_felt")
FELTS = {"new": textured("felt_new", "flat_roof", colour=(0.09, 0.09, 0.09), dirt=0.0, chip=0.0, streak=0.0, seed=2.0),
         "silver": textured("felt_silver", "flat_roof", colour=(0.4, 0.41, 0.43), dirt=0.0, chip=0.0, streak=0.0, seed=3.0)}
# Translucent sheets in a roof: pale against a dark roof, dull against a pale one.
ROOFLIGHTS = {"pale": textured("rooflight_pale", "roof_sheet", colour=(0.5, 0.52, 0.41), dirt=0.0, chip=0.0, streak=0.0, seed=7.0),
              "dull": textured("rooflight_dull", "roof_sheet", colour=(0.27, 0.31, 0.29), dirt=0.0, chip=0.0, streak=0.0, seed=8.0)}
canopy_m = textured("canopy_sheet", "roof_sheet", colour=(0.34, 0.345, 0.35), dirt=0.0, chip=0.5, streak=0.0, seed=8.0)
glazing_m = textured("glazing", "factory_glazing", dirt=0.0, chip=0.0, streak=0.0)
slats_m = textured("shutter_slats", "roller_slats", tint=1.0, dirt=0.5, chip=0.4, streak=0.25, rise=0.8)
louvre_m = textured("louvre_blades", "roller_slats", colour=(0.13, 0.135, 0.14), dirt=0.0, chip=0.3, streak=0.0, seed=2.0)
trim_m = flat_paint("trim_steel", (0.1, 0.105, 0.11), rough=0.7, grime=0.0)
galv_m = flat_paint("galvanised", (0.36, 0.37, 0.38), rough=0.5, metal=0.25, grime=0.0)
frame_m = flat_paint("window_frame", (0.3, 0.3, 0.29), rough=0.6, grime=0.0)
glass_m = flat_paint("window_glass", (0.02, 0.025, 0.03), rough=0.08, grime=0.0)
rubber_m = flat_paint("dock_rubber", (0.02, 0.02, 0.02), rough=0.9, grime=0.0)
hazard_m = flat_paint("hazard_yellow", (0.5, 0.36, 0.035), rough=0.6, wear=0.6, grime=0.6)
dome_m = flat_paint("skylight_dome", (0.55, 0.6, 0.58), rough=0.3, grime=0.0)
door_m = flat_paint("door_steel", (0.5, 0.5, 0.49), rough=0.5, wear=0.5, grime=0.4)
band_m = flat_paint("band_paint", (0.55, 0.55, 0.54), rough=0.7, grime=0.0)
door_m["tint"] = band_m["tint"] = 1.0

# Tints, sRGB: paints on sheet, concrete and render, and the doors.
SAGE, IVORY, STONE, CREAM, BUFF = (112, 140, 118), (232, 230, 220), (214, 208, 196), (230, 216, 188), (212, 200, 168)
NAVY, RUSSET, SLATE, BOTTLE, PRIMER = (52, 84, 132), (150, 66, 48), (78, 88, 100), (58, 86, 70), (150, 92, 76)

# ---------------------------------------------------------------- builders
FITTING = dict(ao_distance=0.4, paint_scale=8.0)  # small parts: no face needs splitting for paint


def face(bm, points, toward):
    """A face through `points` whose normal leans `toward`."""
    f = bm.faces.new([bm.verts.new(p) for p in points])
    f.normal_update()
    if f.normal.dot(Vector(toward)) < 0:
        f.normal_flip()
    return f


def cells(points, size):
    """A four-sided face as a grid of faces no longer than `size` a side."""
    a, b, c, d = (Vector(p) for p in points)
    nu = max(1, math.ceil(max((b - a).length, (c - d).length) / size - 1e-6))
    nv = max(1, math.ceil(max((d - a).length, (c - b).length) / size - 1e-6))

    def at(i, j):
        return tuple(a.lerp(b, i / nu).lerp(d.lerp(c, i / nu), j / nv))

    return [[at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)] for j in range(nv) for i in range(nu)]


ROOF_CELL_M = 4.4  # a roof's paint (rust, damp, fading) lives on vertices: this keeps them down to the third tier


def surface(m, tag, faces, mat, lods=TIERS, uv=None, grid=None):
    """One mesh of flat faces, each `(points, toward)` or `(points, toward, uv)`. With no
    `uv` the export box-projects it. `"slope"` is a roof's own: u along the eave and v up
    the slope, so a sheet's ribs run with the fall. `(origin, u, v)` lays the recipe from
    a corner, so a window's panes start at its own edge. `grid` cuts every face into
    cells that size at all but the coarsest tier."""
    tile = textures.tile_of(TEXTURED[mat.name]) if mat.name in TEXTURED else 1.0

    def build(bm, lod):
        layer = bm.loops.layers.uv.new("UVMap") if uv is not None or any(len(f) > 2 for f in faces) else None
        for points, toward, *own in faces:
            how = own[0] if own else uv
            for quad in cells(points, grid) if grid and lod < 3 else [points]:
                f = face(bm, quad, toward)
                if how == "slope":
                    origin, u = Vector((0, 0, 0)), UP.cross(f.normal).normalized()
                    v = f.normal.cross(u)
                elif how is not None:
                    origin, u, v = (Vector(x) for x in how)
                for loop in f.loops if how is not None else ():
                    d = loop.vert.co - origin
                    loop[layer].uv = (d.dot(u) / tile, d.dot(v) / tile)

    return mesh_part(m.n(tag), build, mat, m.root, lods)


def skin(m, tag, faces, mat, **how):
    """A weathered surface: `mat` at the tiers that have the vertices for its marks, its twin at the coarsest."""
    surface(m, tag, faces, mat, lods=(0, 1, 2), **how)
    surface(m, tag + "_far", faces, FAR.get(mat.name, mat), lods=(3,), **how)


FOOT_M = 1.3


def upright(p, q, top, out, peak=None):
    """A wall from `p` to `q` (x, y) as two faces, its foot and the rest: mud and the
    ground's shade then stay in the foot at every tier. `peak` raises a gable's apex
    over its middle."""
    (x0, y0), (x1, y1) = p, q
    rest = [(x0, y0, FOOT_M), (x1, y1, FOOT_M), (x1, y1, top)] + ([((x0 + x1) / 2, (y0 + y1) / 2, peak)] if peak else []) + [(x0, y0, top)]
    return [([(x0, y0, 0), (x1, y1, 0), (x1, y1, FOOT_M), (x0, y0, FOOT_M)], out), (rest, out)]


def box_walls(x0, x1, y0, y1, top):
    return (upright((x0, y0), (x1, y0), top, (0, -1, 0)) + upright((x0, y1), (x1, y1), top, (0, 1, 0))
            + upright((x0, y0), (x0, y1), top, (-1, 0, 0)) + upright((x1, y0), (x1, y1), top, (1, 0, 0)))


def front(x0, x1, z0, z1, y=0.0):
    """A rectangle on (or `-y` proud of) a wall-mounted module's wall plane."""
    return [(x0, y, z0), (x1, y, z0), (x1, y, z1), (x0, y, z1)]


def bar(name, p, q, width, height, mat, parent, lods=(0, 1, 2)):
    """A straight bar from `p` to `q`: a flashing, a fascia, a tie rod."""
    p, q = Vector(p), Vector(q)
    d = q - p
    box(name, (d.length, width, height), tuple((p + q) / 2), mat, parent,
        rot=(0, -math.asin(d.z / d.length), math.atan2(d.y, d.x)), lods=lods)


def skirt(m, tag, x0, x1, y0, y1, z0, z1, mat, out=0.0, lods=(0, 1, 2)):
    """A band round a rectangle's four sides, `out` proud of them: a plinth, a ring beam.
    Faces only: a box this size would hide a floor's worth of triangles inside the walls."""
    x0, x1, y0, y1 = x0 - out, x1 + out, y0 - out, y1 + out
    surface(m, tag, [([(x0, y0, z0), (x1, y0, z0), (x1, y0, z1), (x0, y0, z1)], (0, -1, 0)),
                     ([(x0, y1, z0), (x1, y1, z0), (x1, y1, z1), (x0, y1, z1)], (0, 1, 0)),
                     ([(x0, y0, z0), (x0, y1, z0), (x0, y1, z1), (x0, y0, z1)], (-1, 0, 0)),
                     ([(x1, y0, z0), (x1, y1, z0), (x1, y1, z1), (x1, y0, z1)], (1, 0, 0))], mat, lods)


def glazed(name, w, h, jambs=True, stone=False):
    """A window of factory glazing, its sill's centre on the wall plane at the origin."""
    m = kit.module(name, **FITTING)
    surface(m, "glass", [(front(-w / 2, w / 2, 0, h, -0.03), SOUTH)], glazing_m, uv=((-w / 2, 0, 0), (1, 0, 0), (0, 0, 1)))
    if stone:  # a brick wall's opening: a concrete lintel and sill
        box(m.n("lintel"), (w + 0.5, 0.12, 0.3), (0, -0.06, h + 0.15), concrete_m, m.root, lods=(0, 1))
        box(m.n("sill"), (w + 0.3, 0.18, 0.14), (0, -0.09, -0.07), concrete_m, m.root, lods=(0, 1))
    else:
        box(m.n("head"), (w + 0.16, 0.1, 0.08), (0, -0.05, h + 0.04), frame_m, m.root, lods=(0,))
        box(m.n("sill"), (w + 0.2, 0.14, 0.07), (0, -0.07, -0.035), frame_m, m.root, lods=(0, 1))
    for s in (-1, 1) if jambs else ():
        box(m.n(f"jamb_{'ab'[s > 0]}"), (0.08, 0.1, h), (s * (w / 2 + 0.04), -0.05, h / 2), frame_m, m.root, lods=(0,))


# ---------------------------------------------------------------- wall modules, a bay each
glazed("strip_window", 2.5, 1.5)
glazed("clerestory", 3.0, 0.75, jambs=False)  # these two fill their bay: side by side they are one band of glass
glazed("window_band", 3.0, 1.5, jambs=False)
glazed("factory_window", 2.0, 4.5, stone=True)

m = kit.module("office_window", **FITTING)  # plain glass in a painted frame, three lights
surface(m, "glass", [(front(-0.8, 0.8, 0, 1.4, -0.02), SOUTH)], glass_m)
box(m.n("sill"), (1.84, 0.16, 0.07), (0, -0.08, -0.035), concrete_m, m.root, lods=(0, 1))
box(m.n("head"), (1.8, 0.1, 0.1), (0, -0.05, 1.45), frame_m, m.root, lods=(0, 1))
box(m.n("transom"), (1.6, 0.05, 0.05), (0, -0.045, 1.0), frame_m, m.root, lods=(0,))
for k, x in enumerate((-0.84, -0.27, 0.27, 0.84)):
    box(m.n(f"post_{k}"), (0.08 if abs(x) > 0.5 else 0.05, 0.1 if abs(x) > 0.5 else 0.05, 1.4), (x, -0.05, 0.7), frame_m, m.root, lods=(0,))

# A sheet of cladding over the wall's own: a band of colour, or a bay re-sheeted in another paint.
m = kit.module("clad_panel", **FITTING)
surface(m, "sheet", [(front(-1.5, 1.5, 0, 3.0, -0.03), SOUTH)], panel_m)

m = kit.module("paint_band", **FITTING)
surface(m, "band", [(front(-1.5, 1.5, 0, 1.0, -0.015), SOUTH)], band_m)

DADO_M = 1.2
m = kit.module("dado_bay", ground=True, **FITTING)
box(m.n("blocks"), (3.0, 0.12, DADO_M), (0, -0.06, DADO_M / 2), dado_m, m.root, lods=(0, 1))
surface(m, "face", [(front(-1.5, 1.5, 0, DADO_M, -0.12), SOUTH)], dado_m, lods=(2, 3))
box(m.n("capping"), (3.0, 0.17, 0.06), (0, -0.085, DADO_M + 0.03), concrete_m, m.root, lods=(0, 1))

# A roller shutter two bays wide, its threshold's centre at the origin: a vehicle door.
ROLLER_W, ROLLER_H = 5.0, 4.2
m = kit.module("roller_door", ground=True, **FITTING)
surface(m, "curtain", [(front(-ROLLER_W / 2, ROLLER_W / 2, 0, ROLLER_H, -0.04), SOUTH)], slats_m)
box(m.n("hood"), (ROLLER_W + 0.5, 0.36, 0.45), (0, -0.18, ROLLER_H + 0.225), trim_m, m.root, lods=(0, 1, 2))
box(m.n("sill"), (ROLLER_W, 0.09, 0.12), (0, -0.085, 0.06), hazard_m, m.root, lods=(0, 1))
box(m.n("apron"), (ROLLER_W + 0.6, 1.0, 0.05), (0, -0.5, 0.025), plinth_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"guide_{'ab'[s > 0]}"), (0.16, 0.14, ROLLER_H), (s * (ROLLER_W / 2 + 0.08), -0.07, ROLLER_H / 2), trim_m, m.root, lods=(0, 1))
    cyl(m.n(f"bollard_{'ab'[s > 0]}"), 0.09, 1.0, (s * (ROLLER_W / 2 + 0.45), -0.8, 0.5), "Z", hazard_m, m.root, seg=10, lods=(0, 1))

# A loading dock, a bay wide: a sectional door at lorry-bed height in a rubber seal,
# over the dock's concrete face, a leveller's lip and two bumpers.
DOCK_SILL, DOCK_W, DOCK_H = 1.1, 2.7, 3.0
m = kit.module("dock_door", ground=True, **FITTING)
surface(m, "door", [(front(-DOCK_W / 2, DOCK_W / 2, DOCK_SILL, DOCK_SILL + DOCK_H, -0.04), SOUTH)], slats_m)
box(m.n("face"), (3.0, 0.16, DOCK_SILL), (0, -0.08, DOCK_SILL / 2), plinth_m, m.root, lods=(0, 1))
box(m.n("seal_head"), (DOCK_W + 0.5, 0.3, 0.35), (0, -0.15, DOCK_SILL + DOCK_H + 0.175), rubber_m, m.root, lods=(0, 1))
surface(m, "seal", [(front(-DOCK_W / 2 - 0.25, DOCK_W / 2 + 0.25, DOCK_SILL, DOCK_SILL + DOCK_H + 0.35, -0.03), SOUTH)], rubber_m, lods=(2,))
box(m.n("lip"), (2.1, 0.45, 0.05), (0, -0.38, DOCK_SILL - 0.03), galv_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"seal_{'ab'[s > 0]}"), (0.3, 0.3, DOCK_H), (s * (DOCK_W / 2 + 0.1), -0.15, DOCK_SILL + DOCK_H / 2), rubber_m, m.root,
        lods=(0, 1))
    box(m.n(f"bumper_{'ab'[s > 0]}"), (0.25, 0.14, 0.5), (s * 1.2, -0.23, DOCK_SILL - 0.3), rubber_m, m.root, lods=(0, 1))

m = kit.module("personnel_door", ground=True, **FITTING)
panel_door(m.n("door"), 0.95, 2.1, door_m, frame_m, m.root)
box(m.n("canopy"), (1.5, 0.6, 0.06), (0, -0.3, 2.42), trim_m, m.root, lods=(0, 1))
box(m.n("step"), (1.4, 0.5, 0.1), (0, -0.25, 0.05), plinth_m, m.root, lods=(0, 1))

m = kit.module("office_door", ground=True, **FITTING)
panel_door(m.n("door"), 1.5, 2.1, door_m, frame_m, m.root, glass_m, light=0.4)
box(m.n("canopy"), (2.8, 1.0, 0.12), (0, -0.5, 2.9), concrete_m, m.root, lods=(0, 1, 2))
box(m.n("step"), (2.4, 0.8, 0.14), (0, -0.4, 0.07), plinth_m, m.root, lods=(0, 1))

# A canopy's bay: sheet falling away from the wall, hung on tie rods from above.
CANOPY_M = 1.15
m = kit.module("canopy_bay", **FITTING)
top = [(-1.5, 0, 0.28), (1.5, 0, 0.28), (1.5, -CANOPY_M, 0.08), (-1.5, -CANOPY_M, 0.08)]
surface(m, "sheet", [(top, UP)], canopy_m, uv="slope")
surface(m, "soffit", [([(x, y, z - 0.05) for x, y, z in top], -UP)], trim_m, lods=(0, 1))
box(m.n("fascia"), (3.0, 0.05, 0.2), (0, -CANOPY_M, 0.02), trim_m, m.root, lods=(0, 1))
for s in (-1, 1):
    bar(m.n(f"rod_{'ab'[s > 0]}"), (s * 0.75, -CANOPY_M + 0.1, 0.1), (s * 0.75, 0, 1.0), 0.04, 0.04, trim_m, m.root, lods=(0, 1))

m = kit.module("louvre", **FITTING)
surface(m, "blades", [(front(-0.75, 0.75, 0, 0.8, -0.04), SOUTH)], louvre_m)
box(m.n("head"), (1.66, 0.1, 0.08), (0, -0.05, 0.84), frame_m, m.root, lods=(0, 1))
box(m.n("sill"), (1.66, 0.1, 0.08), (0, -0.05, -0.04), frame_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"jamb_{'ab'[s > 0]}"), (0.08, 0.1, 0.8), (s * 0.79, -0.05, 0.4), frame_m, m.root, lods=(0, 1))

# Piers, each the height of its own building's wall.
WORKS_WALL_M, DEPOT_WALL_M = 6.2, 7.9
m = kit.module("works_pier", ground=True, **FITTING)
box(m.n("brick"), (0.6, 0.22, WORKS_WALL_M), (0, -0.11, WORKS_WALL_M / 2), brick_m, m.root)
box(m.n("cap"), (0.72, 0.28, 0.14), (0, -0.14, WORKS_WALL_M - 0.07), concrete_m, m.root, lods=(0, 1))
m = kit.module("depot_pier", ground=True, **FITTING)
box(m.n("column"), (0.5, 0.2, DEPOT_WALL_M), (0, -0.1, DEPOT_WALL_M / 2), pier_m, m.root)

# Rainwater: a bay of gutter, and a metre of downpipe a row stretches to the eaves.
m = kit.module("gutter_bay", **FITTING)
box(m.n("trough"), (3.0, 0.16, 0.12), (0, -0.08, -0.06), trim_m, m.root)
m = kit.module("downpipe", **FITTING)
cyl(m.n("pipe"), 0.06, 1.0, (0, -0.08, 0.5), "Z", trim_m, m.root, seg=6, caps=False)

m = kit.module("ladder", **FITTING)  # 3 m of cat ladder
for s in (-1, 1):
    box(m.n(f"rail_{'ab'[s > 0]}"), (0.05, 0.05, 3.0), (s * 0.25, -0.2, 1.5), galv_m, m.root)
    for k in range(2):
        box(m.n(f"stay_{'ab'[s > 0]}_{k}"), (0.04, 0.2, 0.04), (s * 0.25, -0.1, 0.4 + 2.2 * k), galv_m, m.root, lods=(0,))
for k in range(10):
    box(m.n(f"rung_{k}"), (0.5, 0.03, 0.03), (0, -0.2, 0.15 + 0.3 * k), galv_m, m.root, lods=(0,))

# ---------------------------------------------------------------- roof furniture
m = kit.module("turbine_vent", **FITTING)
cyl(m.n("neck"), 0.2, 0.4, (0, 0, 0.2), "Z", galv_m, m.root, seg=10, lods=(0, 1))
cyl(m.n("head"), 0.32, 0.3, (0, 0, 0.55), "Z", galv_m, m.root, seg=12, r2=0.27)
cyl(m.n("cap"), 0.27, 0.1, (0, 0, 0.75), "Z", galv_m, m.root, seg=12, r2=0.05, lods=(0, 1))

m = kit.module("ridge_vent", **FITTING)  # 3 m of ventilator astride a ridge, along +X
prism(m.n("cowl"), [(-0.35, -0.15), (0.35, -0.15), (0.35, 0.3), (0.0, 0.46), (-0.35, 0.3)], 3.0, (0, 0, 0), galv_m, m.root,
      rot=(0, 0, math.pi / 2))
box(m.n("throat"), (2.9, 0.74, 0.14), (0, 0, 0.12), trim_m, m.root, lods=(0, 1))

m = kit.module("skylight_dome", **FITTING)
box(m.n("curb"), (1.3, 1.3, 0.25), (0, 0, 0.125), trim_m, m.root, lods=(0, 1))
box(m.n("dome"), (1.2, 1.2, 0.3), (0, 0, 0.4), dome_m, m.root, taper=(0.55, 0.55))

m = kit.module("roof_unit", **FITTING)  # a packaged air handler on skids
box(m.n("case"), (2.2, 1.3, 0.9), (0, 0, 0.65), galv_m, m.root)
cyl(m.n("fan"), 0.4, 0.08, (-0.45, 0, 1.14), "Z", trim_m, m.root, seg=14, lods=(0, 1, 2))
box(m.n("grille"), (1.6, 0.03, 0.5), (0, -0.66, 0.65), trim_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"skid_{'ab'[s > 0]}"), (0.12, 1.3, 0.2), (s * 0.8, 0, 0.1), trim_m, m.root, lods=(0, 1))

m = kit.module("duct_run", **FITTING)  # 3 m of duct on feet, along +X
box(m.n("duct"), (3.0, 0.5, 0.4), (0, 0, 0.5), galv_m, m.root)
for s in (-1, 1):
    box(m.n(f"foot_{'ab'[s > 0]}"), (0.08, 0.6, 0.3), (s * 1.0, 0, 0.15), trim_m, m.root, lods=(0, 1))
    box(m.n(f"flange_{'ab'[s > 0]}"), (0.05, 0.56, 0.46), (s * 1.475, 0, 0.5), galv_m, m.root, lods=(0, 1))

m = kit.module("flue_stack", **FITTING)
cyl(m.n("flue"), 0.18, 1.5, (0, 0, 0.75), "Z", galv_m, m.root, seg=10)
cyl(m.n("collar"), 0.27, 0.12, (0, 0, 0.06), "Z", trim_m, m.root, seg=10, lods=(0, 1))
cyl(m.n("cowl"), 0.28, 0.15, (0, 0, 1.63), "Z", trim_m, m.root, seg=10, r2=0.08, lods=(0, 1))


# ---------------------------------------------------------------- shells
def gabled_shell(m, length, depth, eave, ridge, wall, roof, spans=1, lights=None, mends=(), monitor=None, plinth=0.0):
    """Walls and sheet roof of a hall `length` along x by `depth`: `spans` gables side by
    side across the depth, their ridges along x at `ridge`. `lights` lays translucent
    sheets up the slopes: (their material, the first one's distance from the end, their
    spacing, the stretch of slope each covers). `mends` are stretches of other sheet:
    (span, side, x, width, the stretch of slope, its material). `monitor` (half width,
    wall height, inset from each end) raises a glazed lantern along the ridge, its own
    ridge that much higher. Returns the pitch's tangent and the height of the eaves' edge."""
    hx, hy, half = length / 2, depth / 2, depth / spans / 2
    tan = (ridge - eave) / half
    edge_z = eave - OVER * tan
    centres = [-hy + (2 * k + 1) * half for k in range(spans)]

    def on(yk, s, x, t, lift=0.0):
        """A point on a slope: `t` 0 over the wall, 1 at the ridge."""
        n = Vector((0, s * tan, 1)).normalized() * lift
        return (x + n.x, yk + s * half * (1 - t) + n.y, eave + (ridge - eave) * t + n.z)

    walls = []
    for s in (-1, 1):
        walls += upright((-hx, s * hy), (hx, s * hy), eave - DROP, (0, s, 0))
        for yk in centres:
            walls += upright((s * hx, yk - half), (s * hx, yk + half), eave - DROP, (s, 0, 0), peak=ridge - DROP)
    skin(m, "walls", walls, wall)
    if plinth:
        skirt(m, "plinth", -hx, hx, -hy, hy, 0.0, plinth, plinth_m, out=0.05)

    # the roof, and its flashings as single faces: down each verge, along each eave, over each ridge
    slopes, trims, x0, x1 = [], [], -hx - VERGE, hx + VERGE
    for k, yk in enumerate(centres):
        for s in (-1, 1):
            outer = (k, s) in ((0, -1), (spans - 1, 1))
            ye, ze = (yk + s * (half + OVER), edge_z) if outer else (yk + s * half, eave)
            slopes.append(([(x0, ye, ze), (x1, ye, ze), (x1, yk, ridge), (x0, yk, ridge)], UP))
            for e, x in ((-1, x0), (1, x1)):
                trims.append(([(x, ye, ze + 0.03), (x, yk, ridge + 0.03), (x, yk, ridge - 0.2), (x, ye, ze - 0.2)], (e, 0, 0)))
            if outer:
                trims.append(([(x0, ye, ze + 0.02), (x1, ye, ze + 0.02), (x1, ye, ze - 0.18), (x0, ye, ze - 0.18)], (0, s, 0)))
            trims.append(([(x0, yk, ridge + 0.03), (x1, yk, ridge + 0.03), (x1, yk + s * 0.16, ridge + 0.03 - 0.16 * tan),
                           (x0, yk + s * 0.16, ridge + 0.03 - 0.16 * tan)], UP))
    skin(m, "roof", slopes, roof, uv="slope", grid=ROOF_CELL_M)
    surface(m, "flashings", trims, trim_m, lods=(0, 1, 2))
    for k in range(spans - 1):  # a box gutter in each valley
        yv = centres[k] + half
        z = eave + 0.35 * tan + 0.02
        surface(m, f"valley_{k}", [([(x0, yv - 0.35, z), (x1, yv - 0.35, z), (x1, yv + 0.35, z), (x0, yv + 0.35, z)], UP)], trim_m)

    def stretch(yk, s, x, w, t0, t1, lift):
        return ([on(yk, s, x - w / 2, t0, lift), on(yk, s, x + w / 2, t0, lift), on(yk, s, x + w / 2, t1, lift),
                 on(yk, s, x - w / 2, t1, lift)], UP)

    if lights:
        light, first, every, (t0, t1) = lights
        sheets, x = [], -hx + first
        while x < hx - 1.0:
            sheets += [stretch(yk, s, x, 1.0, t0, t1, 0.04) for yk in centres for s in (-1, 1)]
            x += every
        surface(m, "rooflights", sheets, light, uv="slope")
    for name in sorted({mend[5] for mend in mends}):
        skin(m, f"mend_{name}", [stretch(centres[k], s, x, w, t0, t1, 0.03) for k, s, x, w, (t0, t1), mat in mends if mat == name],
             ROOFS[name], uv="slope")

    if monitor:
        mw, mh, inset = monitor
        xm, foot = hx - inset, ridge - mw * tan
        head, crest = foot + mh, ridge + mh
        ends, glass, lids = [], [], []
        for s in (-1, 1):
            glass.append(([(-xm, s * mw, foot + 0.3), (xm, s * mw, foot + 0.3), (xm, s * mw, head), (-xm, s * mw, head)], (0, s, 0),
                          ((-s * xm, s * mw, foot + 0.3), (s, 0, 0), (0, 0, 1))))
            ends.append(([(-xm, s * mw, foot - 0.1), (xm, s * mw, foot - 0.1), (xm, s * mw, foot + 0.3), (-xm, s * mw, foot + 0.3)], (0, s, 0)))
            ends.append(([(s * xm, -mw, foot - 0.1), (s * xm, mw, foot - 0.1), (s * xm, mw, head), (s * xm, 0, crest - DROP),
                          (s * xm, -mw, head)], (s, 0, 0)))
            lids.append(([(-xm - 0.2, s * (mw + 0.2), head - 0.2 * tan), (xm + 0.2, s * (mw + 0.2), head - 0.2 * tan),
                          (xm + 0.2, 0, crest), (-xm - 0.2, 0, crest)], UP))
        surface(m, "monitor_glass", glass, glazing_m)
        surface(m, "monitor_ends", ends, panel_m)
        skin(m, "monitor_roof", lids, roof, uv="slope", grid=ROOF_CELL_M)
        xa, xb, low = -xm - 0.2, xm + 0.2, head - 0.2 * tan
        surface(m, "monitor_flashings",
                [([(xa, 0, crest + 0.03), (xb, 0, crest + 0.03), (xb, s * 0.25, crest + 0.03 - 0.25 * tan), (xa, s * 0.25, crest + 0.03 - 0.25 * tan)],
                  UP) for s in (-1, 1)]
                + [([(xa, s * (mw + 0.2), low + 0.02), (xb, s * (mw + 0.2), low + 0.02), (xb, s * (mw + 0.2), low - 0.15),
                     (xa, s * (mw + 0.2), low - 0.15)], (0, s, 0)) for s in (-1, 1)], trim_m, lods=(0, 1, 2))
    return tan, edge_z


PARAPET_M, PARAPET_W = 0.6, 0.25


def flat_shell(m, tag, x0, x1, y0, y1, top, wall, deck, mends=()):
    """A box under a flat roof: the walls rise past the deck as a parapet whose coping is
    the part's top. `mends` are patches on the felt: (x, y, width, depth, which felt).
    Returns the deck's height."""
    under, deck_z, t = top - 0.08, top - PARAPET_M, PARAPET_W
    skin(m, tag + "_walls", box_walls(x0, x1, y0, y1, under), wall)
    skin(m, tag + "_deck", [([(x0 + t, y0 + t, deck_z), (x1 - t, y0 + t, deck_z), (x1 - t, y1 - t, deck_z), (x0 + t, y1 - t, deck_z)], UP)],
         deck, grid=ROOF_CELL_M)
    for name in sorted({mend[4] for mend in mends}):
        surface(m, f"{tag}_mend_{name}", [([(x - w / 2, y - d / 2, deck_z + 0.02), (x + w / 2, y - d / 2, deck_z + 0.02),
                                             (x + w / 2, y + d / 2, deck_z + 0.02), (x - w / 2, y + d / 2, deck_z + 0.02)], UP)
                                           for x, y, w, d, felt_ in mends if felt_ == name], FELTS[name])
    surface(m, tag + "_upstand", [
        ([(x0 + t, y0 + t, deck_z), (x1 - t, y0 + t, deck_z), (x1 - t, y0 + t, under), (x0 + t, y0 + t, under)], (0, 1, 0)),
        ([(x0 + t, y1 - t, deck_z), (x1 - t, y1 - t, deck_z), (x1 - t, y1 - t, under), (x0 + t, y1 - t, under)], (0, -1, 0)),
        ([(x0 + t, y0 + t, deck_z), (x0 + t, y1 - t, deck_z), (x0 + t, y1 - t, under), (x0 + t, y0 + t, under)], (1, 0, 0)),
        ([(x1 - t, y0 + t, deck_z), (x1 - t, y1 - t, deck_z), (x1 - t, y1 - t, under), (x1 - t, y0 + t, under)], (-1, 0, 0))],
        concrete_m, lods=(0, 1, 2))
    for k, y in enumerate((y0 + t / 2, y1 - t / 2)):
        box(m.n(f"{tag}_coping_{k}"), (x1 - x0 + 0.1, t + 0.1, 0.08), ((x0 + x1) / 2, y, top - 0.04), concrete_m, m.root, lods=(0, 1, 2))
    for k, x in enumerate((x0 + t / 2, x1 - t / 2)):
        box(m.n(f"{tag}_coping_{k + 2}"), (t + 0.1, y1 - y0 - 2 * t - 0.1, 0.08), (x, (y0 + y1) / 2, top - 0.04), concrete_m, m.root,
            lods=(0, 1, 2))
    return deck_z


# ---------------------------------------------------------------- rows
def phase(length):
    """The lattice phase that tiles a wall of whole bays: a bay's centre, or a joint, at its middle."""
    return 1.5 if round(length / BAY_PITCH_M) % 2 == 0 else 0.0


def clear(t, edge, taken=()):
    """The bays of `edge` no opening in `taken` reaches into: each (offset, half its width)."""
    return [o for o in t.bays(edge) if all(abs(o - at) > half + 1.4 for at, half in taken)]


def along(t, module, edge, z=0.0, taken=(), every=1, start=0, **row):
    """`module` in every bay of `edge` that is clear (or every `every`th of them)."""
    for o in clear(t, edge, taken)[start::every]:
        t.mount(module, edge, o, z=z, **row)


def roller(t, edge, offset, tint, entrance=True):
    if entrance:
        t.entrance(edge, offset)
    t.mount("roller_door", edge, offset, tint=tint)
    return (offset, 3.0)


def side_door(t, edge, offset, tint, entrance=True, module="personnel_door"):
    if entrance:
        t.entrance(edge, offset)
    t.mount(module, edge, offset, tiers=TIERS_0_TO_2, tint=tint)
    return (offset, 1.5)


def canopy(t, edge, a, b, z):
    """A canopy over the bays from offset `a` to `b`."""
    for k in range(round((b - a) / 3)):
        t.mount("canopy_bay", edge, a + 1.5 + 3 * k, z=z)


def rainwater(t, edge, edge_z, pipes, gutter=True):
    """A gutter under the eave over `edge`, bay by bay, and a downpipe at each offset in `pipes`."""
    for o in t.bays(edge) if gutter else ():
        t.mount("gutter_bay", edge, o, z=edge_z, out=OVER, tiers=TIERS_0_TO_1)
    for o in pipes:
        t.mount("downpipe", edge, o, scale=(1.0, 1.0, edge_z - 0.12), tiers=TIERS_0_TO_1)


def lattices(t, part, width, depth):
    for side, length in (("south", width), ("north", width), ("east", depth), ("west", depth)):
        t.lattice(f"{part}-{side}", phase(length))


def linear(c):
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def untinted(mat, tint):
    """`mat` as a row's tint paints it: for geometry folded into a shell, which has a tint of its own."""
    if not mat.get("tint"):
        return mat
    name = f"{mat.name}_{tint[0]}_{tint[1]}_{tint[2]}"
    if name not in bpy.data.materials:
        new = mat.copy()
        new.name = name
        del new["tint"]
        base, k = PAINTS[mat.name], tuple(linear(c) for c in tint)
        if mat.name in TEXTURED:
            TEXTURED[name] = TEXTURED[mat.name]
            PAINTS[name] = lambda p, n, edge: (lambda c, wear: (tuple(a * b for a, b in zip(c, k)), wear))(*base(p, n, edge))
        else:
            PAINTS[name] = lambda p, n, edge: tuple(a * b for a, b in zip(base(p, n, edge), k))
    return bpy.data.materials[name]


def fold(t, shell):
    """At the two coarse tiers a building is one row, its shell's. What is left of every
    other row's module at those tiers is copied into the shell, in the row's place and
    tint, and the row then draws at the two fine tiers only. Flat panels that meet (a
    band of colour, a run of glazing) become one face."""
    home = next(Matrix.Translation(r[1:4]) @ Matrix.Rotation(r[4], 4, "Z") for r in t.rows["intact"] if r[0] == shell.name).inverted()
    for tier in (2, 3):
        groups = {}
        for module, x, y, z, yaw, sx, sy, sz, tiers, *tint in t.rows["intact"]:
            if module == shell.name or not tiers >> tier & 1:
                continue
            at = home @ Matrix.Translation((x, y, z)) @ Matrix.Rotation(yaw, 4, "Z") @ Matrix.Diagonal((sx, sy, sz, 1))
            for o in kit.modules[module].meshes():
                if tier_of(o) == tier:
                    mat = untinted(o.data.materials[0], tuple(tint))
                    # glazing gives up its own UVs (the export box-projects it), so a run of it can become one face
                    own = bool(o.data.uv_layers) and mat is not glazing_m
                    groups.setdefault((mat.name, own), []).append((o.data, at @ o.matrix_basis))
        for k, ((name, own), meshes) in enumerate(sorted(groups.items())):
            def build(bm, lod, meshes=meshes, own=own):
                flat = all(len(me.polygons) == 1 for me, _ in meshes)
                for me, at in meshes:
                    n = len(bm.verts)
                    bm.from_mesh(me)
                    bm.verts.ensure_lookup_table()
                    bmesh.ops.transform(bm, matrix=at, verts=bm.verts[n:])
                if not own:
                    for layer in list(bm.loops.layers.uv.values()):
                        bm.loops.layers.uv.remove(layer)
                    if flat:
                        bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=0.001)
                        bmesh.ops.dissolve_limit(bm, angle_limit=math.radians(0.5), verts=bm.verts, edges=bm.edges)
                bm.normal_update()

            mesh_part(shell.n(f"fold_{k}"), build, bpy.data.materials[name], shell.root, lods=(tier,))
    t.rows["intact"] = [r if r[0] == shell.name else (*r[:8], r[8] & TIERS_0_TO_1, *r[9:]) for r in t.rows["intact"]]


# ---------------------------------------------------------------- the workshop shed
# Sage-green steel sheet on a block dado, gable to the street, under a galvanised roof
# gone to rust: a vehicle door and a side door in the gable, windows down both sides.
W, D, EAVE, TOP = 15.0, 24.0, 4.8, 6.2
m = kit.module("shed_shell", ground=True, paint_scale=3.0)
tan, edge_z = gabled_shell(m, D, W, EAVE, TOP, clad_m, ROOFS["rusty"], lights=(ROOFLIGHTS["pale"], 6.0, 12.0, (0.2, 0.85)),
                           mends=[(0, -1, -1.5, 2.25, (0.0, 0.6), "new"), (0, 1, 2.0, 1.5, (0.35, 1.0), "new"),
                                  (0, 1, -9.5, 3.0, (0.0, 0.5), "tarred")])
t = kit.template("china-shed-15x24", "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="steel portal frame, block dado",
                                                               Roof="gable", RidgeAlong="y", Wall="cladding", Sheet="rusty"))
t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
t.floors(0.0)
lattices(t, "body", W, D)
t.place(m.name, yaw=math.pi / 2, tint=SAGE)  # the shell is built ridge along x
taken = [side_door(t, "body-south", 3.0, BOTTLE), roller(t, "body-south", -1.5, BOTTLE)]
along(t, "dado_bay", "body-south", taken=taken, tiers=TIERS_0_TO_2)
along(t, "strip_window", "body-south", z=2.0, taken=taken, tiers=TIERS_0_TO_2)
t.mount("louvre", "body-south", -1.5, z=4.9, tiers=TIERS_0_TO_1)
along(t, "dado_bay", "body-north", tiers=TIERS_0_TO_2)
along(t, "strip_window", "body-north", z=2.0, every=2, start=1, tiers=TIERS_0_TO_2)
t.mount("louvre", "body-north", 0.0, z=5.0, tiers=TIERS_0_TO_1)
for side in ("east", "west"):
    edge = f"body-{side}"
    along(t, "dado_bay", edge, tiers=TIERS_0_TO_2)
    for o in t.bays(edge)[1:-1]:
        t.mount("strip_window", edge, o, z=2.0, tiers=TIERS_0_TO_2)
    rainwater(t, edge, edge_z, (-D / 2 + 0.4, D / 2 - 0.4))
t.mount("clad_panel", "body-east", 10.5, z=DADO_M + 0.1, tiers=TIERS_0_TO_2, tint=PRIMER)  # a bay re-sheeted, never painted to match
for y in (-5.0, 5.0):
    t.place("turbine_vent", 0.0, y, TOP - 0.1, tiers=TIERS_0_TO_2)
t.place("flue_stack", 3.4, 8.0, TOP - 3.4 * tan - 0.2, tiers=TIERS_0_TO_2)
fold(t, m)

# ---------------------------------------------------------------- the dock warehouse
# Precast concrete panels under a flat felt roof: a row of loading docks under a canopy
# along the street side, an office corner at one end and a drive-in door at the other.
W, D, TOP = 48.0, 24.0, 9.0
m = kit.module("slab_shell", ground=True, paint_scale=4.0)
deck = flat_shell(m, "box", -W / 2, W / 2, -D / 2, D / 2, TOP, slab_m, felt_m,
                  mends=[(-8.5, 7.5, 5.0, 3.0, "new"), (8.5, -8.0, 3.0, 6.0, "silver"), (17.0, 8.5, 6.0, 4.0, "new"),
                         (-1.0, -9.5, 4.0, 2.0, "silver"), (21.0, -2.0, 3.0, 5.0, "new"), (-14.0, 1.0, 2.0, 7.0, "silver")])
t = kit.template("china-warehouse-48x24", "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="precast concrete panels",
                                                                    Roof="flat", Wall="tilt_slab", Docks=10))
t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
t.floors(0.0)
lattices(t, "body", W, D)
t.place(m.name, tint=STONE)
taken = [side_door(t, "body-south", -22.5, RUSSET), roller(t, "body-south", 18.0, RUSSET)]
t.mount("office_window", "body-south", -19.5, z=0.95, tiers=TIERS_0_TO_2)
for k in range(10):
    t.mount("dock_door", "body-south", -16.5 + 3 * k, tint=IVORY)
canopy(t, "body-south", -18.0, 12.0, 4.55)
canopy(t, "body-south", 15.0, 21.0, 4.75)
for side in ("south", "north", "east", "west"):
    edge = f"body-{side}"
    along(t, "paint_band", edge, z=7.3, tint=RUSSET)
    if side != "south":
        along(t, "clerestory", edge, z=6.0, every=2, tiers=TIERS_0_TO_2)
for o in (-21.0, -9.0, 3.0, 15.0):
    t.mount("downpipe", "body-north", o, scale=(1.0, 1.0, deck), tiers=TIERS_0_TO_1)
for k in range(3):
    t.mount("ladder", "body-east", 9.0, z=3.0 * k, tiers=TIERS_0_TO_1)
for x in (-20.0, -12.0, -4.0, 4.0, 12.0, 20.0):
    for y in (-5.0, 5.0):
        if (x, y) != (-20.0, -5.0):
            t.place("skylight_dome", x, y, deck, tiers=TIERS_0_TO_2)
t.place("roof_unit", -20.0, -6.5, deck, tiers=TIERS_0_TO_2)
t.place("roof_unit", -16.0, -9.0, deck, math.pi / 2, tiers=TIERS_0_TO_2)
for k in range(4):
    t.place("duct_run", -17.4 + 3 * k, -6.5, deck, tiers=TIERS_0_TO_2)
for x in (-8.0, 8.0, 16.0):
    t.place("turbine_vent", x, 0.0, deck, tiers=TIERS_0_TO_2)
t.place("flue_stack", -21.5, 9.5, deck, tiers=TIERS_0_TO_2)
fold(t, m)

# ---------------------------------------------------------------- the distribution warehouse
# Two spans of off-white steel sheet with a blue band under the eaves and a pale roof
# striped with rooflights: three vehicle doors, each under its canopy with a side door.
W, D, EAVE, TOP = 72.0, 33.0, 8.4, 10.0
m = kit.module("span_shell", ground=True, paint_scale=5.0)
tan, edge_z = gabled_shell(m, W, D, EAVE, TOP, clad_m, ROOFS["pale"], spans=2, lights=(ROOFLIGHTS["dull"], 4.5, 9.0, (0.15, 0.88)),
                           mends=[(0, -1, -19.0, 3.0, (0.0, 0.7), "tarred"), (0, 1, 8.5, 4.5, (0.3, 1.0), "new"),
                                  (1, -1, 26.5, 3.0, (0.0, 0.45), "tarred"), (1, 1, -28.5, 3.75, (0.2, 1.0), "new"),
                                  (1, 1, 17.5, 2.25, (0.0, 0.5), "tarred")], plinth=0.5)
t = kit.template("china-warehouse-72x33", "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="steel portal frame, two spans",
                                                                    Roof="twin gable", RidgeAlong="x", Wall="cladding", Sheet="pale"))
t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
t.floors(0.0)
lattices(t, "body", W, D)
t.place(m.name, tint=IVORY)
taken = []
for x in (0.0, -24.0, 24.0):
    taken.append(roller(t, "body-south", x, NAVY))
    taken.append(side_door(t, "body-south", x + 4.5, NAVY, entrance=False))
    canopy(t, "body-south", x - 3.0, x + 6.0, 4.75)
along(t, "clerestory", "body-south", z=5.9, taken=[(x, 4.5) for x in (1.5, -22.5, 25.5)], tiers=TIERS_0_TO_2)
along(t, "clerestory", "body-north", z=5.9, every=2, tiers=TIERS_0_TO_2)
for side in ("south", "north", "east", "west"):
    along(t, "clad_panel", f"body-{side}", z=7.15, scale=(1.0, 1.0, 0.4), tint=NAVY)
for side in ("east", "west"):
    for o in (-D / 4, D / 4):
        t.mount("louvre", f"body-{side}", o - 0.75, z=8.6, tiers=TIERS_0_TO_1)
rainwater(t, "body-south", edge_z, (-35.6, -12.0, 12.0, 35.6))
rainwater(t, "body-north", edge_z, (-35.6, -12.0, 12.0, 35.6))
for y in (-D / 4, D / 4):
    for k in range(6):
        t.place("ridge_vent", -30.0 + 12 * k, y, TOP, tiers=TIERS_0_TO_2)
fold(t, m)

# ---------------------------------------------------------------- the works
# A brick hall under a sawtooth of north lights, tall steel windows between its piers,
# and a two-storey rendered office block joined to its street side. Both boxes are as
# tall as the sawtooth's ridges (a join needs equal tops); the office's parapet meets them.
HALL_W, HALL_D, OFFICE_W, OFFICE_D, TOP, TEETH = 54.0, 27.0, 18.0, 9.0, 8.0, 9
VALLEY, STOREY = WORKS_WALL_M, 3.6
hall_y = OFFICE_D / 2  # the hall stands behind the office: the footprint of the two is centred on the origin
y0, y1 = hall_y - HALL_D / 2, hall_y + HALL_D / 2

m = kit.module("works_shell", ground=True, paint_scale=3.0)
hx = HALL_W / 2
surface(m, "hall_walls", box_walls(-hx, hx, y0, y1, VALLEY), brick_m)
skirt(m, "hall_plinth", -hx, hx, y0, y1, 0.0, 0.6, plinth_m, out=0.06)
skirt(m, "hall_beam", -hx, hx, y0, y1, VALLEY - 0.45, VALLEY, concrete_m, out=0.04)  # the ring beam the roof sits on
office_deck = flat_shell(m, "office", -hx, -hx + OFFICE_W, y0 - OFFICE_D, y0, TOP, render_m, felt_m,
                         mends=[(-hx + 14.5, y0 - 2.0, 4.0, 2.0, "new"), (-hx + 3.0, y0 - 2.5, 2.0, 3.0, "silver")])

# One tooth of the roof, 6 m of the hall: glazing leaning back from its kerb to the
# ridge, sheet falling from the ridge to the next valley, and the brick gablet at each end.
TOOTH = HALL_W / TEETH
m = kit.module("works_tooth", paint_scale=5.0)
hy, kerb, lean = HALL_D / 2, 0.3, 0.45
ridge_x = -TOOTH / 2 + lean
surface(m, "glass", [([(-TOOTH / 2, hy, VALLEY + kerb), (-TOOTH / 2, -hy, VALLEY + kerb), (ridge_x, -hy, TOP), (ridge_x, hy, TOP)],
                      (-1, 0, 0.3))], glazing_m, uv=((-TOOTH / 2, hy, VALLEY + kerb), (0, -1, 0), (0, 0, 1)))
surface(m, "kerb", [([(-TOOTH / 2, hy, VALLEY), (-TOOTH / 2, -hy, VALLEY), (-TOOTH / 2, -hy, VALLEY + kerb), (-TOOTH / 2, hy, VALLEY + kerb)],
                     (-1, 0, 0))], trim_m, lods=(0, 1))
skin(m, "sheet", [([(ridge_x, -hy, TOP), (ridge_x, hy, TOP), (TOOTH / 2, hy, VALLEY), (TOOTH / 2, -hy, VALLEY)], UP)], ROOFS["green"],
     uv="slope", grid=ROOF_CELL_M)
surface(m, "gablets", [([(-TOOTH / 2, s * hy, VALLEY), (TOOTH / 2, s * hy, VALLEY), (ridge_x, s * hy, TOP)], (0, s, 0)) for s in (-1, 1)],
        brick_high_m)
surface(m, "flashing", [([(ridge_x - 0.02, -hy, TOP + 0.03), (ridge_x - 0.02, hy, TOP + 0.03), (ridge_x + 0.35, hy, TOP - 0.08),
                          (ridge_x + 0.35, -hy, TOP - 0.08)], UP)], trim_m, lods=(0, 1))
surface(m, "gutter", [([(TOOTH / 2 - 0.5, -hy, VALLEY + 0.2), (TOOTH / 2 - 0.5, hy, VALLEY + 0.2), (TOOTH / 2, hy, VALLEY + 0.2),
                        (TOOTH / 2, -hy, VALLEY + 0.2)], UP)], trim_m, lods=(0, 1))

t = kit.template("china-works-54x36", "industry", FAMILY, dict(HallWidth=HALL_W, HallDepth=HALL_D, OfficeWidth=OFFICE_W, OfficeDepth=OFFICE_D,
                                                                Floors=2, Structure="brick hall, rendered office", Roof="sawtooth",
                                                                Teeth=TEETH, Sheet="green"))
t.part("hall", -hx, hx, y0, y1, TOP)
t.part("office", -hx, -hx + OFFICE_W, y0 - OFFICE_D, y0, TOP)
t.floors(0.0, STOREY)
for edge, length in (("hall-north", HALL_W), ("hall-east", HALL_D), ("hall-west", HALL_D), ("office-south", OFFICE_W),
                     ("office-east", OFFICE_D), ("office-west", OFFICE_D)):
    t.lattice(edge, phase(length))
t.lattice("hall-south-1", 1.5)  # the stretch east of the office: bays on the hall's own lattice
t.place("works_shell", tint=CREAM)
for k in range(TEETH):
    t.place("works_tooth", -hx + TOOTH * (k + 0.5), hall_y)
taken = [side_door(t, "office-south", -1.5, BOTTLE, module="office_door"), roller(t, "hall-south-1", 12.0, BOTTLE)]
for floor in (0.0, STOREY):
    for edge in ("office-south", "office-east", "office-west"):
        along(t, "office_window", edge, z=floor + 0.95, taken=taken[:1] if floor == 0.0 and edge == "office-south" else (),
              tiers=TIERS_0_TO_2)
for edge in ("hall-north", "hall-east", "hall-west", "hall-south-1"):
    along(t, "factory_window", edge, z=1.2, taken=taken[1:] if edge == "hall-south-1" else (), tiers=TIERS_0_TO_2)
    a, b = t.edges()[edge]["span"]
    piers = [a + 6 * k for k in range(int((b - a) // 6) + 1)]  # a pier every second bay, and one at each corner
    for o in piers + [b] * (b - piers[-1] > 1.0):
        t.mount("works_pier", edge, min(max(o, a + 0.3), b - 0.3), tiers=TIERS_0_TO_1)
for edge in ("hall-north", "hall-south-1"):  # a downpipe from each valley
    for k in range(1, TEETH):
        x = -hx + TOOTH * k
        o = -x if edge == "hall-north" else x
        if t.edges()[edge]["span"][0] < o:
            t.mount("downpipe", edge, o + 0.55, scale=(1.0, 1.0, VALLEY), tiers=TIERS_0_TO_1)
t.place("flue_stack", 21.0 + 2.2, y1 - 4.0, VALLEY + 0.2, tiers=TIERS_0_TO_2)
t.place("flue_stack", 15.0 + 2.2, y1 - 4.0, VALLEY + 0.2, tiers=TIERS_0_TO_2)
t.place("roof_unit", -hx + 4.0, y0 - 5.5, office_deck, tiers=TIERS_0_TO_2)
t.place("skylight_dome", -hx + 9.0, y0 - 4.5, office_deck, tiers=TIERS_0_TO_2)
t.place("skylight_dome", -hx + 13.0, y0 - 4.5, office_deck, tiers=TIERS_0_TO_2)
fold(t, kit.modules["works_shell"])

# ---------------------------------------------------------------- the depot
# A concrete frame filled with painted blockwork under an oxide-red roof with a glazed
# monitor along its ridge: two runs of vehicle doors under canopies, a band of high
# windows all round.
W, D, EAVE, TOP = 90.0, 39.0, DEPOT_WALL_M, 11.0
MONITOR = (4.5, 1.5, 6.0)
m = kit.module("depot_shell", ground=True, paint_scale=5.0)
tan, edge_z = gabled_shell(m, W, D, EAVE, TOP - MONITOR[1], block_m, ROOFS["oxide"], lights=(ROOFLIGHTS["pale"], 7.5, 15.0, (0.15, 0.6)),
                           mends=[(0, -1, -27.0, 4.5, (0.0, 0.75), "new"), (0, -1, 31.5, 3.0, (0.3, 0.75), "new"),
                                  (0, 1, 12.0, 6.0, (0.0, 0.4), "new"), (0, 1, -33.0, 3.0, (0.1, 0.75), "tarred"),
                                  (0, -1, 3.0, 2.25, (0.0, 0.3), "tarred")], monitor=MONITOR, plinth=0.4)
t = kit.template("china-depot-90x39", "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="concrete frame, block infill",
                                                                Roof="gable with monitor", RidgeAlong="x", Wall="concrete_block",
                                                                Sheet="oxide", Doors=8))
t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
t.floors(0.0)
lattices(t, "body", W, D)
t.place(m.name, tint=BUFF)
taken = [side_door(t, "body-south", -1.5, SLATE)]
for x in (-30.0, -24.0, -18.0, -12.0, 12.0, 18.0, 24.0, 30.0):
    taken.append(roller(t, "body-south", x, SLATE, entrance=x in (-24.0, 24.0)))
canopy(t, "body-south", -33.0, -9.0, 4.75)
canopy(t, "body-south", 9.0, 33.0, 4.75)
along(t, "strip_window", "body-south", z=1.3, taken=taken, tiers=TIERS_0_TO_2)
for side in ("south", "north", "east", "west"):
    along(t, "window_band", f"body-{side}", z=5.8)
along(t, "strip_window", "body-north", z=1.3, every=2, tiers=TIERS_0_TO_2)
for edge, length in (("body-south", W), ("body-north", W)):
    for k in range(round(length / 6) + 1):
        t.mount("depot_pier", edge, min(max(-length / 2 + 6 * k, -length / 2 + 0.25), length / 2 - 0.25), tiers=TIERS_0_TO_1)
    rainwater(t, edge, edge_z, [-length / 2 + 6 + 12 * k + 0.4 for k in range(7)])
for edge in ("body-east", "body-west"):
    for o in (-19.25, -13.5, -7.5, -1.5, 1.5, 7.5, 13.5, 19.25):
        t.mount("depot_pier", edge, o, tiers=TIERS_0_TO_1)
    t.mount("louvre", edge, -0.75, z=8.3, tiers=TIERS_0_TO_1)
for x in (-30.0, -15.0, 0.0, 15.0, 30.0):
    for s in (-1, 1):
        t.place("turbine_vent", x, s * 11.0, EAVE + (D / 2 - 11.0) * tan - 0.1, tiers=TIERS_0_TO_2)
for x in (-40.5, 27.0):
    t.place("flue_stack", x, 14.5, EAVE + (D / 2 - 14.5) * tan - 0.15, tiers=TIERS_0_TO_2)
fold(t, m)

kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
