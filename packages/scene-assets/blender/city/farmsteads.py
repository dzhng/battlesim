"""Our own farmsteads: the farms at a town's edge, as one kit.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/farmsteads.py

writes `assets/source/city/farmsteads/kit.glb` and `templates.json` (the format is
in this folder's readme). A farmstead is several buildings in one template: a
house, a barn and perhaps a shed, each a part of its own, standing apart. The
ground between them is open: units walk through the yard, and nothing is drawn
there. Three farms, three kinds of barn:

  yard    a plastered house, a timber-framed barn with wagon doors and an
          open-fronted cart shed, round a yard
  long    a low whitewashed longhouse with a long stone byre behind it
  small   a brick house beside a tarred, boarded barn

They are plain plaster, brick, stone, board and tile, as the houses are
(`homes.py`), and carry the first shipping family's name until a family has
farms of its own.

The physical box is the authority. Every outer wall stands on a face of a part,
and a part's top is its roof's ridge. Doors and windows sit in the bays of their
edge's 3 m lattice, on the floor datums, where a garrison's soldiers stand; a
barn has few of them, and a bay without one is a blank wall. What stands in a
yard (a woodpile, a trough, a rain barrel, straw) hugs a wall, inside the set's
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

FAMILY = "china"
WALL_M = 2.85  # from a house's top floor datum up to its eaves
BYRE_SILL_M, LOFT_SILL_M = 1.3, 0.6  # a barn's small windows: high over the stalls, low under the loft's eaves
ROW_TIERS, FOLDED_TIER, FOLDED = TIERS_0_TO_1, 2, (2, 3)  # fittings are rows near; the shell keeps them at the two far tiers

kit = Kit("farmsteads", "farmsteads.py", fit_side_m=0.6, fit_top_m=0.9, fit_ruin_top_m=0.6)


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
TIMBER = ("tarred boards", "weathered boards")
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
window_module("window_a", 1.0, 1.3)
window_module("window_a_shutters", 1.0, 1.3, shutters=True)
window_module("window_byre", 0.8, 0.6, lights=1, frame=timber_m, sill=timber_m, behind=None, dark=bpy_dark())
window_module("window_shed", 0.8, 0.6, lights=1, frame=timber_m, sill=timber_m, cut=False)

m = kit.module("door_panel", ground=True, **FITTING)
panel_door(m.n("door"), 0.95, 2.05, joinery_m, frame_m, m.root)

m = kit.module("step", ground=True, **FITTING)
box(m.n("slab"), (1.5, 0.4, 0.16), (0, -0.2, 0.08), plinth_m, m.root)

m = kit.module("chimney_brick", **FITTING)
chimney(m.n("c"), (1.05, 0.6), CHIMNEY_M, stack_m, coping_m, m.root, pots=2, pot_mat=pot_m)

# A metre of gutter along +X and a metre of downpipe up +Z: rows stretch them to length.
m = kit.module("gutter", **FITTING)
cyl(m.n("run"), 0.06, 1.0, (0, 0, 0), "X", metal_m, m.root, seg=6, caps=False)
m = kit.module("downpipe", **FITTING)
cyl(m.n("pipe"), 0.045, 1.0, (0, 0, 0.5), "Z", metal_m, m.root, seg=6, caps=False)

# A barn's doors: the wagon doors, a split stable door, and the loft's doors under their hoist.
BARN_DOOR_M, STABLE_DOOR_M, LOFT_DOOR_M = (2.7, 3.2), (1.15, 2.1), (1.3, 1.35)
m = kit.module("barn_door", ground=True, **FITTING)
barn_doors(m.n("doors"), *BARN_DOOR_M, joinery_m, timber_m, m.root)
m = kit.module("stable_door", ground=True, **FITTING)
stable_door(m.n("door"), *STABLE_DOOR_M, joinery_m, timber_m, m.root)
m = kit.module("loft_door", **FITTING)
barn_doors(m.n("doors"), *LOFT_DOOR_M, joinery_m, timber_m, m.root, hoist=0.55)

wreckage(kit, rubble_m, char_m)

# What stands about a farm, each against a wall or under a roof.
BALES = dict(across=3, high=3)
m = kit.module("cart", ground=True, **FITTING)
farm_cart(m.n("cart"), cart_m, iron_m, m.root)
m = kit.module("bales", ground=True, **FITTING)
bale_stack(m.n("bales"), straw_m, m.root, **BALES)
m = kit.module("woodpile", ground=True, **FITTING)
woodpile(m.n("pile"), 1.6, 1.1, bark_m, cut_m, m.root)
m = kit.module("trough", ground=True, **FITTING)
stone_trough(m.n("trough"), trough_m, water_m, m.root)
m = kit.module("water_butt", ground=True, **FITTING)
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

    def __getattr__(self, name):  # the template's own: parts, floors, lattices, bays, entrances
        return getattr(self.t, name)

    def mount(self, module, edge, offset_m, z=0.0, out=0.0, **row):
        self.place(module, *self.t.at(edge, offset_m, z, out), **row)

    def place(self, module, x=0.0, y=0.0, z=0.0, yaw=0.0, tiers=EVERY_TIER, tint=WHITE, **row):
        if tiers & ROW_TIERS:
            self.t.place(module, x, y, z, yaw, tiers=tiers & ROW_TIERS, tint=tint, **row)
        if tiers >> FOLDED_TIER & 1:
            self.far.append((module, x, y, z, yaw, *row.get("scale", (1.0, 1.0, 1.0)), tiers, *tint))

    def building(self, part, centre, size, eave, rise, along, hips, wall, roof, plinth=(plinth_m, 0.4)):
        """A building under a pitched roof: its part, and its walls, roof and plinth in the shell. Its roof's shape."""
        x0, x1, y0, y1 = centre[0] - size[0] / 2, centre[0] + size[0] / 2, centre[1] - size[1] / 2, centre[1] + size[1] / 2
        shape = RoofShape(x0, x1, y0, y1, eave, eave + rise, along, hips, OVER_M, (VERGE_M, VERGE_M))
        house_shell(self.m.n(part), shape, WALLS[wall], ROOFS[roof], trim_m, self.m.root, plinth=(x0, x1, y0, y1) if plinth else None,
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
        fold_far(self.m, self.far, FAR_PANELS, FAR_BOXES, far_paint, FOLDED)
        self.t.place(self.m.name)
        open_walls(self.m, self.t.rows["intact"], kit.openings)  # near, a window is an opening, a house's with its room behind
        furnish(self.t, kit)
        t = self.t
        m = kit.module(self.m.name.replace("_shell", "_ruin"), ground=True, **RUIN)
        high, over, seed = t.ruin_height(), kit.fit["ruin_top_m"], zlib.crc32(t.id.encode()) % 997
        for k, p in enumerate(t.parts):
            wall, roof = self.built[p["id"]]
            timber = wall in TIMBER
            rect = (p["x0"], p["x1"], p["y0"], p["y1"])
            sides = {side: v for side, v in ruin_sides(t, p["id"], OPENINGS).items() if side not in (open_sides or {}).get(p["id"], ())}
            stacks = [(x, y, (1.05, 0.6), stack_m) for module, x, y, *_ in t.rows["intact"]
                      if module == "chimney_brick" and rect[0] < x < rect[1] and rect[2] < y < rect[3]]
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


# ---------------------------------------------------------------- the yard farm
# A plastered two-storey house on the street, a timber-framed barn gable-on beside it,
# and an open-fronted cart shed across the yard behind the house.
farm = Farm("china-farmstead-yard", "farm_yard",
            dict(Layout="yard", Floors=2, House="14x9 plaster, clay gable", Barn="12x22 timber frame, brown gable",
                 Shed="9x6 open-fronted, boarded, slate lean-to"))
house = farm.building("house", (-10, -8), (14, 9), 3.0 + WALL_M, 2.5, "x", (0.0, 0.0), "cream plaster", "clay")
barn = farm.building("barn", (11, 0), (12, 22), 4.6, 3.8, "y", (0.0, 0.0), "straw plaster", "brown", plinth=(brick_plinth_m, 0.6))

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

# ---------------------------------------------------------------- the long farm
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

# ---------------------------------------------------------------- the small farm
# A square brick house under a hipped roof, and beside it, a cart's width away, a tarred,
# boarded barn on a brick plinth, its gable clipped by a half-hip over the loft doors.
farm = Farm("china-farmstead-small", "farm_small",
            dict(Layout="small", Floors=2, House="11x8 brick, brown hip", Barn="10x14 boarded, clay half-hip"))
house = farm.building("house", (-7.75, -3), (11, 8), 3.0 + WALL_M, 2.3, "x", (1.0, 1.0), "brick", "brown")
barn = farm.building("barn", (8.25, 0), (10, 14), 4.5, 3.3, "y", (0.4, 0.4), "tarred boards", "clay", plinth=(brick_plinth_m, 0.7))
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

kit.write(next(iter(script_args()), None))  # an argument writes the two files somewhere else
