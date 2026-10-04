"""The New York apartment blocks: pre-war brick walk-ups from the vendored NYC graph.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/nyc.py [out_dir] [--dry]

writes `assets/source/city/apartments_new_york/kit.glb` and `templates.json` (the
format is in this folder's readme), or the two files into `out_dir`; with `--dry`
it only prints. It needs the ambientCG sets in the pack cache.

The source is the vendored `NYC_CornerBuilding.blend`, a corner building: two
street facades (`NYC_Facade`) of stretched brick and stone cubes round sash
windows, shops on the ground floor, a fire escape, a big bracketed cornice, a
water tank and a stair bulkhead on the roof; its two back walls are one blank
box. What every graph set shares is `graphset.py`; this is the NYC graph's part:

- **Every run of a template's outline is the graph's first facade**, evaluated at
  that run's length with its room clip muted (`open_graph`), so its rooms are
  instances. A run on the street has shops, as many as divide its bays into 9 m
  fronts, so a shop's piers fall between bays; a back run has none, and the graph
  leaves a shopless ground floor empty, so its first floor is brought down a
  storey to be its ground floor (`facade`).
- **The shell** is the graph's brick cubes and stone trim (tinted with the
  template's brick colour), its cornice, frames, fire escapes and sign boards,
  and our parapet and tar roof. Its shop glass is a pane module, one face.
- **Rooms** are ours behind every window and shop front, where the graph stood its.
- **Templates are single blocks**: the graph mitres its cornice for the convex
  corners of a box, so a compound's inside corner has no cornice of its own.

A template's frame has its origin at the footprint's centre and its entrance on
-Y, the street side: the southmost run takes the shops and doors.
"""
import math
import os
import sys
from types import SimpleNamespace

import bpy
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ambientcg  # noqa: E402
import graph  # noqa: E402
import textures  # noqa: E402
from graph import Soup  # noqa: E402
from graphset import MASK, GraphSet, bars, decompose, frame, lining, grime, linear_of, outline, quad, ring, scorched, toned  # noqa: E402

BRICK, STONE, ROOF, GLASS, CORNICE = "NYC_Brick", "NYC_Stone", "NYC_Roof", "NYC_Glass", "NYC_Cornice"
UNIT = "~ba89b02d"  # the graph's stretched unit cube, named by its shape (`graph.tap`)
ROOM_FRONT_M = 0.26  # a room's open face, just behind the sashes' and shop glass's plane
SHOP_M = 9.0  # a shop front's pitch: three bays
BACK_DEPTH_M = 80.0  # the far facade of a building evaluated for a back run: long enough that the run has no shop
# What the graph builds that a building of ours does not draw: a shop's furnishings, lit panels, posters.
INSIDE = ("SHOP_Shelf", "SHOP_Counter", "SHOP_Crate", "P_", "INT_", "SHOP_Box")
INSIDE_CUBES = frozenset(("NYC_LightPanel", "NYC_Poster", "NYC_Product", "NYC_Prop"))
# A back run's ground floor is its first floor brought down: but for what belongs up there only.
NOT_BROUGHT = ("BAL_", "R_", "SIGN_", "TH_", "TV_", "TS_", "SHOP_", "primitive:NYC_Iron", "primitive:NYC_SignBoard")
# The graph's roof furniture, by the materials of its realized mesh: our names for it.
ROOF_KINDS = {"NYC_Iron+NYC_Roof+NYC_Wood": "NYC_WaterTank", "NYC_Brick+NYC_Chrome+NYC_Iron+NYC_LampGlow+NYC_Metal+NYC_Stone": "NYC_Bulkhead",
              "NYC_AC+NYC_Concrete+NYC_Iron+NYC_Metal": "NYC_HVAC", "NYC_Brick+NYC_Iron+NYC_Stone": "NYC_Chimney",
              "NYC_Iron": "NYC_RoofPole", "NYC_Concrete+NYC_Iron+NYC_Metal": "NYC_RoofHatch",
              "NYC_Iron+NYC_Metal": "NYC_Vent", "NYC_Metal": "NYC_Vent", "NYC_Glass+NYC_Iron+NYC_Metal": "NYC_RoofHatch"}


def brick(albedo, roughness):
    """Common brick in running bond, 22.5 by 7.5 cm with 1 cm joints: a pale, nearly grey
    clay, so the row's tint is the brick's colour, each brick a hair off its neighbours."""
    albedo, roughness = toned((0.62, 0.6, 0.58), 0.2)(albedo, roughness)
    size = albedo.shape[0]
    yy, xx = np.mgrid[0:size, 0:size].astype(float) / size  # the tile is 0.9 m: 12 courses, 4 bricks a course
    course = np.floor(yy * 12)
    u = (xx * 4 + 0.5 * (course % 2)) % 1.0
    v = (yy * 12) % 1.0
    joint = np.maximum(textures.smoothstep(0.16, 0.1, np.minimum(v, 1 - v)), textures.smoothstep(0.05, 0.03, np.minimum(u, 1 - u)))
    block = np.floor(xx * 4 + 0.5 * (course % 2)) + 5 * course
    tone = 0.92 + 0.14 * ((block * 2654435761) % 97 / 96.0)
    out = albedo * tone[..., None] * (1.0 - joint[..., None]) + np.array((0.55, 0.53, 0.5)) * joint[..., None]
    return np.clip(out, 0.0, 1.0), np.clip(roughness + 0.1 * joint, 0.0, 1.0)


class NYC(GraphSet):
    SET = "apartments_new_york"
    BLEND = "vendor/procedural-buildings/NYC_CornerBuilding.blend"
    SCRIPT = "nyc.py"
    ID_PREFIX, FAMILY = "nyc-apartment-", "new_york"
    SHAPED = True
    # What we set of the graph's hundred inputs; the rest are as the file is saved. Bays are 3 m
    # with 1 m corner piers, floors 3.2 m over a 4 m shop floor. What is switched off has no place
    # in a building of ours: the street, lit rooms, the corner hotel sign, the escalator, the
    # shops' furnishings and the balconies a walk-up does not have.
    INPUTS = {
        "Building Height": 0.0, "Floor Height": 3.2, "Ground Floor Height": 4.0, "Corner Angle": math.pi / 2,
        "Facade Offset": 1.0, "Bay Width": 3.0, "Wall Thickness": 0.4, "Detail Level": "HIGH",
        "Sidewalk": False, "Sidewalk All Around": False, "Window Light Probability": 0.0, "Window Light Strength": 0.0,
        "Interior Mode": "Image Rooms", "Interior Probability": 1.0, "Shop Interior Detail": 0,
        "Escalator Enabled": False, "Custom Sign": False, "Clotheslines": 0, "Clothes Density": 0.0,
        "Balcony Probability": 0.04, "Blank Probability": 0.0, "Sign Probability": 0.12,
    }
    GROUND_M, STOREY_M, WALL_M = 4.0, 3.2, 0.4
    PARAPET_M = 1.3  # the roof to the cornice's top, as the graph builds it
    ROOF_RISE_M = 0.2  # the graph's roof stands this far over the walls' head
    # A template: its id's tail, floors, bays along x and y, the graph's seed, the runs (south, east,
    # north, west) with shops and with a fire escape, the water tank's chance, and the brick (sRGB).
    TEMPLATES = (
        ("slab-38x11-4f", 4, 12, 3, 101, "SE", "N", 1.0, (128, 74, 60)),
        ("slab-47x11-5f", 5, 15, 3, 102, "S", "S", 1.0, (110, 80, 66)),
        ("slab-56x14-6f", 6, 18, 4, 103, "S", "SN", 1.0, (148, 120, 94)),
        ("slab-47x14-8f", 8, 15, 4, 104, "S", "N", 1.0, (116, 64, 52)),
        ("point-20x20-7f", 7, 6, 6, 105, "SE", "W", 1.0, (96, 78, 70)),
    )
    # a blade sign hangs 1.8 m out over the street, and a water tank stands on its tower 8.7 m over the roof
    FIT = {"side_m": 1.9, "top_m": 8.0}
    FAMILIES = (
        ("SASH_", 0b0011), ("NYC_Pane", 0b0011), ("SHOP_Door", 0b0011), ("NYC_WaterTank", 0b1111), ("NYC_Bulkhead", 0b1111),
        ("AC_", 0b0011), ("BAL_", 0b0011), ("SHOP_Awning", 0b1111), ("NYC_HVAC", 0b0111), ("NYC_Chimney", 0b0111),
        ("L_", 0b0011), ("SILL", 0b0011), ("PANEL_", 0b0011), ("BAND_", 0b0011), ("CRN_Bracket", 0b0011), ("R_", 0b0011),
        ("SH_", 0b0011), ("NYC_Room_", 0b0011), ("NYC_Roof", 0b0011), ("NYC_Vent", 0b0011),
        ("CRN_Dentil", 0b0001), ("CV_", 0b0001), ("SIGN_", 0b0001), ("TS_", 0b0001), ("TH_", 0b0001), ("TV_", 0b0001),
    )
    OPENINGS = ("SASH_", "NYC_Pane", "SHOP_Door")
    HULLS = ("AC_", "SHOP_Awning")
    ROOM_MESHES = {"NYC_Room_Home": "X_Room", "NYC_Room_Shop": "X_ShopRoom"}
    ROOM_SHEETS = {"X_Room": "rooms", "X_ShopRoom": "shops"}
    SHEETED = ("R_", "SIGN_Bracket")
    SIGN_SWAPS = {"TH_04": "TH_07", "TV_04": "TV_07"}  # "taste Vietnam" names a country: a bakery instead
    ENTRANCE = "SHOP_Door"

    WALL, STONE, BAND, ROOF, GLASS = BRICK, STONE, STONE, ROOF, GLASS
    SURFACES = {
        BRICK: ("nyc_brick", "nyc_brick", (1.0, 1.0, 1.0), 1.0, 0.0),
        STONE: ("nyc_stone", "nyc_stone", (1.0, 1.0, 1.0), 1.0, 0.0),
        ROOF: ("nyc_tar", "nyc_tar", (1.0, 1.0, 1.0), 1.0, 0.0),
        CORNICE: ("nyc_cornice", "nyc_stone", (0.5, 0.48, 0.45), 0.8, 0.1),
        "NYC_Granite": ("nyc_granite", "nyc_stone", (0.3, 0.3, 0.32), 0.8, 0.0),
        "NYC_Concrete": ("nyc_concrete", "nyc_concrete", (0.85, 0.85, 0.85), 1.0, 0.0),
        "NYC_Iron": ("nyc_iron", "nyc_metal", (0.03, 0.03, 0.032), 0.8, 0.4),
        "NYC_Metal": ("nyc_metal", "nyc_metal", (0.6, 0.6, 0.6), 0.7, 0.8),
        "NYC_Chrome": ("nyc_chrome", "nyc_metal", (0.9, 0.9, 0.9), 0.4, 1.0),
        "NYC_Frame": ("nyc_frame", "nyc_paint", (0.7, 0.68, 0.62), 0.6, 0.0),
        "NYC_ShopFrame": ("nyc_shop_frame", "nyc_paint", (0.035, 0.06, 0.05), 0.6, 0.0),
        "NYC_Wood": ("nyc_wood", "nyc_wood", (0.8, 0.7, 0.6), 1.0, 0.0),
        "NYC_AC": ("nyc_ac", "nyc_plastic", (0.62, 0.6, 0.55), 0.8, 0.0),
        "NYC_Awning": ("nyc_awning", "nyc_fabric", (0.32, 0.05, 0.04), 1.0, 0.0),
        "NYC_Blind": ("nyc_blind", "nyc_fabric", (0.62, 0.6, 0.54), 1.0, 0.0),
        "NYC_Cloth": ("nyc_cloth", "nyc_fabric", (0.42, 0.36, 0.3), 1.0, 0.0),
        "NYC_Curtain": ("nyc_curtain", "nyc_fabric", (0.46, 0.38, 0.3), 1.0, 0.0),
        "NYC_Rubber": ("nyc_rubber", "nyc_plastic", (0.04, 0.04, 0.04), 1.0, 0.0),
        "NYC_SignBoard": ("nyc_sign_board", "nyc_plastic", (0.05, 0.08, 0.07), 0.7, 0.0),
        "NYC_SignText": ("nyc_sign_text", None, (0.8, 0.7, 0.45), 0.4, 0.0),
        # a lamp, unlit: nothing glows
        "NYC_LampGlow": ("nyc_lamp", None, (0.7, 0.68, 0.6), 0.3, 0.0),
        GLASS: ("nyc_glass", None, (0.05, 0.075, 0.09), 0.08, 0.0),
        "X_Pane": ("nyc_pane", None, (0.06, 0.066, 0.068), 0.25, 0.0),
        "X_Void": ("nyc_void", None, (0.005, 0.005, 0.006), 1.0, 0.0),
        "X_Burnt": ("nyc_brick_burnt", "nyc_burnt", (1.0, 1.0, 1.0), 1.0, 0.0),
        "X_Rubble": ("nyc_rubble", "nyc_concrete", (0.46, 0.3, 0.24), 1.0, 0.0),  # broken brick
        "X_Soot": ("nyc_soot", "nyc_concrete", (0.5, 0.48, 0.46), 1.0, 0.0),
        "X_Room": ("nyc_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
        "X_ShopRoom": ("nyc_shop_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
    }
    COVERAGE = {GLASS: ("blended", 0.35)}
    TAKES_COLOUR = frozenset((BRICK,))
    LEFT_OUT = frozenset(INSIDE_CUBES | {"NYC_RoomApartment", "NYC_RoomShop"})
    BARS = "nyc_bars"
    BURNS = frozenset((GLASS, "NYC_Awning", "NYC_Blind", "NYC_Cloth", "NYC_Curtain", "NYC_SignBoard", "NYC_SignText", "NYC_Wood"))
    SURVIVES = (
        ("SASH_", 0.2, ("burnt",)), ("AC_", 0.45, ("burnt", "hanging")), ("BAL_", 1.0, ("burnt",)), ("R_", 0.8, ("burnt",)),
        ("L_", 1.0, ("burnt",)), ("SILL", 1.0, ("burnt",)), ("CRN_Bracket", 0.7, ("burnt",)), ("SHOP_Door", 0.5, ("burnt",)),
        ("NYC_WaterTank", 1.0, ("burnt",)), ("NYC_Bulkhead", 1.0, ("burnt",)), ("NYC_Chimney", 1.0, ("burnt",)),
    )
    BURNT_ROOF_CELL_M = (1.0, 1.0, 3.0, 8.0)
    SMOKED_ROOF = (0.6, 0.58, 0.56)  # smoke on tar, not on clay
    # a walk-up has twice a China block's windows to a wall: its far soot is lighter on triangles
    FLAT_SOOT, FLAT_SOOT_SIDES, FAR_SOOT_SHARE = False, False, 0.0
    BLOWN = 0.025  # its bays are narrow: fewer of them blown out, as many metres of wall  # a far block's intact roof is a few faces: so is its burnt one
    WRECKS = ("AC_0_Window", "AC_1_Split", "AC_2_Box", "R_0_Vertical", "SASH_1_Split")

    def recipes(self):
        ambientcg.bake("nyc_brick", "Concrete034", 0.9, rough=(1.0, 0.05), normal=0.5, grime=brick)
        ambientcg.bake("nyc_burnt", "Concrete034", 0.9, rough=(1.0, 0.05), normal=0.5, grime=lambda a, r: scorched(*brick(a, r)))
        # limestone lintels, sills, quoins and the cornice: a warm grey, darkened by the city
        ambientcg.bake("nyc_stone", "Concrete034", 1.6, rough=(0.95, 0.05), normal=0.4, grime=toned((0.4, 0.38, 0.33), 0.35, level_cycles=6))
        # the tar roof: the largest surface of a town seen from above, dark and quiet
        ambientcg.bake("nyc_tar", "Concrete034", 2.0, repeat=2, rough=(1.0, 0.05), normal=0.4, grime=toned((0.055, 0.055, 0.058), 0.1, level_cycles=8))
        ambientcg.bake("nyc_concrete", "Concrete034", 2.0, rough=(1.0, 0.05), normal=0.6, grime=grime(0.35, 0.3))
        ambientcg.bake("nyc_metal", "Metal009", 1.0, rough=(1.0, 0.05), normal=0.4)
        ambientcg.bake("nyc_paint", "Plastic010", 1.0, rough=(0.8, 0.12), normal=0.3)
        ambientcg.bake("nyc_plastic", "Plastic010", 1.0, rough=(0.8, 0.12), normal=0.3)
        ambientcg.bake("nyc_fabric", "Fabric036", 0.6, rough=(1.0, 0.1), normal=0.6)
        ambientcg.bake("nyc_wood", "WoodFloor041", 1.5, rough=(1.0, 0.05), normal=0.6)
        textures.recipe(self.BARS, tile=0.5)(bars)

    # ------------------------------------------------------------ the graph
    def open_graph(self):
        graph.open_blend(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), self.BLEND))
        # in memory only: the rooms as instances (the graph realizes them to clip them to its walls)
        bpy.data.node_groups["NYC_Interior"].nodes["CLIP_realize"].mute = True
        ob, mod = graph.modifier("NYC_Building", "NYC_Building")
        saved = {}
        for it in mod.node_group.interface.items_tree:
            if it.item_type == "SOCKET" and it.in_out == "INPUT":
                value = getattr(mod.properties.inputs, it.identifier).value
                saved[it.name] = tuple(value) if hasattr(value, "__len__") and not isinstance(value, str) else value
        # every input is set each time: what we do not choose is as the vendored file is saved
        self.saved = saved
        return ob, mod

    def inputs(self, **own):
        out = dict(self.saved)
        out.update({k: v for k, v in self.INPUTS.items() if k in out})
        out.update(own)
        return out

    def drawn(self, tap):
        out = []
        for name, m, tint in tap.rows:
            if name.startswith("primitive:") and not name.endswith(UNIT):
                kind = ROOF_KINDS.get(name.split(":")[1].split("~")[0])
                name = f"{kind}~{name.split('~')[1]}" if kind else name
            name = self.SIGN_SWAPS.get(name, name)
            if np.linalg.det(m[:3, :3]) < 0:  # a left curtain is a right one mirrored: a row cannot mirror, a module can
                self.mirrored.add(name)
                name, m = name + "Mirror", m @ np.diag([-1.0, 1.0, 1.0, 1.0])
            if not name.startswith("primitive:") and decompose(m) is None:  # the graph's imperfection leans a few things a degree or two: stood up straight
                scale = np.linalg.norm(m[:3, :3], axis=0)
                yaw = math.atan2(m[1, 0], m[0, 0])
                straight = frame(yaw, m[0, 3], m[1, 3]) @ np.diag([*scale, 1.0])
                straight[2, 3] = m[2, 3]
                if np.abs(straight[:3, :3] - m[:3, :3]).max() > 0.1 * scale.max():
                    raise SystemExit(f"{name} leans too far to stand up straight")
                m = straight
            out.append((name, m, None))
        return out

    def kit_meshes(self, graph_):
        meshes = super().kit_meshes(graph_)
        for tap, _ in graph_.taps.values():
            for name, soup in tap.meshes.items():
                if name.startswith("primitive:") and not name.endswith(UNIT):
                    kind = ROOF_KINDS.get(name.split(":")[1].split("~")[0])
                    if kind:
                        meshes[f"{kind}~{name.split('~')[1]}"] = soup
        # glass is one face (README, "Surfaces that are not opaque"): a shop window's pane, a unit square
        meshes["NYC_Pane"] = quad([(-0.5, 0.0, -0.5), (0.5, 0.0, -0.5), (0.5, 0.0, 0.5), (-0.5, 0.0, 0.5)], GLASS)
        # the cornice's brackets and dentils are its sheet metal too
        meshes.update({n: Soup(m.v, m.c, m.t, m.m, m.s, [CORNICE if k == STONE else k for k in m.mats]) for n, m in meshes.items() if n.startswith("CRN_")})
        meshes.update(self.opening_meshes)
        meshes.update({name + "Mirror": meshes[name].transformed(np.diag([-1.0, 1.0, 1.0, 1.0])) for name in sorted(self.mirrored)})
        return meshes

    def module_id(self, name):
        return super().module_id(name.replace("NYC_", "", 1) if name.startswith("NYC_") else name)

    # ------------------------------------------------------------ a run
    def evaluate(self, graph_, length, floors, seed, shops, escape):
        """The graph's first facade, `length` long: with `shops` 9 m shops, or none."""
        own = dict(**{"Building Width": length, "Floor Count": floors, "Facade Seed": seed, "Window Seed": seed + 1,
                      "Module Seed": seed + 2, "AC Seed": seed + 3, "Balcony Seed": seed + 4, "Shop Seed": seed + 5,
                      "Sign Seed": seed + 6, "Fire Escape Seed": seed + 7, "Detail Seed": seed + 8, "Interior Seed": seed + 9,
                      "Fire Escape Side": "Right" if escape else "None", "Fire Escape Levels": floors - 1,
                      "Roof Access": False, "Water Tower Probability": 0.0, "HVAC Units": 0, "Vents": 0,
                      "Dish Probability": 0.0, "Hatch Probability": 0.0})
        if shops:  # the first facade takes round(count x width / (width + depth)) of `count` shops (`NYC_build_building`)
            own.update({"Shop Count": shops + 1, "Building Depth": length / shops})
        else:
            own.update({"Shop Count": 1, "Building Depth": BACK_DEPTH_M})
        inputs = self.inputs(**own)
        return graph_.tap(inputs), inputs

    def facade(self, tap, rows, run, floors, back):
        """The graph's first facade (along +x from its corner, out to -y) moved onto a run.
        Its brick and stone are the wall; its cornice, frames, iron and boards the cubes;
        its windows and shop fronts the openings, each with our room behind it."""
        length = math.dist(*run)
        start, end = np.array(run[0], dtype=np.float64), np.array(run[1], dtype=np.float64)
        along = (end - start) / length
        out_n = np.array([along[1], -along[0]])
        place = frame(math.atan2(along[1], along[0]), *start)
        head = self.GROUND_M + self.STOREY_M * (floors - 1)
        gh, fh = self.GROUND_M, self.STOREY_M
        on_run = lambda x: -1.3 < x < length + 1.3

        own = tap.own
        name = np.array(own.material_names())
        normals, _ = own.normals()
        centre = own.v[own.t].mean(1)
        x, y, z = centre[:, 0], centre[:, 1], centre[:, 2]
        wallish = (name == STONE) | (name == BRICK)
        cornice = wallish & (z > head - 0.35) & (y < 0.25) & (x > y - 0.01) & (x < length - y + 0.01)
        facing = -normals[:, 1]
        ours = wallish & ~cornice & (y > -0.6) & (y < self.WALL_M + 0.01) & (z < head) & \
            ((facing > 0.5) | ((x > self.WALL_M + 0.01) & (x < length - self.WALL_M - 0.01)))
        ours &= normals[:, 1] < 0.5  # its inner face is never seen: every opening has a room or a lining
        wall_parts = [self.masked(own.keep(ours), True)]
        # the cornice is pressed sheet metal, painted: its own grey, not the stone of the lintels
        crown = own.keep(cornice)
        cubes = [self.masked(Soup(crown.v, crown.c, crown.t, crown.m, crown.s, [CORNICE if n == STONE else n for n in crown.mats]), False)]
        placed, openings, glass = [], [], []
        for n, m, _ in rows:
            p = m[:3, 3]
            if n.startswith(INSIDE) or not on_run(p[0]):
                continue
            if n.startswith("primitive:"):
                if not n.endswith(UNIT) or p[2] > head + 0.15 or p[1] > 0.8:
                    continue
                material = tap.meshes[n].mats[-1]
                if material in INSIDE_CUBES:
                    continue
                cube = self.masked(tap.meshes[n], material == BRICK).transformed(m)
                if material == GLASS:
                    glass.append(m)
                elif material == BRICK and p[1] < self.WALL_M + 0.05:
                    cn, _ = cube.normals()
                    wall_parts.append(cube.keep(cn[:, 1] < 0.5))
                else:
                    cn, _ = cube.normals()
                    cubes.append(cube.keep(cn[:, 1] < 0.5))
                continue
            if p[1] > 0.8 or (p[2] > head + 0.15 and not n.startswith("CRN_")):
                continue
            placed.append((n, m))
        # the rooms the graph stood behind its windows and shops: ours, the same boxes
        rooms = []
        for n, m, _ in rows:
            if n.startswith(("INT_Box", "SHOP_Box")) and on_run(m[0, 3]):
                lo, hi = tap.meshes[n].bounds()
                corners = np.array([[a, b, c] for a in (lo[0], hi[0]) for b in (lo[1], hi[1]) for c in (lo[2], hi[2])]) @ m[:3, :3].T + m[:3, 3]
                a, b = corners.min(0), corners.max(0)
                if a[1] > 1.0:
                    continue
                z0 = self.ROOM_STEP_M if a[2] < 0.01 else a[2]
                # from just behind the glass: the graph's box stands back of it, and through the gap the eye would
                # pass the box's side into the hollow block and out of its far wall
                y0 = min(a[1], ROOM_FRONT_M)
                box = np.diag([b[0] - a[0], b[1] - y0, b[2] - 0.03 - z0, 1.0])
                box[:3, 3] = ((a[0] + b[0]) / 2, y0, z0)
                kind = "NYC_Room_Shop" if n.startswith("SHOP_Box") else "NYC_Room_Home"
                rooms.append((kind, box))
                if kind == "NYC_Room_Shop":  # a shop front: the open front its box stands behind
                    openings.append(self.opening("NYC_ShopFront", a[0], b[0], 0.0, b[2]))
        # the windows: one opening each, from its sill up the graph's window height
        sills = sorted({(round(m[0, 3], 2), round(m[2, 3] + tap.meshes[n].bounds()[1][2] * m[2, 2], 2)) for n, m in placed if n == "SILL"})
        width, height = self.saved_or("Window Width"), self.saved_or("Window Height")
        for sx, sz in sills:
            openings.append(self.opening("NYC_Window", sx - width / 2, sx + width / 2, sz, sz + height))
        panes = []
        for m in glass:  # a pane across the glass box's two long sides, at its middle
            size = np.linalg.norm(m[:3, :3], axis=0)
            turn = 0.0 if size[1] <= size[0] else math.pi / 2
            pane = frame(turn, m[0, 3], m[1, 3]) @ np.diag([max(size[0], size[1]), 1.0, size[2], 1.0])
            pane[2, 3] = m[2, 3]
            panes.append(("NYC_Pane", pane))
        placed += panes
        placed += rooms

        if back:  # the first floor brought down to be the ground floor, and brick over it to the first floor
            band = lambda zc: (gh - 0.01 < zc) & (zc < gh + fh - 0.01)
            down = np.eye(4)
            down[2, 3] = -gh
            placed += [(n, down @ m) for n, m in placed if band(m[2, 3]) and not n.startswith(NOT_BROUGHT)]
            openings += [(n, down @ m) for n, m in openings if band(m[2, 3])]
            # what stands wholly in the first floor's band comes down (a fire escape's ladder runs on past it)
            inside = lambda part: (part.v[part.t][..., 2].min(1) > gh - 0.01) & (part.v[part.t][..., 2].max(1) < gh + fh + 0.01)
            wall_parts += [part.keep(inside(part)).transformed(down) for part in list(wall_parts) if len(part) and inside(part).any()]
            cubes += [part.keep(inside(part)).transformed(down) for part in list(cubes[1:]) if len(part) and inside(part).any()]
            fill = quad([(0.0, 0.0, fh), (length, 0.0, fh), (length, 0.0, gh), (0.0, 0.0, gh)], BRICK + MASK)
            wall_parts.append(fill)

        moved = [(n, place @ m, None) for n, m in placed]
        openings = [(n, place @ m) for n, m in openings]
        wall = Soup.join(wall_parts).transformed(place)
        stone = wall.keep(np.array(wall.mats)[wall.m] == STONE) if len(wall) else wall
        return SimpleNamespace(
            start=start, along=along, out=out_n, length=length,
            wall=wall, inner=Soup.empty(), cubes=Soup.join(cubes).transformed(place), stone=stone,
            # tier 1's wall is flat, but for its quoins and stone bands, which stand proud of it
            flat=Soup.join([self.holed(start, along, out_n, length, floors, openings, {**tap.meshes, **self.opening_meshes}), stone]),
            rows=moved, openings=openings, bare=[])

    def far_panes(self, b, meshes, tier):
        """A window and a shop front at the coarse tiers: a dark pane on the flat wall (the sashes and
        shop glass stand back in their reveals, where a flat wall would hide them); and at tier 2 the
        quoins and stone bands, which are the walk-up's frame from afar."""
        panes = [lining(self.bounds_of(meshes, n), m, -0.03, "X_Pane", grow=0.0) for run in b.runs for n, m in run.openings]
        return panes + ([run.stone for run in b.runs] if tier == 2 else [])

    def flat_walls(self, loops, floors, wall, shade=1.0, trim=1.0):
        """The coarse tiers' walls: brick to the ground, as the piers between the shops are."""
        _, roof_m, _, _ = self.heights(floors)
        return Soup.join([quad([(ax, ay, 0.0), (bx, by, 0.0), (bx, by, roof_m), (ax, ay, roof_m)], wall, shade)
                          for loop in loops for (ax, ay), (bx, by) in loop])

    def far_tier(self, name, far, soup, tier, g):
        if name.startswith("NYC_WaterTank") and tier == 3:  # its tank and tower are its outline: a box would be a chimney
            return super().far_tier(name, far, soup, 2, self.FEATURE_M[2])
        return super().far_tier(name, far, soup, tier, g)

    def near_walls(self, run, tier):
        """The graph's brick and stone at tier 0; at tier 1 the same wall flat, its openings cut and lined."""
        return run.wall if tier == 0 else run.flat

    def saved_or(self, key):
        return self.INPUTS.get(key) if self.INPUTS.get(key) is not None else self.saved[key]

    def opening(self, kind, x0, x1, z0, z1):
        """An opening's place, and the flat mesh whose bounds are it (drawn by nothing)."""
        key = f"{kind}_{round(x1 - x0, 2)}x{round(z1 - z0, 2)}"
        if key not in self.opening_meshes:
            w, h = x1 - x0, z1 - z0
            self.opening_meshes[key] = quad([(-w / 2, 0.0, 0.0), (w / 2, 0.0, 0.0), (w / 2, 0.0, h), (-w / 2, 0.0, h)], "X_Void")
        m = np.eye(4)
        m[:3, 3] = ((x0 + x1) / 2, 0.0, z0)
        return key, m

    def __init__(self):
        super().__init__()
        self.opening_meshes = {}
        self.mirrored = set()

    def doors(self, b):
        south = min(cy - hy for _, cy, _, hy in b.rects)
        # a shop's door stands back in its doorway: the entrance is where its doorway meets the street wall
        return [tuple(run.start + run.along * float((m[:2, 3] - run.start) @ run.along)) for run in b.runs for n, m, _ in run.rows
                if n.startswith(self.ENTRANCE) and abs(run.start[1] - south) < 1e-6 and run.along[0] > 0.5]

    # ------------------------------------------------------------ the roof and crown
    def roof_rows(self, graph_, rect, floors, seed, tank):
        cx, cy, hx, hy = rect
        inputs = self.inputs(**{"Building Width": 2 * hx, "Building Depth": 2 * hy, "Floor Count": floors, "Facade Seed": seed,
                                "Roof Seed": seed + 11, "Water Tower Probability": tank, "Roof Access": True, "HVAC Units": 2,
                                "Vents": 3, "Dish Probability": 0.0, "Hatch Probability": 0.5, "Shop Count": 2,
                                "Fire Escape Side": "None"})
        _, rows = graph_.tap(inputs)
        head = self.GROUND_M + self.STOREY_M * (floors - 1)
        shift = np.eye(4)
        shift[:2, 3] = (cx - hx, cy - hy)
        return [(n, shift @ m, None) for n, m, _ in rows if m[2, 3] > head + 0.15 and n.split("~")[0] in ROOF_KINDS.values()]

    def intact_roof(self, b, roof_m, tier):
        return self.roof(b.rects, roof_m + self.ROOF_RISE_M, self.ROOF_CELL_M[tier], b.seed)

    def crown(self, loops, floors, tier, wall, shade=1.0, trim=1.0):
        """The parapet behind the graph's cornice, and its coping: the cornice is the facades' own."""
        _, roof_m, top_m, _ = self.heights(floors)
        walls = Soup.join([ring(loop, roof_m, top_m - 0.1, 0.0, self.WALL_M, wall, inner=True) for loop in loops])
        coping = Soup.join([ring(loop, top_m - 0.1, top_m, 0.02, self.WALL_M + 0.05, STONE, inner=True) for loop in loops]) if tier < 3 else Soup.empty()
        return Soup.join([walls.coloured(shade) if shade != 1.0 else walls, coping.coloured(trim) if trim != 1.0 else coping])

    # ------------------------------------------------------------ a block
    def assemble(self, graph_, template):
        name, floors, bays_x, bays_y, seed, shops_on, escapes_on, tank, brick_colour = template
        width, depth = 3.0 * bays_x + 2.0, 3.0 * bays_y + 2.0
        rect = (0.0, 0.0, width / 2, depth / 2)
        loops = outline([rect])
        runs, recipe = [], []
        for k, run in enumerate(loops[0]):
            side = "SENW"[k]  # `outline` walks a box from its south-west corner: south, east, north, west
            length = math.dist(*run)
            bays = round((length - 2.0) / 3.0)
            shops = bays // 3 if side in shops_on and bays % 3 == 0 else 0
            if side in shops_on and not shops:
                raise SystemExit(f"{name}: the {side} run's {bays} bays do not divide into 9 m shops")
            (tap, rows), inputs = self.evaluate(graph_, length, floors, seed + 20 * k, shops, side in escapes_on)
            runs.append(self.fit_rooms(self.facade(tap, rows, run, floors, back=not shops), [rect]))
            recipe.append({"from": list(run[0]), "to": list(run[1]), "shops": shops, "fire_escape": side in escapes_on,
                           "Building Width": inputs["Building Width"], "Building Depth": inputs["Building Depth"],
                           "Facade Seed": inputs["Facade Seed"]})
        on_roof = self.roof_rows(graph_, rect, floors, seed, tank)
        return SimpleNamespace(
            name=name, floors=floors, parts={"body": rect}, rects=[rect], loops=loops, runs=runs, seed=seed, wall=brick_colour,
            roof_rows=on_roof, rows=[row for run in runs for row in run.rows] + on_roof,
            recipe={"inputs": {k: (list(v) if isinstance(v, tuple) else v) for k, v in self.inputs().items()},
                    "facades": recipe, "roof": {"water_tank": tank}})


if __name__ == "__main__":
    NYC().main()
