"""Our own houses: the detached and attached homes of a town, as one kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/homes.py

writes `assets/source/city/homes/kit.glb` and `templates.json` (the format is in
this folder's readme). The houses are plain plaster, brick, tile and concrete,
so they sit beside any region's apartment blocks; they carry the first shipping
family's name until a family has houses of its own.

The physical box is the authority. Every outer wall stands on a face of a part,
and a part's top is its roof's ridge: the box is what stops rounds and sight.
Eaves, gutters, steps and door canopies overhang by at most the set's side fit,
chimneys and capping tiles rise by at most its top fit. Windows and doors sit in
the bays of their edge's 3 m lattice, on the floor datums, where a garrison's
soldiers stand.

Fittings (windows, doors, shopfronts, chimneys, gutters, steps) are modelled
once and placed by rows. Each template's walls and roof are a module of their
own; a terrace is one unit module repeated, each unit in its own tint.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from masonry import *  # noqa: E402,F403

FAMILY = "china"
WALL_M = 2.85  # from the top floor's datum up to the eaves

kit = Kit("homes", "homes.py", fit_side_m=0.5, fit_top_m=0.9)

# ---------------------------------------------------------------- materials
# Walls and joinery are pale and tint-masked: a row's tint is the house's colour.
plaster_m = textured("wall_plaster", "plaster", tint=1.0, dirt=0.55, chip=0.5, streak=0.3, rise=0.8)
roughcast_m = textured("wall_roughcast", "roughcast", tint=1.0, dirt=0.55, chip=0.3, streak=0.4, rise=0.8)
brick_m = textured("wall_brick", "brick", tint=1.0, dirt=0.4, chip=0.12, streak=0.25, rise=0.6)
joinery_m = textured("joinery", "joinery", tint=1.0, dirt=0.0, chip=0.5, streak=0.0)
ROOFS = {
    "clay": textured("roof_clay", "roof_tile", dirt=0.0, chip=0.0, streak=0.0, lichen=0.12),
    "brown": textured("roof_brown", "roof_tile", colour=(0.12, 0.078, 0.058), dirt=0.0, chip=0.0, streak=0.0, lichen=0.12, seed=3.0),
    "slate": textured("roof_slate", "roof_tile", colour=(0.082, 0.086, 0.092), dirt=0.0, chip=0.0, streak=0.0, lichen=0.1, seed=6.0),
}
WALLS = {"plaster": plaster_m, "roughcast": roughcast_m, "brick": brick_m}
plinth_m = textured("plinth_concrete", "concrete", colour=(0.2, 0.2, 0.19), dirt=0.8, chip=0.4, rise=0.5)
stone_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.2)
coping_m = textured("coping_concrete", "concrete", colour=(0.13, 0.13, 0.125), dirt=0.0, chip=0.3, streak=0.2)
stack_m = textured("stack_brick", "brick", dirt=0.0, chip=0.2, streak=0.3, soot=0.6)
render_stack_m = textured("stack_render", "plaster", colour=(0.4, 0.38, 0.35), dirt=0.0, chip=0.4, streak=0.3, soot=0.5)
trim_m = flat_paint("roof_trim", (0.2, 0.18, 0.15), rough=0.8, grime=0.0)
frame_m = flat_paint("window_frame", (0.46, 0.45, 0.42), rough=0.6, grime=0.0)
glass_m = flat_paint("window_glass", (0.02, 0.025, 0.03), rough=0.08, grime=0.0)
metal_m = flat_paint("gutter_metal", (0.07, 0.07, 0.07), rough=0.5, metal=0.6, grime=0.0)
pot_m = flat_paint("chimney_pot", (0.3, 0.13, 0.08), rough=0.8, grime=0.0)
sign_m = flat_paint("shop_sign", (0.45, 0.45, 0.45), rough=0.6, grime=0.0)
sign_m["tint"] = 1.0

# Tints, sRGB. Plasters, bricks (which only darken: the recipe is the lightest brick), paints.
CREAM, OCHRE, CHALK, SALMON, SAGE, ASH, DOVE = ((233, 221, 196), (222, 196, 150), (238, 236, 228), (219, 180, 160),
                                               (198, 206, 186), (202, 201, 196), (192, 202, 209))
RED_BRICK, BROWN_BRICK, PALE_BRICK = (255, 255, 255), (205, 188, 182), (255, 238, 220)
GREEN, BLUE, OXBLOOD, UMBER, IVORY = (66, 96, 76), (70, 92, 122), (122, 58, 48), (74, 60, 48), (226, 222, 208)
SIGNS = ((150, 62, 50), (62, 92, 124), (190, 160, 84), (70, 108, 82))

# ---------------------------------------------------------------- shared modules
# (`FITTING`, the sill and overhang conventions and the rows that hang fittings are `kit.py`'s.)
def window_module(name, w, h, shutters=False, lights=2):
    m = kit.module(name, **FITTING)
    window(m.n("w"), (0, -0.02, h / 2), (0, -1), w, h, frame_m, glass_m, joinery_m if shutters else None, m.root, stone_m,
           bevel=0.0, glass_lods=TIERS, lights=lights)


window_module("window_a", 1.0, 1.3)
window_module("window_a_shutters", 1.0, 1.3, shutters=True)
window_module("window_wide", 1.6, 1.3, lights=3)
window_module("window_tall", 0.9, 1.5, lights=1)

m = kit.module("door_panel", ground=True, **FITTING)
panel_door(m.n("door"), 0.95, 2.05, joinery_m, frame_m, m.root)

m = kit.module("door_canopy", ground=True, **FITTING)
panel_door(m.n("door"), 0.95, 2.05, joinery_m, frame_m, m.root, glass_m, light=0.32)
box(m.n("canopy"), (1.8, 0.45, 0.09), (0, -0.225, 2.66), stone_m, m.root, lods=(0, 1, 2))
for s in (-1, 1):
    box(m.n(f"bracket_{'ab'[s > 0]}"), (0.07, 0.34, 0.07), (s * 0.75, -0.16, 2.48), stone_m, m.root, rot=(0.6, 0, 0), lods=(0,))

m = kit.module("step", ground=True, **FITTING)
box(m.n("slab"), (1.5, 0.4, 0.16), (0, -0.2, 0.08), plinth_m, m.root)

m = kit.module("chimney_brick", **FITTING)
chimney(m.n("c"), (1.05, 0.6), 1.8, stack_m, coping_m, m.root, pots=2, pot_mat=pot_m)
m = kit.module("chimney_render", **FITTING)
chimney(m.n("c"), (0.7, 0.7), 1.8, render_stack_m, coping_m, m.root, pots=1, pot_mat=pot_m)

# A metre of gutter along +X and a metre of downpipe up +Z: rows stretch them to length.
m = kit.module("gutter", **FITTING)
cyl(m.n("run"), 0.06, 1.0, (0, 0, 0), "X", metal_m, m.root, seg=6, caps=False)
m = kit.module("downpipe", **FITTING)
cyl(m.n("pipe"), 0.045, 1.0, (0, 0, 0.5), "Z", metal_m, m.root, seg=6, caps=False)

# A shopfront, a bay each: a display window, a glazed door between sidelights, and a
# metre of sign board that a row stretches across the front and tints.
SHOP_HEAD_M = 2.5
m = kit.module("shop_window", ground=True, **FITTING)
box(m.n("glass"), (2.5, 0.05, SHOP_HEAD_M - 0.55), (0, -0.025, 0.5 + (SHOP_HEAD_M - 0.55) / 2), glass_m, m.root)
box(m.n("riser"), (2.6, 0.1, 0.5), (0, -0.05, 0.25), joinery_m, m.root, lods=(0, 1, 2))
box(m.n("head"), (2.6, 0.1, 0.1), (0, -0.05, SHOP_HEAD_M), frame_m, m.root, lods=(0, 1))
for s in (-1, 0, 1):
    box(m.n(f"post_{s + 1}"), (0.07, 0.09, SHOP_HEAD_M - 0.5), (s * 1.265, -0.045, 0.5 + (SHOP_HEAD_M - 0.5) / 2), frame_m, m.root,
        lods=(0, 1) if s else (0,))

m = kit.module("shop_door", ground=True, **FITTING)
box(m.n("glass"), (1.7, 0.04, SHOP_HEAD_M - 0.1), (0, -0.02, 0.1 + (SHOP_HEAD_M - 0.1) / 2), glass_m, m.root)
box(m.n("kick"), (0.95, 0.06, 0.35), (0, -0.03, 0.175), joinery_m, m.root, lods=(0, 1, 2))
for s in (-1, 1):
    box(m.n(f"riser_{'ab'[s > 0]}"), (0.36, 0.1, 0.5), (s * 0.69, -0.05, 0.25), joinery_m, m.root, lods=(0, 1, 2))
    box(m.n(f"stile_{'ab'[s > 0]}"), (0.07, 0.08, 2.1), (s * 0.475, -0.04, 1.05), frame_m, m.root, lods=(0, 1))
    box(m.n(f"post_{'ab'[s > 0]}"), (0.07, 0.09, SHOP_HEAD_M), (s * 0.865, -0.045, SHOP_HEAD_M / 2), frame_m, m.root, lods=(0, 1))
box(m.n("transom"), (1.0, 0.08, 0.07), (0, -0.04, 2.13), frame_m, m.root, lods=(0, 1))
box(m.n("head"), (1.8, 0.1, 0.1), (0, -0.05, SHOP_HEAD_M), frame_m, m.root, lods=(0, 1))

m = kit.module("shop_sign", **FITTING)
box(m.n("board"), (1.0, 0.1, 0.7), (0, -0.05, 0.35), sign_m, m.root)
box(m.n("cornice"), (1.0, 0.18, 0.08), (0, -0.09, 0.74), stone_m, m.root, lods=(0, 1))


# ---------------------------------------------------------------- shells
def shell(m, tag, shape, wall, roof, **options):
    """Walls, roof and plinth of one box of a house, into module `m`."""
    house_shell(m.n(tag), shape, wall, roof, trim_m, m.root, plinth_mat=plinth_m, **options)


def house(id_, tag, category, w, d, floors, rise, along, hips, wall, roof, tint, lattice):
    """A house that is one box under one roof: its template, shell module and roof shape."""
    hx, hy = w / 2, d / 2
    eave = floors[-1] + WALL_M
    shape = RoofShape(-hx, hx, -hy, hy, eave, eave + rise, along, hips, OVER_M, (VERGE_M, VERGE_M))
    m = kit.module(f"{tag}_shell", ground=True)
    shell(m, "body", shape, WALLS[wall], ROOFS[roof], plinth=(-hx, hx, -hy, hy))
    roof_form = "hip" if min(hips) >= 1 else "gable" if max(hips) == 0 else "half-hip"
    t = kit.template(id_, category, FAMILY, dict(Width=w, Depth=d, Floors=len(floors), Roof=roof_form, RidgeAlong=along,
                                                  Wall=wall, Tiles=roof))
    t.part("body", -hx, hx, -hy, hy, shape.ridge)
    t.floors(*floors)
    for side, bay_at in lattice.items():
        t.lattice(f"body-{side}", bay_at)
    t.place(m.name, tint=tint)
    # gutters run under every level eave: the two long sides, and a full hip's end
    eaves = {"x": ("south", "north"), "y": ("west", "east")}[along]
    ends = {"x": ("west", "east"), "y": ("south", "north")}[along]
    for side in eaves:
        rainwater(t, f"body-{side}", shape, shape.ends[1] - shape.ends[0])
    for k, side in enumerate(ends):
        if hips[k] >= 1:
            rainwater(t, f"body-{side}", shape, 2 * shape.reach, pipes=())
    return t, shape


# ---------------------------------------------------------------- detached homes
# A bungalow, long side to the street: cream plaster, clay tiles, shuttered windows.
t, shape = house("china-home-10x8-1f", "home_10x8_1f", "detached_home", 10, 8, (0.0,), 2.35, "x", (0.0, 0.0), "plaster", "clay",
                 CREAM, dict(south=0.0, north=0.0, east=1.5, west=1.5))
front_door(t, "body-south", 0.0, "door_panel", GREEN)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0,), "window_a_shutters", skip=(0.0,) if side == "south" else (), tint=GREEN)
stack(t, "chimney_brick", -2.6, 0.0, shape.ridge)

# A square two-storey villa under a pyramid of slate: pale roughcast.
t, shape = house("china-home-9x9-2f", "home_9x9_2f", "detached_home", 9, 9, (0.0, 3.0), 2.4, "x", (1.0, 1.0), "roughcast", "slate",
                 CHALK, dict(south=0.0, north=0.0, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_canopy", UMBER)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_a", ground="window_wide", skip=(0.0,) if side == "south" else ())
stack(t, "chimney_render", 1.6, 1.2, shape.ridge - 0.4)

# The big family house: brick, half-hipped, the door between two pairs of windows.
t, shape = house("china-home-12x9-2f", "home_12x9_2f", "detached_home", 12, 9, (0.0, 3.0), 2.6, "x", (0.45, 0.45), "brick", "brown",
                 RED_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_canopy", IVORY)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_tall", tint=IVORY)
stack(t, "chimney_brick", -3.2, 0.0, shape.ridge)
stack(t, "chimney_brick", 3.2, 0.0, shape.ridge)

# A deep cottage, gable to the street: ochre plaster under brown tiles.
t, shape = house("china-home-8x11-1f", "home_8x11_1f", "detached_home", 8, 11, (0.0,), 2.3, "y", (0.0, 0.0), "plaster", "brown",
                 OCHRE, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_panel", OXBLOOD)
glaze(t, "body-south", (0.0,), "window_a")
glaze(t, "body-north", (0.0,), "window_a")
for side in ("east", "west"):
    glaze(t, f"body-{side}", (0.0,), "window_a_shutters", tint=OXBLOOD)
stack(t, "chimney_render", 0.0, 2.5, shape.ridge, yaw=math.pi / 2)

# An L: a two-storey house with a lower wing behind it, sage plaster, clay tiles.
# The wing's box is as tall as the house's (a join needs both tops equal); its ridge sits under it.
eave = 3.0 + WALL_M
main = RoofShape(-5, 5, -5, 3, eave, eave + 2.35, "x", (0.0, 0.0), OVER_M, (VERGE_M, VERGE_M))
wing_walls = RoofShape(-2, 4, 3, 9, eave, eave + 3 * main.tan, "y", (0.0, 0.0), OVER_M, (0.0, VERGE_M))
wing_roof = RoofShape(-2, 4, -0.5, 9, eave, wing_walls.ridge, "y", (0.0, 0.0), OVER_M, (0.0, VERGE_M))
m = kit.module("home_ell_2f_shell", ground=True)
shell(m, "main", main, plaster_m, ROOFS["clay"], plinth=(-5, 5, -5, 3))
shell(m, "wing", wing_walls, plaster_m, None, sides=("a1", "b0", "b1"), plinth=(-2, 4, 3, 9))
# the wing's roof runs on into the house's, as far as its ridge goes before it meets the slope
pitched_roof(m.n("wing_roof"), wing_roof, ROOFS["clay"], trim_m, m.root, fascia_ends=(False, True))
t = kit.template("china-home-ell-2f", "detached_home", FAMILY,
                 dict(Width=10, Depth=8, Floors=2, Roof="gable", RidgeAlong="x", Wall="plaster", Tiles="clay", WingWidth=6, WingDepth=6))
t.part("main", -5, 5, -5, 3, main.ridge)
t.part("wing", -2, 4, 3, 9, main.ridge)
t.floors(0.0, 3.0)
for edge, bay_at in (("main-south", 0.0), ("main-east", 1.5), ("main-west", 1.5), ("main-north-0", 0.5), ("main-north-2", 3.5),
                     ("wing-east", 1.5), ("wing-west", 1.5), ("wing-north", 1.5)):
    t.lattice(edge, bay_at)
t.place(m.name, tint=SAGE)
front_door(t, "main-south", 0.0, "door_canopy", UMBER)
glaze(t, "main-south", (0.0, 3.0), "window_a_shutters", skip=(0.0,), tint=UMBER)
for edge in ("main-east", "main-west", "main-north-2", "wing-east", "wing-west", "wing-north"):
    glaze(t, edge, (0.0, 3.0), "window_a")
rainwater(t, "main-south", main, main.ends[1] - main.ends[0])
rainwater(t, "main-north-2", main, 3.0, pipes=(1,))
rainwater(t, "wing-east", wing_walls, 6 + VERGE_M, pipes=(1,))
rainwater(t, "wing-west", wing_walls, 6 + VERGE_M, pipes=(-1,))
stack(t, "chimney_brick", -3.0, -1.0, main.ridge)
stack(t, "chimney_render", 1.0, 6.5, wing_walls.ridge, yaw=math.pi / 2)

# ---------------------------------------------------------------- attached homes
# A town house standing alone, its narrow gable to the street: dark brick, slate.
t, shape = house("china-townhouse-2f", "townhouse_2f", "attached_home", 7, 10, (0.0, 3.0), 2.45, "y", (0.0, 0.0), "brick", "slate",
                 BROWN_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", -1.5, "door_canopy", BLUE)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_tall", skip=(-1.5,) if side == "south" else (), tint=BLUE)
stack(t, "chimney_brick", 0.0, 1.5, shape.ridge, yaw=math.pi / 2)


def terrace(id_, tag, units, unit_w, d, floors, rise, wall, roof, tints, window, door, joinery, side_bay, shop=False,
            parapets=False):
    """A row of `units` houses under one roof, ridge along the street. Each unit is one
    module in its own tint; the end walls are a module turned to face each way.
    `side_bay` is a bay's offset on the two end walls."""
    eave = floors[-1] + WALL_M
    ridge, hy, length = eave + rise, d / 2, units * unit_w
    verge = 0.0 if parapets else VERGE_M
    unit = RoofShape(-unit_w / 2, unit_w / 2, -hy, hy, eave, ridge, "x", (0.0, 0.0), OVER_M, (0.0, 0.0))
    um = kit.module(f"{tag}_unit", ground=True)
    shell(um, "unit", unit, WALLS[wall], ROOFS[roof], sides=("b0", "b1"), fascia_ends=(False, False),
          plinth=(-unit_w / 2, unit_w / 2, -hy, hy))
    em = kit.module(f"{tag}_end", ground=True)
    shell(em, "end", RoofShape(0, 0, -hy, hy, eave, ridge, "x", (0.0, 0.0), OVER_M, (0.0, verge)), WALLS[wall], ROOFS[roof],
          sides=("a1",), fascia_ends=(False, True))
    if parapets:  # a party wall carried up through the roof between neighbours
        pm = kit.module(f"{tag}_parapet")
        lo, hi = unit.edge_z, ridge
        prism(pm.n("wall"), [(-unit.reach, lo + 0.16), (0, hi + 0.16), (unit.reach, lo + 0.16), (unit.reach, lo - 0.12),
                             (0, hi - 0.12), (-unit.reach, lo - 0.12)], 0.22, (0, 0, 0), coping_m, pm.root, rot=(0, 0, math.pi / 2))
    t = kit.template(id_, "attached_home", FAMILY, dict(Units=units, UnitWidth=unit_w, Depth=d, Floors=len(floors), Roof="gable",
                                                        RidgeAlong="x", Wall=wall, Tiles=roof, Shopfronts=shop, Parapets=parapets))
    centres = [(k - (units - 1) / 2) * unit_w for k in range(units)]
    for k, cx in enumerate(centres):
        t.part(f"unit-{k}", cx - unit_w / 2, cx + unit_w / 2, -hy, hy, ridge)
    t.floors(*floors)
    t.lattice("unit-0-west", side_bay)
    t.lattice(f"unit-{units - 1}-east", side_bay)
    for k, cx in enumerate(centres):
        tint, paint = tints[k % len(tints)], joinery[k % len(joinery)]
        south, north = f"unit-{k}-south", f"unit-{k}-north"
        t.lattice(south, 1.5)
        t.lattice(north, 1.5)
        t.place(um.name, x=cx, tint=tint)
        if shop:
            t.entrance(south, -1.5)
            t.mount("shop_door", south, -1.5, tiers=TIERS_0_TO_2, tint=paint)
            t.mount("shop_window", south, 1.5, tiers=TIERS_0_TO_2, tint=paint)
            t.mount("shop_sign", south, 0.0, z=SHOP_HEAD_M + 0.15, scale=(unit_w - 0.5, 1.0, 1.0), tiers=TIERS_0_TO_2,
                    tint=SIGNS[k % len(SIGNS)])
            glaze(t, south, floors[1:], window, tint=paint)
        else:
            front_door(t, south, -1.5, door, paint)
            glaze(t, south, floors, window, skip=(-1.5,), tint=paint)
        glaze(t, north, floors, window, tint=paint)
        rainwater(t, south, unit, unit_w, pipes=(1,) if k < units - 1 else ())
        rainwater(t, north, unit, unit_w, pipes=(-1,) if k < units - 1 else ())
        if k:
            stack(t, "chimney_brick" if wall == "brick" else "chimney_render", cx - unit_w / 2, 0.0, ridge, yaw=math.pi / 2)
    for k, (edge, x, yaw) in enumerate((("unit-0-west", -length / 2, math.pi), (f"unit-{units - 1}-east", length / 2, 0.0))):
        t.place(em.name, x=x, yaw=yaw, tint=tints[(0, units - 1)[k] % len(tints)])
        glaze(t, edge, floors, window)
    if parapets:
        for k in range(units + 1):
            x = -length / 2 + k * unit_w
            t.place(pm.name, x=min(max(x, -length / 2 + 0.11), length / 2 - 0.11), tiers=TIERS_0_TO_2)
    return t


# Three plastered cottages in a row, each its own colour.
terrace("china-terrace-3x2f", "terrace_3x2f", 3, 6, 10, (0.0, 3.0), 2.6, "plaster", "clay", (SALMON, CREAM, DOVE), "window_a",
        "door_panel", (UMBER, GREEN, OXBLOOD), side_bay=0.0)
# Five brick houses, three storeys, party walls through the roof.
terrace("china-terrace-5x3f", "terrace_5x3f", 5, 6, 11, (0.0, 3.0, 6.0), 2.9, "brick", "slate",
        (RED_BRICK, BROWN_BRICK, PALE_BRICK, RED_BRICK, BROWN_BRICK), "window_tall", "door_canopy", (IVORY, BLUE, GREEN, OXBLOOD, IVORY),
        side_bay=0.0, parapets=True)
# Four shops with two floors of flats over them.
terrace("china-shops-4x3f", "shops_4x3f", 4, 7.5, 13, (0.0, 4.0, 7.0), 3.0, "roughcast", "brown", (ASH, CREAM, CHALK, OCHRE), "window_a",
        None, (UMBER, BLUE, OXBLOOD, GREEN), side_bay=1.5, shop=True, parapets=True)

# The corner shop: shopfronts on the street and round the corner, flats above, a hipped roof.
t, shape = house("china-corner-shop-3f", "corner_shop_3f", "attached_home", 12, 12, (0.0, 4.0, 7.0), 2.8, "x", (1.0, 1.0), "plaster",
                 "clay", OCHRE, dict(south=1.5, north=1.5, east=1.5, west=1.5))
t.entrance("body-south", -1.5)
t.mount("shop_door", "body-south", -1.5, tiers=TIERS_0_TO_2, tint=GREEN)
for o in (-4.5, 1.5, 4.5):
    t.mount("shop_window", "body-south", o, tiers=TIERS_0_TO_2, tint=GREEN)
for o in (-4.5, -1.5):
    t.mount("shop_window", "body-east", o, tiers=TIERS_0_TO_2, tint=GREEN)
t.mount("shop_sign", "body-south", 0.0, z=SHOP_HEAD_M + 0.15, scale=(11.5, 1.0, 1.0), tiers=TIERS_0_TO_2, tint=SIGNS[3])
t.mount("shop_sign", "body-east", -3.0, z=SHOP_HEAD_M + 0.15, scale=(5.5, 1.0, 1.0), tiers=TIERS_0_TO_2, tint=SIGNS[3])
glaze(t, "body-south", (4.0, 7.0), "window_wide")
glaze(t, "body-east", (0.0, 4.0, 7.0), "window_wide", skip=(-4.5, -1.5))
for side in ("north", "west"):
    glaze(t, f"body-{side}", (0.0, 4.0, 7.0), "window_a")
stack(t, "chimney_render", -2.0, 2.0, shape.ridge - 0.6)

kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
