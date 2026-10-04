"""Our own houses: the detached and attached homes of a town, one regional family to a kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/homes.py [family] [out]

writes one family's set (the format is in this folder's readme): `china`, the default, to
`assets/source/city/homes/`, and `new_york` or `paris` to `assets/source/city/homes_<family>/`;
`out` writes the two files somewhere else. A family is a design over one builder: its
materials, fittings and plans are its own (`china`, `new_york` and `paris`, at the foot of
this file), and the shells, rows, far tiers and ruins are this file's, the same for all three.

- China's houses are plain plaster, brick, tile and concrete.
- New York's are American frame houses in clapboard under shingle, with porches, shutters and
  sash windows, and rows of brick and brownstone under flat roofs, with cornices and stoops.
- Paris's are pavillons in meulière and render under tile and slate with shuttered French
  windows, and maisons de ville and stone rows under mansards.

Each family covers the same plans at the same sizes and floors, so the map generator fills
the same lots with any of them.

The physical box is the authority. Every outer wall stands on a face of a part,
and a part's top is its roof's ridge: the box is what stops rounds and sight.
Eaves, gutters, steps, stoops, cornices and door canopies overhang by at most the
set's side fit, chimneys and capping tiles rise by at most its top fit. A porch is
inside its house's box, under the house's own roof or upper floor. Windows and
doors sit in the bays of their edge's 3 m lattice, on the floor datums, where a
garrison's soldiers stand.

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
from regional import *  # noqa: E402,F403

FAMILIES = ("china", "new_york", "paris")
ARGS = script_args()
FAMILY = ARGS.pop(0) if ARGS and ARGS[0] in FAMILIES else "china"
WALL_M = 2.85  # from the top floor's datum up to the eaves

kit = Kit("homes" if FAMILY == "china" else f"homes_{FAMILY}", "homes.py" if FAMILY == "china" else f"homes.py {FAMILY}",
          fit_side_m=0.5, fit_top_m=0.9, fit_ruin_top_m=0.6)

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
DUST = {"brick": BRICK_DUST}  # the shared heaps' tint about a house of each wall; the house's own tint otherwise
RUIN_STYLE = {}  # how a house of each wall falls (`ruin_block`'s options), when not as masonry does
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
# A family builds the fittings its design places (every module of a set is placed), in its own order.
def window_module(name, w, h, shutters=False, lights=2):
    casement(kit, name, w, h, frame_m, glass_m, pane_m, stone_m, joinery_m if shutters else None, lights)


def door_modules():
    door_module(kit, "door_panel", 0.95, 2.05, joinery_m, frame_m)
    m = door_module(kit, "door_canopy", 0.95, 2.05, joinery_m, frame_m, pane_m, light=0.32)
    box(m.n("canopy"), (1.8, 0.45, 0.09), (0, -0.225, 2.66), stone_m, m.root, lods=(0, 1, 2))
    for s in (-1, 1):
        box(m.n(f"bracket_{'ab'[s > 0]}"), (0.07, 0.34, 0.07), (s * 0.75, -0.16, 2.48), stone_m, m.root, rot=(0.6, 0, 0), lods=(0,))


def step_module():
    m = kit.module("step", ground=True, **FITTING)
    box(m.n("slab"), (1.5, 0.4, 0.16), (0, -0.2, 0.08), plinth_m, m.root)


def chimney_module(name, plan=(1.05, 0.6), mat=stack_m, pots=2):
    m = kit.module(name, **FITTING)
    chimney(m.n("c"), plan, 1.8, mat, coping_m, m.root, pots=pots, pot_mat=pot_m)


def rainwater_modules():
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


def shop_modules():
    m = kit.module("shop_window", ground=True, **FITTING)
    shop_glass(m, *SHOP_WINDOW)
    # What stands in the window: a stall board inside the glass with a few things on it. From the street a shop's
    # room is mostly its bare side wall; the display is what says the shop is in use. Near only.
    goods = [flat_paint(f"shop_goods_{k}", colour, rough=0.8, grime=0.0)
             for k, colour in enumerate(((0.15, 0.06, 0.045), (0.06, 0.09, 0.12), (0.17, 0.14, 0.08), (0.2, 0.195, 0.18)))]  # dull: tins, cartons, sacks
    box(m.n("stall"), (2.3, 0.55, 0.5), (0, 0.5, 0.33), joinery_m, m.root, lods=(0,))
    for k_, (x_, w_, d_, h_) in enumerate(((-0.85, 0.4, 0.3, 0.35), (-0.3, 0.3, 0.3, 0.55), (0.2, 0.45, 0.35, 0.25), (0.8, 0.35, 0.3, 0.45),
                                           (0.45, 0.2, 0.2, 0.6))):
        box(m.n(f"goods_{k_}"), (w_, d_, h_), (x_, 0.5 + 0.08 * (k_ % 2), 0.58 + h_ / 2), goods[k_ % len(goods)], m.root, lods=(0,))
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


def shop_sign_module():
    m = kit.module("shop_sign", **FITTING)
    box(m.n("board"), (1.0, 0.1, 0.7), (0, -0.05, 0.35), sign_m, m.root)
    box(m.n("cornice"), (1.0, 0.18, 0.08), (0, -0.09, 0.74), stone_m, m.root, lods=(0, 1))


def dormer_module(tiles, cheeks=plaster_m, pitched=False):
    build_dormer(kit, f"dormer_{tiles}", cheeks, ROOFS[tiles], glass_m, pane_m, frame_m, pitched)


def dormer(t, shape, tiles, side, along, inset, tint, tiers=TIERS_0_TO_2):
    place_dormer(t, shape, f"dormer_{tiles}", side, along, inset, tint, tiers)


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
                          RUBBLE[wall], seed_of(t.id) + 17 * k, stacks=stacks_of(t, rect), coarse=12, **RUIN_STYLE.get(wall, {}))
        litter(t, ruin, DUST.get(wall, tint), heaps=4)
    t.place(m.name, tint=tint, state="ruin")


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
FAR_BOXES |= {f"dormer_{tiles}": dormer_far(plaster_m, ROOFS[tiles]) for tiles in ("brown", "clay", "slate")}


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


def house(id_, tag, category, w, d, floors, rise, along, hips, wall, roof, tint, lattice, porch=None, broken=None, flat=None):
    """A house that is one box under one roof: its template, shell module and roof shape.
    `porch` (depth, ceiling, posts, post, deck: `porch_shell`) sets its ground floor back behind
    an inset porch on its south front. `broken` (knee, run, hipped: `BrokenRoof`) breaks its
    roof's pitch: a gambrel, or hipped a mansard. `flat` (cornices, cornice) stands its walls
    `rise` over its eaves as parapets round a flat roof, with a cornice along each side named."""
    hx, hy = w / 2, d / 2
    eave = floors[-1] + WALL_M
    if flat:
        shape = FlatShape(-hx, hx, -hy, hy, eave + rise)
    elif broken:
        shape = BrokenRoof(-hx, hx, -hy, hy, eave, broken[0], eave + rise, broken[1], along, broken[2], MANSARD_OVER_M if broken[2] else OVER_M)
    else:
        shape = RoofShape(-hx, hx, -hy, hy, eave, eave + rise, along, hips, OVER_M, (VERGE_M, VERGE_M))
    m = kit.module(f"{tag}_shell", ground=True)
    if porch:
        porch_shell(m, "body", shape, porch["depth"], porch["ceiling"], WALLS[wall], ROOFS[roof], trim_m, porch["post"], porch["deck"], plinth_m,
                    porch["posts"])
    else:
        shell(m, "body", shape, WALLS[wall], None if flat else ROOFS[roof], plinth=(-hx, hx, -hy, hy))
    if flat:
        flat_top(m, "body", (-hx, hx, -hy, hy), shape.ridge, ROOFS[roof], coping_m)
        for side in flat["cornices"]:
            cornice(m, f"body_{side}", (-hx, hx, -hy, hy), side, shape.ridge, flat["cornice"])
    roof_form = ("flat" if flat else ("mansard" if broken[2] else "gambrel") if broken
                 else "hip" if min(hips) >= 1 else "gable" if max(hips) == 0 else "half-hip")
    recipe = dict(Width=w, Depth=d, Floors=len(floors), Roof=roof_form, RidgeAlong=along, Wall=wall, Tiles=roof)
    t = kit.template(id_, category, FAMILY, recipe | (dict(Porch=True) if porch else {}))
    t.part("body", -hx, hx, -hy, hy, shape.ridge)
    t.floors(*floors)
    for side, bay_at in lattice.items():
        t.lattice(f"body-{side}", bay_at)
    t.place(m.name, tint=tint)
    if flat:
        return t, shape
    # gutters run under every level eave: the two long sides, and a full hip's end
    eaves = {"x": ("south", "north"), "y": ("west", "east")}[along]
    ends = {"x": ("west", "east"), "y": ("south", "north")}[along]
    for side in eaves:
        rainwater(t, f"body-{side}", shape, shape.ends[1] - shape.ends[0], over=getattr(shape, "over", OVER_M))
    for k, side in enumerate(ends):
        if shape.hips[k] >= 1:
            rainwater(t, f"body-{side}", shape, 2 * shape.reach, pipes=(), over=getattr(shape, "over", OVER_M))
    return t, shape


def ell(id_, tag, wall, roof, tint, lattice, rise=2.35, porch=None):
    """An L: a two-storey house with a lower wing behind it. The wing's box is as tall as the
    house's (a join needs both tops equal); its ridge sits under it. Returns the template and
    the house's and the wing's roof shapes."""
    eave = 3.0 + WALL_M
    main = RoofShape(-5, 5, -5, 3, eave, eave + rise, "x", (0.0, 0.0), OVER_M, (VERGE_M, VERGE_M))
    wing_walls = RoofShape(-2, 4, 3, 9, eave, eave + 3 * main.tan, "y", (0.0, 0.0), OVER_M, (0.0, VERGE_M))
    wing_roof = RoofShape(-2, 4, -0.5, 9, eave, wing_walls.ridge, "y", (0.0, 0.0), OVER_M, (0.0, VERGE_M))
    m = kit.module(f"{tag}_shell", ground=True)
    if porch:
        porch_shell(m, "main", main, porch["depth"], porch["ceiling"], WALLS[wall], ROOFS[roof], trim_m, porch["post"], porch["deck"], plinth_m,
                    porch["posts"])
    else:
        shell(m, "main", main, WALLS[wall], ROOFS[roof], plinth=(-5, 5, -5, 3))
    shell(m, "wing", wing_walls, WALLS[wall], None, sides=("a1", "b0", "b1"), plinth=(-2, 4, 3, 9))
    # the wing's roof runs on into the house's, as far as its ridge goes before it meets the slope
    pitched_roof(m.n("wing_roof"), wing_roof, ROOFS[roof], trim_m, m.root, fascia_ends=(False, True))
    recipe = dict(Width=10, Depth=8, Floors=2, Roof="gable", RidgeAlong="x", Wall=wall, Tiles=roof, WingWidth=6, WingDepth=6)
    t = kit.template(id_, "detached_home", FAMILY, recipe | (dict(Porch=True) if porch else {}))
    t.part("main", -5, 5, -5, 3, main.ridge)
    t.part("wing", -2, 4, 3, 9, main.ridge)
    t.floors(0.0, 3.0)
    for edge, bay_at in lattice:
        t.lattice(edge, bay_at)
    t.place(m.name, tint=tint)
    return t, main, wing_walls


def ell_rainwater(t, main, wing):
    rainwater(t, "main-south", main, main.ends[1] - main.ends[0])
    rainwater(t, "main-north-2", main, 3.0, pipes=(1,))
    rainwater(t, "wing-east", wing, 6 + VERGE_M, pipes=(1,))
    rainwater(t, "wing-west", wing, 6 + VERGE_M, pipes=(-1,))


def shopfronts(t, paint, sign):
    """The corner shop's ground floor: shopfronts on the street and round the corner, its door in the street's."""
    t.entrance("body-south", -1.5)
    t.mount("shop_door", "body-south", -1.5, tiers=TIERS_0_TO_2, tint=paint)
    for o in (-4.5, 1.5, 4.5):
        t.mount("shop_window", "body-south", o, tiers=TIERS_0_TO_2, tint=paint)
    for o in (-4.5, -1.5):
        t.mount("shop_window", "body-east", o, tiers=TIERS_0_TO_2, tint=paint)
    t.mount("shop_sign", "body-south", 0.0, z=SHOP_HEAD_M + 0.15, scale=(11.5, 1.0, 1.0), tint=sign)
    t.mount("shop_sign", "body-east", -3.0, z=SHOP_HEAD_M + 0.15, scale=(5.5, 1.0, 1.0), tint=sign)


def terrace(id_, tag, units, unit_w, d, floors, rise, wall, roof, tints, window, door, joinery, side_bay, shop=False,
            parapets=False, ground=None, form=None, stoop=None, cornice_mat=None, dormers=None, chimney=None, wall_windows=False):
    """A row of `units` houses under one roof, ridge along the street. Each unit is one
    module in its own tint; the end walls are a module turned to face each way.
    `side_bay` is a bay's offset on the two end walls; `ground` the ground floor's window when it
    differs. `form` is the roof: a gable (None), "flat" (walls standing `rise` over the eaves as
    parapets, a cornice of `cornice_mat` along the street) or (knee, run), a mansard's broken
    pitch along the street. `stoop` raises each front door on a stoop; `dormers` puts one in each
    roof; `wall_windows` gives the windows the wall's tint (their stone hoods are the wall's)."""
    eave = floors[-1] + WALL_M
    ridge, hy, length = eave + rise, d / 2, units * unit_w
    verge = 0.0 if parapets else VERGE_M
    flat = form == "flat"
    rect = (-unit_w / 2, unit_w / 2, -hy, hy)
    if flat:
        unit = FlatShape(*rect, ridge)
    elif form:
        unit = BrokenRoof(*rect, eave, form[0], ridge, form[1], "x", False, MANSARD_OVER_M, (0.0, 0.0))
    else:
        unit = RoofShape(-unit_w / 2, unit_w / 2, -hy, hy, eave, ridge, "x", (0.0, 0.0), OVER_M, (0.0, 0.0))
    um = kit.module(f"{tag}_unit", ground=True)
    shell(um, "unit", unit, WALLS[wall], None if flat else ROOFS[roof], sides=("b0", "b1"), fascia_ends=(False, False),
          plinth=(-unit_w / 2, unit_w / 2, -hy, hy))
    if flat:
        flat_top(um, "unit", rect, ridge, ROOFS[roof], coping_m, walled=("south", "north"))
        cornice(um, "unit", rect, "south", ridge, cornice_mat)
    em = kit.module(f"{tag}_end", ground=True)
    end = (FlatShape(0, 0, -hy, hy, ridge) if flat else BrokenRoof(0, 0, -hy, hy, eave, form[0], ridge, form[1], "x", False, MANSARD_OVER_M, (0.0, verge))
           if form else RoofShape(0, 0, -hy, hy, eave, ridge, "x", (0.0, 0.0), OVER_M, (0.0, verge)))
    shell(em, "end", end, WALLS[wall], None if flat else ROOFS[roof], sides=("a1",), fascia_ends=(False, True))
    if flat:
        flat_top(em, "end", (-PARAPET_THICK_M, 0.0, -hy, hy), ridge, None, coping_m, walled=("east",))
    if parapets:  # a party wall carried up through the roof between neighbours
        pm = kit.module(f"{tag}_parapet")
        if flat:
            box(pm.n("wall"), (0.22, d - 2 * PARAPET_THICK_M, PARAPET_M), (0, 0, ridge - PARAPET_M / 2), coping_m, pm.root)
        else:
            lo, hi = unit.edge_z, ridge
            prism(pm.n("wall"), [(-unit.reach, lo + 0.16), (0, hi + 0.16), (unit.reach, lo + 0.16), (unit.reach, lo - 0.12),
                                 (0, hi - 0.12), (-unit.reach, lo - 0.12)], 0.22, (0, 0, 0), coping_m, pm.root, rot=(0, 0, math.pi / 2))
    roof_form = "flat" if flat else "mansard" if form else "gable"
    t = kit.template(id_, "attached_home", FAMILY, dict(Units=units, UnitWidth=unit_w, Depth=d, Floors=len(floors), Roof=roof_form,
                                                        RidgeAlong="x", Wall=wall, Tiles=roof, Shopfronts=shop, Parapets=parapets))
    centres = [(k - (units - 1) / 2) * unit_w for k in range(units)]
    for k, cx in enumerate(centres):
        t.part(f"unit-{k}", cx - unit_w / 2, cx + unit_w / 2, -hy, hy, ridge)
    t.floors(*floors)
    t.lattice("unit-0-west", side_bay)
    t.lattice(f"unit-{units - 1}-east", side_bay)
    stack_module = chimney or ("chimney_brick" if wall == "brick" else "chimney_render")
    for k, cx in enumerate(centres):
        tint, paint = tints[k % len(tints)], joinery[k % len(joinery)]
        pane_tint = tint if wall_windows else paint
        south, north = f"unit-{k}-south", f"unit-{k}-north"
        t.lattice(south, 1.5)
        t.lattice(north, 1.5)
        t.place(um.name, x=cx, tint=tint)
        if shop:
            t.entrance(south, -1.5)
            t.mount("shop_door", south, -1.5, tiers=TIERS_0_TO_2, tint=paint)
            t.mount("shop_window", south, 1.5, tiers=TIERS_0_TO_2, tint=paint)
            t.mount("shop_sign", south, 0.0, z=SHOP_HEAD_M + 0.15, scale=(unit_w - 0.5, 1.0, 1.0), tint=SIGNS[k % len(SIGNS)])
            glaze(t, south, floors[1:], window, tint=pane_tint)
        else:
            if stoop:
                t.entrance(south, -1.5)
                t.mount(door, south, -1.5, z=STOOPS[stoop], tiers=TIERS_0_TO_2, tint=paint)
                t.mount(stoop, south, -1.5, tiers=TIERS_0_TO_2, tint=tint)
            else:
                front_door(t, south, -1.5, door, paint)
            glaze(t, south, floors, window, ground=ground, skip=(-1.5,), tint=pane_tint)
        glaze(t, north, floors, window, ground=ground, tint=pane_tint)
        if not flat:
            rainwater(t, south, unit, unit_w, pipes=(1,) if k < units - 1 else (), over=getattr(unit, "over", OVER_M))
            rainwater(t, north, unit, unit_w, pipes=(-1,) if k < units - 1 else (), over=getattr(unit, "over", OVER_M))
        if k:
            stack(t, stack_module, cx - unit_w / 2, 0.0, ridge, yaw=math.pi / 2)
        if dormers:  # a family's own pitched dormers, rows at every tier
            dormer(t, unit, dormers, "south", cx, 0.3 if form else 1.0, tint, EVERY_TIER)
    for k, (edge, x, yaw) in enumerate((("unit-0-west", -length / 2, math.pi), (f"unit-{units - 1}-east", length / 2, 0.0))):
        t.place(em.name, x=x, yaw=yaw, tint=tints[(0, units - 1)[k] % len(tints)])
        glaze(t, edge, floors, window, ground=ground, tint=tints[(0, units - 1)[k] % len(tints)] if wall_windows else WHITE)
    if parapets:
        for k in range(1, units) if flat else range(units + 1):
            x = -length / 2 + k * unit_w
            t.place(pm.name, x=min(max(x, -length / 2 + 0.11), length / 2 - 0.11), tiers=EVERY_TIER if flat else TIERS_0_TO_2)
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
    breast = STACKS[stack_module]
    ruins = []
    for v in "ab":
        m = kit.module(f"{tag}_ruin_{v}", ground=True, **RUIN)
        ruins.append((m, ruin_block(m, "unit", (-unit_w / 2, unit_w / 2, -hy, hy), high, over, sides, WALLS[wall], ROOFS[roof],
                                    RUBBLE[wall], seed + 7 * (v == "b"), stacks=[(-unit_w / 2 + 0.4, 0.0, breast[0][::-1], breast[1])],
                                    far=("south", "north"), coarse=10, **RUIN_STYLE.get(wall, {}))))
    for k, cx in enumerate(centres):
        m, ruin = ruins[k % 2]
        level = (1.0, 0.8, 0.92, 0.74, 0.96)[k % 5]
        t.place(m.name, x=cx, scale=(1.0, 1.0, level), tint=tints[k % len(tints)], state="ruin")
        litter(t, ruin, DUST.get(wall, tints[k % len(tints)]), at=(cx, 0.0), beams=2, heaps=3, seed=k, level=level)
    em = kit.module(f"{tag}_ruin_end", ground=True, **RUIN)
    burnt = scorched(WALLS[wall], high + over, seed)
    ragged_wall(em.n("wall"), -hy, hy, -RUIN_WALL_M / 2, False, 0.3 * high, high + over - 0.06, seed + 5, burnt, em.root, RUIN_WALL_M,
                lods=(0, 1, 2), groups=(1, 3, 6, 8), jagged=1.5)
    wall_panels(em.n("far"), [(0.0, 0.0, 0.0, math.pi / 2, d, 0.8 * high)], burnt, em.root, proud=0.0, lods=(3,))
    t.place(em.name, x=length / 2, tint=tints[(units - 1) % len(tints)], state="ruin")
    return t


def stoop_door(t, edge, offset, door, stoop_module, tint, stone_tint=WHITE):
    """A front door raised on a stoop."""
    t.entrance(edge, offset)
    t.mount(door, edge, offset, z=STOOPS[stoop_module], tiers=TIERS_0_TO_2, tint=tint)
    t.mount(stoop_module, edge, offset, tiers=TIERS_0_TO_2, tint=stone_tint)


# ---------------------------------------------------------------- China
def china():
    window_module("window_a", 1.0, 1.3)
    window_module("window_a_shutters", 1.0, 1.3, shutters=True)
    window_module("window_wide", 1.6, 1.3, lights=3)
    window_module("window_tall", 0.9, 1.5, lights=1)
    door_modules()
    step_module()
    chimney_module("chimney_brick")
    chimney_module("chimney_render", (0.7, 0.7), render_stack_m, 1)
    rainwater_modules()
    shop_modules()
    for tiles_ in ("brown", "clay", "slate"):
        dormer_module(tiles_)
    shop_sign_module()
    wreckage(kit, rubble_m, char_m)

    # ------------------------------------------------------------ detached homes
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
    t, main, wing = ell("china-home-ell-2f", "home_ell_2f", "plaster", "clay", SAGE,
                        (("main-south", 0.0), ("main-east", 1.5), ("main-west", 1.5), ("main-north-0", 0.5), ("main-north-2", 3.5),
                         ("wing-east", 1.5), ("wing-west", 1.5), ("wing-north", 1.5)))
    front_door(t, "main-south", 0.0, "door_canopy", UMBER)
    glaze(t, "main-south", (0.0, 3.0), "window_a_shutters", skip=(0.0,), tint=UMBER)
    for edge in ("main-east", "main-west", "main-north-2", "wing-east", "wing-west", "wing-north"):
        glaze(t, edge, (0.0, 3.0), "window_a")
    ell_rainwater(t, main, wing)
    stack(t, "chimney_brick", -3.0, -1.0, main.ridge)
    stack(t, "chimney_render", 1.0, 6.5, wing.ridge, yaw=math.pi / 2)
    fallen(t, "home_ell_2f", "plaster", "clay", SAGE)

    # ------------------------------------------------------------ attached homes
    # A town house standing alone, its narrow gable to the street: dark brick, slate.
    t, shape = house("china-townhouse-2f", "townhouse_2f", "attached_home", 7, 10, (0.0, 3.0), 2.45, "y", (0.0, 0.0), "brick", "slate",
                     BROWN_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0))
    front_door(t, "body-south", -1.5, "door_canopy", BLUE)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_tall", skip=(-1.5,) if side == "south" else (), tint=BLUE)
    stack(t, "chimney_brick", 0.0, 1.5, shape.ridge, yaw=math.pi / 2)
    fallen(t, "townhouse_2f", "brick", "slate", BROWN_BRICK)

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
    shopfronts(t, GREEN, SIGNS[3])
    glaze(t, "body-south", (4.0, 7.0), "window_wide")
    glaze(t, "body-east", (0.0, 4.0, 7.0), "window_wide", skip=(-4.5, -1.5))
    for side in ("north", "west"):
        glaze(t, f"body-{side}", (0.0, 4.0, 7.0), "window_a")
    stack(t, "chimney_render", -2.0, 2.0, shape.ridge - 0.6)
    dormer(t, shape, "clay", "west", -2.0, 1.4, OCHRE)
    fallen(t, "corner_shop_3f", "plaster", "clay", OCHRE)


# ---------------------------------------------------------------- New York
def new_york():
    """American frame houses in clapboard under asphalt shingle, white trim, shutters, sash windows and
    porches set into the house; rows of brick and brownstone under flat roofs behind bracketed cornices,
    their front doors up stoops."""
    clap_m = textured("wall_clapboard", "clapboard", tint=1.0, dirt=0.5, chip=0.6, streak=0.25, rise=0.7)
    # A brownstone front is smooth: coursed, it read as one more brick row from the camera
    brownstone_m = textured("wall_brownstone", "plaster", tint=1.0, dirt=0.45, chip=0.25, streak=0.35, rise=0.6)
    lintel_m = textured("trim_brownstone", "plaster", tint=1.0, dirt=0.0, chip=0.3, streak=0.2)
    WALLS.update(clapboard=clap_m, brownstone=brownstone_m)
    # asphalt shingle in its three common colours: charcoal, a red-brown and a grey-green
    for name, colour, seed in (("shingle", (0.042, 0.043, 0.045), 12.0), ("shingle_brown", (0.07, 0.045, 0.034), 14.0),
                               ("shingle_green", (0.042, 0.054, 0.046), 15.0)):
        ROOFS[name] = weathered_roof(textured(f"roof_{name}", "roof_slate", colour=colour, seed=seed, **ROOFING), seed=seed, moss=0.15)
    ROOFS["tar"] = textured("roof_tar", "flat_roof", colour=(0.075, 0.074, 0.07), dirt=0.0, chip=0.0, streak=0.0, grain=0.0)
    ashes_m = textured("rubble_ashes", "rubble", colour=(0.05, 0.047, 0.044), dirt=0.0, chip=0.0, streak=0.0, ash=0.25, seed=4.0)
    RUBBLE.update(clapboard=ashes_m, brownstone=rubble_m)
    DUST.update(clapboard=(78, 74, 70))
    RUIN_STYLE["clapboard"] = dict(level=0.8, charred=0.7, thick=0.12, run=(1.2, 2.6), jagged=2.5)  # a frame house burns to its sills
    white_m = flat_paint("trim_white", (0.6, 0.6, 0.58), rough=0.7, grime=0.0)
    cornice_m = flat_paint("cornice_paint", (0.055, 0.05, 0.045), rough=0.6, metal=0.2, grime=0.0)
    deck_m = textured("porch_deck", "joinery", colour=(0.17, 0.16, 0.15), dirt=0.4, chip=0.3, streak=0.0)

    sash(kit, "window_sash", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m)
    sash(kit, "window_sash_shutters", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m, shutter=joinery_m, panes=(3, 2))
    sash(kit, "window_sash_porch", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m, behind=None, dark=bpy_dark())
    sash(kit, "window_sash_hood", 0.9, 1.6, frame_m, glass_m, pane_m, stone_m, hood=stone_m, panes=(2, 1))
    sash(kit, "window_sash_hood_tint", 0.9, 1.6, frame_m, glass_m, pane_m, lintel_m, hood=lintel_m, panes=(2, 1))
    for name, portico in (("door_transom", False), ("door_portico", True)):
        m = door_module(kit, name, 0.95, 2.05, joinery_m, white_m, pane_m, light=0.4)
        if portico:  # a pediment on pilasters
            prism(m.n("pediment"), [(-0.85, 2.62), (0.85, 2.62), (0.0, 3.0)], 0.4, (0, -0.2, 0), white_m, m.root, lods=(0, 1, 2))
            for s in (-1, 1):
                box(m.n(f"pilaster_{'ab'[s > 0]}"), (0.16, 0.1, 2.62), (s * 0.7, -0.05, 1.31), white_m, m.root, lods=(0, 1))
    step_module()
    stoop(kit, "stoop", stone_m, railing_rail())
    stoop(kit, "stoop_brown", lintel_m, railing_rail(), top=0.95, steps=5)
    porch_rail(kit, "porch_rail", PORCH_RAIL_M, white_m)
    porch_steps(kit, "porch_steps", 1.6, deck_m)
    chimney_module("chimney_brick")
    rainwater_modules()
    shop_modules()
    dormer_module("shingle", cheeks=clap_m, pitched=True)
    dormer_module("shingle_brown", cheeks=clap_m, pitched=True)
    shop_sign_module()
    wreckage(kit, rubble_m, char_m)
    FAR_PANELS.update({
        "window_sash": window_far(0.9, 1.6, pane_m, white_m), "window_sash_porch": window_far(0.9, 1.6, pane_m, white_m),
        "window_sash_shutters": window_far(0.9, 1.6, pane_m, white_m, "tint"),
        "window_sash_hood": window_far(0.9, 1.6, pane_m, stone_m) + [(1.44, 0.24, 1.72, stone_m, 0.016)],
        "window_sash_hood_tint": window_far(0.9, 1.6, pane_m, frame_m),
        "door_transom": [(1.19, 2.62, 0.0, white_m, 0.012), (0.95, 2.05, 0.0, "tint"), (0.95, 0.4, 2.11, pane_m)],
        "door_portico": [(1.56, 3.0, 0.0, white_m, 0.012), (0.95, 2.05, 0.0, "tint"), (0.95, 0.4, 2.11, pane_m)],
        "porch_rail": [(PORCH_RAIL_M, 0.7, PORCH_DECK_M, white_m)],
    })
    FAR_BOXES.update({"stoop": [((1.6, 0.48, STOOP_M), (0, -0.24, STOOP_M / 2), stone_m)],
                      "stoop_brown": [((1.6, 0.48, 0.95), (0, -0.24, 0.475), lintel_m)],
                      # from the oblique camera a chimney is its brick stack, not only its cap
                      "chimney_brick": [((1.05, 0.6, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), stack_m)],
                      "door_portico": [((1.7, 0.4, 0.38), (0, -0.2, 2.81), white_m)]})

    # Tints, sRGB: clapboard paints, brownstones, and the shutters' and doors' paints.
    white, butter, sage, olive, blue_grey = (242, 240, 234), (238, 222, 168), (184, 196, 170), (168, 160, 120), (178, 194, 206)
    brownstones = ((128, 104, 92), (120, 98, 88), (134, 108, 94), (124, 100, 90), (130, 106, 96))
    black_green, navy, black = (38, 52, 44), (40, 50, 72), (34, 34, 34)
    porch = dict(ceiling=2.9, post=white_m, deck=deck_m)

    # ------------------------------------------------------------ detached homes
    # A Cape Cod: white clapboard under a steep shingle roof, a door between shuttered windows, two dormers.
    t, shape = house("nyc-home-10x8-1f", "home_10x8_1f", "detached_home", 10, 8, (0.0,), 3.0, "x", (0.0, 0.0), "clapboard", "shingle",
                     white, dict(south=0.0, north=0.0, east=1.5, west=1.5))
    front_door(t, "body-south", 0.0, "door_transom", navy)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0,), "window_sash_shutters", skip=(0.0,) if side == "south" else (), tint=black)
    stack(t, "chimney_brick", 0.0, 0.0, shape.ridge)
    for along in (-2.6, 2.6):
        dormer(t, shape, "shingle", "south", along, 1.0, white, EVERY_TIER)
    fallen(t, "home_10x8_1f", "clapboard", "shingle", white)

    # A Foursquare: a square two-storey box under a pyramid roof and a dormer, the ground floor set back behind a porch.
    t, shape = house("nyc-home-9x9-2f", "home_9x9_2f", "detached_home", 9, 9, (0.0, 3.0), 2.4, "x", (1.0, 1.0), "clapboard", "shingle_brown",
                     butter, dict(south=0.0, north=0.0, east=0.0, west=0.0), porch=porch | dict(depth=2.6, posts=(-1.5, 1.5)))
    porch_front(t, "body-south", 2.6, 0.0, (-3.0, 3.0), "door_transom", black_green, windows=(-3.0, 3.0))
    for edge, o in (("body-east", -3.0), ("body-west", 3.0)):
        porch_side(t, edge, o, "window_sash", (0.0, 3.0))
    glaze(t, "body-south", (3.0,), "window_sash")
    glaze(t, "body-north", (0.0, 3.0), "window_sash")
    stack(t, "chimney_brick", 2.0, 2.0, shape.ridge - 0.6)
    dormer(t, shape, "shingle_brown", "south", 0.0, 1.0, butter, EVERY_TIER)
    fallen(t, "home_9x9_2f", "clapboard", "shingle_brown", butter)

    # A Dutch Colonial: a gambrel roof over two storeys, a pedimented door, shuttered six-over-six windows.
    t, shape = house("nyc-home-12x9-2f", "home_12x9_2f", "detached_home", 12, 9, (0.0, 3.0), 3.0, "x", (0.0, 0.0), "clapboard", "shingle_green",
                     sage, dict(south=0.0, north=0.0, east=1.5, west=1.5), broken=(1.9, 1.1, False))
    front_door(t, "body-south", 0.0, "door_portico", black_green)
    glaze(t, "body-south", (0.0, 3.0), "window_sash_shutters", skip=(0.0,), tint=black_green)
    for side in ("north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_sash_shutters", tint=black_green)
    stack(t, "chimney_brick", -5.0, 0.0, shape.ridge)
    fallen(t, "home_12x9_2f", "clapboard", "shingle_green", sage)

    # A Craftsman bungalow, gable to the street, its front porch under the gable.
    t, shape = house("nyc-home-8x11-1f", "home_8x11_1f", "detached_home", 8, 11, (0.0,), 2.3, "y", (0.0, 0.0), "clapboard", "shingle_brown",
                     olive, dict(south=1.5, north=1.5, east=1.5, west=1.5), porch=porch | dict(depth=2.4, ceiling=2.6, posts=(0.0,)))
    porch_front(t, "body-south", 2.4, -1.5, (1.5,), "door_transom", (122, 58, 48), windows=(1.5,))
    for edge, o in (("body-east", -4.5), ("body-west", 4.5)):
        porch_side(t, edge, o, "window_sash", (0.0,))
    glaze(t, "body-north", (0.0,), "window_sash")
    stack(t, "chimney_brick", 0.0, 2.5, shape.ridge, yaw=math.pi / 2)
    fallen(t, "home_8x11_1f", "clapboard", "shingle_brown", olive)

    # A gable-and-wing farmhouse: an L in pale blue clapboard, its front porch set into the house.
    t, main, wing = ell("nyc-home-ell-2f", "home_ell_2f", "clapboard", "shingle", blue_grey,
                        (("main-south", 0.0), ("main-east", 0.0), ("main-west", 0.0), ("main-north-0", 0.5), ("main-north-2", 3.5),
                         ("wing-east", 1.5), ("wing-west", 1.5), ("wing-north", 1.5)), rise=2.8,
                        porch=porch | dict(depth=2.4, posts=(-1.5, 1.5)))
    porch_front(t, "main-south", 2.4, 0.0, (-3.0, 3.0), "door_transom", navy, windows=(-3.0, 3.0))
    glaze(t, "main-south", (3.0,), "window_sash")
    for edge, o in (("main-east", -3.0), ("main-west", 3.0)):
        porch_side(t, edge, o, "window_sash", (0.0, 3.0))
    for edge in ("main-north-2", "wing-east", "wing-west", "wing-north"):
        glaze(t, edge, (0.0, 3.0), "window_sash")
    ell_rainwater(t, main, wing)
    stack(t, "chimney_brick", -3.0, -1.0, main.ridge)
    stack(t, "chimney_brick", 1.0, 6.5, wing.ridge, yaw=math.pi / 2)
    fallen(t, "home_ell_2f", "clapboard", "shingle", blue_grey)

    # ------------------------------------------------------------ attached homes
    # A brick town house standing alone: a flat roof behind a bracketed cornice, its door up a stoop.
    t, shape = house("nyc-townhouse-2f", "townhouse_2f", "attached_home", 7, 10, (0.0, 3.0), 0.9, "x", (0.0, 0.0), "brick", "tar",
                     RED_BRICK, dict(south=1.5, north=1.5, east=0.0, west=0.0), flat=dict(cornices=("south",), cornice=cornice_m))
    stoop_door(t, "body-south", -1.5, "door_transom", "stoop", OXBLOOD)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_sash_hood", skip=(-1.5,) if side == "south" else ())
    stack(t, "chimney_brick", -2.4, 2.5, shape.ridge)
    fallen(t, "townhouse_2f", "brick", "tar", RED_BRICK)

    # Three two-storey brick row houses, each with its stoop and its cornice.
    terrace("nyc-rowhouses-3x2f", "terrace_3x2f", 3, 6, 10, (0.0, 3.0), 0.9, "brick", "tar", (RED_BRICK, BROWN_BRICK, PALE_BRICK),
            "window_sash_hood", "door_transom", (OXBLOOD, black_green, navy), side_bay=0.0, parapets=True, form="flat", stoop="stoop",
            cornice_mat=cornice_m, chimney="chimney_brick")
    # Five brownstones, three storeys: stone hoods over the windows, stoops, cornices.
    terrace("nyc-brownstones-5x3f", "terrace_5x3f", 5, 6, 11, (0.0, 3.0, 6.0), 0.9, "brownstone", "tar", brownstones,
            "window_sash_hood_tint", "door_transom", (black, OXBLOOD, black_green, navy, UMBER), side_bay=0.0, parapets=True, form="flat",
            stoop="stoop_brown", cornice_mat=cornice_m, chimney="chimney_brick", wall_windows=True)
    # Four shops with two floors of flats over them, in brick under one cornice line.
    terrace("nyc-shops-4x3f", "shops_4x3f", 4, 7.5, 13, (0.0, 4.0, 7.0), 0.9, "brick", "tar", (BROWN_BRICK, RED_BRICK, PALE_BRICK, RED_BRICK),
            "window_sash_hood", None, (black_green, OXBLOOD, navy, black), side_bay=1.5, shop=True, parapets=True, form="flat",
            cornice_mat=cornice_m, chimney="chimney_brick")

    # The corner store: shopfronts on the street and round the corner, flats above, a cornice on both fronts.
    t, shape = house("nyc-corner-shop-3f", "corner_shop_3f", "attached_home", 12, 12, (0.0, 4.0, 7.0), 0.9, "x", (0.0, 0.0), "brick", "tar",
                     BROWN_BRICK, dict(south=1.5, north=1.5, east=1.5, west=1.5), flat=dict(cornices=("south", "east"), cornice=cornice_m))
    shopfronts(t, black_green, SIGNS[1])
    glaze(t, "body-south", (4.0, 7.0), "window_sash_hood")
    glaze(t, "body-east", (0.0, 4.0, 7.0), "window_sash_hood", skip=(-4.5, -1.5))
    for side in ("north", "west"):
        glaze(t, f"body-{side}", (0.0, 4.0, 7.0), "window_sash_hood")
    stack(t, "chimney_brick", -3.0, 3.0, shape.ridge)
    fallen(t, "corner_shop_3f", "brick", "tar", BROWN_BRICK)


# ---------------------------------------------------------------- Paris
def paris():
    """Pavillons in meulière and render under tile and slate hips, tall French windows between louvred
    shutters, door-windows behind iron guards; maisons de ville in coloured render, and rows of stone
    and render under slate mansards with lucarnes, chimney stacks on the party walls."""
    limestone_m = textured("wall_limestone", "ashlar", tint=1.0, dirt=0.45, chip=0.25, streak=0.35, rise=0.6)
    meuliere_m = textured("wall_meuliere", "meuliere", dirt=0.45, chip=0.15, streak=0.2, rise=0.6, lichen=0.1)
    WALLS.update(limestone=limestone_m, meuliere=meuliere_m)
    RUBBLE.update(limestone=rubble_m, meuliere=textured("rubble_meuliere", "rubble", colour=(0.16, 0.1, 0.06), dirt=0.0, chip=0.0, streak=0.0,
                                                        ash=0.25, seed=5.0))
    DUST.update(meuliere=(176, 132, 96))
    white_m = flat_paint("window_white", (0.58, 0.57, 0.54), rough=0.6, grime=0.0)  # French joinery, painted
    shutter_m = textured("shutter_slats", "roller_slats", tint=1.0, dirt=0.0, chip=0.4, streak=0.0)
    guard_m = textured("guard_iron", "grille", dirt=0.0, chip=0.6, streak=0.0, coverage=("cutout", 0.5))
    brick_trim_m = textured("trim_brick", "brick", dirt=0.0, chip=0.2, streak=0.2)
    for name in ("dormer_brown", "dormer_clay", "dormer_slate"):  # this family's dormers are pitched rows at every tier, never folded
        FAR_BOXES.pop(name)
    # Paris slate is the blue-black of Angers, darker than the grey slate China's houses carry
    ROOFS["slate"] = weathered_roof(textured("roof_slate_blue", "roof_slate_matte", colour=(0.078, 0.082, 0.09), seed=6.0, **ROOFING),
                                    seed=6.0, moss=0.08)

    french_window(kit, "window_persienne", 1.0, 1.5, white_m, glass_m, pane_m, stone_m, shutter_m)
    french_window(kit, "window_persienne_brick", 1.0, 1.5, white_m, glass_m, pane_m, brick_trim_m, shutter_m)
    french_window(kit, "window_balcon", 1.0, 2.2, white_m, glass_m, pane_m, stone_m, shutter_m, railing=guard_m, foot=-0.8)
    door_module(kit, "door_tall", 1.15, 2.25, joinery_m, stone_m, pane_m, light=0.3)
    m = door_module(kit, "door_marquise", 1.0, 2.2, joinery_m, stone_m, pane_m, light=0.3)  # a glass canopy on iron brackets over a villa's door
    sheet(m.n("marquise"), 1.7, 0.47, (0, -0.46, 2.82), glass_m, m.root, rot=(-(math.pi / 2 - 0.25), 0, 0), lods=(0, 1, 2))
    box(m.n("marquise_edge"), (1.74, 0.04, 0.05), (0, -0.46, 2.82), railing_rail(), m.root, lods=(0, 1, 2))
    for s in (-1, 1):
        box(m.n(f"marquise_arm_{'ab'[s > 0]}"), (0.04, 0.46, 0.04), (s * 0.84, -0.23, 2.86), railing_rail(), m.root, lods=(0, 1))
    step_module()
    chimney_module("chimney_render", (0.7, 0.7), render_stack_m, 1)
    chimney_module("chimney_souche", (1.5, 0.55), render_stack_m, 4)  # a party wall's stack, a pot to each flue
    STACKS["chimney_souche"] = ((1.5, 0.55), render_stack_m)
    FAR_BOXES["chimney_souche"] = [((1.5, 0.55, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), render_stack_m)]
    FAR_BOXES["chimney_render"] = [((0.7, 0.7, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), render_stack_m)]
    rainwater_modules()
    shop_modules()
    for tiles_ in ("brown", "slate"):  # a lucarne's front and cheeks are dressed stone, not the house's render
        dormer_module(tiles_, cheeks=stone_m, pitched=True)
    shop_sign_module()
    wreckage(kit, rubble_m, char_m)

    def french_far(w, h, foot, surround):  # its shutters in their paint either side, its surround and its pane
        return [(2 * w + 0.41, h + 0.04, foot - 0.02, "tint", 0.02), (w + 0.28, h + 0.17, foot - 0.07, surround, 0.012), (w, h, foot, pane_m)]

    FAR_PANELS.update({
        "window_persienne": french_far(1.0, 1.5, 0.0, stone_m), "window_persienne_brick": french_far(1.0, 1.5, 0.0, brick_trim_m),
        "window_balcon": french_far(1.0, 2.2, -0.8, stone_m),
        "door_tall": [(1.39, 2.71, 0.0, stone_m, 0.012), (1.15, 2.25, 0.0, "tint"), (1.15, 0.3, 2.31, pane_m)],
        "door_marquise": [(1.24, 2.66, 0.0, stone_m, 0.012), (1.0, 2.2, 0.0, "tint"), (1.0, 0.3, 2.26, pane_m)],
    })

    # Tints, sRGB: renders and stone, and the shutters' and doors' paints.
    cream, rose, ochre, grey, stone = (236, 226, 204), (232, 204, 182), (230, 208, 166), (222, 220, 212), (236, 231, 220)
    stones = ((236, 231, 220), (230, 226, 216), (238, 232, 218), (226, 223, 216), (234, 228, 214))
    gris_bleu, gris, vert, creme, bordeaux, vert_fonce, bleu_nuit = ((118, 138, 150), (190, 190, 182), (104, 128, 102), (228, 222, 202),
                                                                     (124, 52, 50), (50, 78, 62), (44, 56, 82))

    # ------------------------------------------------------------ detached homes
    # A pavillon de plain-pied in meulière, brick round its windows, a hipped roof of red tiles.
    t, shape = house("paris-home-10x8-1f", "home_10x8_1f", "detached_home", 10, 8, (0.0,), 2.6, "x", (1.0, 1.0), "meuliere", "clay",
                     WHITE, dict(south=0.0, north=0.0, east=1.5, west=1.5))
    front_door(t, "body-south", 0.0, "door_tall", vert_fonce)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0,), "window_persienne_brick", skip=(0.0,) if side == "south" else (), tint=gris_bleu)
    stack(t, "chimney_render", 2.0, 0.0, shape.ridge)
    fallen(t, "home_10x8_1f", "meuliere", "clay", WHITE)

    # A villa in rose render under a steep pyramid of brown tiles: a marquise over the door, door-windows above.
    t, shape = house("paris-home-9x9-2f", "home_9x9_2f", "detached_home", 9, 9, (0.0, 3.0), 3.0, "x", (1.0, 1.0), "plaster", "brown",
                     rose, dict(south=0.0, north=0.0, east=0.0, west=0.0))
    front_door(t, "body-south", 0.0, "door_marquise", bleu_nuit)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_balcon", ground="window_persienne", skip=(0.0,) if side == "south" else (), tint=creme)
    stack(t, "chimney_render", 1.6, 1.2, shape.ridge - 0.4)
    dormer(t, shape, "brown", "south", 0.0, 1.3, rose, EVERY_TIER)
    fallen(t, "home_9x9_2f", "plaster", "brown", rose)

    # A maison bourgeoise in dressed stone under a slate mansard, lucarnes in its steep slopes.
    t, shape = house("paris-home-12x9-2f", "home_12x9_2f", "detached_home", 12, 9, (0.0, 3.0), 3.4, "x", (1.0, 1.0), "limestone", "slate",
                     stone, dict(south=0.0, north=0.0, east=1.5, west=1.5), broken=(2.4, 0.7, True))
    front_door(t, "body-south", 0.0, "door_tall", bleu_nuit)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_balcon", ground="window_persienne", skip=(0.0,) if side == "south" else (), tint=gris)
    for along in (-3.0, 3.0):
        dormer(t, shape, "slate", "south", along, 0.25, stone, EVERY_TIER)
    stack(t, "chimney_render", -4.0, 0.0, shape.ridge)
    stack(t, "chimney_render", 4.0, 0.0, shape.ridge)
    fallen(t, "home_12x9_2f", "limestone", "slate", stone)

    # A small pavillon, gable to the street and half-hipped, cream render, slate.
    t, shape = house("paris-home-8x11-1f", "home_8x11_1f", "detached_home", 8, 11, (0.0,), 3.0, "y", (0.45, 0.45), "plaster", "slate",
                     cream, dict(south=1.5, north=1.5, east=1.5, west=1.5))
    front_door(t, "body-south", -1.5, "door_tall", bordeaux)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0,), "window_persienne", skip=(-1.5,) if side == "south" else (), tint=vert)
    stack(t, "chimney_render", 0.0, 2.5, shape.ridge, yaw=math.pi / 2)
    fallen(t, "home_8x11_1f", "plaster", "slate", cream)

    # An L in grey render under slate: green shutters, door-windows upstairs.
    t, main, wing = ell("paris-home-ell-2f", "home_ell_2f", "plaster", "slate", grey,
                        (("main-south", 0.0), ("main-east", 1.5), ("main-west", 1.5), ("main-north-0", 0.5), ("main-north-2", 3.5),
                         ("wing-east", 1.5), ("wing-west", 1.5), ("wing-north", 1.5)), rise=3.0)
    front_door(t, "main-south", 0.0, "door_marquise", vert_fonce)
    glaze(t, "main-south", (0.0, 3.0), "window_balcon", ground="window_persienne", skip=(0.0,), tint=vert)
    for edge in ("main-east", "main-west", "main-north-2", "wing-east", "wing-west", "wing-north"):
        glaze(t, edge, (0.0, 3.0), "window_balcon", ground="window_persienne", tint=vert)
    ell_rainwater(t, main, wing)
    stack(t, "chimney_render", -3.0, -1.0, main.ridge)
    stack(t, "chimney_render", 1.0, 6.5, wing.ridge, yaw=math.pi / 2)
    fallen(t, "home_ell_2f", "plaster", "slate", grey)

    # ------------------------------------------------------------ attached homes
    # A maison de ville standing alone, its ridge along the street: ochre render, slate, a lucarne.
    t, shape = house("paris-townhouse-2f", "townhouse_2f", "attached_home", 7, 10, (0.0, 3.0), 3.2, "x", (0.0, 0.0), "plaster", "slate",
                     ochre, dict(south=1.5, north=1.5, east=0.0, west=0.0))
    front_door(t, "body-south", -1.5, "door_tall", bordeaux)
    for side in ("south", "north", "east", "west"):
        glaze(t, f"body-{side}", (0.0, 3.0), "window_balcon", ground="window_persienne", skip=(-1.5,) if side == "south" else (), tint=gris)
    dormer(t, shape, "slate", "south", 1.5, 1.0, ochre, EVERY_TIER)
    stack(t, "chimney_render", 2.8, 0.0, shape.ridge)
    fallen(t, "townhouse_2f", "plaster", "slate", ochre)

    # Three maisons de ville in a row, each its own render and shutters, under red tiles.
    terrace("paris-terrace-3x2f", "terrace_3x2f", 3, 6, 10, (0.0, 3.0), 3.2, "plaster", "brown", (cream, rose, grey), "window_balcon",
            "door_tall", (gris_bleu, vert, bordeaux), side_bay=0.0, ground="window_persienne", dormers="brown", chimney="chimney_souche")
    # Five stone houses of three storeys under one slate mansard, a lucarne to each.
    terrace("paris-terrace-5x3f", "terrace_5x3f", 5, 6, 11, (0.0, 3.0, 6.0), 3.3, "limestone", "slate", stones, "window_balcon",
            "door_tall", (gris, gris_bleu, vert_fonce, creme, bleu_nuit), side_bay=0.0, ground="window_persienne", form=(2.3, 0.75),
            dormers="slate", chimney="chimney_souche")
    # Four shops with two floors of flats over them, their painted shopfronts under a mansard.
    terrace("paris-shops-4x3f", "shops_4x3f", 4, 7.5, 13, (0.0, 4.0, 7.0), 3.3, "plaster", "slate", (cream, ochre, grey, rose),
            "window_balcon", None, (bordeaux, vert_fonce, bleu_nuit, gris_bleu), side_bay=1.5, shop=True, ground="window_persienne",
            form=(2.3, 0.75), dormers="slate", chimney="chimney_souche")

    # The corner café: shopfronts on the street and round the corner, flats above, a hipped mansard.
    t, shape = house("paris-corner-shop-3f", "corner_shop_3f", "attached_home", 12, 12, (0.0, 4.0, 7.0), 3.4, "x", (1.0, 1.0), "limestone",
                     "slate", stone, dict(south=1.5, north=1.5, east=1.5, west=1.5), broken=(2.4, 0.7, True))
    shopfronts(t, bordeaux, SIGNS[0])
    glaze(t, "body-south", (4.0, 7.0), "window_balcon", tint=gris)
    glaze(t, "body-east", (0.0, 4.0, 7.0), "window_balcon", ground="window_persienne", skip=(-4.5, -1.5), tint=gris)
    for side in ("north", "west"):
        glaze(t, f"body-{side}", (0.0, 4.0, 7.0), "window_balcon", ground="window_persienne", tint=gris)
    for side, along in (("south", -1.5), ("south", 1.5), ("east", 1.5)):
        dormer(t, shape, "slate", side, along, 0.25, stone, EVERY_TIER)
    stack(t, "chimney_render", -2.0, 2.0, shape.ridge)
    fallen(t, "corner_shop_3f", "limestone", "slate", stone)


DESIGNS = {"china": china, "new_york": new_york, "paris": paris}
DESIGNS[FAMILY]()
kit.write(next(iter(ARGS), None))  # an argument after the family writes the two files somewhere else
