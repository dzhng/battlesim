"""Our own farmsteads: the farms at a town's edge, one regional family to a kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/farmsteads.py [family] [out]

writes one family's set (the format is in this folder's readme): `china`, the default,
to `assets/source/city/farmsteads/`, and `new_york` or `paris` to
`assets/source/city/farmsteads_<family>/`; `out` writes the two files somewhere else.
A farmstead is several buildings in one template: a house, a barn and perhaps a shed,
each a part of its own, standing apart. The ground between them is open: units walk
through the yard, and nothing is drawn there. Each family has three farms, at the same
sizes and floors as the others' (`china`, `new_york` and `paris` at the foot of this
file; the `Farm` builder is theirs in common):

  yard    China: a plastered house, a timber-framed barn with wagon doors and an
          open-fronted cart shed, round a yard. New York: a white clapboard
          farmhouse with a porch, a red gambrel barn and a silo. Paris: a stone
          corps de ferme, its high-roofed barn and a dovecote tower.
  long    China: a low whitewashed longhouse with a long stone byre behind it. New
          York: a frame house and a long red dairy barn. Paris: a longère with
          lucarnes and a stone barn under brown tiles.
  small   China: a brick house beside a tarred, boarded barn. New York: a frame house
          and a red barn under a tin roof. Paris: a rendered house and a
          half-timbered barn under a steep roof.

The physical box is the authority. Every outer wall stands on a face of a part,
and a part's top is its roof's ridge. Doors and windows sit in the bays of their
edge's 3 m lattice, on the floor datums, where a garrison's soldiers stand; a
barn keeps its small windows wherever a declared fighting bay has no door
or open front. What stands in a yard (a woodpile, a trough, a rain barrel, straw) hugs a wall, inside the set's
side fit; the cart shed keeps its cart inside its own box.

A farm's walls and roofs are one module, its shell, in the template's frame.
Fittings are modelled once and placed by rows at the two fine tiers. At the two
coarse ones a farm is one row: what is left of a fitting (a window's dark pane,
a door's leaf, a chimney's stack) is folded into the shell (`Farm.place`).
"""
import math
import os
import random
import sys
import zlib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from kit import *  # noqa: E402,F403
from masonry import *  # noqa: E402,F403
from regional import *  # noqa: E402,F403

FAMILY, SET, SOURCE, OUT = family_set("farmsteads", "farmsteads.py")
WALL_M = 2.85  # from a house's top floor datum up to its eaves
BYRE_SILL_M, LOFT_SILL_M = 1.3, 1.3  # small barn windows span the physical eye and muzzle heights
ROW_TIERS, FOLDED_TIER, FOLDED = TIERS_0_TO_1, 2, (2, 3)  # fittings are rows near; the shell keeps them at the two far tiers

kit = Kit(SET, SOURCE, fit_side_m=0.6, fit_top_m=0.9, fit_ruin_top_m=0.6)


def linear(c):
    """An sRGB byte as linear light."""
    c /= 255
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def in_colour(recipe, tint):
    """The linear colour a recipe's mean takes under an sRGB tint."""
    return tuple(m * linear(c) for m, c in zip(textures.baked(recipe).mean(), tint))


# ---------------------------------------------------------------- materials
# Tints, sRGB. Plasters, stone, boards, paints.
CREAM, STRAW, WHITEWASH = (233, 221, 196), (226, 204, 160), (240, 238, 230)
WARM_STONE, TARRED, WEATHERED = (206, 192, 168), (96, 78, 64), (158, 148, 132)
GREEN, BLUE, OXBLOOD, UMBER, IVORY = (66, 96, 76), (84, 104, 128), (122, 58, 48), (74, 60, 48), (226, 222, 208)

# A farm is one row, so each building's wall carries its own colour. The walls stay
# tint-masked: a row's tint would shift the whole farm.
WALLS = {
    "cream plaster": textured("wall_plaster_cream", "plaster", tint=1.0, colour=in_colour("plaster", CREAM), dirt=0.6, chip=0.5,
                              streak=0.35, rise=0.9),
    "straw plaster": textured("wall_plaster_straw", "plaster", tint=1.0, colour=in_colour("plaster", STRAW), dirt=0.6, chip=0.6,
                              streak=0.35, rise=0.9, seed=2.0),
    "whitewash": textured("wall_roughcast_white", "roughcast", tint=1.0, colour=in_colour("roughcast", WHITEWASH), dirt=0.6,
                          chip=0.3, streak=0.4, rise=0.9),
    "brick": textured("wall_brick", "brick", tint=1.0, dirt=0.45, chip=0.12, streak=0.25, rise=0.7),
    "stone": textured("wall_stone", "rubble_stone", tint=1.0, colour=in_colour("rubble_stone", WARM_STONE), dirt=0.5, chip=0.0,
                      streak=0.3, rise=0.9, lichen=0.2),
    "tarred boards": textured("wall_boards_tarred", "joinery", tint=1.0, colour=in_colour("joinery", TARRED), dirt=0.6, chip=0.6,
                              streak=0.35, rise=0.8),
    "weathered boards": textured("wall_boards_weathered", "joinery", tint=1.0, colour=in_colour("joinery", WEATHERED), dirt=0.6,
                                 chip=0.6, streak=0.35, rise=0.8, seed=4.0),
}
# A roof's recipe carries what is smaller than a tile; its moss and stains are its own, in soft patches (`weathered_roof`).
ROOFING = dict(dirt=0.0, chip=0.0, streak=0.0, grain=0.0)
ROOFS = {
    "clay": weathered_roof(textured("roof_clay", "roof_tile", **ROOFING), seed=1.0, moss=0.3),
    "brown": weathered_roof(textured("roof_brown", "roof_tile", colour=(0.12, 0.078, 0.058), seed=3.0, **ROOFING), seed=3.0, moss=0.3),
    "slate": weathered_roof(textured("roof_slate", "roof_slate", colour=(0.082, 0.086, 0.092), seed=6.0, **ROOFING), seed=6.0, moss=0.2),
    "stone": weathered_roof(textured("roof_stone", "roof_slate", colour=(0.13, 0.122, 0.105), seed=9.0, **ROOFING), seed=9.0, moss=0.45),
}
joinery_m = textured("joinery", "joinery", tint=1.0, dirt=0.3, chip=0.6, streak=0.0)
timber_m = textured("timber", "pallet_wood", colour=(0.05, 0.036, 0.026), dirt=0.3, chip=0.25, streak=0.2)
cart_m = textured("cart_wood", "pallet_wood", colour=(0.13, 0.1, 0.07), dirt=0.6, chip=0.4, streak=0.0)
bark_m = textured("log_bark", "pallet_wood", colour=(0.075, 0.057, 0.04), dirt=0.3, chip=0.0, streak=0.0)
plinth_m = textured("plinth_concrete", "concrete", colour=(0.2, 0.2, 0.19), dirt=0.8, chip=0.4, rise=0.5)
brick_plinth_m = textured("plinth_brick", "brick", colour=(0.14, 0.07, 0.05), dirt=0.35, chip=0.0, rise=0.6)
trough_m = textured("trough_stone", "field_stone", dirt=0.6, chip=0.4, lichen=0.3)
sill_m = textured("trim_concrete", "concrete", dirt=0.0, chip=0.3, streak=0.2)
coping_m = textured("coping_concrete", "concrete", colour=(0.13, 0.13, 0.125), dirt=0.0, chip=0.3, streak=0.2)
stack_m = textured("stack_brick", "brick", dirt=0.0, chip=0.2, streak=0.3, soot=0.6)
trim_m = flat_paint("roof_trim", (0.2, 0.18, 0.15), rough=0.8, grime=0.0)
frame_m = flat_paint("window_frame", (0.46, 0.45, 0.42), rough=0.6, grime=0.0)
glass_m, pane_m = window_glass(), window_pane()  # a window near, and the dark pane it is from far off
metal_m = flat_paint("gutter_metal", (0.07, 0.07, 0.07), rough=0.5, metal=0.6, grime=0.0)
iron_m = flat_paint("cart_iron", (0.05, 0.045, 0.04), rough=0.6, metal=0.7, grime=0.3)
pot_m = flat_paint("chimney_pot", (0.3, 0.13, 0.08), rough=0.8, grime=0.0)
straw_m = flat_paint("straw", (0.38, 0.29, 0.12), rough=0.95, grime=0.25)
cut_m = flat_paint("log_end", (0.3, 0.21, 0.11), rough=0.9, grime=0.0)
water_m = flat_paint("trough_water", (0.02, 0.028, 0.03), rough=0.1, grime=0.0)
# What a fallen farm building is made of, by its wall: a farm's ruin is one row, so each heap carries its own colour.
char_m = charred("charred_timber", ember=0.04)
rubble_m = textured("rubble_masonry", "rubble", tint=1.0, dirt=0.0, chip=0.0, streak=0.0, ash=0.25)  # the shared heaps': a row tints them


def rubble_of(name, colour, seed):
    return textured(f"rubble_{name}", "rubble", colour=colour, dirt=0.0, chip=0.0, streak=0.0, ash=0.25, seed=seed)


PLASTER_RUBBLE, ASHES = rubble_of("plaster", (0.17, 0.155, 0.13), 1.0), rubble_of("ashes", (0.045, 0.042, 0.04), 4.0)
RUBBLE = {"cream plaster": PLASTER_RUBBLE, "straw plaster": PLASTER_RUBBLE, "whitewash": rubble_of("whitewash", (0.2, 0.195, 0.185), 2.0),
          "brick": rubble_of("brick", (0.13, 0.068, 0.05), 3.0), "stone": rubble_of("stone", (0.15, 0.14, 0.12), 5.0),
          "tarred boards": ASHES, "weathered boards": ASHES}
# The shared heaps' tints (sRGB), and which walls are timber: they burn to the foot and leave less standing.
DUST = {"cream plaster": (214, 200, 176), "straw plaster": (208, 190, 150), "whitewash": (224, 222, 214), "brick": (176, 104, 82),
        "stone": (190, 178, 156), "tarred boards": (70, 66, 62), "weathered boards": (70, 66, 62)}
# A design extends these tables (and FAR_PANELS, FAR_BOXES, STACKS) with its own family's entries; a run builds one family, so none leaks into another.
TIMBER = {"tarred boards", "weathered boards"}
# The chimneys a fallen building keeps the stumps of: module -> its plan and material.
STACKS = {"chimney_brick": ((1.05, 0.6), stack_m)}
# What breaks a wall on the ground floor, for a ruin's stumps: fitting -> its opening's width.
OPENINGS = {"window_a": 1.1, "window_a_shutters": 1.1, "window_byre": 0.9, "window_shed": 0.9, "door_panel": 1.15, "barn_door": 2.9, "stable_door": 1.25}


def far_paint(tint):
    """A door's paint as a far tier keeps it: the joinery's own colour under a row's tint."""
    return flat_paint("far_paint_%d_%d_%d" % tint, in_colour("joinery", tint), rough=0.7, grime=0.0)


# ---------------------------------------------------------------- fittings
def window_module(name, w, h, shutters=False, lights=2, frame=frame_m, sill=sill_m, **behind):
    casement(kit, name, w, h, frame, glass_m, pane_m, sill, joinery_m if shutters else None, lights, **behind)  # (far off it is the shell's: `FAR_PANELS`)


# A house's windows have its rooms behind them. A barn's is one pane in a dark timber frame, over the dark of
# the barn: nobody lives behind it. The cart shed's stands in boards seen from both sides, and stays a dark pane.
# A barn's doors: the wagon doors, a split stable door, and the loft's doors under their hoist.
BARN_DOOR_M, STABLE_DOOR_M, LOFT_DOOR_M = (2.7, 3.2), (1.15, 2.1), (1.3, 1.35)


def barn_door_modules(leaf=joinery_m, frame=timber_m):
    m = kit.module("barn_door", ground=True, **FITTING)
    m.opening = (BARN_DOOR_M[0], BARN_DOOR_M[1], 0.0)
    barn_doors(m.n("doors"), *BARN_DOOR_M, leaf, frame, m.root)
    stable_door_module(leaf, frame)
    m = kit.module("loft_door", **FITTING)
    m.opening = (LOFT_DOOR_M[0], LOFT_DOOR_M[1], 0.0)
    barn_doors(m.n("doors"), *LOFT_DOOR_M, leaf, frame, m.root, hoist=0.55)


def stable_door_module(leaf=joinery_m, frame=timber_m):
    m = kit.module("stable_door", ground=True, **FITTING)
    m.opening = (STABLE_DOOR_M[0], STABLE_DOOR_M[1], 0.0)
    stable_door(m.n("door"), *STABLE_DOOR_M, leaf, frame, m.root)


# What stands about a farm, each against a wall or under a roof.
BALES = dict(across=3, high=3)


def prop_module(name):
    m = kit.module(name, ground=True, **FITTING)
    if name == "cart":
        farm_cart(m.n("cart"), cart_m, iron_m, m.root)
    elif name == "bales":
        bale_stack(m.n("bales"), straw_m, m.root, **BALES)
    elif name == "woodpile":
        woodpile(m.n("pile"), 1.6, 1.1, bark_m, cut_m, m.root)
    elif name == "trough":
        stone_trough(m.n("trough"), trough_m, water_m, m.root)
    else:
        water_butt(m.n("butt"), cart_m, iron_m, water_m, m.root)


# What a far tier keeps of a fitting, in the fitting's own frame: panels on its wall as
# (width, height, foot, paint), where the paint is a material or "tint" for the row's own;
# and boxes as (size, centre, material).
STABLE_LEAF_M = STABLE_DOOR_M[1] * 0.55
FAR_PANELS = {
    "window_a": window_far(1.0, 1.3, pane_m, sill_m),
    "window_a_shutters": window_far(1.0, 1.3, pane_m, sill_m, "tint"),
    "window_byre": [(0.8, 0.6, 0.0, pane_m)],
    "window_shed": [(0.8, 0.6, 0.0, pane_m)],
    "door_panel": [(0.95, 2.05, 0.0, "tint")],
    "barn_door": [(*BARN_DOOR_M, 0.0, "tint")],
    "stable_door": [(STABLE_DOOR_M[0], STABLE_LEAF_M, 0.0, "tint"),
                    (STABLE_DOOR_M[0], STABLE_DOOR_M[1] - STABLE_LEAF_M, STABLE_LEAF_M, bpy_dark())],
    "loft_door": [(*LOFT_DOOR_M, 0.0, "tint")],
}
FAR_BOXES = {
    "chimney_brick": [((1.05, 0.6, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), coping_m)],  # from above a chimney is its cap
    "bales": [((2.85, 0.48, 1.0), (0, -0.27, 0.5), straw_m)],
}


# ---------------------------------------------------------------- a farm
class Farm:
    """One farmstead: its template and its shell, the one module that holds every
    building's walls and roof in the template's frame. It stands in for the template
    where rows are placed (`glaze`, `front_door` and the rest take it), so that a
    fitting asked for at the far tiers is folded into the shell instead."""

    def __init__(self, id_, tag, recipe):
        self.t = kit.template(id_, "farmstead", FAMILY, recipe)
        self.m = kit.module(f"{tag}_shell", ground=True, paint_scale=4.0)
        self.far, self.built, self.framed = [], {}, set()  # (`framed`: parts whose walls are timber-framed)
        self.round = {}  # part -> (x, y, radius): a round building in its box (a silo), which falls as a round stump

    def __getattr__(self, name):  # the template's own: parts, floors, lattices, bays, entrances
        return getattr(self.t, name)

    def mount(self, module, edge, offset_m, z=0.0, out=0.0, **row):
        self.place(module, *self.t.at(edge, offset_m, z, out), **row)

    def place(self, module, x=0.0, y=0.0, z=0.0, yaw=0.0, tiers=EVERY_TIER, tint=WHITE, **row):
        if tiers & ROW_TIERS:
            self.t.place(module, x, y, z, yaw, tiers=tiers & ROW_TIERS, tint=tint, **row)
        if tiers >> FOLDED_TIER & 1:
            self.far.append((module, x, y, z, yaw, *row.get("scale", (1.0, 1.0, 1.0)), tiers, *tint))

    def building(self, part, centre, size, eave, rise, along, hips, wall, roof, plinth=(plinth_m, 0.4), broken=None, porch=None, trim=trim_m):
        """A building under a pitched roof: its part, and its walls, roof and plinth in the shell. Its roof's shape.
        `broken` (knee, run) makes the roof a gambrel (`BrokenRoof`); `porch` sets the ground floor back
        behind an inset porch on the south (`porch_shell`); `trim` is the eaves' and verges' boards."""
        x0, x1, y0, y1 = centre[0] - size[0] / 2, centre[0] + size[0] / 2, centre[1] - size[1] / 2, centre[1] + size[1] / 2
        if broken:
            shape = BrokenRoof(x0, x1, y0, y1, eave, broken[0], eave + rise, broken[1], along, False)
        else:
            shape = RoofShape(x0, x1, y0, y1, eave, eave + rise, along, hips, OVER_M, (VERGE_M, VERGE_M))
        if porch:
            porch_shell(self.m, part, shape, porch["depth"], porch["ceiling"], WALLS[wall], ROOFS[roof], trim, porch["post"], porch["deck"],
                        plinth_m, porch["posts"])
        else:
            house_shell(self.m.n(part), shape, WALLS[wall], ROOFS[roof], trim, self.m.root, plinth=(x0, x1, y0, y1) if plinth else None,
                        plinth_mat=plinth and plinth[0], plinth_m=plinth and plinth[1])
        self.t.part(part, x0, x1, y0, y1, shape.ridge)
        self.built[part] = (wall, roof)
        return shape

    def bays_of(self, part, **lattice):
        for side, bay_at in lattice.items():
            self.t.lattice(f"{part}-{side}", bay_at)

    def gutters(self, part, shape):
        """Gutters under every level eave of a house: the two long sides, and a full hip's end."""
        eaves, ends = (("west", "east"), ("south", "north")) if shape.swap else (("south", "north"), ("west", "east"))
        for side in eaves:
            rainwater(self, f"{part}-{side}", shape, shape.ends[1] - shape.ends[0])
        for k, side in enumerate(ends):
            if shape.hips[k] >= 1:
                rainwater(self, f"{part}-{side}", shape, 2 * shape.reach, pipes=())

    def butt(self, edge, end):
        """A rain barrel under the downpipe at one end (-1 or 1) of an eave."""
        a, b = self.t.edges()[edge]["span"]
        self.mount("water_butt", edge, (a + b) / 2 + end * ((b - a) / 2 - 0.3), out=0.3, tiers=ROW_TIERS)

    def hang(self, edge, floor, tint=WHITE, sill=None, **at):
        """A barn's fittings in bays of `edge`: `at` is module -> its bays' offsets. A door stands on
        the ground; a window, or the loft's doors, on its sill."""
        window_sill = sill if sill is not None else LOFT_SILL_M if floor else BYRE_SILL_M
        for module, offsets in sorted(at.items()):
            sill = 0.0 if module in ("barn_door", "stable_door") else window_sill
            for o in offsets:
                if not any(abs(o - bay) < 1e-6 for bay in self.t.bays(edge)):
                    raise SystemExit(f"{self.t.id}: {module} at {o} is not in a bay of {edge}")
                self.mount(module, edge, o, z=floor + sill, tiers=TIERS_0_TO_2, tint=tint)

    def wagon_entrance(self, edge, offset, tint):
        self.t.entrance(edge, offset)
        self.hang(edge, 0.0, tint, barn_door=(offset,))

    def done(self, open_sides=None):
        """Fold the far panels into the shell and stand it on the template; then its ruin: one
        module holding every building's stumps and heap in its own materials, with the set's
        wreckage lying on it. `open_sides` names sides that never had a wall (part -> sides)."""
        self.t.open_sides = open_sides or {}
        for edge, offset, floor in self.t.uncovered_bays():
            self.mount("window_byre", edge, offset, z=floor + BYRE_SILL_M, tiers=TIERS_0_TO_2)
        fold_far(self.m, self.far, FAR_PANELS, FAR_BOXES, far_paint, FOLDED)
        self.t.place(self.m.name)
        open_walls(self.m, self.t.rows["intact"], kit.openings)  # near, a window is an opening, a house's with its room behind
        furnish(self.t, kit)
        t = self.t
        m = kit.module(self.m.name.replace("_shell", "_ruin"), ground=True, **RUIN)
        high, over, seed = t.ruin_height(), kit.fit["ruin_top_m"], zlib.crc32(t.id.encode()) % 997
        for k, p in enumerate(t.parts):
            wall, roof = self.built[p["id"]]
            if p["id"] in self.round:
                cx, cy, r = self.round[p["id"]]
                tall = 0.8 * high + 0.3
                cyl(m.n(f"{p['id']}_stump"), r, tall, (cx, cy, tall / 2), "Z", scorched(WALLS[wall], high + over, seed + 17 * k), m.root, seg=20)
                rubble(m.n(f"{p['id']}_heap"), (cx, cy), r - 0.4, 0.9 * high, 10, RUBBLE[wall], seed + 17 * k, m.root, seg=(12, 8, 5, 4),
                       rings=(3, 2, 1, 1))
                for o in m.meshes():  # a tumbled block rests on the ground, not in it
                    if o.name.startswith(m.n(f"{p['id']}_heap")):
                        for v in o.data.vertices:
                            v.co.z = max(v.co.z, 0.0)
                continue
            timber = wall in TIMBER
            rect = (p["x0"], p["x1"], p["y0"], p["y1"])
            sides = {side: v for side, v in ruin_sides(t, p["id"], OPENINGS).items() if side not in (open_sides or {}).get(p["id"], ())}
            stacks = [(x, y, *STACKS[module]) for module, x, y, *_ in t.rows["intact"]
                      if module in STACKS and rect[0] < x < rect[1] and rect[2] < y < rect[3]]
            # a boarded wall burns to the foot and stands in lengths of board between its posts, not in courses
            boarded = dict(level=0.8, charred=0.7, thick=0.12, run=(1.2, 2.6), jagged=2.5) if timber else {}
            ruin = ruin_block(m, p["id"], rect, high, over, sides, WALLS[wall], ROOFS[roof], RUBBLE[wall], seed + 17 * k, stacks=stacks,
                              coarse=18, **boarded)
            if timber or p["id"] in self.framed:  # the frame's posts stand charred when the walls between them are gone
                rng = random.Random(seed + 91 * k)
                crest = ruin.height + over - 0.06
                for side in sorted(sides):
                    a0, a1, fixed, along_x = wall_line(rect, side, 0.1)
                    for i in range(int((a1 - a0) // 3) + 1):
                        at, tall = min(max(a0 + 3.0 * i, a0 + 0.1), a1 - 0.1), rng.uniform(0.5, 1.0) * crest
                        box(m.n(f"{p['id']}_post_{side}_{i}"), (0.2, 0.2, tall), (at, fixed, tall / 2) if along_x else (fixed, at, tall / 2),
                            char_m, m.root, lods=(0, 1))
            litter(t, ruin, DUST[wall], beams=8 if timber or p["id"] in self.framed else 4, heaps=4, level=1.0)
        t.place(m.name, state="ruin")


# ---------------------------------------------------------------- China
def china():
    window_module("window_a", 1.0, 1.3)
    window_module("window_a_shutters", 1.0, 1.3, shutters=True)
    window_module("window_byre", 0.8, 0.6, lights=1, frame=timber_m, sill=timber_m, behind=None, dark=bpy_dark())
    window_module("window_shed", 0.8, 0.6, lights=1, frame=timber_m, sill=timber_m, cut=False)
    door_module(kit, "door_panel", 0.95, 2.05, joinery_m, frame_m)
    build_step(kit, plinth_m)
    build_chimney(kit, "chimney_brick", (1.05, 0.6), stack_m, coping_m, pot_m, 2)
    build_rainwater(kit, metal_m)
    barn_door_modules()
    wreckage(kit, rubble_m, char_m)
    for name in ("cart", "bales", "woodpile", "trough", "water_butt"):
        prop_module(name)

    # ------------------------------------------------------------ the yard farm
    # A plastered two-storey house on the street, a timber-framed barn gable-on beside it,
    # and an open-fronted cart shed across the yard behind the house.
    farm = Farm("china-farmstead-yard", "farm_yard",
                dict(Layout="yard", Floors=2, House="14x9 plaster, clay gable", Barn="12x22 timber frame, brown gable",
                     Shed="9x6 open-fronted, boarded, slate lean-to"))
    house = farm.building("house", (-10, -8), (14, 9), 3.0 + WALL_M, 2.5, "x", (0.0, 0.0), "cream plaster", "clay")
    barn = farm.building("barn", (11, 0), (12, 22), 5.2, 3.2, "y", (0.0, 0.0), "straw plaster", "brown", plinth=(brick_plinth_m, 0.6))

    # The cart shed: boarded walls behind and at the ends, three open bays between posts in
    # front, under one slope that falls to the back. Its box stays under the farm's upper
    # floor datum: the simulation seats a garrison on every datum a part reaches.
    (SX, SY), LOW, HIGH, THICK = (-11, 9), 2.1, 2.9, 0.22
    shed = LeanToShape(SX - 4.5, SX + 4.5, SY - 3, SY + 3, LOW, HIGH, rises="south", over=0.3, over_top=0.25, verge=(0.25, 0.25))
    m, boards_m = farm.m, WALLS["weathered boards"]
    box(m.n("shed_back"), (9, THICK, LOW - 0.03), (SX, SY + 3 - THICK / 2, (LOW - 0.03) / 2), boards_m, m.root)
    for s in (-1, 1):
        tag = "ab"[s > 0]
        prism(m.n(f"shed_end_{tag}"), [(SY - 3, 0), (SY + 3 - THICK, 0), (SY + 3 - THICK, LOW + THICK * shed.tan - 0.03), (SY - 3, HIGH - 0.03)],
              THICK, (SX + s * (4.5 - THICK / 2), 0, 0), boards_m, m.root, rot=(0, 0, math.pi / 2))
        box(m.n(f"shed_post_{tag}"), (0.2, 0.2, HIGH - 0.3), (SX + s * 1.5, SY - 2.9, (HIGH - 0.3) / 2), timber_m, m.root, lods=(0, 1, 2))
        for k in (-1, 1):  # a brace from each post up to the beam
            box(m.n(f"shed_brace_{tag}_{'ab'[k > 0]}"), (0.9, 0.12, 0.12), (SX + s * 1.5 + k * 0.38, SY - 2.9, HIGH - 0.72), timber_m, m.root,
                rot=(0, -k * math.pi / 4, 0), lods=(0, 1))
    box(m.n("shed_beam"), (9 - 2 * THICK, 0.2, 0.24), (SX, SY - 2.9, HIGH - 0.2), timber_m, m.root, lods=(0, 1, 2))
    pitched_roof(m.n("shed_roof"), shed, ROOFS["slate"], trim_m, m.root)
    farm.part("shed", SX - 4.5, SX + 4.5, SY - 3, SY + 3, shed.top)
    farm.built["shed"] = ("weathered boards", "slate")
    farm.floors(0.0, 3.0)

    farm.bays_of("house", south=1.5, north=1.5, east=0.0, west=0.0)
    front_door(farm, "house-south", 0.0, "door_panel", GREEN)
    glaze(farm, "house-south", (0.0, 3.0), "window_a_shutters", tint=GREEN)
    for side in ("north", "east", "west"):
        glaze(farm, f"house-{side}", (0.0, 3.0), "window_a")
    farm.gutters("house", house)
    stack(farm, "chimney_brick", -14.2, -8.0, house.ridge)
    stack(farm, "chimney_brick", -5.8, -8.0, house.ridge)
    farm.mount("woodpile", "house-north", 3.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=0.0, west=0.0)
    half_door = BARN_DOOR_M[0] / 2 + 0.15
    framing = dict(posts=dict(a0=(6.5, 9.5, 12.5, 15.5), a1=(6.5, 9.5, 12.5, 15.5), b0=(-7.5, -4.5, -1.5, 1.5, 4.5, 7.5), b1=(-7.5, -4.5, -1.5, 1.5, 4.5, 7.5)),
                   openings=dict(a0=[(11 - half_door, 11 + half_door, 3.5)],
                                 b0=[(-half_door, half_door, 3.5), (-6.7, -5.3, 2.3), (5.3, 6.7, 2.3)]),
                   foot=0.6, rail=2.55)
    farm.framed.add("barn")
    fm = kit.module("farm_yard_barn_frame", ground=True, **FITTING)
    timber_frame(fm.n("frame"), barn, timber_m, fm.root, **framing)
    farm.t.place(fm.name, tiers=ROW_TIERS)
    timber_frame(m.n("barn_frame"), barn, timber_m, m.root, lods=(FOLDED_TIER,), **framing)
    farm.wagon_entrance("barn-south", 0.0, GREEN)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, GREEN, loft_door=(0.0,))
    farm.hang("barn-west", 0.0, GREEN, barn_door=(0.0,), stable_door=(-6.0, 6.0), window_byre=(-9.0, -3.0, 3.0, 9.0))
    farm.hang("barn-west", 3.0, window_byre=(-6.0, 6.0))
    farm.hang("barn-east", 0.0, window_byre=(-6.0, 0.0, 6.0))
    farm.hang("barn-east", 3.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-north", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-north", 3.0, window_byre=(0.0,))
    farm.mount("trough", "barn-west", 4.4, tiers=ROW_TIERS)

    farm.bays_of("shed", south=0.0, north=0.0, east=1.5, west=1.5)
    farm.hang("shed-north", 0.0, sill=1.05, window_shed=(-3.0, 0.0, 3.0))
    back = SY + 3 - THICK  # the inside of the back wall: straw and logs are stacked against it
    farm.place("cart", SX - 3.0, SY + 0.3, tiers=ROW_TIERS)
    farm.place("woodpile", SX, back, tiers=ROW_TIERS)
    farm.place("bales", SX + 3.0, back, tiers=TIERS_0_TO_2)
    farm.place("bales", SX + 3.0, back - 0.5, tiers=TIERS_0_TO_2)
    farm.done(open_sides={"shed": ("south",)})

    # ------------------------------------------------------------ the long farm
    # A low whitewashed longhouse under a half-hipped slate roof, and behind it, across a
    # narrow yard, a stone byre half as long again: stable doors, small windows, a wagon door at one end.
    farm = Farm("china-farmstead-long", "farm_long",
                dict(Layout="long", Floors=1, House="18x8 roughcast, slate half-hip", Barn="26x12 stone byre, stone-slate gable"))
    house = farm.building("house", (0, -9), (18, 8), WALL_M, 2.35, "x", (0.45, 0.45), "whitewash", "slate")
    barn = farm.building("barn", (0, 8), (26, 12), 3.8, 3.7, "x", (0.0, 0.0), "stone", "stone", plinth=None)
    farm.floors(0.0)

    farm.bays_of("house", south=0.0, north=0.0, east=1.5, west=1.5)
    front_door(farm, "house-south", 0.0, "door_panel", BLUE)
    glaze(farm, "house-south", (0.0,), "window_a_shutters", skip=(0.0,), tint=BLUE)
    for side in ("north", "east", "west"):
        glaze(farm, f"house-{side}", (0.0,), "window_a")
    farm.gutters("house", house)
    stack(farm, "chimney_brick", -5.0, -9.0, house.ridge)
    stack(farm, "chimney_brick", 2.0, -9.0, house.ridge)
    farm.mount("woodpile", "house-north", 1.5, tiers=ROW_TIERS)
    farm.butt("house-north", -1)

    farm.bays_of("barn", south=1.5, north=1.5, east=0.0, west=0.0)
    farm.wagon_entrance("barn-south", 10.5, UMBER)
    farm.hang("barn-south", 0.0, UMBER, stable_door=(-7.5, -1.5, 4.5), window_byre=(-10.5, -4.5, 1.5, 7.5))
    farm.hang("barn-north", 0.0, window_byre=(-10.5, -4.5, 4.5, 10.5))
    for side in ("east", "west"):
        farm.hang(f"barn-{side}", 0.0, window_byre=(-3.0, 3.0))
    farm.mount("trough", "barn-south", 2.9, tiers=ROW_TIERS)
    farm.mount("bales", "barn-east", 0.0, tiers=TIERS_0_TO_2)
    farm.done()

    # ------------------------------------------------------------ the small farm
    # A square brick house under a hipped roof, and beside it, a cart's width away, a tarred,
    # boarded barn on a brick plinth, its gable clipped by a half-hip over the loft doors.
    farm = Farm("china-farmstead-small", "farm_small",
                dict(Layout="small", Floors=2, House="11x8 brick, brown hip", Barn="10x14 boarded, clay half-hip"))
    house = farm.building("house", (-7.75, -3), (11, 8), 3.0 + WALL_M, 2.3, "x", (1.0, 1.0), "brick", "brown")
    barn = farm.building("barn", (8.25, 0), (10, 14), 5.2, 2.6, "y", (0.4, 0.4), "tarred boards", "clay", plinth=(brick_plinth_m, 0.7))
    farm.floors(0.0, 3.0)

    farm.bays_of("house", south=0.0, north=0.0, east=1.5, west=1.5)
    front_door(farm, "house-south", 0.0, "door_panel", IVORY)
    for side in ("south", "north", "east", "west"):
        glaze(farm, f"house-{side}", (0.0, 3.0), "window_a", skip=(0.0,) if side == "south" else ())
    farm.gutters("house", house)
    stack(farm, "chimney_brick", -6.75, -3.0, house.ridge)
    farm.mount("woodpile", "house-east", 0.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=1.5, west=1.5)
    farm.wagon_entrance("barn-south", 0.0, OXBLOOD)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, OXBLOOD, loft_door=(0.0,))
    farm.hang("barn-east", 0.0, OXBLOOD, stable_door=(-1.5,), window_byre=(-4.5, 1.5, 4.5))
    farm.hang("barn-east", 3.0, window_byre=(-1.5, 1.5))
    farm.hang("barn-west", 0.0, window_byre=(-4.5, 4.5))
    farm.hang("barn-west", 3.0, window_byre=(-1.5, 1.5))
    farm.hang("barn-north", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-north", 3.0, window_byre=(0.0,))
    farm.mount("trough", "barn-east", 3.0, tiers=ROW_TIERS)
    farm.mount("bales", "barn-west", 0.0, tiers=TIERS_0_TO_2)
    farm.done()


# ---------------------------------------------------------------- New York
def new_york():
    """American farms: white and yellow clapboard farmhouses with porches set into them, red board barns
    under gambrel and gable roofs of galvanised sheet, white trim and X-braced doors, and a silo."""
    white, yellow, barn_red = (240, 238, 230), (232, 214, 160), (150, 42, 34)
    navy, black_green, black = (40, 50, 72), (38, 52, 44), (34, 34, 34)
    WALLS.update({
        "white clapboard": textured("wall_clapboard_white", "clapboard", tint=1.0, colour=in_colour("clapboard", white), dirt=0.55, chip=0.6,
                                    streak=0.3, rise=0.8),
        "yellow clapboard": textured("wall_clapboard_yellow", "clapboard", tint=1.0, colour=in_colour("clapboard", yellow), dirt=0.55,
                                     chip=0.6, streak=0.3, rise=0.8, seed=2.0),
        "red boards": textured("wall_boards_red", "joinery", tint=1.0, colour=in_colour("joinery", barn_red), dirt=0.6, chip=0.6, streak=0.35,
                               rise=0.8, seed=5.0),
        "concrete stave": textured("wall_silo", "concrete", colour=(0.36, 0.35, 0.33), dirt=0.5, chip=0.2, streak=0.45, rise=1.0),
    })
    ROOFS.update({
        "shingle": weathered_roof(textured("roof_shingle", "roof_slate", colour=(0.042, 0.043, 0.045), seed=12.0, **ROOFING), seed=12.0,
                                  moss=0.25),
        "galvanised": weathered_roof(textured("roof_galvanised", "roof_sheet", colour=(0.15, 0.155, 0.16), seed=13.0, **ROOFING), seed=13.0,
                                     moss=0.1),
    })
    RUBBLE.update({"white clapboard": ASHES, "yellow clapboard": ASHES, "red boards": ASHES,
                   "concrete stave": rubble_of("concrete", (0.17, 0.165, 0.155), 6.0)})
    DUST.update({"white clapboard": (70, 66, 62), "yellow clapboard": (70, 66, 62), "red boards": (70, 66, 62), "concrete stave": (196, 192, 182)})
    TIMBER.update(("white clapboard", "yellow clapboard", "red boards"))
    white_m = flat_paint("trim_white", (0.6, 0.6, 0.58), rough=0.7, grime=0.0)
    deck_m = textured("porch_deck", "joinery", colour=(0.17, 0.16, 0.15), dirt=0.4, chip=0.3, streak=0.0)

    sash(kit, "window_sash", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m)
    sash(kit, "window_sash_shutters", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m, shutter=joinery_m, panes=(3, 2))
    sash(kit, "window_sash_porch", 0.9, 1.6, white_m, glass_m, pane_m, white_m, casing=white_m, behind=None, dark=bpy_dark())
    window_module("window_byre", 0.8, 0.6, lights=1, frame=white_m, sill=white_m, behind=None, dark=bpy_dark())
    door_module(kit, "door_transom", 0.95, 2.05, joinery_m, white_m, pane_m, light=0.4)
    build_step(kit, plinth_m)
    porch_rail(kit, "porch_rail", PORCH_RAIL_M, white_m)
    porch_steps(kit, "porch_steps", 1.6, deck_m)
    build_chimney(kit, "chimney_brick", (1.05, 0.6), stack_m, coping_m, pot_m, 2)
    build_rainwater(kit, metal_m)
    x_barn_doors(kit, "barn_door", *BARN_DOOR_M, joinery_m, white_m, white_m)
    stable_door_module(joinery_m, white_m)
    x_barn_doors(kit, "loft_door", *LOFT_DOOR_M, joinery_m, white_m, white_m, hoist=0.55, ground=False)
    m = kit.module("silo_hatch", **FITTING)  # one of the silo's doors, up its chute
    m.opening = (0.7, 0.9, 0.0)
    box(m.n("frame"), (0.9, 0.05, 1.1), (0, -0.025, 0.45), white_m, m.root, lods=(0, 1))
    box(m.n("door"), (0.7, 0.04, 0.9), (0, -0.06, 0.45), bpy_dark(), m.root)
    m = kit.module("cupola", **FITTING)  # a ventilator on a dairy barn's ridge
    box(m.n("base"), (0.9, 0.9, 0.6), (0, 0, 0.3), white_m, m.root)
    cyl(m.n("roof"), 0.78, 0.4, (0, 0, 0.8), "Z", ROOFS["galvanised"], m.root, seg=4, r2=0.0, rot=(0, 0, math.pi / 4), min_seg=4)
    wreckage(kit, rubble_m, char_m)
    for name in ("bales", "woodpile", "water_butt"):
        prop_module(name)
    FAR_PANELS.update({
        "window_sash": window_far(0.9, 1.6, pane_m, white_m), "window_sash_porch": window_far(0.9, 1.6, pane_m, white_m),
        "window_sash_shutters": window_far(0.9, 1.6, pane_m, white_m, "tint"),
        "window_byre": [(0.96, 0.76, -0.08, white_m, 0.012), (0.8, 0.6, 0.0, pane_m)],
        "door_transom": [(1.19, 2.62, 0.0, white_m, 0.012), (0.95, 2.05, 0.0, "tint"), (0.95, 0.4, 2.11, pane_m)],
        "porch_rail": [(PORCH_RAIL_M, 0.7, PORCH_DECK_M, white_m)],
        "barn_door": [(BARN_DOOR_M[0] + 0.1, BARN_DOOR_M[1] + 0.1, 0.0, white_m, 0.012), (BARN_DOOR_M[0] - 0.3, BARN_DOOR_M[1] - 0.3, 0.15, "tint")],
        "loft_door": [(LOFT_DOOR_M[0] + 0.1, LOFT_DOOR_M[1] + 0.1, 0.0, white_m, 0.012), (LOFT_DOOR_M[0] - 0.3, LOFT_DOOR_M[1] - 0.3, 0.15, "tint")],
        "silo_hatch": [(0.9, 1.1, -0.1, white_m, 0.012), (0.7, 0.9, 0.0, bpy_dark())],
    })
    FAR_BOXES.update({"cupola": [((0.9, 0.9, 0.6), (0, 0, 0.3), white_m), ((1.1, 1.1, 0.2), (0, 0, 0.7), ROOFS["galvanised"])],
                      "chimney_brick": [((1.05, 0.6, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), stack_m)]})  # from the oblique camera, its brick

    def corner_boards(farm, part, rect, top, foot=0.5):
        """White boards up a timber building's corners."""
        x0, x1, y0, y1 = rect
        for k, (x, y) in enumerate(((x0, y0), (x1, y0), (x1, y1), (x0, y1))):
            box(farm.m.n(f"{part}_corner_{k}"), (0.24, 0.24, top - foot), (x, y, (top + foot) / 2), white_m, farm.m.root, lods=(0, 1, 2))

    porch = dict(ceiling=2.9, post=white_m, deck=deck_m)

    # ------------------------------------------------------------ the yard farm
    # A white clapboard farmhouse with its porch, a red gambrel barn gable-on to the yard, and a silo beside it.
    farm = Farm("nyc-farmstead-yard", "farm_yard",
                dict(Layout="yard", Floors=2, House="12x8 white clapboard, shingle gable, porch", Barn="12x22 red boards, galvanised gambrel",
                     Silo="4x4 concrete stave silo, galvanised dome"))
    house = farm.building("house", (-10, -8), (12, 8), 3.0 + WALL_M, 2.8, "x", (0.0, 0.0), "white clapboard", "shingle", trim=white_m,
                          porch=porch | dict(depth=2.4, posts=(-11.5, -8.5)))
    barn = farm.building("barn", (8, 0), (12, 22), 5.2, 4.6, "y", (0.0, 0.0), "red boards", "galvanised", plinth=(plinth_m, 0.5),
                         broken=(2.6, 1.6), trim=white_m)
    corner_boards(farm, "barn", (2, 14, -11, 11), 5.2)
    sx, sy, tall = 17.5, -7.0, 12.0
    m = farm.m
    cyl(m.n("silo_staves"), 1.98, tall, (sx, sy, tall / 2), "Z", WALLS["concrete stave"], m.root, seg=20, caps=False)
    cyl(m.n("silo_dome"), 2.02, 1.1, (sx, sy, tall + 0.55), "Z", ROOFS["galvanised"], m.root, seg=20, r2=0.3)
    cyl(m.n("silo_vent"), 0.3, 0.18, (sx, sy, tall + 1.19), "Z", ROOFS["galvanised"], m.root, seg=8, r2=0.06, lods=(0, 1, 2))
    farm.part("silo", sx - 2, sx + 2, sy - 2, sy + 2, tall + 1.3)
    farm.built["silo"] = ("concrete stave", "galvanised")
    farm.round["silo"] = (sx, sy, 1.98)
    farm.floors(0.0, 3.0)

    farm.bays_of("house", south=0.0, north=0.0, east=0.0, west=0.0)
    porch_front(farm, "house-south", 2.4, 0.0, (-3.0, 3.0), "door_transom", navy, windows=(-3.0, 3.0))
    glaze(farm, "house-south", (3.0,), "window_sash_shutters", tint=black)
    porch_side(farm, "house-east", -3.0, "window_sash", (0.0, 3.0))
    porch_side(farm, "house-west", 3.0, "window_sash", (0.0, 3.0))
    glaze(farm, "house-north", (0.0, 3.0), "window_sash")
    farm.gutters("house", house)
    stack(farm, "chimney_brick", -14.0, -8.0, house.ridge)
    farm.mount("woodpile", "house-north", 3.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=0.0, west=0.0)
    farm.wagon_entrance("barn-south", 0.0, barn_red)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, barn_red, loft_door=(0.0,))
    farm.hang("barn-west", 0.0, barn_red, barn_door=(0.0,), stable_door=(-6.0, 6.0), window_byre=(-9.0, -3.0, 3.0, 9.0))
    farm.hang("barn-west", 3.0, window_byre=(-6.0, 6.0))
    farm.hang("barn-east", 0.0, window_byre=(-6.0, 0.0, 6.0))
    farm.hang("barn-north", 3.0, barn_red, loft_door=(0.0,))
    farm.mount("bales", "barn-east", 3.0, tiers=TIERS_0_TO_2)

    farm.bays_of("silo", south=0.0, north=0.0, east=0.0, west=0.0)
    for side in ("south", "north", "east", "west"):
        for floor in (0.0, 3.0):
            farm.mount("silo_hatch", f"silo-{side}", 0.0, z=floor + 1.0, tiers=TIERS_0_TO_2)
    farm.done()

    # ------------------------------------------------------------ the long farm
    # A one-storey yellow farmhouse with a porch along its front, and behind it a long red dairy barn
    # under a gambrel, ventilators on its ridge.
    farm = Farm("nyc-farmstead-long", "farm_long",
                dict(Layout="long", Floors=1, House="14x8 yellow clapboard, shingle gable, porch",
                     Barn="26x12 red boards dairy barn, galvanised gambrel, cupolas"))
    house = farm.building("house", (0, -9), (14, 8), WALL_M, 2.6, "x", (0.0, 0.0), "yellow clapboard", "shingle", trim=white_m,
                          porch=porch | dict(depth=2.4, ceiling=2.6, posts=(-3.0, 0.0, 3.0)))
    barn = farm.building("barn", (0, 8), (26, 12), 3.8, 4.4, "x", (0.0, 0.0), "red boards", "galvanised", plinth=(plinth_m, 0.5),
                         broken=(2.4, 1.5), trim=white_m)
    corner_boards(farm, "barn", (-13, 13, 2, 14), 3.8)
    farm.floors(0.0)

    farm.bays_of("house", south=1.5, north=1.5, east=0.0, west=0.0)
    porch_front(farm, "house-south", 2.4, -1.5, (-4.5, 1.5, 4.5), "door_transom", black_green, windows=(-4.5, 1.5, 4.5))
    porch_side(farm, "house-east", -3.0, "window_sash", (0.0,))
    porch_side(farm, "house-west", 3.0, "window_sash", (0.0,))
    glaze(farm, "house-north", (0.0,), "window_sash")
    farm.gutters("house", house)
    stack(farm, "chimney_brick", 4.0, -9.0, house.ridge)
    farm.butt("house-north", -1)

    farm.bays_of("barn", south=1.5, north=1.5, east=0.0, west=0.0)
    farm.wagon_entrance("barn-south", 10.5, barn_red)
    farm.hang("barn-south", 0.0, barn_red, stable_door=(-7.5, -1.5, 4.5), window_byre=(-10.5, -4.5, 1.5, 7.5))
    farm.hang("barn-north", 0.0, window_byre=(-10.5, -4.5, 4.5, 10.5))
    farm.hang("barn-west", 0.0, barn_red, barn_door=(0.0,), window_byre=(-3.0, 3.0))
    farm.hang("barn-east", 0.0, window_byre=(-3.0, 3.0))
    for x in (-6.5, 6.5):
        farm.place("cupola", x, 8.0, barn.ridge - 0.15, tiers=TIERS_0_TO_2)
    farm.mount("bales", "barn-east", 0.0, tiers=TIERS_0_TO_2)
    farm.done()

    # ------------------------------------------------------------ the small farm
    # A two-storey white farmhouse, shuttered, beside a red barn under a tin roof.
    farm = Farm("nyc-farmstead-small", "farm_small",
                dict(Layout="small", Floors=2, House="10x8 white clapboard, shingle gable", Barn="10x14 red boards, galvanised gable"))
    house = farm.building("house", (-7.75, -3), (10, 8), 3.0 + WALL_M, 2.8, "x", (0.0, 0.0), "white clapboard", "shingle", trim=white_m)
    barn = farm.building("barn", (8.25, 0), (10, 14), 5.2, 3.8, "y", (0.0, 0.0), "red boards", "galvanised", plinth=(plinth_m, 0.6),
                         trim=white_m)
    corner_boards(farm, "barn", (3.25, 13.25, -7, 7), 5.2, foot=0.6)
    farm.floors(0.0, 3.0)

    farm.bays_of("house", south=0.0, north=0.0, east=1.5, west=1.5)
    front_door(farm, "house-south", 0.0, "door_transom", black_green)
    for side in ("south", "north", "east", "west"):
        glaze(farm, f"house-{side}", (0.0, 3.0), "window_sash_shutters", skip=(0.0,) if side == "south" else (), tint=black)
    farm.gutters("house", house)
    stack(farm, "chimney_brick", -7.75, -3.0, house.ridge)
    farm.mount("woodpile", "house-east", 0.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=1.5, west=1.5)
    farm.wagon_entrance("barn-south", 0.0, barn_red)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, barn_red, loft_door=(0.0,))
    farm.hang("barn-east", 0.0, barn_red, stable_door=(-1.5,), window_byre=(-4.5, 1.5, 4.5))
    farm.hang("barn-east", 3.0, window_byre=(-1.5, 1.5))
    farm.hang("barn-west", 0.0, window_byre=(-4.5, 4.5))
    farm.hang("barn-west", 3.0, window_byre=(-1.5, 1.5))
    farm.hang("barn-north", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-north", 3.0, window_byre=(0.0,))
    farm.mount("bales", "barn-west", 0.0, tiers=TIERS_0_TO_2)
    farm.done()


# ---------------------------------------------------------------- Paris
def paris():
    """Farms of the country round Paris: a corps de ferme of pale limestone rubble under steep brown
    tiles with a dovecote tower; a longère under slate with lucarnes; a rendered house and a
    half-timbered barn. Shuttered French windows on the houses, plank doors on the barns."""
    gris_bleu, vert, bordeaux, vert_fonce = (118, 138, 150), (104, 128, 102), (124, 52, 50), (50, 78, 62)
    WALLS.update({
        "limestone rubble": textured("wall_stone_pale", "rubble_stone", tint=1.0, colour=in_colour("rubble_stone", (228, 224, 212)), dirt=0.5,
                                     chip=0.0, streak=0.3, rise=0.9, lichen=0.15),
        "colombage": textured("wall_render_colombage", "plaster", tint=1.0, colour=in_colour("plaster", (236, 228, 208)), dirt=0.6, chip=0.4,
                              streak=0.3, rise=0.9, seed=3.0),
    })
    RUBBLE.update({"limestone rubble": rubble_of("limestone", (0.17, 0.165, 0.15), 7.0), "colombage": PLASTER_RUBBLE})
    ROOFS["slate"] = weathered_roof(textured("roof_slate_blue", "roof_slate_matte", colour=(0.078, 0.082, 0.09), seed=6.0, **ROOFING),
                                    seed=6.0, moss=0.15)
    DUST.update({"limestone rubble": (210, 204, 188), "colombage": (214, 206, 186)})
    white_m = flat_paint("window_white", (0.58, 0.57, 0.54), rough=0.6, grime=0.0)
    shutter_m = textured("shutter_slats", "roller_slats", tint=1.0, dirt=0.0, chip=0.4, streak=0.0)
    render_stack_m = textured("stack_render", "plaster", colour=(0.4, 0.38, 0.35), dirt=0.0, chip=0.4, streak=0.3, soot=0.5)
    STACKS["chimney_render"] = ((0.7, 0.7), render_stack_m)

    french_window(kit, "window_persienne", 1.0, 1.5, white_m, glass_m, pane_m, sill_m, shutter_m)
    window_module("window_byre", 0.8, 0.6, lights=1, frame=timber_m, sill=timber_m, behind=None, dark=bpy_dark())
    door_module(kit, "door_tall", 1.15, 2.25, joinery_m, sill_m, pane_m, light=0.3)
    build_step(kit, plinth_m)
    build_chimney(kit, "chimney_render", (0.7, 0.7), render_stack_m, coping_m, pot_m, 1)
    build_rainwater(kit, metal_m)
    barn_door_modules()
    build_dormer(kit, "dormer_slate", WALLS["limestone rubble"], ROOFS["slate"], glass_m, pane_m, frame_m, pitched=True)
    wreckage(kit, rubble_m, char_m)
    for name in ("bales", "woodpile", "trough", "water_butt"):
        prop_module(name)
    FAR_PANELS.update({
        "window_persienne": [(2.41, 1.54, -0.02, "tint", 0.02), (1.28, 1.67, -0.07, sill_m, 0.012), (1.0, 1.5, 0.0, pane_m)],
        "door_tall": [(1.39, 2.71, 0.0, sill_m, 0.012), (1.15, 2.25, 0.0, "tint"), (1.15, 0.3, 2.31, pane_m)],
    })
    FAR_BOXES.update({"chimney_render": [((0.7, 0.7, CHIMNEY_M), (0, 0, CHIMNEY_M / 2), render_stack_m)],
                      "dormer_slate": dormer_far(WALLS["limestone rubble"], ROOFS["slate"])})

    def house_rows(farm, part, shape, floors, door_at, tint, shutters, lattice):
        """A farmhouse's door, its shuttered windows in every other bay, its gutters."""
        farm.bays_of(part, **lattice)
        front_door(farm, f"{part}-south", door_at, "door_tall", tint)
        for side in ("south", "north", "east", "west"):
            glaze(farm, f"{part}-{side}", floors, "window_persienne", skip=(door_at,) if side == "south" else (), tint=shutters)
        farm.gutters(part, shape)

    # ------------------------------------------------------------ the yard farm
    # A corps de ferme: a stone house under a hipped roof of brown tiles, a tall barn gable-on to the
    # yard under a steep one, and across the yard a square dovecote tower under a slate pyramid.
    farm = Farm("paris-farmstead-yard", "farm_yard",
                dict(Layout="yard", Floors=2, House="14x9 limestone rubble, brown tile hip", Barn="12x22 limestone rubble, steep brown tile gable",
                     Dovecote="5x5 rendered tower, slate pyramid"))
    house = farm.building("house", (-10, -8), (14, 9), 3.0 + WALL_M, 3.4, "x", (1.0, 1.0), "limestone rubble", "brown")
    barn = farm.building("barn", (11, 0), (12, 22), 5.2, 5.6, "y", (0.0, 0.0), "limestone rubble", "brown", plinth=None)
    tower = farm.building("dovecote", (-11, 8.5), (5, 5), 7.2, 3.6, "x", (1.0, 1.0), "cream plaster", "slate", plinth=None)
    for k, (size, at) in enumerate((((5.2, 0.14, 0.16), (-11, 6.0)), ((5.2, 0.14, 0.16), (-11, 11.0)), ((0.14, 5.0, 0.16), (-13.5, 8.5)),
                                    ((0.14, 5.0, 0.16), (-8.5, 8.5)))):
        box(farm.m.n(f"dovecote_ledge_{k}"), size, (*at, 5.4), sill_m, farm.m.root, lods=(0, 1, 2))  # the rat ledge round the nesting floor
    farm.floors(0.0, 3.0)

    house_rows(farm, "house", house, (0.0, 3.0), 0.0, bordeaux, gris_bleu, dict(south=0.0, north=0.0, east=1.5, west=1.5))
    stack(farm, "chimney_render", -14.5, -8.0, house.ridge)
    stack(farm, "chimney_render", -5.5, -8.0, house.ridge)
    farm.mount("woodpile", "house-north", 3.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=0.0, west=0.0)
    farm.wagon_entrance("barn-south", 0.0, vert_fonce)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, vert_fonce, loft_door=(0.0,))
    farm.hang("barn-west", 0.0, vert_fonce, barn_door=(0.0,), stable_door=(-6.0, 6.0), window_byre=(-9.0, -3.0, 3.0, 9.0))
    farm.hang("barn-west", 3.0, window_byre=(-6.0, 6.0))
    farm.hang("barn-east", 0.0, window_byre=(-6.0, 0.0, 6.0))
    farm.mount("trough", "barn-west", 4.4, tiers=ROW_TIERS)
    farm.mount("bales", "barn-east", 3.0, tiers=TIERS_0_TO_2)

    farm.bays_of("dovecote", south=0.0, north=0.0, east=0.0, west=0.0)
    farm.hang("dovecote-south", 0.0, vert_fonce, stable_door=(0.0,))
    farm.gutters("dovecote", tower)
    farm.done()

    # ------------------------------------------------------------ the long farm
    # A longère: a long low stone house under a steep slate roof with lucarnes, shuttered; behind it
    # across a narrow yard, a stone barn under brown tiles.
    farm = Farm("paris-farmstead-long", "farm_long",
                dict(Layout="long", Floors=1, House="18x8 limestone rubble longere, slate gable, lucarnes",
                     Barn="26x12 limestone rubble, brown tile gable"))
    house = farm.building("house", (0, -9), (18, 8), WALL_M, 3.8, "x", (0.0, 0.0), "limestone rubble", "slate")
    barn = farm.building("barn", (0, 8), (26, 12), 3.8, 5.0, "x", (0.0, 0.0), "limestone rubble", "brown", plinth=None)
    farm.floors(0.0)

    house_rows(farm, "house", house, (0.0,), 0.0, vert_fonce, bordeaux, dict(south=0.0, north=0.0, east=1.5, west=1.5))
    for along in (-4.5, 4.5):
        place_dormer(farm, house, "dormer_slate", "south", along, 1.0, WHITE)
    stack(farm, "chimney_render", -8.4, -9.0, house.ridge)
    stack(farm, "chimney_render", 8.4, -9.0, house.ridge)
    farm.mount("woodpile", "house-north", 1.5, tiers=ROW_TIERS)
    farm.butt("house-north", -1)

    farm.bays_of("barn", south=1.5, north=1.5, east=0.0, west=0.0)
    farm.wagon_entrance("barn-south", 10.5, vert_fonce)
    farm.hang("barn-south", 0.0, vert_fonce, stable_door=(-7.5, -1.5, 4.5), window_byre=(-10.5, -4.5, 1.5, 7.5))
    farm.hang("barn-north", 0.0, window_byre=(-10.5, -4.5, 4.5, 10.5))
    for side in ("east", "west"):
        farm.hang(f"barn-{side}", 0.0, window_byre=(-3.0, 3.0))
    farm.mount("trough", "barn-south", 2.9, tiers=ROW_TIERS)
    farm.mount("bales", "barn-east", 0.0, tiers=TIERS_0_TO_2)
    farm.done()

    # ------------------------------------------------------------ the small farm
    # A rendered house under a hipped roof of red tiles, and beside it a half-timbered barn under a
    # steep half-hipped roof of brown tiles, the loft's doors in its gable.
    farm = Farm("paris-farmstead-small", "farm_small",
                dict(Layout="small", Floors=2, House="11x8 cream render, clay tile hip", Barn="10x14 half-timbered, brown tile half-hip"))
    house = farm.building("house", (-7.75, -3), (11, 8), 3.0 + WALL_M, 3.0, "x", (1.0, 1.0), "cream plaster", "clay")
    barn = farm.building("barn", (8.25, 0), (10, 14), 5.2, 4.8, "y", (0.35, 0.35), "colombage", "brown", plinth=(brick_plinth_m, 0.6))
    farm.floors(0.0, 3.0)

    house_rows(farm, "house", house, (0.0, 3.0), 0.0, bordeaux, vert, dict(south=0.0, north=0.0, east=1.5, west=1.5))
    stack(farm, "chimney_render", -7.75, -3.0, house.ridge)
    farm.mount("woodpile", "house-east", 0.0, tiers=ROW_TIERS)
    farm.butt("house-north", 1)

    farm.bays_of("barn", south=0.0, north=0.0, east=1.5, west=1.5)
    half_door = BARN_DOOR_M[0] / 2 + 0.15
    framing = dict(posts=dict(a0=(6.75, 9.75), a1=(6.75, 9.75), b0=(-6.0, -3.0, 0.0, 3.0, 6.0), b1=(-6.0, -3.0, 0.0, 3.0, 6.0)),
                   openings=dict(a0=[(8.25 - half_door, 8.25 + half_door, 3.5)], b1=[(-2.2, -0.8, 2.3)]), foot=0.6, rail=2.55)
    farm.framed.add("barn")
    fm = kit.module("farm_small_barn_frame", ground=True, **FITTING)
    timber_frame(fm.n("frame"), barn, timber_m, fm.root, **framing)
    farm.t.place(fm.name, tiers=ROW_TIERS)
    timber_frame(farm.m.n("barn_frame"), barn, timber_m, farm.m.root, lods=(2, 3), **framing)  # the colombage is the barn: kept to the last tier
    farm.wagon_entrance("barn-south", 0.0, bordeaux)
    farm.hang("barn-south", 0.0, window_byre=(-3.0, 3.0))
    farm.hang("barn-south", 3.0, bordeaux, loft_door=(0.0,))
    farm.hang("barn-east", 0.0, bordeaux, stable_door=(-1.5,), window_byre=(-4.5, 1.5, 4.5))
    farm.hang("barn-east", 3.0, window_byre=(-1.5, 1.5))
    farm.hang("barn-west", 0.0, window_byre=(-4.5, 4.5))
    farm.hang("barn-north", 0.0, window_byre=(-3.0, 3.0))
    farm.mount("trough", "barn-east", 3.0, tiers=ROW_TIERS)
    farm.mount("bales", "barn-west", 0.0, tiers=TIERS_0_TO_2)
    farm.done()


DESIGNS = {"china": china, "new_york": new_york, "paris": paris}
design(DESIGNS, FAMILY, "farmsteads.py")()
kit.write(OUT)
