"""Warehouses and light industry: a town's sheds, works and depots, one regional family's to a set.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/industry.py [family [out_dir]]

writes `assets/source/city/<set>/kit.glb` and `templates.json` (the format is in
this folder's readme) for `family` (`china` when none is named), or into `out_dir`.
Every family covers the same five lots (a 15 x 24 workshop shed, warehouses 48 x 24
and 72 x 33, a 54 x 36 works of a hall and a two-storey office, a 90 x 39 depot), each
building in its own region's dress, from one set of builders: the materials, the
fittings, the shells (gabled, flat, sawtooth, vaulted, a stepped false front), the
rows, the far tiers' fold and the ruins are shared, and a family's function says
which builder each lot is and how it is dressed.

- `china` (set `industry`): a steel workshop shed on a block dado, a precast dock
  warehouse under a flat roof, a twin-span steel distribution warehouse, a brick works
  under a sawtooth with a rendered office, and a blockwork depot with a roof monitor.
- `new_york`: a galvanised shed, a three-storey brick loft with its water tank, a
  brick warehouse and a car barn whose gables hide behind stepped brick fronts, and a
  brick works with a limestone-trimmed office.
- `paris`: a steel shed under fibre cement, a rendered atelier under a sawtooth, a
  twin-span steel warehouse, a rendered works with brick piers and a brick office, and
  a depot under a concrete barrel vault.

The physical box is the authority. Every outer wall stands on a face of a part.
A pitched, sawtooth or vaulted roof crowns at its part's top and a flat roof's parapet
is the top: the box is what stops rounds and sight. Canopies, gutters, dock faces
and bollards reach past a face by at most the set's side fit; vents, stacks, roof
plant, a stepped front's crest and a water tank rise above the top by at most its top
fit. Doors and windows sit in the bays of their edge's 3 m lattice.

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
import random
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from masonry import panel_door, ragged_wall, scorched  # noqa: E402
from water_tank import HEIGHT_M as TANK_M, water_tank  # noqa: E402

# A family's set and the top fit its roofs need (New York's water tanks stand high).
FAMILIES = {"china": dict(set="industry", fit_top_m=1.2), "new_york": dict(set="industry_new_york", fit_top_m=4.4),
            "paris": dict(set="industry_paris", fit_top_m=1.2)}
FAMILY = next(iter(script_args()), "china")
if FAMILY not in FAMILIES:
    raise SystemExit(f"industry.py: the family is one of {sorted(FAMILIES)}, not {FAMILY}")
UP, SOUTH = Vector((0, 0, 1)), (0, -1, 0)
OVER, VERGE = 0.25, 0.12  # how far a sheet roof's eaves and verges overhang
DROP = 0.02  # a wall stops this far under its roof

kit = Kit(FAMILIES[FAMILY]["set"], "industry.py" + ("" if FAMILY == "china" else f" {FAMILY}"), fit_side_m=1.2,
          fit_top_m=FAMILIES[FAMILY]["fit_top_m"], fit_ruin_top_m=0.6)
OFFER = FITTING | dict(optional=True)  # a fitting every family is offered: a set whose buildings place none leaves it out


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
        # fields wider than two of a roof's cells, so every tier's vertices find the same marks
        patch = smoothstep(-0.2, 0.45, fbm(p, 0.075, 2, 71.0 + seed))
        drift = 0.5 + 0.5 * fbm(p, 0.13, 2, 83.0 + seed)
        fade = 1.0 + 0.16 * fbm(p, 0.05, 2, 61.0 + seed)  # whole stretches bleached or dulled
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
        patch = smoothstep(0.0, 0.6, fbm(p, 0.1, 2, 91.0 + seed))
        return c, max(wear, min(0.62, amount * patch))

    return weather(mat, fn, lambda c, wear: (c, max(wear, 0.3 * amount)))


def felt(name, seed=0.0, recipe="flat_roof", colour=None):
    """A flat roof's felt (or another `recipe` that lies under the weather, as a concrete vault does): damp
    darkens it in wide pools and moss takes the wettest of them."""
    mat = textured(name, recipe, colour=colour, dirt=0.0, chip=0.0, streak=0.0, seed=seed)

    def fn(c, wear, p, n):
        level = fbm(p, 0.075, 2, 97.0 + seed)
        damp = smoothstep(0.1, 0.4, level)
        silt = math.exp(-((level - 0.08) / 0.07) ** 2)  # the pale tide mark a pool leaves as it dries
        c = lerp3(tuple(x * (1.0 - 0.5 * damp) for x in c), (0.3, 0.29, 0.26), 0.45 * silt)
        return c, max(wear, 0.6 * damp * smoothstep(0.0, 0.5, fbm(p, 0.13, 2, 99.0 + seed)))

    return weather(mat, fn, lambda c, wear: (tuple(x * 0.8 for x in c), wear))


# Walls. The pale ones are tint-masked: a row's tint is the building's paint.
clad_m = rusting(textured("wall_cladding", "cladding", tint=1.0, dirt=0.5, chip=0.4, streak=0.3, rise=1.2), 0.5)
panel_m = textured("panel_cladding", "cladding", tint=1.0, dirt=0.0, chip=0.3, streak=0.0, seed=4.0)
block_m = rusting(textured("wall_block", "concrete_block", tint=1.0, dirt=0.7, chip=0.25, streak=0.4, rise=1.1), 0.35, 3.0)
slab_m = rusting(textured("wall_tilt_slab", "tilt_slab", tint=1.0, dirt=0.6, chip=0.3, streak=0.45, rise=1.0), 0.5, 5.0)
brick_m = textured("wall_brick", "brick", colour=(0.2, 0.09, 0.066), dirt=0.5, chip=0.12, streak=0.45, rise=0.8, mottle=0.08)
brick_high_m = textured("gable_brick", "brick", colour=(0.2, 0.09, 0.066), dirt=0.0, chip=0.12, streak=0.45, mottle=0.08)
render_m = textured("wall_render", "roughcast", tint=1.0, dirt=0.55, chip=0.3, streak=0.45, rise=0.8)
dado_m = textured("dado_block", "concrete_block", colour=(0.3, 0.3, 0.29), dirt=0.8, chip=0.3, streak=0.3, rise=0.7)
plinth_m = textured("plinth_concrete", "concrete", colour=(0.2, 0.2, 0.19), dirt=0.8, chip=0.4, rise=0.5)
concrete_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.3)
pier_m = textured("pier_concrete", "concrete", colour=(0.27, 0.27, 0.255), dirt=0.35, chip=0.4, streak=0.3, rise=0.6)
# Roofs take no tint: a row has one, and it is the walls'.
ROOFS = {
    "rusty": sheet("roof_rusty", (0.25, 0.2, 0.16), rust=1.35, soil=0.5, seed=1.0),  # galvanised, long unpainted and browning
    "pale": sheet("roof_pale", (0.6, 0.6, 0.57), rust=0.4, soil=0.7, seed=2.0),
    "green": sheet("roof_green", (0.115, 0.135, 0.08), rust=0.75, soil=0.6, seed=3.0),  # paint long faded
    "oxide": sheet("roof_oxide", (0.2, 0.082, 0.058), rust=0.6, soil=0.7, seed=4.0),
    # what a roof is mended with: new galvanised sheets, and sheets tarred over
    "new": sheet("roof_new", (0.36, 0.375, 0.39), rust=0.0, soil=0.15, seed=5.0),
    "tarred": sheet("roof_tarred", (0.1, 0.095, 0.09), rust=0.0, soil=0.3, seed=6.0),
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
glass_m, pane_m = window_glass(), window_pane()  # an office's window near, and the dark pane it is from far off
rubber_m = flat_paint("dock_rubber", (0.02, 0.02, 0.02), rough=0.9, grime=0.0)
hazard_m = flat_paint("hazard_yellow", (0.5, 0.36, 0.035), rough=0.6, wear=0.6, grime=0.6)
dome_m = flat_paint("skylight_dome", (0.55, 0.6, 0.58), rough=0.3, grime=0.0)
door_m = flat_paint("door_steel", (0.5, 0.5, 0.49), rough=0.5, wear=0.5, grime=0.4)
band_m = textured("band_paint", "tilt_slab", tint=1.0, dirt=0.0, chip=0.5, streak=0.5, seed=6.0)  # a stripe painted on the panels
door_m["tint"] = 1.0

# What a fallen building is made of: burnt steel, and the heaps of each kind of wall. The
# shared heaps take a row's tint; a ruin's own heap carries its colour, as its walls do.
steel_m = textured("steel_burnt", "burnt_metal", dirt=0.2, chip=0.3, streak=0.0, ash=0.2)
rubble_m = textured("rubble_masonry", "rubble", tint=1.0, dirt=0.0, chip=0.0, streak=0.0, ash=0.1)


def rubble_of(name, colour, seed):
    return textured(f"rubble_{name}", "rubble", colour=colour, dirt=0.0, chip=0.0, streak=0.0, ash=0.1, seed=seed)


def burnt_sheet(mat, seed=0.0):
    """A sheet roof's material after the fire (`<name>_burnt`): its paint gone to scale and
    rust in wide stretches, and sooted black in others."""
    name = mat.name + "_burnt"
    if name not in bpy.data.materials:
        base = PAINTS[mat.name]
        twin = mat.copy()
        twin.name = name

        def fn(p, n, edge):
            c, wear = base(p, n, edge)
            c = lerp3(c, RUST, 0.45 + 0.25 * fbm(p, 0.3, 2, 37.0 + seed))
            c = lerp3(c, (0.014, 0.013, 0.012), 0.85 * smoothstep(-0.3, 0.4, fbm(p, 0.17, 2, 51.0 + seed)))
            return c, max(wear, 0.3)

        PAINTS[name], TEXTURED[name] = fn, TEXTURED[mat.name]
    return bpy.data.materials[name]


# Tints, sRGB: paints on sheet, concrete and render, and the doors.
SAGE, IVORY, STONE, CREAM, BUFF = (112, 140, 118), (232, 230, 220), (214, 208, 196), (230, 216, 188), (212, 200, 168)
NAVY, RUSSET, SLATE, BOTTLE, PRIMER = (52, 84, 132), (150, 66, 48), (78, 88, 100), (58, 86, 70), (150, 92, 76)

# ---------------------------------------------------------------- builders


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
ROOF_FAR_CELL_M = 6.6  # and this, coarser, at the coarsest: the same marks, where a roof is all the camera sees of a shed


def surface(m, tag, faces, mat, lods=TIERS, uv=None, grid=None, far_grid=None):
    """One mesh of flat faces, each `(points, toward)` or `(points, toward, uv)`. With no
    `uv` the export box-projects it. `"slope"` is a roof's own: u along the eave and v up
    the slope, so a sheet's ribs run with the fall. `(origin, u, v)` lays the recipe from
    a corner, so a window's panes start at its own edge. `grid` cuts every face into
    cells that size at all but the coarsest tier, and `far_grid` into cells that size there."""
    tile = textures.tile_of(TEXTURED[mat.name]) if mat.name in TEXTURED else 1.0

    def build(bm, lod):
        layer = bm.loops.layers.uv.new("UVMap") if uv is not None or any(len(f) > 2 for f in faces) else None
        for points, toward, *own in faces:
            how = own[0] if own else uv
            for quad in cells(points, grid) if grid and lod < 3 else cells(points, far_grid) if far_grid and lod == 3 else [points]:
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
    """A weathered surface: `mat` at the tiers that have the vertices for its marks, its twin at the coarsest.
    A roof (one that has a `grid`) keeps its marks there too, on a coarser grid: no twin."""
    if how.get("grid"):
        return surface(m, tag, faces, mat, far_grid=ROOF_FAR_CELL_M, **how)
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
    m = kit.module(name, **OFFER)
    m.opening = (w, h, 0.0)
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

# An office's window: plain glass in a painted frame, three lights, with the office behind it (`furnish`).
# The sheds' own glazing above is wired and dirty, and stays what it was: nobody looks through it.
casement(kit, "office_window", 1.6, 1.4, frame_m, glass_m, pane_m, concrete_m, lights=3)

# A sheet of cladding over the wall's own: a band of colour, or a bay re-sheeted in another paint.
m = kit.module("clad_panel", **OFFER)
surface(m, "sheet", [(front(-1.5, 1.5, 0, 3.0, -0.03), SOUTH)], panel_m)

m = kit.module("clad_panel_window", **OFFER)
pw, ph, _ = kit.modules["strip_window"].opening
surface(m, "sheet", [(front(a, b, c, d, -0.03), SOUTH) for a, b, c, d in (
    (-1.5, -pw / 2, 0, ph), (pw / 2, 1.5, 0, ph), (-1.5, 1.5, ph, 3.0))], panel_m)

m = kit.module("paint_band", **OFFER)
surface(m, "band", [(front(-1.5, 1.5, 0, 1.0, -0.015), SOUTH)], band_m, uv=((-3.0, 0, -0.35), (1, 0, 0), (0, 0, 1)))

DADO_M = 1.2
m = kit.module("dado_bay", ground=True, **OFFER)
box(m.n("blocks"), (3.0, 0.12, DADO_M), (0, -0.06, DADO_M / 2), dado_m, m.root, lods=(0, 1))
surface(m, "face", [(front(-1.5, 1.5, 0, DADO_M, -0.12), SOUTH)], dado_m, lods=(2, 3))
box(m.n("capping"), (3.0, 0.17, 0.06), (0, -0.085, DADO_M + 0.03), concrete_m, m.root, lods=(0, 1))

# A roller shutter two bays wide, its threshold's centre at the origin: a vehicle door.
ROLLER_W, ROLLER_H = 5.0, 4.2
m = kit.module("roller_door", ground=True, **OFFER)
m.opening = (ROLLER_W, ROLLER_H, 0.0)
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
m = kit.module("dock_door", ground=True, **OFFER)
m.opening = (DOCK_W, DOCK_H, DOCK_SILL)
surface(m, "door", [(front(-DOCK_W / 2, DOCK_W / 2, DOCK_SILL, DOCK_SILL + DOCK_H, -0.04), SOUTH)], slats_m, lods=(0, 1))
surface(m, "door_far", [(front(-DOCK_W / 2 + 0.35, DOCK_W / 2 - 0.35, DOCK_SILL + 0.3, DOCK_SILL + DOCK_H - 0.35, -0.05), SOUTH)], slats_m,
        lods=(2, 3))
box(m.n("face"), (3.0, 0.16, DOCK_SILL), (0, -0.08, DOCK_SILL / 2), plinth_m, m.root, lods=(0, 1))
box(m.n("seal_head"), (DOCK_W + 0.5, 0.3, 0.35), (0, -0.15, DOCK_SILL + DOCK_H + 0.175), rubber_m, m.root, lods=(0, 1))
surface(m, "seal", [(front(-DOCK_W / 2 - 0.25, DOCK_W / 2 + 0.25, DOCK_SILL - 0.4, DOCK_SILL + DOCK_H + 0.35, -0.03), SOUTH)], rubber_m,
        lods=(2, 3))
box(m.n("lip"), (2.1, 0.45, 0.05), (0, -0.38, DOCK_SILL - 0.03), galv_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"seal_{'ab'[s > 0]}"), (0.3, 0.3, DOCK_H), (s * (DOCK_W / 2 + 0.1), -0.15, DOCK_SILL + DOCK_H / 2), rubber_m, m.root,
        lods=(0, 1))
    box(m.n(f"bumper_{'ab'[s > 0]}"), (0.25, 0.14, 0.5), (s * 1.2, -0.23, DOCK_SILL - 0.3), rubber_m, m.root, lods=(0, 1))

m = kit.module("personnel_door", ground=True, **OFFER)
m.opening = (0.95, 2.1, 0.0)
panel_door(m.n("door"), 0.95, 2.1, door_m, frame_m, m.root)
box(m.n("canopy"), (1.5, 0.6, 0.06), (0, -0.3, 2.42), trim_m, m.root, lods=(0, 1))
box(m.n("step"), (1.4, 0.5, 0.1), (0, -0.25, 0.05), plinth_m, m.root, lods=(0, 1))

m = kit.module("office_door", ground=True, **OFFER)
m.opening = (1.5, 2.1, 0.0)
panel_door(m.n("door"), 1.5, 2.1, door_m, frame_m, m.root, pane_m, light=0.4)
box(m.n("canopy"), (2.8, 1.0, 0.12), (0, -0.5, 2.9), concrete_m, m.root, lods=(0, 1, 2))
box(m.n("step"), (2.4, 0.8, 0.14), (0, -0.4, 0.07), plinth_m, m.root, lods=(0, 1))

# A canopy's bay: sheet falling away from the wall, hung on a tie rod from above.
CANOPY_M = 1.15
m = kit.module("canopy_bay", **OFFER)
top = [(-1.5, 0, 0.28), (1.5, 0, 0.28), (1.5, -CANOPY_M, 0.08), (-1.5, -CANOPY_M, 0.08)]
surface(m, "sheet", [(top, UP)], canopy_m, uv="slope")
surface(m, "soffit", [([(x, y, z - 0.05) for x, y, z in top], -UP)], trim_m, lods=(0, 1))
box(m.n("fascia"), (3.0, 0.05, 0.2), (0, -CANOPY_M, 0.02), trim_m, m.root, lods=(0, 1))
bar(m.n("rod"), (0, -CANOPY_M + 0.1, 0.1), (0, 0, 0.75), 0.04, 0.04, trim_m, m.root, lods=(0, 1))

m = kit.module("louvre", **OFFER)
surface(m, "blades", [(front(-0.75, 0.75, 0, 0.8, -0.04), SOUTH)], louvre_m)
box(m.n("head"), (1.66, 0.1, 0.08), (0, -0.05, 0.84), frame_m, m.root, lods=(0, 1))
box(m.n("sill"), (1.66, 0.1, 0.08), (0, -0.05, -0.04), frame_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"jamb_{'ab'[s > 0]}"), (0.08, 0.1, 0.8), (s * 0.79, -0.05, 0.4), frame_m, m.root, lods=(0, 1))

# Piers, each the height of its own building's wall.
WORKS_WALL_M, DEPOT_WALL_M = 6.2, 7.9
m = kit.module("works_pier", ground=True, **OFFER)
box(m.n("brick"), (0.6, 0.22, WORKS_WALL_M), (0, -0.11, WORKS_WALL_M / 2), brick_m, m.root)
box(m.n("cap"), (0.72, 0.28, 0.14), (0, -0.14, WORKS_WALL_M - 0.07), concrete_m, m.root, lods=(0, 1))
m = kit.module("depot_pier", ground=True, **OFFER)
box(m.n("column"), (0.5, 0.2, DEPOT_WALL_M), (0, -0.1, DEPOT_WALL_M / 2), pier_m, m.root)

# Rainwater: a bay of gutter, and a metre of downpipe a row stretches to the eaves.
m = kit.module("gutter_bay", **OFFER)
box(m.n("trough"), (3.0, 0.16, 0.12), (0, -0.08, -0.06), trim_m, m.root)
m = kit.module("downpipe", **OFFER)
cyl(m.n("pipe"), 0.06, 1.0, (0, -0.08, 0.5), "Z", trim_m, m.root, seg=6, caps=False)

m = kit.module("ladder", **OFFER)  # 3 m of cat ladder
for s in (-1, 1):
    box(m.n(f"rail_{'ab'[s > 0]}"), (0.05, 0.05, 3.0), (s * 0.25, -0.2, 1.5), galv_m, m.root)
    for k in range(2):
        box(m.n(f"stay_{'ab'[s > 0]}_{k}"), (0.04, 0.2, 0.04), (s * 0.25, -0.1, 0.4 + 2.2 * k), galv_m, m.root, lods=(0,))
for k in range(10):
    box(m.n(f"rung_{k}"), (0.5, 0.03, 0.03), (0, -0.2, 0.15 + 0.3 * k), galv_m, m.root, lods=(0,))

# ---------------------------------------------------------------- roof furniture
m = kit.module("turbine_vent", **OFFER)
cyl(m.n("neck"), 0.2, 0.4, (0, 0, 0.2), "Z", galv_m, m.root, seg=10, lods=(0, 1))
cyl(m.n("head"), 0.32, 0.3, (0, 0, 0.55), "Z", galv_m, m.root, seg=12, r2=0.27)
cyl(m.n("cap"), 0.27, 0.1, (0, 0, 0.75), "Z", galv_m, m.root, seg=12, r2=0.05, lods=(0, 1))

m = kit.module("ridge_vent", **OFFER)  # 3 m of ventilator astride a ridge, along +X
prism(m.n("cowl"), [(-0.35, -0.15), (0.35, -0.15), (0.35, 0.3), (0.0, 0.46), (-0.35, 0.3)], 3.0, (0, 0, 0), galv_m, m.root,
      rot=(0, 0, math.pi / 2))
box(m.n("throat"), (2.9, 0.74, 0.14), (0, 0, 0.12), trim_m, m.root, lods=(0, 1))

m = kit.module("skylight_dome", **OFFER)
box(m.n("curb"), (1.3, 1.3, 0.25), (0, 0, 0.125), trim_m, m.root, lods=(0, 1))
box(m.n("dome"), (1.2, 1.2, 0.3), (0, 0, 0.4), dome_m, m.root, taper=(0.55, 0.55))

m = kit.module("roof_unit", **OFFER)  # a packaged air handler on skids
box(m.n("case"), (2.2, 1.3, 0.9), (0, 0, 0.65), galv_m, m.root)
cyl(m.n("fan"), 0.4, 0.08, (-0.45, 0, 1.14), "Z", trim_m, m.root, seg=14, lods=(0, 1, 2))
box(m.n("grille"), (1.6, 0.03, 0.5), (0, -0.66, 0.65), trim_m, m.root, lods=(0, 1))
for s in (-1, 1):
    box(m.n(f"skid_{'ab'[s > 0]}"), (0.12, 1.3, 0.2), (s * 0.8, 0, 0.1), trim_m, m.root, lods=(0, 1))

m = kit.module("duct_run", **OFFER)  # 3 m of duct on feet, along +X
box(m.n("duct"), (3.0, 0.5, 0.4), (0, 0, 0.5), galv_m, m.root)
for s in (-1, 1):
    box(m.n(f"foot_{'ab'[s > 0]}"), (0.08, 0.6, 0.3), (s * 1.0, 0, 0.15), trim_m, m.root, lods=(0, 1))
    box(m.n(f"flange_{'ab'[s > 0]}"), (0.05, 0.56, 0.46), (s * 1.475, 0, 0.5), galv_m, m.root, lods=(0, 1))

m = kit.module("flue_stack", **OFFER)
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
    surface(m, "flashings", trims, trim_m)
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
        concrete_m)
    for k, y in enumerate((y0 + t / 2, y1 - t / 2)):
        box(m.n(f"{tag}_coping_{k}"), (x1 - x0 + 0.1, t + 0.1, 0.08), ((x0 + x1) / 2, y, top - 0.04), concrete_m, m.root)
    for k, x in enumerate((x0 + t / 2, x1 - t / 2)):
        box(m.n(f"{tag}_coping_{k + 2}"), (t + 0.1, y1 - y0 - 2 * t - 0.1, 0.08), (x, (y0 + y1) / 2, top - 0.04), concrete_m, m.root)
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


def drain(t, edge, edge_z, pipes, gutter=True):
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
    t.cover_bays("strip_window", FOOT_M)
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
                    # a panel on a wall gives up its own UVs (the export box-projects it), so a run of them can become one face
                    own = bool(o.data.uv_layers) and o.data.materials[0] not in (glazing_m, band_m)
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
    open_walls(shell, t.rows["intact"], kit.openings)  # near, an office's window is an opening with its room behind
    furnish(t, kit)


# ---------------------------------------------------------------- ruins
# A shed does not fall as a house does. A steel one leaves its block dado as stumps, its
# cladding torn and standing in buckled lengths, its portal frames' legs leaning, and its
# roof lying over the wreck as buckled sheets. A concrete or masonry one leaves broken
# panels or ragged walls, and its roof the same way: sheets, or slabs of a flat roof's deck.
wreckage(kit, rubble_m, steel_m, beam="steel_beam", section=(0.2, 0.3))
RUIN_BAKE = RUIN | dict(ao_distance=2.5, paint_scale=8.0)  # a heap's cell is 3.4 m: no split for paint
# Each structure's stumps (thickness, a run's width, how far the top walks), what its heap is, and
# what its frame's legs are made of (None: its walls bore the roof).
STRUCTURES = {
    "steel": dict(thick=0.2, run=(1.0, 2.4), jagged=1.0, rubble=rubble_of("ash", (0.05, 0.048, 0.045), 1.0), legs=steel_m, dust=(96, 92, 88)),
    "precast": dict(thick=0.2, run=(1.6, 3.0), jagged=3.0, rubble=rubble_of("concrete", (0.15, 0.148, 0.14), 2.0), legs=None, dust=(196, 194, 186)),
    "brick": dict(thick=0.35, run=(0.3, 0.9), jagged=1.5, rubble=rubble_of("brick", (0.12, 0.062, 0.046), 3.0), legs=None, dust=(170, 100, 80)),
    "render": dict(thick=0.3, run=(0.3, 0.9), jagged=1.5, rubble=rubble_of("render", (0.16, 0.15, 0.135), 4.0), legs=None, dust=(204, 196, 178)),
    "block": dict(thick=0.25, run=(0.4, 1.2), jagged=1.5, rubble=rubble_of("block", (0.14, 0.14, 0.13), 5.0), legs=pier_m, dust=(190, 190, 182)),
}
# What breaks a wall near the ground: fitting -> its opening's width.
OPENINGS = {"roller_door": 5.2, "personnel_door": 1.2, "office_door": 1.8, "dock_door": 2.8, "factory_window": 2.1, "office_window": 1.8,
            "strip_window": 2.6, "loft_window": 2.1, "barn_window": 2.1, "atelier_window": 2.1}


def buckled_sheets(m, tag, ruin, crest, mat, seed, pitch=(9.0, 6.5), stiff=False):
    """A roof fallen over a ruin's heap: lengths of it, each lying where it was with a fold
    or two thrown up (or, `stiff`, a flat roof's deck in tilted slabs), none above `crest`.
    The finest tier gives each an underside; the third draws every other one, flat."""
    rng = random.Random(seed * 53 + 1)
    ix0, ix1, iy0, iy1 = ruin.inner
    nx, ny = max(1, round((ix1 - ix0) / pitch[0])), max(1, round((iy1 - iy0) / pitch[1]))
    sheets = []
    for j in range(ny):
        for i in range(nx):
            w, d = (ix1 - ix0) / nx * rng.uniform(0.9, 1.15), (iy1 - iy0) / ny * rng.uniform(0.85, 1.1)
            x = ix0 + (i + 0.5 + rng.uniform(-0.12, 0.12)) * (ix1 - ix0) / nx
            y = iy0 + (j + 0.5 + rng.uniform(-0.12, 0.12)) * (iy1 - iy0) / ny
            yaw, folds = rng.uniform(-0.16, 0.16), [rng.random() for _ in range(2 if stiff else 4)]
            if rng.random() > 0.14:
                sheets.append((x, y, w, d, yaw, folds))

    def build(bm, lod):
        for k, (x, y, w, d, yaw, folds) in enumerate(sheets):
            if lod == 2 and k % 2:
                continue
            stations = folds if lod == 0 else (folds[0], folds[len(folds) // 2], folds[-1]) if lod == 1 and not stiff else (folds[0], folds[-1])
            c, s = math.cos(yaw), math.sin(yaw)

            def at(u, v, lift):
                px = min(max(x + u * c - v * s, ix0 + 0.05), ix1 - 0.05)
                py = min(max(y + u * s + v * c, iy0 + 0.05), iy1 - 0.05)
                base = ruin.heap(px, py)
                return (px, py, min(crest - 0.04, base + 0.06 + lift * min(1.1, crest - base - 0.1)))

            for under in (False, True) if lod == 0 else (False,):
                for q in range(len(stations) - 1):
                    u0, u1 = -w / 2 + w * q / (len(stations) - 1), -w / 2 + w * (q + 1) / (len(stations) - 1)
                    quad = [at(u0, -d / 2, stations[q]), at(u1, -d / 2, stations[q + 1]), at(u1, d / 2, stations[q + 1]), at(u0, d / 2, stations[q])]
                    if under:
                        quad = [(px, py, pz - 0.04) for px, py, pz in reversed(quad)]
                    bm.faces.new([bm.verts.new(v) for v in quad])
        return bool(sheets)

    mesh_part(m.n(f"{tag}_roof"), build, mat, m.root, lods=(0, 1, 2))


STIFF = [felt_m]  # the roofs that fall as slabs of their deck, not as buckled sheet


def wrecked(t, tag, tint, blocks):
    """A template's ruin: one module in the building's paint (`tint`), and the set's wreckage
    about it. `blocks` is each part's (id, structure, wall material, roof material): a sheet
    roof falls as buckled sheets, a flat one as slabs of its deck."""
    m = kit.module(f"{tag}_ruin", ground=True, **RUIN_BAKE)
    high, over, seed = t.ruin_height(), kit.fit["ruin_top_m"], zlib.crc32(t.id.encode()) % 997
    crest = high + over - 0.06
    for k, (part, structure, wall, roof) in enumerate(blocks):
        how = STRUCTURES[structure]
        p = next(p for p in t.parts if p["id"] == part)
        rect = (p["x0"], p["x1"], p["y0"], p["y1"])
        sides = ruin_sides(t, part, OPENINGS)
        steel, flat = structure == "steel", roof in STIFF
        torn = scorched(wall, high + over, seed + k, floor=0.3 if steel else 0.0)
        long = max(rect[1] - rect[0], rect[3] - rect[2])
        ruin = ruin_block(m, part, rect, high, over, sides, dado_m if steel else wall, None, how["rubble"], seed + 17 * k,
                          thick=how["thick"], run=how["run"], jagged=how["jagged"], stump=DADO_M if steel else None,
                          cells=(3.4, 6.8, 14.0), coarse=max(6, round(long / 5)), roofing=False, far_wall=torn)
        for j, side in enumerate(sorted(sides)) if steel else ():  # the cladding, torn off its rails and standing in lengths
            ragged_wall(m.n(f"{part}_{side}_sheet"), *wall_line(rect, side, 0.05), 0.2 * high, crest, seed * 37 + j,
                        torn, m.root, 0.06, gaps=sides[side]["gaps"], openings=sides[side]["openings"], lods=(0, 1, 2), groups=(1, 2, 5, 8),
                        jagged=3.0, run=(1.2, 3.0), buckle=0.22)
        fallen = scorched(roof, 3.0 * (high + over), seed + 2.0, floor=0.6) if flat else burnt_sheet(roof, seed)
        buckled_sheets(m, part, ruin, crest, fallen, seed + k, stiff=flat)
        x0, x1, y0, y1 = rect
        surface(m, f"{part}_far_top", [([(x0, y0, 0.8 * high), (x1, y0, 0.8 * high), (x1, y1, 0.8 * high), (x0, y1, 0.8 * high)], UP)], fallen,
                lods=(3,), far_grid=2 * ROOF_FAR_CELL_M)  # in cells, so the rust and the soot keep their patches
        rng = random.Random(seed * 71 + k)
        if how["legs"] is not None:  # the frame's legs down the two long sides, a bay apart, leaning as the roof pulled them
            along_x = x1 - x0 >= y1 - y0
            a0, a1 = (x0, x1) if along_x else (y0, y1)
            for i in range(int((a1 - a0) // 6) + 1):
                for s in (-1, 1):
                    at = min(max(a0 + 6.0 * i, a0 + 0.5), a1 - 0.5)
                    across = (y0 + y1) / 2 + s * ((y1 - y0) / 2 - 0.55) if along_x else (x0 + x1) / 2 + s * ((x1 - x0) / 2 - 0.55)
                    tall, lean = rng.uniform(0.55, 0.92) * (crest - 0.25), rng.uniform(0.05, 0.22)
                    rot = (s * lean, 0, 0) if along_x else (0, -s * lean, 0)
                    mid = tall / 2 * math.cos(lean) + 0.13 * math.sin(lean) + 0.01  # its low corner on the ground
                    box(m.n(f"{part}_leg_{i}_{'ab'[s > 0]}"), (0.26, 0.26, tall), (at, across, mid) if along_x else (across, at, mid),
                        how["legs"], m.root, rot=rot, lods=(0, 1))
        area = (x1 - x0) * (y1 - y0)
        litter(t, ruin, how["dust"], beams=min(30, round(area / 55)), heaps=min(7 if steel else 12, round(area / 80)), long=9.0)
    t.place(m.name, tint=tint, state="ruin")


# ---------------------------------------------------------------- the workshop shed
# Steel sheet on a block dado, gable to the street, under a sheet roof: a vehicle door and a
# side door in the gable, windows down both sides, one bay re-sheeted in another paint.
def shed(id_, tint, door, roof, light, patch, mends):
    W, D, EAVE, TOP = 15.0, 24.0, 4.8, 6.2
    m = kit.module("shed_shell", ground=True, paint_scale=3.0)
    tan, edge_z = gabled_shell(m, D, W, EAVE, TOP, clad_m, ROOFS[roof], lights=(ROOFLIGHTS[light], 6.0, 12.0, (0.2, 0.85)), mends=mends)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="steel portal frame, block dado",
                                                   Roof="gable", RidgeAlong="y", Wall="cladding", Sheet=roof))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, yaw=math.pi / 2, tint=tint)  # the shell is built ridge along x
    taken = [side_door(t, "body-south", 3.0, door), roller(t, "body-south", -1.5, door)]
    along(t, "dado_bay", "body-south", taken=taken, tiers=TIERS_0_TO_2)
    along(t, "strip_window", "body-south", z=2.0, taken=taken)
    t.mount("louvre", "body-south", -1.5, z=4.9, tiers=TIERS_0_TO_1)
    along(t, "dado_bay", "body-north", tiers=TIERS_0_TO_2)
    along(t, "strip_window", "body-north", z=2.0, every=2, start=1)
    t.mount("louvre", "body-north", 0.0, z=5.0, tiers=TIERS_0_TO_1)
    for side in ("east", "west"):
        edge = f"body-{side}"
        along(t, "dado_bay", edge, tiers=TIERS_0_TO_2)
        for o in t.bays(edge)[1:-1]:
            t.mount("strip_window", edge, o, z=2.0)
        drain(t, edge, edge_z, (-D / 2 + 0.4, D / 2 - 0.4))
    t.mount("clad_panel_window", "body-east", 10.5, z=DADO_M + 0.1, tiers=TIERS_0_TO_2, tint=patch)  # a bay re-sheeted, never painted to match
    for y in (-5.0, 5.0):
        t.place("turbine_vent", 0.0, y, TOP - 0.1)
    t.place("flue_stack", 3.4, 8.0, TOP - 3.4 * tan - 0.2, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "shed", tint, [("body", "steel", clad_m, ROOFS[roof])])


# ---------------------------------------------------------------- the dock warehouse
# Precast concrete panels under a flat felt roof: a row of loading docks under a canopy
# along the street side, an office corner at one end and a drive-in door at the other.
def dock_warehouse(id_):
    W, D, TOP = 48.0, 24.0, 9.0
    m = kit.module("slab_shell", ground=True, paint_scale=4.0)
    deck = flat_shell(m, "box", -W / 2, W / 2, -D / 2, D / 2, TOP, slab_m, felt_m,
                      mends=[(-8.5, 7.5, 5.0, 3.0, "new"), (8.5, -8.0, 3.0, 6.0, "silver"), (17.0, 8.5, 6.0, 4.0, "new"),
                             (-1.0, -9.5, 4.0, 2.0, "silver"), (21.0, -2.0, 3.0, 5.0, "new"), (-14.0, 1.0, 2.0, 7.0, "silver")])
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="precast concrete panels", Roof="flat",
                                                   Wall="tilt_slab", Docks=10))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, tint=STONE)
    taken = [side_door(t, "body-south", -22.5, RUSSET), roller(t, "body-south", 18.0, RUSSET)]
    t.mount("office_window", "body-south", -19.5, z=0.95)
    for k in range(10):
        t.mount("dock_door", "body-south", -16.5 + 3 * k, tint=IVORY)
    canopy(t, "body-south", -18.0, 12.0, 4.55)
    canopy(t, "body-south", 15.0, 21.0, 4.75)
    for side in ("south", "north", "east", "west"):
        edge = f"body-{side}"
        along(t, "paint_band", edge, z=7.3, tint=RUSSET)
        if side != "south":
            along(t, "clerestory", edge, z=6.0, every=2)
    for o in (-21.0, -9.0, 3.0, 15.0):
        t.mount("downpipe", "body-north", o, scale=(1.0, 1.0, deck), tiers=TIERS_0_TO_1)
    for k in range(3):
        t.mount("ladder", "body-east", 9.0, z=3.0 * k, tiers=TIERS_0_TO_1)
    for x in (-20.0, -12.0, -4.0, 4.0, 12.0, 20.0):
        for y in (-5.0, 5.0):
            if (x, y) != (-20.0, -5.0):
                t.place("skylight_dome", x, y, deck)
    t.place("roof_unit", -20.0, -6.5, deck)
    t.place("roof_unit", -16.0, -9.0, deck, math.pi / 2)
    for k in range(4):
        t.place("duct_run", -17.4 + 3 * k, -6.5, deck, tiers=TIERS_0_TO_2)
    for x in (-8.0, 8.0, 16.0):
        t.place("turbine_vent", x, 0.0, deck)
    t.place("flue_stack", -21.5, 9.5, deck, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "slab", STONE, [("body", "precast", slab_m, felt_m)])


# ---------------------------------------------------------------- the distribution warehouse
# Two spans of steel sheet with a band of colour under the eaves and a roof striped with
# rooflights: three vehicle doors, each under its canopy with a side door.
def span_warehouse(id_, tint, band, roof, light, mends):
    W, D, EAVE, TOP = 72.0, 33.0, 8.4, 10.0
    m = kit.module("span_shell", ground=True, paint_scale=5.0)
    tan, edge_z = gabled_shell(m, W, D, EAVE, TOP, clad_m, ROOFS[roof], spans=2, lights=(ROOFLIGHTS[light], 4.5, 9.0, (0.15, 0.88)),
                               mends=mends, plinth=0.5)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="steel portal frame, two spans",
                                                   Roof="twin gable", RidgeAlong="x", Wall="cladding", Sheet=roof))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, tint=tint)
    taken = []
    for x in (0.0, -24.0, 24.0):
        taken.append(roller(t, "body-south", x, band))
        taken.append(side_door(t, "body-south", x + 4.5, band, entrance=False))
        canopy(t, "body-south", x - 3.0, x + 6.0, 4.75)
    along(t, "clerestory", "body-south", z=5.9, taken=[(x, 4.5) for x in (1.5, -22.5, 25.5)])
    along(t, "clerestory", "body-north", z=5.9, every=2)
    for side in ("south", "north", "east", "west"):
        along(t, "clad_panel", f"body-{side}", z=7.15, scale=(1.0, 1.0, 0.4), tint=band)
    for side in ("east", "west"):
        for o in (-D / 4, D / 4):
            t.mount("louvre", f"body-{side}", o - 0.75, z=8.6, tiers=TIERS_0_TO_1)
    drain(t, "body-south", edge_z, (-35.6, -12.0, 12.0, 35.6))
    drain(t, "body-north", edge_z, (-35.6, -12.0, 12.0, 35.6))
    for y in (-D / 4, D / 4):
        for k in range(6):
            t.place("ridge_vent", -30.0 + 12 * k, y, TOP)
    fold(t, m)
    wrecked(t, "span", tint, [("body", "steel", clad_m, ROOFS[roof])])


# ---------------------------------------------------------------- the works
# A hall under a sawtooth of north lights, tall steel windows between its piers, and a
# two-storey office block joined to its street side. Both boxes are as tall as the
# sawtooth's ridges (a join needs equal tops); the office's parapet meets them.
WORKS_HALL = (54.0, 27.0, 18.0, 9.0, 8.0, 9)  # the hall's width and depth, the office's, the top, the teeth


def tooth(name, width, depth, valley, top, sheet_, gablet):
    """One tooth of a sawtooth roof, `width` of the hall across `depth`: glazing leaning back from
    its kerb to the ridge, sheet falling from the ridge to the next valley, and a gablet at each end."""
    m = kit.module(name, paint_scale=5.0)
    hy, kerb, lean = depth / 2, 0.3, 0.45
    ridge_x = -width / 2 + lean
    surface(m, "glass", [([(-width / 2, hy, valley + kerb), (-width / 2, -hy, valley + kerb), (ridge_x, -hy, top), (ridge_x, hy, top)],
                          (-1, 0, 0.3))], glazing_m, uv=((-width / 2, hy, valley + kerb), (0, -1, 0), (0, 0, 1)))
    surface(m, "kerb", [([(-width / 2, hy, valley), (-width / 2, -hy, valley), (-width / 2, -hy, valley + kerb), (-width / 2, hy, valley + kerb)],
                         (-1, 0, 0))], trim_m, lods=(0, 1))
    skin(m, "sheet", [([(ridge_x, -hy, top), (ridge_x, hy, top), (width / 2, hy, valley), (width / 2, -hy, valley)], UP)], sheet_,
         uv="slope", grid=ROOF_CELL_M)
    surface(m, "gablets", [([(-width / 2, s * hy, valley), (width / 2, s * hy, valley), (ridge_x, s * hy, top)], (0, s, 0)) for s in (-1, 1)],
            gablet)
    surface(m, "flashing", [([(ridge_x - 0.02, -hy, top + 0.03), (ridge_x - 0.02, hy, top + 0.03), (ridge_x + 0.35, hy, top - 0.08),
                              (ridge_x + 0.35, -hy, top - 0.08)], UP)], trim_m, lods=(0, 1))
    surface(m, "gutter", [([(width / 2 - 0.5, -hy, valley + 0.2), (width / 2 - 0.5, hy, valley + 0.2), (width / 2, hy, valley + 0.2),
                            (width / 2, -hy, valley + 0.2)], UP)], trim_m, lods=(0, 1))
    return m


def works(id_, hall, gablet, office, tint, roof, door, recipe, ruin, office_roof=(), office_steps=()):
    """`hall` and `office` are their walls' materials, `gablet` the sawtooth's ends'; `ruin` the
    structure each falls as; `office_roof` what stands on the office's roof besides its plant, and
    `office_steps` a stepped parapet over its street front: [(from, to, how high over the top)]
    about the front's middle."""
    HALL_W, HALL_D, OFFICE_W, OFFICE_D, TOP, TEETH = WORKS_HALL
    VALLEY, STOREY = WORKS_WALL_M, 3.6
    hall_y = OFFICE_D / 2  # the hall stands behind the office: the footprint of the two is centred on the origin
    y0, y1 = hall_y - HALL_D / 2, hall_y + HALL_D / 2

    m = kit.module("works_shell", ground=True, paint_scale=3.0)
    hx = HALL_W / 2
    surface(m, "hall_walls", box_walls(-hx, hx, y0, y1, VALLEY), hall)
    skirt(m, "hall_plinth", -hx, hx, y0, y1, 0.0, 0.6, plinth_m, out=0.06)
    skirt(m, "hall_beam", -hx, hx, y0, y1, VALLEY - 0.45, VALLEY, concrete_m, out=0.04)  # the ring beam the roof sits on
    office_deck = flat_shell(m, "office", -hx, -hx + OFFICE_W, y0 - OFFICE_D, y0, TOP, office, felt_m,
                             mends=[(-hx + 14.5, y0 - 2.0, 4.0, 2.0, "new"), (-hx + 3.0, y0 - 2.5, 2.0, 3.0, "silver")])
    tooth("works_tooth", HALL_W / TEETH, HALL_D, VALLEY, TOP, ROOFS[roof], gablet)
    if office_steps:
        mid = -hx + OFFICE_W / 2
        stepped(m, "office_front", 1, y0 - OFFICE_D, -1, TOP, [(mid + a, mid + b, TOP + h) for a, b, h in office_steps], office, thick=PARAPET_W)
        box(m.n("office_band"), (OFFICE_W + 0.2, 0.3, 0.35), (mid, y0 - OFFICE_D - 0.06, TOP - 0.95), concrete_m, m.root, lods=(0, 1, 2))

    TOOTH = HALL_W / TEETH
    t = kit.template(id_, "industry", FAMILY, dict(HallWidth=HALL_W, HallDepth=HALL_D, OfficeWidth=OFFICE_W, OfficeDepth=OFFICE_D,
                                                   Floors=2, **recipe, Roof="sawtooth", Teeth=TEETH, Sheet=roof))
    t.part("hall", -hx, hx, y0, y1, TOP)
    t.part("office", -hx, -hx + OFFICE_W, y0 - OFFICE_D, y0, TOP)
    t.floors(0.0, STOREY)
    for edge, length in (("hall-north", HALL_W), ("hall-east", HALL_D), ("hall-west", HALL_D), ("office-south", OFFICE_W),
                         ("office-east", OFFICE_D), ("office-west", OFFICE_D)):
        t.lattice(edge, phase(length))
    t.lattice("hall-south-1", 1.5)  # the stretch east of the office: bays on the hall's own lattice
    t.place("works_shell", tint=tint)
    for k in range(TEETH):
        t.place("works_tooth", -hx + TOOTH * (k + 0.5), hall_y)
    taken = [side_door(t, "office-south", -1.5, door, module="office_door"), roller(t, "hall-south-1", 12.0, door)]
    for floor in (0.0, STOREY):
        for edge in ("office-south", "office-east", "office-west"):
            along(t, "office_window", edge, z=floor + 0.95, taken=taken[:1] if floor == 0.0 and edge == "office-south" else ())
    for edge in ("hall-north", "hall-east", "hall-west", "hall-south-1"):
        along(t, "factory_window", edge, z=1.2, taken=taken[1:] if edge == "hall-south-1" else ())
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
    t.place("roof_unit", -hx + 4.0, y0 - 5.5, office_deck)
    t.place("skylight_dome", -hx + 9.0, y0 - 4.5, office_deck)
    t.place("skylight_dome", -hx + 13.0, y0 - 4.5, office_deck)
    for module, x, y in office_roof:
        t.place(module, -hx + x, y0 - y, office_deck)
    fold(t, kit.modules["works_shell"])
    wrecked(t, "works", tint, [("hall", ruin[0], hall, ROOFS[roof]), ("office", ruin[1], office, felt_m)])


# ---------------------------------------------------------------- a stepped false front
def stepped(m, tag, axis, plane, out, base, segments, mat, thick=0.3):
    """A brick parapet standing on a wall's top (`base`) or before a gable: faces on the
    plane `plane` across `axis` (0: the wall runs along y, at x = plane; 1: along x),
    looking `out` (+1 or -1), and `thick` behind it, its top in steps: `segments` is
    [(from, to, height)] along the wall. Each step's top carries a coping."""
    quads = []
    back = plane - out * thick

    def at(u, w, z):
        return (w, u, z) if axis == 0 else (u, w, z)

    normal = (out, 0, 0) if axis == 0 else (0, out, 0)
    for k, (a, b, h) in enumerate(segments):
        quads.append(([at(a, plane, base), at(b, plane, base), at(b, plane, h), at(a, plane, h)], normal))
        quads.append(([at(a, back, base), at(b, back, base), at(b, back, h), at(a, back, h)], tuple(-c for c in normal)))
        quads.append(([at(a, plane, h), at(b, plane, h), at(b, back, h), at(a, back, h)], (0, 0, 1)))
        for u, h2, side in ((a, segments[k - 1][2] if k else base, -1), (b, segments[k + 1][2] if k + 1 < len(segments) else base, 1)):
            if h > h2:  # the step's end where it stands over its neighbour (or the wall's end)
                quads.append(([at(u, plane, h2), at(u, back, h2), at(u, back, h), at(u, plane, h)], at(side, 0, 0)))
        mid, length = (a + b) / 2, b - a
        box(m.n(f"{tag}_coping_{k}"), (thick + 0.12, length + 0.1, 0.1) if axis == 0 else (length + 0.1, thick + 0.12, 0.1),
            at(mid, plane - out * thick / 2, h + 0.05), concrete_m, m.root, lods=(0, 1, 2))
    surface(m, tag, quads, mat)


# ---------------------------------------------------------------- New York's brick loft
# Three storeys of tinted brick round a flat roof: a steel loft window in every bay of every
# floor under a stone lintel, a stone band over the ground floor and a cornice under the
# parapet, stepped parapets over the two ends, a row of loading docks under a canopy, and the
# water tank on its stand on the roof.
def loft(id_, tint, door, wall):
    W, D, STOREY, FLOORS = 48.0, 24.0, 4.2, 3
    TOP = STOREY * FLOORS + 1.0
    m = kit.module("loft_shell", ground=True, paint_scale=4.0)
    deck = flat_shell(m, "box", -W / 2, W / 2, -D / 2, D / 2, TOP, wall, felt_m,
                      mends=[(-12.5, 6.5, 5.0, 3.0, "new"), (10.0, -7.0, 3.0, 5.0, "silver"), (-2.0, -8.5, 4.0, 2.0, "new")])
    skirt(m, "band", -W / 2, W / 2, -D / 2, D / 2, STOREY - 0.3, STOREY, concrete_m, out=0.08)
    skirt(m, "cornice", -W / 2, W / 2, -D / 2, D / 2, TOP - 1.05, TOP - 0.7, concrete_m, out=0.22)
    for s in (-1, 1):  # the ends' parapets stand up in two steps over their middles
        stepped(m, f"step_{'ab'[s > 0]}", 0, s * W / 2, s, TOP, [(-6.0, -3.0, TOP + 0.5), (-3.0, 3.0, TOP + 1.0), (3.0, 6.0, TOP + 0.5)], wall,
                thick=PARAPET_W)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=FLOORS, Structure="brick loft, timber floors", Roof="flat",
                                                   Wall="brick", Docks=4, Tank=True))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(*(STOREY * k for k in range(FLOORS)))
    lattices(t, "body", W, D)
    t.place(m.name, tint=tint)
    taken = [side_door(t, "body-south", -22.5, door), roller(t, "body-south", 0.0, door, entrance=False)]
    for k in range(4):
        t.mount("dock_door", "body-south", -16.5 + 3 * k, tint=door)
        taken.append((-16.5 + 3 * k, 1.4))
    canopy(t, "body-south", -18.0, -6.0, 4.55)
    t.mount("roller_door", "body-north", 0.0, tint=door)
    for side in ("south", "north", "east", "west"):
        edge = f"body-{side}"
        for floor in range(FLOORS):
            gone = taken if side == "south" and floor == 0 else [(0.0, 3.0)] if side == "north" and floor == 0 else ()
            along(t, "loft_window", edge, z=STOREY * floor + 0.9, taken=gone)
    for o in (-21.0, -3.0, 15.0):
        t.mount("downpipe", "body-north", o, scale=(1.0, 1.0, deck), tiers=TIERS_0_TO_1)
    t.place("water_tank", 13.5, 4.0, deck)
    for x, y in ((-15.0, 4.0), (-5.0, 4.0), (5.0, -4.0)):
        t.place("skylight_dome", x, y, deck)
    t.place("turbine_vent", -9.0, -5.0, deck)
    t.place("turbine_vent", 2.0, 6.0, deck)
    t.place("flue_stack", 21.0, -8.5, deck, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "loft", tint, [("body", "brick", wall, felt_m)])


# ---------------------------------------------------------------- the gabled barn behind a stepped front
# A long hall of gabled spans side by side, their gable ends to the street hidden behind a
# brick false front that steps up over each span: a vehicle door in every span, tall windows
# either side, the side walls windowed to the eaves, a ridge vent along each ridge. New York's
# warehouse and car barn.
def barn(id_, tag, W, D, spans, eave, ridge, tint, door, wall, roof, light, recipe, mends=()):
    m = kit.module(f"{tag}_shell", ground=True, paint_scale=5.0)
    # built with its ridges along x and turned a quarter: module x is the template's y, module y its -x
    tan, edge_z = gabled_shell(m, D, W, eave, ridge, wall, ROOFS[roof], spans=spans, lights=(ROOFLIGHTS[light], 4.5, 9.0, (0.2, 0.8)),
                               mends=mends, plinth=0.5)
    half = W / spans / 2
    front = []
    for k in range(spans):  # each span's front in three steps, standing well clear of the roof behind it
        yk = -W / 2 + (2 * k + 1) * half
        for a, b, rise, over in ((-1.0, -2 / 3, 1 / 3, 1.0), (-2 / 3, -1 / 3, 2 / 3, 1.3), (-1 / 3, 1 / 3, 1.0, 1.9), (1 / 3, 2 / 3, 2 / 3, 1.3),
                                 (2 / 3, 1.0, 1 / 3, 1.0)):
            front.append((yk + a * half, yk + b * half, eave + rise * (ridge - eave) + over))
    stepped(m, "front", 0, -D / 2 - 0.2, -1, eave - 0.6, front, wall)
    surface(m, "front_soffit", [([(-D / 2 - 0.2, -W / 2, eave - 0.6), (-D / 2 - 0.2, W / 2, eave - 0.6), (-D / 2, W / 2, eave - 0.6),
                                  (-D / 2, -W / 2, eave - 0.6)], (0, 0, -1))], wall, lods=(0, 1))
    box(m.n("front_band"), (0.36, W + 0.1, 0.3), (-D / 2 - 0.2, 0.0, eave - 0.45), concrete_m, m.root, lods=(0, 1, 2))
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="brick walls, steel trusses", Roof="gables to the street",
                                                   Spans=spans, RidgeAlong="y", Wall="brick", Sheet=roof, **recipe))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, ridge)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, yaw=math.pi / 2, tint=tint)
    taken = []
    for k in range(spans):
        x = -W / 2 + (2 * k + 1) * half
        taken.append(roller(t, "body-south", x, door, entrance=k in (0, spans - 1)))
        if k == spans // 2:
            taken.append(side_door(t, "body-south", x + 4.5, door))
    along(t, "barn_window", "body-south", z=1.2, taken=taken)
    along(t, "barn_window", "body-north", z=1.2, taken=[roller(t, "body-north", -W / 2 + (2 * (spans // 2) + 1) * half, door, entrance=False)])
    for side in ("east", "west"):
        along(t, "barn_window", f"body-{side}", z=1.2)
        drain(t, f"body-{side}", edge_z, (-D / 2 + 0.4, D / 2 - 0.4))
    for k in range(spans):
        x = -W / 2 + (2 * k + 1) * half
        for j in range(int((D - 6.0) // 3)):
            t.place("ridge_vent", x, -D / 2 + 4.5 + 3 * j, ridge, math.pi / 2, tiers=EVERY_TIER if j % 3 else TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, tag, tint, [("body", "brick", wall, ROOFS[roof])])


# ---------------------------------------------------------------- the atelier under a sawtooth
# Paris's: a rendered hall on a brick plinth with brick piers, tall steel windows between them,
# and a sawtooth of north lights across its whole width.
def atelier(id_, tint, door, wall, plinth, roof, gablet):
    W, D, TEETH = 48.0, 24.0, 8
    VALLEY, TOP = WORKS_WALL_M, WORKS_WALL_M + 1.6
    m = kit.module("atelier_shell", ground=True, paint_scale=3.0)
    surface(m, "walls", box_walls(-W / 2, W / 2, -D / 2, D / 2, VALLEY), wall)
    skirt(m, "plinth", -W / 2, W / 2, -D / 2, D / 2, 0.0, 0.9, plinth, out=0.06)
    skirt(m, "beam", -W / 2, W / 2, -D / 2, D / 2, VALLEY - 0.45, VALLEY, concrete_m, out=0.04)
    tooth("atelier_tooth", W / TEETH, D, VALLEY, TOP, ROOFS[roof], gablet)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="rendered brick, steel trusses", Roof="sawtooth",
                                                   Teeth=TEETH, Wall="render", Sheet=roof))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, tint=tint)
    for k in range(TEETH):
        t.place("atelier_tooth", -W / 2 + W / TEETH * (k + 0.5), 0.0)
    taken = [side_door(t, "body-south", -16.5, door), roller(t, "body-south", 12.0, door), roller(t, "body-west", 0.0, door, entrance=False)]
    for side in ("south", "north", "east", "west"):
        edge = f"body-{side}"
        along(t, "atelier_window", edge, z=1.0, taken=[o for o in taken[:2] if side == "south"] + ([taken[2]] if side == "west" else []))
        a, b = t.edges()[edge]["span"]
        piers = [a + 6 * k for k in range(int((b - a) // 6) + 1)]
        for o in piers + [b] * (b - piers[-1] > 1.0):
            t.mount("works_pier", edge, min(max(o, a + 0.3), b - 0.3), tiers=TIERS_0_TO_1)
    for k in range(1, TEETH):
        t.mount("downpipe", "body-north", W / 2 - W / TEETH * k + 0.55, scale=(1.0, 1.0, VALLEY), tiers=TIERS_0_TO_1)
    t.place("flue_stack", 20.0, 9.0, VALLEY + 0.2, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "atelier", tint, [("body", "render", wall, ROOFS[roof])])


# ---------------------------------------------------------------- the depot
# A concrete frame filled with painted blockwork under an oxide-red roof with a glazed
# monitor along its ridge: two runs of vehicle doors under canopies, a band of high
# windows all round.
def monitor_depot(id_):
    W, D, EAVE, TOP = 90.0, 39.0, DEPOT_WALL_M, 11.0
    MONITOR = (4.5, 1.5, 6.0)
    m = kit.module("depot_shell", ground=True, paint_scale=5.0)
    tan, edge_z = gabled_shell(m, W, D, EAVE, TOP - MONITOR[1], block_m, ROOFS["oxide"], lights=(ROOFLIGHTS["pale"], 7.5, 15.0, (0.15, 0.6)),
                               mends=[(0, -1, -27.0, 4.5, (0.0, 0.75), "new"), (0, -1, 31.5, 3.0, (0.3, 0.75), "new"),
                                      (0, 1, 12.0, 6.0, (0.0, 0.4), "new"), (0, 1, -33.0, 3.0, (0.1, 0.75), "tarred"),
                                      (0, -1, 3.0, 2.25, (0.0, 0.3), "tarred")], monitor=MONITOR, plinth=0.4)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="concrete frame, block infill",
                                                   Roof="gable with monitor", RidgeAlong="x", Wall="concrete_block", Sheet="oxide", Doors=8))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, tint=BUFF)
    taken = [side_door(t, "body-south", -1.5, SLATE)]
    for x in (-30.0, -24.0, -18.0, -12.0, 12.0, 18.0, 24.0, 30.0):
        taken.append(roller(t, "body-south", x, SLATE, entrance=x in (-24.0, 24.0)))
    canopy(t, "body-south", -33.0, -9.0, 4.75)
    canopy(t, "body-south", 9.0, 33.0, 4.75)
    along(t, "strip_window", "body-south", z=1.3, taken=taken)
    for side in ("south", "north", "east", "west"):
        along(t, "window_band", f"body-{side}", z=5.8)
    along(t, "strip_window", "body-north", z=1.3, every=2)
    for edge, length in (("body-south", W), ("body-north", W)):
        for k in range(round(length / 6) + 1):
            t.mount("depot_pier", edge, min(max(-length / 2 + 6 * k, -length / 2 + 0.25), length / 2 - 0.25), tiers=TIERS_0_TO_1)
        drain(t, edge, edge_z, [-length / 2 + 6 + 12 * k + 0.4 for k in range(7)])
    for edge in ("body-east", "body-west"):
        for o in (-19.25, -13.5, -7.5, -1.5, 1.5, 7.5, 13.5, 19.25):
            t.mount("depot_pier", edge, o, tiers=TIERS_0_TO_1)
        t.mount("louvre", edge, -0.75, z=8.3, tiers=TIERS_0_TO_1)
    for x in (-30.0, -15.0, 0.0, 15.0, 30.0):
        for s in (-1, 1):
            t.place("turbine_vent", x, s * 11.0, EAVE + (D / 2 - 11.0) * tan - 0.1)
    for x in (-40.5, 27.0):
        t.place("flue_stack", x, 14.5, EAVE + (D / 2 - 14.5) * tan - 0.15, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "depot", BUFF, [("body", "block", block_m, ROOFS["oxide"])])


# ---------------------------------------------------------------- a barrel-vaulted depot
def vaulted_shell(m, length, depth, eave, crown, wall, roof, facets=12, lights=(), plinth=0.0):
    """Walls and a barrel vault over a hall `length` along x by `depth`: the vault an arc of a
    circle across the depth from eave to `crown`, in `facets` strips along x, overhanging the
    long walls; each end's wall closes it to the eaves and the arch over them is glazed. `lights`
    are the facets that carry rooflights, in lengths along them. Returns the eaves' edge height."""
    hx, hy, rise = length / 2, depth / 2, crown - eave
    radius = (hy * hy + rise * rise) / (2 * rise)

    def arc(y):
        return eave + math.sqrt(radius * radius - y * y) - (radius - rise)

    ys = [-hy + depth * k / facets for k in range(facets + 1)]
    pts = [(y, arc(y)) for y in ys]
    slope = hy / math.sqrt(radius * radius - hy * hy)  # the arc's fall at the eaves
    edge = (hy + OVER, eave - OVER * slope)
    outline = [(-edge[0], edge[1])] + pts + [edge]
    walls = []
    for s in (-1, 1):
        walls += upright((-hx, s * hy), (hx, s * hy), eave - DROP, (0, s, 0))
        walls += upright((s * hx, -hy), (s * hx, hy), eave - DROP, (s, 0, 0))
    skin(m, "walls", walls, wall)
    if plinth:
        skirt(m, "plinth", -hx, hx, -hy, hy, 0.0, plinth, plinth_m, out=0.05)
    x0, x1 = -hx - VERGE, hx + VERGE
    strips = [([(x0, a[0], a[1]), (x1, a[0], a[1]), (x1, b[0], b[1]), (x0, b[0], b[1])], UP) for a, b in zip(outline, outline[1:])]
    skin(m, "roof", strips, roof, uv="slope", grid=ROOF_CELL_M)
    trims = []
    for s, x in ((-1, x0), (1, x1)):  # the arch's edge down each end, and the glass under it
        trims += [([(x, a[0], a[1] + 0.03), (x, b[0], b[1] + 0.03), (x, b[0], b[1] - 0.3), (x, a[0], a[1] - 0.3)], (s, 0, 0))
                  for a, b in zip(outline, outline[1:])]
    for s in (-1, 1):
        trims.append(([(x0, s * edge[0], edge[1] + 0.02), (x1, s * edge[0], edge[1] + 0.02), (x1, s * edge[0], edge[1] - 0.2),
                       (x0, s * edge[0], edge[1] - 0.2)], (0, s, 0)))
    surface(m, "trims", trims, concrete_m)
    glass = []
    for s in (-1, 1):
        x = s * hx
        glass.append(([(x, -hy, eave - DROP)] + [(x, y, z - 0.05) for y, z in pts] + [(x, hy, eave - DROP)], (s, 0, 0),
                      ((x, -hy, eave), (0, -s, 0), (0, 0, 1))))
    surface(m, "lunettes", glass, glazing_m)
    ribs = []
    for s in (-1, 1):  # the glazing's mullions: a concrete rib up each end every third of its width
        x = s * (hx + 0.01)
        for y in (-hy / 3, hy / 3, 0.0):
            ribs.append(([(x, y - 0.2, eave - DROP), (x, y + 0.2, eave - DROP), (x, y + 0.2, arc(y + 0.2) - 0.05), (x, y - 0.2, arc(y - 0.2) - 0.05)],
                         (s, 0, 0)))
    surface(m, "ribs", ribs, concrete_m, lods=(0, 1, 2))
    sheets = []
    for k in lights:
        (ya, za), (yb, zb) = pts[k], pts[k + 1]
        n = Vector((0, za - zb, yb - ya)).normalized() * 0.04
        n = n if n.z > 0 else -n
        x = -hx + 3.0
        while x < hx - 3.0:
            sheets.append(([(x, ya + n.y, za + n.z), (x + 4.0, ya + n.y, za + n.z), (x + 4.0, yb + n.y, zb + n.z), (x, yb + n.y, zb + n.z)], UP))
            x += 9.0
    if sheets:
        surface(m, "rooflights", sheets, ROOFLIGHTS["pale"], uv="slope")
    return edge[1]


# A concrete barrel vault on rendered walls between a concrete frame's piers: vehicle doors
# along the street side under canopies, a band of high windows round the walls, and each end
# an arch of glass.
def vault_depot(id_, tint, door, wall, roof):
    W, D, EAVE, TOP = 90.0, 39.0, DEPOT_WALL_M, DEPOT_WALL_M + 4.6
    m = kit.module("vault_shell", ground=True, paint_scale=5.0)
    edge_z = vaulted_shell(m, W, D, EAVE, TOP, wall, roof, lights=(4, 7), plinth=0.4)
    t = kit.template(id_, "industry", FAMILY, dict(Width=W, Depth=D, Floors=1, Structure="concrete frame, rendered infill",
                                                   Roof="concrete barrel vault", VaultAlong="x", Wall="render", Doors=6))
    t.part("body", -W / 2, W / 2, -D / 2, D / 2, TOP)
    t.floors(0.0)
    lattices(t, "body", W, D)
    t.place(m.name, tint=tint)
    taken = [side_door(t, "body-south", -1.5, door)]
    for x in (-36.0, -27.0, -18.0, 18.0, 27.0, 36.0):
        taken.append(roller(t, "body-south", x, door, entrance=x in (-27.0, 27.0)))
        canopy(t, "body-south", x - 3.0, x + 3.0, 4.75)
    along(t, "strip_window", "body-south", z=1.3, taken=taken)
    along(t, "strip_window", "body-north", z=1.3, every=2)
    for side in ("south", "north"):
        along(t, "window_band", f"body-{side}", z=5.8)
    for edge, length in (("body-south", W), ("body-north", W)):
        for k in range(round(length / 6) + 1):
            t.mount("depot_pier", edge, min(max(-length / 2 + 6 * k, -length / 2 + 0.25), length / 2 - 0.25), tiers=TIERS_0_TO_1)
        drain(t, edge, edge_z, [-length / 2 + 6 + 12 * k + 0.4 for k in range(7)])
    for edge in ("body-east", "body-west"):
        for o in (-19.25, -13.5, -7.5, -1.5, 1.5, 7.5, 13.5, 19.25):
            t.mount("depot_pier", edge, o, tiers=TIERS_0_TO_1)
    for x in (-30.0, -10.0, 10.0, 30.0):
        t.place("turbine_vent", x, 0.0, TOP - 0.1)
    t.place("flue_stack", -40.0, 12.0, TOP - 1.6, tiers=TIERS_0_TO_2)
    fold(t, m)
    wrecked(t, "vault", tint, [("body", "render", wall, roof)])


# ---------------------------------------------------------------- each family's five
def china():
    shed("china-shed-15x24", SAGE, BOTTLE, "rusty", "pale", PRIMER,
         mends=[(0, -1, -1.5, 2.25, (0.0, 0.6), "new"), (0, 1, 2.0, 1.5, (0.35, 1.0), "new"), (0, 1, -9.5, 3.0, (0.0, 0.5), "tarred")])
    dock_warehouse("china-warehouse-48x24")
    span_warehouse("china-warehouse-72x33", IVORY, NAVY, "pale", "dull",
                   mends=[(0, -1, -19.0, 3.0, (0.0, 0.7), "tarred"), (0, 1, 8.5, 4.5, (0.3, 1.0), "new"),
                          (1, -1, 26.5, 3.0, (0.0, 0.45), "tarred"), (1, 1, -28.5, 3.75, (0.2, 1.0), "new"),
                          (1, 1, 17.5, 2.25, (0.0, 0.5), "tarred")])
    works("china-works-54x36", brick_m, brick_high_m, render_m, CREAM, "green", BOTTLE, dict(Structure="brick hall, rendered office"),
          ("brick", "render"))
    monitor_depot("china-depot-90x39")


# New York's: galvanised and painted steel, and brick in its reds, browns and buffs.
GALVANISED, NYC_RED, NYC_BROWN, NYC_BUFF = (186, 190, 192), (164, 82, 60), (128, 76, 58), (200, 168, 124)
FIRE_RED, DARK_GREEN, BLACK_GREEN = (150, 40, 32), (44, 74, 56), (34, 44, 40)


def new_york():
    for name in ("loft_window", "barn_window"):  # steel windows of many small panes under a stone lintel
        glazed(name, 2.0, 2.6 if name == "loft_window" else 3.0, stone=True)
    tank_wood = flat_paint("tank_wood", (0.13, 0.1, 0.075), rough=0.9, grime=0.4)
    tank_cone = flat_paint("tank_cone", (0.06, 0.055, 0.05), rough=0.8, grime=0.3)
    water_tank(kit.module("water_tank", **OFFER), "tank", 0.0, 0.0, 0.0, tank_wood, tank_cone, trim_m)
    brick = textured("wall_brick_tinted", "facing_brick_pale", tint=1.0, dirt=0.5, chip=0.12, streak=0.45, rise=0.8)
    shed("nyc-shed-15x24", GALVANISED, FIRE_RED, "new", "pale", PRIMER,
         mends=[(0, -1, 4.5, 3.0, (0.0, 0.7), "new"), (0, 1, -6.0, 2.25, (0.2, 0.8), "tarred")])
    loft("nyc-loft-48x24", NYC_RED, DARK_GREEN, brick)
    barn("nyc-warehouse-72x33", "warehouse", 72.0, 33.0, 4, 6.0, 8.4, NYC_BROWN, BLACK_GREEN, brick, "new", "dull", dict(Doors=4),
         mends=[(1, -1, 4.0, 3.0, (0.0, 0.6), "new"), (3, 1, -9.0, 4.5, (0.3, 1.0), "new")])
    office = textured("office_brick", "brick", colour=(0.3, 0.2, 0.13), dirt=0.5, chip=0.12, streak=0.45, rise=0.8, mottle=0.08)
    works("nyc-works-54x36", brick_m, brick_high_m, office, WHITE, "tarred", DARK_GREEN, dict(Structure="brick hall, brick office"),
          ("brick", "brick"), office_roof=[("water_tank", 13.0, 4.5)], office_steps=[(-7.5, -2.5, 0.6), (-2.5, 2.5, 1.3), (2.5, 7.5, 0.6)])
    barn("nyc-depot-90x39", "depot", 90.0, 39.0, 5, 6.6, 9.6, NYC_BUFF, FIRE_RED, brick, "tarred", "pale", dict(Doors=5),
         mends=[(0, 1, 6.0, 4.5, (0.0, 0.7), "new"), (2, -1, -12.0, 3.0, (0.2, 1.0), "tarred"), (4, 1, 10.0, 3.0, (0.0, 0.5), "new")])


# Paris's: grey and blue steel, fibre cement, render in its creams and ochres, red brick.
STEEL_BLUE, PARIS_WHITE, PARIS_CREAM, PARIS_OCHRE, PARIS_GREY = (112, 132, 150), (226, 228, 222), (232, 218, 190), (214, 182, 134), (196, 196, 190)
VERMILION, PARIS_GREEN, WINDOW_BLUE = (184, 70, 42), (44, 104, 84), (62, 96, 136)


def paris():
    glazed("atelier_window", 2.0, 3.4, stone=True)
    ROOFS["fibre"] = sheet("roof_fibre", (0.25, 0.25, 0.235), rust=0.0, soil=0.9, seed=9.0)  # corrugated fibre cement, lichened
    vault = felt("vault_concrete", 7.0, recipe="concrete", colour=(0.22, 0.218, 0.21))
    STIFF.append(vault)
    plinth = textured("plinth_brick", "brick", colour=(0.17, 0.08, 0.06), dirt=0.8, chip=0.12, rise=0.7, mottle=0.08)
    shed("paris-shed-15x24", STEEL_BLUE, VERMILION, "fibre", "pale", PARIS_GREY,
         mends=[(0, -1, -4.5, 3.0, (0.0, 0.6), "new"), (0, 1, 6.0, 2.25, (0.3, 1.0), "tarred")])
    atelier("paris-atelier-48x24", PARIS_CREAM, WINDOW_BLUE, render_m, plinth, "fibre", brick_high_m)
    span_warehouse("paris-warehouse-72x33", PARIS_WHITE, PARIS_GREEN, "new", "dull",
                   mends=[(0, 1, -12.0, 4.5, (0.0, 0.7), "new"), (1, -1, 20.0, 3.0, (0.3, 1.0), "tarred")])
    works("paris-works-54x36", render_m, brick_high_m, brick_m, PARIS_OCHRE, "fibre", WINDOW_BLUE, dict(Structure="rendered hall, brick office"),
          ("render", "brick"))
    vault_depot("paris-depot-90x39", PARIS_GREY, VERMILION, render_m, vault)


{"china": china, "new_york": new_york, "paris": paris}[FAMILY]()

kit.write(next(iter(script_args()[1:]), None))  # a second argument writes the two files somewhere else
