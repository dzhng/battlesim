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
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from masonry import *  # noqa: E402,F403

FAMILY = "china"
WALL_M = 2.85  # from the top floor's datum up to the eaves

kit = Kit("homes", "homes.py", fit_side_m=0.5, fit_top_m=0.9, fit_ruin_top_m=0.6)

# ---------------------------------------------------------------- materials
# Walls and joinery are pale and tint-masked: a row's tint is the house's colour.
plaster_m = textured("wall_plaster", "plaster", tint=1.0, dirt=0.55, chip=0.5, streak=0.3, rise=0.8)
roughcast_m = textured("wall_roughcast", "roughcast", tint=1.0, dirt=0.55, chip=0.3, streak=0.4, rise=0.8)
brick_m = textured("wall_brick", "brick", tint=1.0, dirt=0.4, chip=0.12, streak=0.25, rise=0.6)
joinery_m = textured("joinery", "joinery", tint=1.0, dirt=0.0, chip=0.5, streak=0.0)
# A roof's recipe carries what is smaller than a tile; its stains are its own, in soft patches (`weathered_roof`).
ROOFING = dict(dirt=0.0, chip=0.0, streak=0.0, grain=0.0)
ROOFS = {
    "clay": weathered_roof(textured("roof_clay", "roof_tile", **ROOFING), seed=1.0),
    "brown": weathered_roof(textured("roof_brown", "roof_tile", colour=(0.12, 0.078, 0.058), seed=3.0, **ROOFING), seed=3.0),
    "slate": weathered_roof(textured("roof_slate", "roof_slate", colour=(0.082, 0.086, 0.092), seed=6.0, **ROOFING), seed=6.0, moss=0.12),
}
WALLS = {"plaster": plaster_m, "roughcast": roughcast_m, "brick": brick_m}
plinth_m = textured("plinth_concrete", "concrete", colour=(0.2, 0.2, 0.19), dirt=0.8, chip=0.4, rise=0.5)
stone_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.2)
coping_m = textured("coping_concrete", "concrete", colour=(0.13, 0.13, 0.125), dirt=0.0, chip=0.3, streak=0.2)
stack_m = textured("stack_brick", "brick", dirt=0.0, chip=0.2, streak=0.3, soot=0.6)
render_stack_m = textured("stack_render", "plaster", colour=(0.4, 0.38, 0.35), dirt=0.0, chip=0.4, streak=0.3, soot=0.5)
trim_m = flat_paint("roof_trim", (0.2, 0.18, 0.15), rough=0.8, grime=0.0)
frame_m = flat_paint("window_frame", (0.46, 0.45, 0.42), rough=0.6, grime=0.0)
glass_m, pane_m = window_glass(), window_pane()  # a window near, and the dark pane it is from far off
metal_m = flat_paint("gutter_metal", (0.07, 0.07, 0.07), rough=0.5, metal=0.6, grime=0.0)
pot_m = flat_paint("chimney_pot", (0.3, 0.13, 0.08), rough=0.8, grime=0.0)
sign_m = flat_paint("shop_sign", (0.45, 0.45, 0.45), rough=0.6, grime=0.0)
sign_m["tint"] = 1.0
# What a fallen house is made of: its rubble takes a row's tint (the plaster's colour, or
# a brick's) on the shared heaps, and inside a brick house's stumps it is brick already.
rubble_m = textured("rubble_masonry", "rubble", tint=1.0, dirt=0.0, chip=0.0, streak=0.0, ash=0.25)
brick_rubble_m = textured("rubble_brick", "rubble", colour=(0.13, 0.068, 0.05), dirt=0.0, chip=0.0, streak=0.0, ash=0.25, seed=2.0)
char_m = charred("charred_timber", ember=0.04)
RUBBLE = {"plaster": rubble_m, "roughcast": rubble_m, "brick": brick_rubble_m}
BRICK_DUST = (176, 104, 82)  # the shared heaps' tint about a brick house
# What breaks a wall on the ground floor, for a ruin's stumps: fitting -> its opening's width.
OPENINGS = {"window_a": 1.1, "window_a_shutters": 1.1, "window_wide": 1.7, "window_tall": 1.0, "door_panel": 1.15, "door_canopy": 1.15,
            "shop_window": 2.6, "shop_door": 1.8}
STACKS = {"chimney_brick": ((1.05, 0.6), stack_m), "chimney_render": ((0.7, 0.7), render_stack_m)}

# Tints, sRGB. Plasters, bricks (which only darken: the recipe is the lightest brick), paints.
CREAM, OCHRE, CHALK, SALMON, SAGE, ASH, DOVE = ((233, 221, 196), (222, 196, 150), (238, 236, 228), (219, 180, 160),
                                               (198, 206, 186), (202, 201, 196), (192, 202, 209))
RED_BRICK, BROWN_BRICK, PALE_BRICK = (255, 255, 255), (205, 188, 182), (255, 238, 220)
GREEN, BLUE, OXBLOOD, UMBER, IVORY = (66, 96, 76), (70, 92, 122), (122, 58, 48), (74, 60, 48), (226, 222, 208)
SIGNS = ((150, 62, 50), (62, 92, 124), (190, 160, 84), (70, 108, 82))

# ---------------------------------------------------------------- shared modules
# (`FITTING`, the sill and overhang conventions and the rows that hang fittings are `kit.py`'s.)
def window_module(name, w, h, shutters=False, lights=2):
    casement(kit, name, w, h, frame_m, glass_m, pane_m, stone_m, joinery_m if shutters else None, lights)


window_module("window_a", 1.0, 1.3)
window_module("window_a_shutters", 1.0, 1.3, shutters=True)
window_module("window_wide", 1.6, 1.3, lights=3)
window_module("window_tall", 0.9, 1.5, lights=1)

m = kit.module("door_panel", ground=True, **FITTING)
m.opening = (0.95, 2.05, 0.0)
panel_door(m.n("door"), 0.95, 2.05, joinery_m, frame_m, m.root)

m = kit.module("door_canopy", ground=True, **FITTING)
m.opening = (0.95, 2.05, 0.0)
panel_door(m.n("door"), 0.95, 2.05, joinery_m, frame_m, m.root, pane_m, light=0.32)
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
# metre of sign board that a row stretches across the front and tints. Each is an opening
# with the shop behind its glass.
SHOP_HEAD_M = 2.5
SHOP_WINDOW, SHOP_DOOR = (2.5, SHOP_HEAD_M - 0.55, 0.5), (1.7, SHOP_HEAD_M - 0.47, 0.42)  # each opening: its width, height and foot


def shop_glass(m, w, h, foot):
    reveal(m, w, h, window_reveal(), foot)
    sheet(m.n("glass"), w, h, (0, 0.01, foot), glass_m, m.root, lods=OPEN_TIERS)  # in its frame, on the wall's face: a shopfront has no reveal outside
    sheet(m.n("pane"), w, h, (0, -0.025, foot), pane_m, m.root, lods=(2, 3))
    opening(kit, m.name, w, h, foot, "shops")


m = kit.module("shop_window", ground=True, **FITTING)
shop_glass(m, *SHOP_WINDOW)
# What stands in the window: a stall board inside the glass with a few things on it. From the street a shop's
# room is mostly its bare side wall; the display is what says the shop is in use. Near only.
GOODS = [flat_paint(f"shop_goods_{k}", colour, rough=0.8, grime=0.0)
         for k, colour in enumerate(((0.15, 0.06, 0.045), (0.06, 0.09, 0.12), (0.17, 0.14, 0.08), (0.2, 0.195, 0.18)))]  # dull: tins, cartons, sacks
box(m.n("stall"), (2.3, 0.55, 0.5), (0, 0.5, 0.33), joinery_m, m.root, lods=(0,))
for k_, (x_, w_, d_, h_) in enumerate(((-0.85, 0.4, 0.3, 0.35), (-0.3, 0.3, 0.3, 0.55), (0.2, 0.45, 0.35, 0.25), (0.8, 0.35, 0.3, 0.45),
                                       (0.45, 0.2, 0.2, 0.6))):
    box(m.n(f"goods_{k_}"), (w_, d_, h_), (x_, 0.5 + 0.08 * (k_ % 2), 0.58 + h_ / 2), GOODS[k_ % len(GOODS)], m.root, lods=(0,))
box(m.n("riser"), (2.6, 0.1, 0.5), (0, -0.05, 0.25), joinery_m, m.root, lods=(0, 1, 2))
box(m.n("head"), (2.6, 0.1, 0.1), (0, -0.05, SHOP_HEAD_M), frame_m, m.root, lods=(0, 1))
for s in (-1, 0, 1):
    box(m.n(f"post_{s + 1}"), (0.07, 0.09, SHOP_HEAD_M - 0.5), (s * 1.265, -0.045, 0.5 + (SHOP_HEAD_M - 0.5) / 2), frame_m, m.root,
        lods=(0, 1) if s else (0,))

m = kit.module("shop_door", ground=True, **FITTING)
shop_glass(m, *SHOP_DOOR)
box(m.n("kick"), (0.95, 0.06, 0.5), (0, -0.03, 0.25), joinery_m, m.root, lods=(0, 1, 2))
for s in (-1, 1):
    box(m.n(f"riser_{'ab'[s > 0]}"), (0.36, 0.1, 0.5), (s * 0.69, -0.05, 0.25), joinery_m, m.root, lods=(0, 1, 2))
    box(m.n(f"stile_{'ab'[s > 0]}"), (0.07, 0.08, 2.1), (s * 0.475, -0.04, 1.05), frame_m, m.root, lods=(0, 1))
    box(m.n(f"post_{'ab'[s > 0]}"), (0.07, 0.09, SHOP_HEAD_M), (s * 0.865, -0.045, SHOP_HEAD_M / 2), frame_m, m.root, lods=(0, 1))
box(m.n("transom"), (1.0, 0.08, 0.07), (0, -0.04, 2.13), frame_m, m.root, lods=(0, 1))
box(m.n("head"), (1.8, 0.1, 0.1), (0, -0.05, SHOP_HEAD_M), frame_m, m.root, lods=(0, 1))

# A dormer, one to each roof covering: a small gabled window standing on a slope, its foot's
# middle at the origin and its back run into the roof behind it. It faces -Y; a row tints its cheeks
# the house's colour. One, off to a side, is what tells a house from its mirror image from above.
DORMER_WINDOW = (0.8, 0.7, 0.25)  # its opening: width, height and foot


def dormer_module(tiles):
    m = kit.module(f"dormer_{tiles}", **FITTING)
    # near, its face is open round the window, with glass in the opening over the dark of the attic
    x, d, top, (w, h, foot) = 0.65, 2.3, 1.05, DORMER_WINDOW
    face = [(-x, -w / 2, 0, top), (w / 2, x, 0, top), (-w / 2, w / 2, 0, foot), (-w / 2, w / 2, foot + h, top)]
    quads = [([(a, 0, c), (b, 0, c), (b, 0, e), (a, 0, e)], (0, -1, 0)) for a, b, c, e in face]
    quads += [([(s * x, 0, 0), (s * x, d, 0), (s * x, d, top), (s * x, 0, top)], (s, 0, 0)) for s in (-1, 1)]
    flat_faces(m.n("cheeks"), quads, plaster_m, m.root, OPEN_TIERS)
    box(m.n("cheeks_far"), (2 * x, d, top), (0, d / 2, top / 2), plaster_m, m.root, lods=(2,))
    reveal(m, w, h, window_reveal(), foot, depth=0.1)
    sheet(m.n("glass"), w, h, (0, 0.06, foot), glass_m, m.root, lods=OPEN_TIERS)
    reveal(m, w, h, bpy_dark(), foot, depth=0.9, start=0.1, back=True, tag="recess")  # (too small for a room box: a squeezed room is a pale panel)
    box(m.n("pane"), (0.8, 0.04, 0.7), (0, -0.02, 0.6), pane_m, m.root, lods=(2,))
    box(m.n("frame"), (0.96, 0.05, 0.08), (0, -0.025, 0.2), frame_m, m.root, lods=(0,))
    prism(m.n("roof"), [(-0.85, 0.98), (0.85, 0.98), (0.0, 1.45)], 2.5, (0, 1.1, 0), ROOFS[tiles], m.root)


for tiles_ in ("brown", "clay", "slate"):
    dormer_module(tiles_)


def dormer(t, shape, tiles, side, along, inset, tint):
    """A dormer on the `side` slope of a house's roof, `along` the eave from the house's middle, its
    face `inset` metres up the slope from the wall."""
    nx, ny = SIDES[side][1]
    reach = (shape.half if (side in ("south", "north")) != shape.swap else (shape.a1 - shape.a0) / 2) - inset
    cx, cy = (shape.b0 + shape.b1) / 2 if shape.swap else (shape.a0 + shape.a1) / 2, shape.bc if not shape.swap else (shape.a0 + shape.a1) / 2
    x, y = (cx + along, cy + ny * reach) if ny else (cx + nx * reach, cy + along)
    t.place(f"dormer_{tiles}", x, y, shape.eave + inset * shape.tan, math.atan2(ny, nx) + math.pi / 2, tiers=TIERS_0_TO_2, tint=tint)


m = kit.module("shop_sign", **FITTING)
box(m.n("board"), (1.0, 0.1, 0.7), (0, -0.05, 0.35), sign_m, m.root)
box(m.n("cornice"), (1.0, 0.18, 0.08), (0, -0.09, 0.74), stone_m, m.root, lods=(0, 1))


# ---------------------------------------------------------------- ruins
def seed_of(id_):
    """A template's own number: every ruin breaks its own way, the same way every run."""
    return zlib.crc32(id_.encode()) % 997


def stacks_of(t, rect):
    """The chimneys standing inside `rect`, from the intact rows: (x, y, plan, material), turned as placed."""
    out = []
    for module, x, y, z, yaw, *_ in t.rows["intact"]:
        if module in STACKS and rect[0] < x < rect[1] and rect[2] < y < rect[3]:
            (w, d), mat = STACKS[module]
            out.append((x, y, (d, w) if abs(math.sin(yaw)) > 0.7 else (w, d), mat))
    return out


def fallen(t, tag, wall, roof, tint):
    """What a house is from far off, and what it is fallen. Far off, its shell keeps its
    fittings (`far_fittings`). Fallen, it is one module holding every part's stumps and
    heap, placed in the house's own tint, with the set's wreckage lying on it."""
    far_fittings(t, kit.modules[f"{tag}_shell"])
    open_walls(kit.modules[f"{tag}_shell"], t.rows["intact"], kit.openings)
    furnish(t, kit)
    m = kit.module(f"{tag}_ruin", ground=True, **RUIN)
    high, over = t.ruin_height(), kit.fit["ruin_top_m"]
    for k, p in enumerate(t.parts):
        rect = (p["x0"], p["x1"], p["y0"], p["y1"])
        ruin = ruin_block(m, p["id"].replace("-", "_"), rect, high, over, ruin_sides(t, p["id"], OPENINGS), WALLS[wall], ROOFS[roof],
                          RUBBLE[wall], seed_of(t.id) + 17 * k, stacks=stacks_of(t, rect), coarse=12)
        litter(t, ruin, BRICK_DUST if wall == "brick" else tint, heaps=4)
    t.place(m.name, tint=tint, state="ruin")


wreckage(kit, rubble_m, char_m)

# What the two coarse tiers keep of a fitting, folded into its house's shell (`fold_far`): each window
# its pane in its pale surround between its shutters, each door its own paint, each chimney a block
# the colour of its cap. A fitting named here is a row at the two fine tiers only.
FAR_PANELS = {
    "window_a": window_far(1.0, 1.3, pane_m, stone_m), "window_a_shutters": window_far(1.0, 1.3, pane_m, stone_m, "tint"),
    "window_wide": window_far(1.6, 1.3, pane_m, stone_m), "window_tall": window_far(0.9, 1.5, pane_m, stone_m),
    "door_panel": [(1.15, 2.15, 0.0, frame_m, 0.012), (0.95, 2.05, 0.0, "tint")],
    "door_canopy": [(1.15, 2.55, 0.0, frame_m, 0.012), (0.95, 2.05, 0.0, "tint"), (0.95, 0.32, 2.11, pane_m)],
    "shop_window": [(2.6, 0.5, 0.0, "tint", 0.012), (2.5, SHOP_HEAD_M - 0.5, 0.5, pane_m)],
    "shop_door": [(1.7, SHOP_HEAD_M - 0.1, 0.1, pane_m), (0.95, 0.5, 0.0, "tint", 0.04)],
}
FAR_BOXES = {"chimney_brick": [((1.05, 0.6, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), coping_m)],
             "chimney_render": [((0.7, 0.7, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), coping_m)]}
FAR_BOXES |= {f"dormer_{tiles}": [((1.3, 2.3, 1.05), (0, 1.15, 0.525), plaster_m), ((1.7, 2.5, 0.44), (0, 1.1, 1.2), ROOFS[tiles])]
              for tiles in ("brown", "clay", "slate")}


def far_paint(tint):
    """A door's or a sign's paint as the coarsest tier keeps it: the joinery's own colour under a row's tint."""
    colour = tuple(m * (c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4)
                   for m, c in zip(textures.baked("joinery").mean(), (v / 255 for v in tint)))
    return flat_paint("far_paint_%d_%d_%d" % tint, colour, rough=0.7, grime=0.0)


def far_fittings(t, m, origin=(0.0, 0.0), within=None):
    """Fold what the two coarse tiers keep of a template's fittings into shell `m`: all of its rows, or
    those `within` (x0, x1) of it, for a module that is one house of a row. The folded fittings' own
    rows then stop at the second tier. A row of houses folds no chimney: a chimney stands on the wall
    between two houses, and every house is the one module, so its chimneys stay rows at every tier."""
    rows = [r for r in t.rows["intact"] if within is None or within[0] <= r[1] < within[1]]
    boxes = FAR_BOXES if within is None else {}
    fold_far(m, rows, FAR_PANELS, boxes, far_paint, (2, 3), origin)
    if within is not None:  # a house of a row: near, its walls are open where the same rows stand
        open_walls(m, rows, kit.openings, origin)
    t.rows["intact"] = [(*r[:8], r[8] & TIERS_0_TO_1, *r[9:]) if r[0] in FAR_PANELS or r[0] in boxes
                        else (*r[:8], EVERY_TIER, *r[9:]) if r[0] in FAR_BOXES else r for r in t.rows["intact"]]


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
fallen(t, "home_10x8_1f", "plaster", "clay", CREAM)

# A square two-storey villa under a pyramid of slate: pale roughcast.
t, shape = house("china-home-9x9-2f", "home_9x9_2f", "detached_home", 9, 9, (0.0, 3.0), 2.4, "x", (1.0, 1.0), "roughcast", "slate",
                 CHALK, dict(south=0.0, north=0.0, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_canopy", UMBER)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_a", ground="window_wide", skip=(0.0,) if side == "south" else ())
stack(t, "chimney_render", 1.6, 1.2, shape.ridge - 0.4)
dormer(t, shape, "slate", "north", -1.5, 1.3, CHALK)
fallen(t, "home_9x9_2f", "roughcast", "slate", CHALK)

# The big family house: brick, half-hipped, the door between two pairs of windows.
t, shape = house("china-home-12x9-2f", "home_12x9_2f", "detached_home", 12, 9, (0.0, 3.0), 2.6, "x", (0.45, 0.45), "brick", "brown",
                 RED_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_canopy", IVORY)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_tall", tint=IVORY)
stack(t, "chimney_brick", -3.2, 0.0, shape.ridge)
stack(t, "chimney_brick", 3.2, 0.0, shape.ridge)
dormer(t, shape, "brown", "north", 2.4, 1.2, IVORY)
fallen(t, "home_12x9_2f", "brick", "brown", RED_BRICK)

# A deep cottage, gable to the street: ochre plaster under brown tiles.
t, shape = house("china-home-8x11-1f", "home_8x11_1f", "detached_home", 8, 11, (0.0,), 2.3, "y", (0.0, 0.0), "plaster", "brown",
                 OCHRE, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", 0.0, "door_panel", OXBLOOD)
glaze(t, "body-south", (0.0,), "window_a")
glaze(t, "body-north", (0.0,), "window_a")
for side in ("east", "west"):
    glaze(t, f"body-{side}", (0.0,), "window_a_shutters", tint=OXBLOOD)
stack(t, "chimney_render", 0.0, 2.5, shape.ridge, yaw=math.pi / 2)
fallen(t, "home_8x11_1f", "plaster", "brown", OCHRE)

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
fallen(t, "home_ell_2f", "plaster", "clay", SAGE)

# ---------------------------------------------------------------- attached homes
# A town house standing alone, its narrow gable to the street: dark brick, slate.
t, shape = house("china-townhouse-2f", "townhouse_2f", "attached_home", 7, 10, (0.0, 3.0), 2.45, "y", (0.0, 0.0), "brick", "slate",
                 BROWN_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0))
front_door(t, "body-south", -1.5, "door_canopy", BLUE)
for side in ("south", "north", "east", "west"):
    glaze(t, f"body-{side}", (0.0, 3.0), "window_tall", skip=(-1.5,) if side == "south" else (), tint=BLUE)
stack(t, "chimney_brick", 0.0, 1.5, shape.ridge, yaw=math.pi / 2)
fallen(t, "townhouse_2f", "brick", "slate", BROWN_BRICK)


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
            t.mount("shop_sign", south, 0.0, z=SHOP_HEAD_M + 0.15, scale=(unit_w - 0.5, 1.0, 1.0), tint=SIGNS[k % len(SIGNS)])
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
    # From far off each house's module keeps the fittings of the row's second house, and the end wall its windows.
    far_fittings(t, um, origin=(centres[1], 0.0), within=(centres[1] - unit_w / 2 - 0.01, centres[1] + unit_w / 2 - 0.01))
    far_fittings(t, em, origin=(length / 2, 0.0), within=(length / 2 - 0.01, length / 2 + 0.01))
    furnish(t, kit)
    # Fallen, the row is not one flat line: each house is one of two ruins, in its own tint
    # and down to its own level, its party wall and chimney breast standing on its west
    # side; the row's east end wall is a module of its own.
    high, over, seed = t.ruin_height(), kit.fit["ruin_top_m"], seed_of(id_)
    sides = {side: dict(openings=[(at - centres[1], w, sill) for at, w, sill in wall["openings"]], gaps=[])
             for side, wall in ruin_sides(t, "unit-1", OPENINGS).items()} | {"west": dict(openings=[], gaps=[])}
    breast = STACKS["chimney_brick" if wall == "brick" else "chimney_render"]
    ruins = []
    for v in "ab":
        m = kit.module(f"{tag}_ruin_{v}", ground=True, **RUIN)
        ruins.append((m, ruin_block(m, "unit", (-unit_w / 2, unit_w / 2, -hy, hy), high, over, sides, WALLS[wall], ROOFS[roof],
                                    RUBBLE[wall], seed + 7 * (v == "b"), stacks=[(-unit_w / 2 + 0.4, 0.0, breast[0][::-1], breast[1])],
                                    far=("south", "north"), coarse=10)))
    for k, cx in enumerate(centres):
        m, ruin = ruins[k % 2]
        level = (1.0, 0.8, 0.92, 0.74, 0.96)[k % 5]
        t.place(m.name, x=cx, scale=(1.0, 1.0, level), tint=tints[k % len(tints)], state="ruin")
        litter(t, ruin, BRICK_DUST if wall == "brick" else tints[k % len(tints)], at=(cx, 0.0), beams=2, heaps=3, seed=k, level=level)
    em = kit.module(f"{tag}_ruin_end", ground=True, **RUIN)
    burnt = scorched(WALLS[wall], high + over, seed)
    ragged_wall(em.n("wall"), -hy, hy, -RUIN_WALL_M / 2, False, 0.3 * high, high + over - 0.06, seed + 5, burnt, em.root, RUIN_WALL_M,
                lods=(0, 1, 2), groups=(1, 3, 6, 8), jagged=1.5)
    wall_panels(em.n("far"), [(0.0, 0.0, 0.0, math.pi / 2, d, 0.8 * high)], burnt, em.root, proud=0.0, lods=(3,))
    t.place(em.name, x=length / 2, tint=tints[(units - 1) % len(tints)], state="ruin")
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
t.mount("shop_sign", "body-south", 0.0, z=SHOP_HEAD_M + 0.15, scale=(11.5, 1.0, 1.0), tint=SIGNS[3])
t.mount("shop_sign", "body-east", -3.0, z=SHOP_HEAD_M + 0.15, scale=(5.5, 1.0, 1.0), tint=SIGNS[3])
glaze(t, "body-south", (4.0, 7.0), "window_wide")
glaze(t, "body-east", (0.0, 4.0, 7.0), "window_wide", skip=(-4.5, -1.5))
for side in ("north", "west"):
    glaze(t, f"body-{side}", (0.0, 4.0, 7.0), "window_a")
stack(t, "chimney_render", -2.0, 2.0, shape.ridge - 0.6)
dormer(t, shape, "clay", "west", -2.0, 1.4, OCHRE)
fallen(t, "corner_shop_3f", "plaster", "clay", OCHRE)

kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
