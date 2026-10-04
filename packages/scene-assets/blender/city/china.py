"""The China apartment blocks: one shared kit and its templates.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/china.py [out_dir] [--dry]

writes `assets/source/city/china_apartments/kit.glb` and `templates.json` (the
format is in this folder's readme), or the two files into `out_dir`, and prints
what each template draws at each tier; with `--dry` it only prints. It needs the
ambientCG sets in the pack cache (`packs.py fetch ambientcg`).

The source is the vendored `CN_ApartmentBuilding.blend`, a geometry-nodes
building, evaluated with its two closing Realize Instances nodes muted in
memory so its instances can be read (`graph.py`). What every graph set shares
(outline, rooms, tiers, damage, the descriptor and the files) is `graphset.py`;
this is the China graph's own part:

- **Each straight run of a template's outline is one facade of the graph**,
  evaluated at that run's length; each part's roof furniture is the graph's for
  a building of its size.
- **The shell** is the graph's walls with their window openings, our bands,
  parapet and tiled roof, and the unit cubes the graph stretches into window
  surrounds and sign boards. A stretched cube cannot carry a baked texture, so
  they are folded into the shell with UVs in metres.
- **What is not opaque.** A cage's, a grille's and a railing's thin bars are
  cutout sheets; the graph's rain stains and leaf cards are cutouts of our own
  images.

A template's frame has its origin at the footprint's centre and its entrance
on -Y, the street side: the southmost run takes the graph's entrance facade.
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
from graphset import (GraphSet, bars, frame, grime, linear_of, outline, right_of, scorched,  # noqa: E402
                      toned)

STUCCO = "M_CN_WallStucco"
# The graph's four facades, each as the corner it starts at and the corner it ends at
# (in units of its Width along x and its Depth along y), walked with the street on the
# right. Facade 1 has the entrance and shops, 0 shops, 2 and 3 are backs.
FACADES = {0: ((0, 0), (1, 0)), 1: ((0, 1), (0, 0)), 2: ((1, 0), (1, 1)), 3: ((1, 1), (0, 1))}
ACROSS_M = 12.0  # the other side of a building evaluated for one facade; a facade does not depend on it
ROOF_FAMILIES = ("CNK_RoofBulk_", "CNK_RoofProp_", "CNK_RoofSmall_")


def streak():
    """The stain a wall carries under a sill or a pipe bracket: dribbles of dirty water,
    dark at the top where they start and thinning as they run down. One decal shows the
    whole image (FITTED), its top edge at the top. A decal is 25 pixels wide at 30 m, so
    the image is three or four broad dribbles and nothing finer: detail under an eighth of
    it is averaged away by then, and the cutout with it."""
    size = textures.SIZE
    yy, xx = np.mgrid[0:size, 0:size].astype(float) / size  # yy runs down from the top
    lanes = textures.fbm((5, 1), 951, 1)[0:1, :].repeat(size, 0)  # a value per column, the same all the way down
    # how far down each dribble runs, and a ragged end to it: a dribble that tapers to a point is a nail in the wall
    reach = 0.25 + 0.75 * textures.fbm((5, 1), 953, 1)[0:1, :].repeat(size, 0) + 0.06 * (textures.fbm((24, 1), 955, 1)[0:1, :].repeat(size, 0) - 0.5)
    edge = textures.smoothstep(0.0, 0.08, xx) * textures.smoothstep(1.0, 0.92, xx)
    cover = textures.smoothstep(0.42, 0.46, lanes) * (yy < reach) * edge
    # damp and dirt on paint, not tar: the wall's own colour (its row carries it) a shade down, so a stain's
    # hard edge is quiet on a wall of any colour
    colour = np.array((0.52, 0.5, 0.47)) * (0.85 + 0.3 * textures.fbm(6, 957, 2))[..., None]
    flat, ones = np.zeros((size, size)), np.ones((size, size))
    return textures.Baked(colour, 1.0, textures.normals_from_height(flat, 1.0), 1.0, ones, ones, coverage=cover)


def leaves():
    """Pot-plant foliage on a card: broad leaves, more of the card covered than not."""
    size = textures.SIZE
    f1, f2, ident = textures.worley(7, 961)
    blade = textures.smoothstep(0.62, 0.5, f1) * textures.smoothstep(0.0, 0.08, f2 - f1)
    shade = ((ident * 2654435761) % 97) / 96.0
    colour = np.array((0.045, 0.11, 0.03)) * (0.6 + 0.7 * shade)[..., None] * (0.8 + 0.4 * textures.fbm(10, 963, 3))[..., None]
    height = textures.blur(blade * (1.0 - f1))
    ones = np.ones((size, size))
    return textures.Baked(colour, 1.0, textures.normals_from_height(height, 1.0), 1.0, ones, ones, coverage=blade)


class China(GraphSet):
    SET = "china_apartments"
    BLEND = "vendor/procedural-buildings/CN_ApartmentBuilding.blend"
    SCRIPT = "china.py"
    ID_PREFIX, FAMILY = "china-apartment-", "china"

    # Every input the graph has, set each time: the file is saved in a state that is
    # not its defaults. Bays are 3 m, and floors 3 m over a 3.2 m ground floor (S2).
    # What is switched off has no place in a building of ours: the street (L5), the
    # rooftop sign (unique text and a cable mesh), lit rooms and anything that glows,
    # the shops' pavement clutter, and the rain-streak decals (alpha-blended; the
    # grime is in the textures instead). The drainpipe input does nothing: in the
    # vendored file the nodes that instance the pipes point at no object.
    INPUTS = {
        "Ground Floor Height": 3.2, "Floor Height": 3.0, "Bay Width": 3.0, "Wall Thickness": 0.3,
        "Detail Level": "LOD0", "Facade Finish": "Stucco", "Wall Tint": (1.0, 1.0, 1.0, 1.0), "Stone Ground Floor": True,
        "Floor Bands": True, "Parapet Height": 1.1, "Weathering": 0.5,
        "Window Width": 1.5, "Window Height": 1.5, "Sill Height": 0.9, "Small Window Probability": 0.15,
        "Curtain Probability": 0.35, "Lit Window Probability": 0.0,
        "Balcony Probability": 0.35, "Enclosed Balcony Probability": 0.45, "Balcony Min Floor": 1,
        "Security Grille Probability": 0.45, "AC Unit Probability": 0.5, "Laundry Probability": 0.35,
        "Plant Probability": 0.3, "Drainpipe Probability": 0.7,
        "Shutter Probability": 0.3, "Stall Probability": 0.15, "Awning Probability": 0.35, "Lantern Probability": 0.35,
        "Blade Sign Probability": 0.3, "Shop Props": False,
        "Rooftop Sign": False, "Sign Text": "", "Sign Height": 2.0,
        "Stair Bulkhead": True, "Antennas": 2,
        "Sidewalk": False, "Sidewalk Width": 4.0, "Curb Height": 0.15, "Corner Radius": 3.0, "Tactile Paving": False,
        "Street Trees": False, "Street Lamps": False, "Street Props": False,
    }
    GROUND_M, STOREY_M, WALL_M, PARAPET_M = INPUTS["Ground Floor Height"], INPUTS["Floor Height"], INPUTS["Wall Thickness"], \
        INPUTS["Parapet Height"]
    # A template: its id's tail, floors, parts as {id: (centre x, centre y, half x, half y)} in
    # the template's frame, the graph's seed and shop seed, each part's roof furniture (solar
    # heaters, props, vents), the wall colour (sRGB), and inputs of its own. A run of the
    # outline is 3n + 2 m (1 m corner piers) or 3n m (1.5 m piers), so its windows are the
    # whole 3 m lattice of every edge it crosses; a compound's parts are sized for that.
    TEMPLATES = (
        ("slab-35x11-4f", 4, {"body": (0, 0, 17.5, 5.5)}, 11, 31, (3, 5, 3), (233, 229, 218), {}),
        ("slab-47x11-5f", 5, {"body": (0, 0, 23.5, 5.5)}, 12, 32, (4, 7, 4), (226, 214, 184), {}),
        ("slab-59x14-6f", 6, {"body": (0, 0, 29.5, 7.0)}, 13, 33, (6, 10, 6), (200, 208, 200), {}),
        ("slab-53x14-8f", 8, {"body": (0, 0, 26.5, 7.0)}, 14, 34, (5, 9, 5), (224, 204, 192), {}),
        ("point-20x20-7f", 7, {"body": (0, 0, 10.0, 10.0)}, 15, 35, (3, 5, 3), (204, 211, 219), {}),
        # a 42 x 12 m front on the street and two 12 x 20 m wings behind it: a U open to the back
        ("block-u-5f", 5, {"front": (0, -10, 21, 6), "west": (-15, 6, 6, 10), "east": (15, 6, 6, 10)}, 16, 36, (2, 3, 2),
         (230, 220, 200), {}),
        # four 12 m wings round a 17 x 14 m court
        ("block-court-6f", 6, {"south": (0, -13, 20.5, 6), "north": (0, 13, 20.5, 6), "west": (-14.5, 0, 6, 7),
                               "east": (14.5, 0, 6, 7)}, 17, 37, (2, 3, 2), (214, 206, 196),
         # 408 bays in one budget: a little less on its walls than a slab has
         {"Plant Probability": 0.15, "AC Unit Probability": 0.4, "Laundry Probability": 0.28}),
    )

    FAMILIES = (
        ("CNK_Win_", 0b1111), ("CNK_Shop_", 0b1111), ("CNK_Ent_", 0b1111), ("CNK_Balc_", 0b1111),
        ("CNK_RoofBulk_", 0b1111),
        ("CNK_AC_", 0b0111), ("CNK_Awn_", 0b0111), ("CNK_RoofProp_", 0b0111), ("CNK_Laundry", 0b0111),
        ("CNK_Grille_", 0b0011), ("CNK_RoofSmall_", 0b0011), ("CNK_Room_", 0b0011),
        ("CNK_Text", 0b0001), ("CNK_Lant_", 0b0001), ("CNK_Plant_", 0b0001), ("CNK_Decal_", 0b0001), ("CNK_Curt_", 0b0001),
    )
    # Families whose finest mesh keeps a larger feature than the tier's: what hangs behind glass (a curtain's
    # pleats, the washing on a glazed-in balcony) and what is a hand across (a pot, a lantern's ribs).
    TIER0_FEATURE_M = (("CNK_Curt_", 0.25), ("CNK_LaundryBalc", 0.25), ("CNK_Plant_", 0.12), ("CNK_Lant_", 0.1))
    OPENINGS = ("CNK_Win_", "CNK_Shop_", "CNK_Ent_")
    HULLS = ("CNK_Balc_",)
    HULL_BAND_M = (1.0, 1.3)  # at tiers 2 and 3: a glazed balcony keeps its pale parapet under its dark glass
    # Behind every window and shop front the graph stands a room: a unit box scaled to the bay,
    # the floor and the depth the building has there.
    ROOMS = {"CNK_Int_00_Room": "CNK_Room_Home", "CNK_Int_01_RoomB": "CNK_Room_Home", "CNK_Int_04_Shop": "CNK_Room_Shop"}
    ROOM_SHEETS = {"X_Room": "rooms", "X_ShopRoom": "shops"}
    ROOM_MESHES = {"CNK_Room_Home": "X_Room", "CNK_Room_Shop": "X_ShopRoom"}
    # The graph's own furnished shop interiors: the atlas's shops stand for them.
    INSIDE = ("CNK_ShopInt_",)
    # Inside a glazed-in balcony: its laundry and the door onto it, seen through its glass.
    ON_BALCONY = ("CNK_LaundryBalc", "CNK_Win_05_BalcDoor")
    GLAZED = "CNK_Balc_01_Enclosed"
    SHEETED = ("CNK_Grille_", "CNK_Balc_00_OpenRail")
    # Sign text is the kit's own generic shop words (tea, pharmacy, fast food). This one names a real city.
    SIGN_SWAPS = {"CNK_Text_01": "CNK_Text_09"}
    KIT_PREFIX, DECAL, ENTRANCE = "CNK_", "CNK_Decal_", "CNK_Ent_"
    # The set's bays are owned by the graph's window lattice (`descriptor`); a few back ground-floor bays
    # have no opening over the eye and muzzle heights, and are printed, not refused.
    BAYS_REFUSED = False

    # The graph's 42 materials read 26 ambientCG sets at 1K and 2K. Ours share nine
    # recipes at the size every texture in the game has; a material is a recipe at
    # its own base colour, roughness and metalness.
    WALL, STONE, BAND, ROOF, GLASS = STUCCO, "M_CN_Stone", "M_CN_Band", "M_CN_RoofTile", "M_CN_Glass"
    SURFACES = {
        STUCCO: ("cn_wall_stucco", "cn_stucco", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_Band": ("cn_band", "cn_concrete", (0.7, 0.72, 0.75), 1.0, 0.0),
        "M_CN_Stone": ("cn_stone", "cn_stone", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_RoofTile": ("cn_roof_tile", "cn_roof_tile", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_Concrete": ("cn_concrete", "cn_concrete", (0.9, 0.9, 0.9), 1.0, 0.0),
        "M_CN_Brick": ("cn_brick", "cn_concrete", (0.62, 0.3, 0.24), 1.0, 0.0),
        "M_CN_Aluminum": ("cn_aluminium", "cn_metal", (1.0, 1.0, 1.0), 0.8, 1.0),
        "M_CN_SteelBlack": ("cn_steel_black", "cn_metal", (0.07, 0.07, 0.075), 1.0, 0.5),
        "M_CN_Stainless": ("cn_stainless", "cn_metal", (1.0, 1.0, 1.0), 0.9, 1.0),
        "M_CN_PaintWhite": ("cn_paint_white", "cn_plastic", (0.9, 0.9, 0.9), 1.0, 0.0),
        "M_CN_Rust": ("cn_rust", "cn_concrete", (0.5, 0.22, 0.11), 1.0, 0.0),
        "M_CN_Shutter": ("cn_shutter", "cn_corrugated", (1.0, 1.0, 1.0), 1.0, 1.0),
        "M_CN_Plastic": ("cn_plastic", "cn_plastic", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_PlasticDirty": ("cn_plastic_dirty", "cn_plastic", (0.72, 0.7, 0.64), 1.0, 0.0),
        "M_CN_Fabric": ("cn_fabric", "cn_fabric", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_Wood": ("cn_wood", "cn_wood", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_Rubber": ("cn_rubber", "cn_plastic", (0.04, 0.04, 0.04), 1.0, 0.0),
        "M_CN_Soil": ("cn_soil", "cn_concrete", (0.3, 0.22, 0.15), 1.0, 0.0),
        "M_CN_IntWall": ("cn_int_wall", "cn_stucco", (0.92, 0.88, 0.8), 1.0, 0.0),
        "M_CN_IntLit": ("cn_int_wall", "cn_stucco", (0.92, 0.88, 0.8), 1.0, 0.0),
        "M_CN_IntShop": ("cn_int_shop", "cn_stucco", (0.98, 0.98, 1.0), 1.0, 0.0),
        "M_CN_IntFloor": ("cn_int_floor", "cn_wood", (0.8, 0.75, 0.7), 1.0, 0.0),
        "M_CN_SignBoard": ("cn_sign_board", "cn_plastic", (1.0, 1.0, 1.0), 0.8, 0.0),
        "M_CN_RedPaper": ("cn_red_paper", "cn_fabric", (0.75, 0.04, 0.03), 1.0, 0.0),
        "M_CN_Lantern": ("cn_lantern", "cn_fabric", (0.85, 0.05, 0.03), 1.0, 0.0),
        "M_CN_Gold": ("cn_gold", "cn_metal", (0.95, 0.72, 0.28), 0.8, 1.0),
        # glass is blended over the room behind it (COVERAGE); from tier 2 out a window is a dark, glossy pane
        "M_CN_Glass": ("cn_glass", None, (0.05, 0.075, 0.09), 0.08, 0.0),
        "X_Pane": ("cn_pane", None, (0.06, 0.066, 0.068), 0.25, 0.0),
        # a frosted pane and a strip curtain stay opaque: they are there to hide what is behind them
        "M_CN_GlassFrosted": ("cn_glass_frosted", None, (0.19, 0.22, 0.23), 0.4, 0.0),
        "M_CN_PVC": ("cn_pvc", None, (0.22, 0.3, 0.29), 0.3, 0.0),
        "M_CN_SolarTube": ("cn_solar_tube", None, (0.03, 0.05, 0.09), 0.15, 0.3),
        "M_CN_SignText": ("cn_sign_text", None, (1.0, 1.0, 1.0), 0.4, 0.0),
        # a lamp, unlit: nothing glows
        "M_CN_Emissive": ("cn_lamp", None, (0.8, 0.78, 0.72), 0.3, 0.0),
        # ours: the dark behind an opening, and what a fire and a collapse leave
        "X_Void": ("cn_void", None, (0.005, 0.005, 0.006), 1.0, 0.0),
        "X_Burnt": ("cn_wall_burnt", "cn_burnt", (1.0, 1.0, 1.0), 1.0, 0.0),
        "X_Rubble": ("cn_rubble", "cn_concrete", (0.66, 0.63, 0.58), 1.0, 0.0),
        "X_Soot": ("cn_soot", "cn_concrete", (0.5, 0.48, 0.46), 1.0, 0.0),
        # ours: cutouts (the stain a wall carries under a sill, a pot plant's leaves) and the rooms
        "M_CN_Decal": ("cn_streak", "cn_streak", (1.0, 1.0, 1.0), 1.0, 0.0),
        "M_CN_Leaves": ("cn_leaves", "cn_leaves", (1.0, 1.0, 1.0), 0.75, 0.0),
        "X_Room": ("cn_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
        "X_ShopRoom": ("cn_shop_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
    }
    COVERAGE = {"M_CN_Glass": ("blended", 0.35), "M_CN_Decal": ("cutout", 0.5), "M_CN_Leaves": ("cutout", 0.5)}
    FITTED = frozenset(("M_CN_Decal",))
    TAKES_COLOUR = frozenset((
        STUCCO, "M_CN_Aluminum", "M_CN_SteelBlack", "M_CN_PaintWhite", "M_CN_Shutter", "M_CN_Plastic", "M_CN_PlasticDirty",
        "M_CN_Fabric", "M_CN_Wood", "M_CN_IntWall", "M_CN_SignBoard", "M_CN_SignText", "M_CN_Emissive",
        "M_CN_Decal",  # ours: a stain's row carries its wall's colour (`assemble`)
    ))
    LEFT_OUT = frozenset(("M_CN_Bark",))
    TURNED = frozenset(("M_CN_Shutter",))
    BARS = "cn_bars"

    BURNS = frozenset(("M_CN_Glass", "M_CN_GlassFrosted", "M_CN_PVC", "M_CN_Fabric", "M_CN_Lantern", "M_CN_RedPaper"))
    SURVIVES = (
        ("CNK_Balc_", 1.0, ("burnt",)), ("CNK_Win_", 0.25, ("burnt",)), ("CNK_Grille_", 0.5, ("hanging", "burnt")),
        ("CNK_AC_", 0.45, ("burnt",)), ("CNK_Shop_02_", 1.0, ("burnt",)), ("CNK_Shop_03_", 1.0, ("burnt",)),
        ("CNK_Ent_", 1.0, ("burnt",)), ("CNK_RoofBulk_", 1.0, ("burnt",)), ("CNK_RoofProp_", 0.6, ("burnt",)),
        ("CNK_RoofSmall_", 0.5, ("burnt",)),
    )
    WRECKS = ("CNK_AC_00_Std", "CNK_AC_02_Old", "CNK_Grille_00_CageSS", "CNK_Grille_01_CageBlack", "CNK_Balc_00_OpenRail",
              "CNK_Win_00_Slide2", "CNK_Shop_02_ShutterDown", "CNK_Laundry_01_Bamboo")

    def recipes(self):
        # the stucco covers 4.8 m (the set twice each way) so its streaks do not repeat bay to bay
        ambientcg.bake("cn_stucco", "PaintedPlaster017", 2.4, repeat=2, tint=(1.0, 0.985, 0.955), rough=(0.9, 0.08),
                       normal=0.55, grime=grime(0.35, 0.3))
        ambientcg.bake("cn_burnt", "PaintedPlaster017", 2.4, repeat=2, tint=(1.0, 0.985, 0.955), rough=(0.9, 0.08),
                       normal=0.55, grime=scorched)
        ambientcg.bake("cn_concrete", "Concrete034", 2.0, rough=(1.0, 0.05), normal=0.6, grime=grime(0.35, 0.3))
        # the ground floor is a plinth of mid-grey stone under the shop fronts; the set's own is near black
        ambientcg.bake("cn_stone", "Tiles138", 1.2, normal=0.8, grime=toned((0.2, 0.19, 0.175), 0.6))
        # the roof is the largest surface in every view of a town: a dull, weathered clay, its dirt in the mesh
        ambientcg.bake("cn_roof_tile", "Tiles047", 1.2, repeat=4, rough=(1.0, 0.05), normal=0.8,
                       grime=toned((0.2, 0.115, 0.085), 0.5, level_cycles=10))
        ambientcg.bake("cn_metal", "Metal009", 1.0, rough=(1.0, 0.05), normal=0.4)
        ambientcg.bake("cn_corrugated", "CorrugatedSteel005", 1.0)
        ambientcg.bake("cn_plastic", "Plastic010", 1.0, rough=(0.8, 0.12), normal=0.3)
        ambientcg.bake("cn_fabric", "Fabric036", 0.6, rough=(1.0, 0.1), normal=0.6)
        ambientcg.bake("cn_wood", "WoodFloor041", 1.5, rough=(1.0, 0.05), normal=0.6)
        # the three cutouts share one image of occlusion, roughness and metalness (all ones: a bundle stores an
        # image once), and each material's own factors say what it is
        textures.recipe(self.BARS, tile=0.5)(bars)
        textures.recipe("cn_streak", tile=1.0)(streak)
        textures.recipe("cn_leaves", tile=0.45)(leaves)

    # ------------------------------------------------------------ the graph
    def open_graph(self):
        graph.open_blend(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), self.BLEND))
        groups = bpy.data.node_groups
        # in memory only: the building as instances, and the tint the graph stores on each
        groups["CN_Facade"].nodes["Realize Instances.001"].mute = True
        groups["CN_Building"].nodes["Realize Instances.003"].mute = True
        for node in groups["CN_Finalize"].nodes:
            if node.bl_idname == "GeometryNodeRemoveAttribute":
                node.mute = True
        return graph.modifier("CN_CornerApartment", "CN_Building")

    def drawn(self, tap):
        """The instances a building of ours draws, from all the graph makes: its room boxes
        as ours, and not its furnished shop interiors."""
        out = []
        for name, m, tint in tap.rows:
            if name.startswith(self.INSIDE):
                continue
            if name in self.ROOMS:
                out.append((self.ROOMS[name], m, None))
            elif name.startswith("CNK_Curt_") and tint is not None:  # the graph's are flags; cloth behind glass has faded
                grey = sum(tint[:3]) / 3
                out.append((name, m, (*(0.85 * (grey + (c - grey) * 0.45) for c in tint[:3]), 1.0)))
            else:
                out.append((self.SIGN_SWAPS.get(name, name), m, tint))
        return out

    # ------------------------------------------------------------ facades and roofs
    def facade_of(self, run, south, east):
        """Which of the graph's facades dresses a run of the outline: the entrance facade on
        the street (the southmost runs), shops on the east end, backs everywhere else."""
        (ax, ay), (bx, by) = run
        n = right_of(*run)
        if n[1] < -0.5:
            return 1 if ay == south else 2
        if n[0] > 0.5:
            return 0 if ax == east else 3
        return 2 if n[1] > 0.5 else 3

    def run_inputs(self, length, side, floors, seed, shop_seed, own):
        """The inputs that make `side` a facade `length` long. Its corner piers are 1 m when
        the length is 3n + 2 and 1.5 m when it is 3n; no other length keeps windows off a corner
        and on every point of the lattice."""
        bays = math.floor((length - 2.0) / self.BAY_M + 1e-9)
        margin = (length - self.BAY_M * bays) / 2
        if margin > 1.5 + 1e-9:
            raise SystemExit(f"a {length} m run needs {margin} m corner piers: make it 3n or 3n + 2 metres")
        size = {"Width": length, "Depth": ACROSS_M} if side in (0, 3) else {"Width": ACROSS_M, "Depth": length}
        return dict(self.INPUTS, **own, **size, **{"Corner Margin": margin, "Floors": floors, "Seed": seed, "Shop Seed": shop_seed,
                                                   "Solar Heaters": 0, "Roof Props": 0, "Roof Vents": 0})

    def facade(self, tap, rows, side, run, roof_m):
        """One facade of an evaluated building, moved onto a run of the outline: where it
        stands, its wall (the faces we can see of it, and apart from them its inner face),
        its instances, its stretched cubes as one mesh, and every opening in it."""
        inputs_w, inputs_d = (math.dist(*run), ACROSS_M) if side in (0, 3) else (ACROSS_M, math.dist(*run))
        (sx, sy), (ex, ey) = FACADES[side]
        start, end = np.array([sx * inputs_w, sy * inputs_d]), np.array([ex * inputs_w, ey * inputs_d])
        length = np.linalg.norm(end - start)
        along = (end - start) / length
        out = np.array([along[1], -along[0]])
        depth = lambda p: -(p[..., :2] - start) @ out
        turn = math.atan2(run[1][1] - run[0][1], run[1][0] - run[0][0]) - math.atan2(along[1], along[0])
        c, s = math.cos(turn), math.sin(turn)
        place = frame(turn, run[0][0] - (c * start[0] - s * start[1]), run[0][1] - (s * start[0] + c * start[1]))

        wall_m = self.WALL_M
        own = tap.own
        name = np.array(own.material_names())
        normals, _ = own.normals()
        centre = own.v[own.t].mean(1)
        deep, far = depth(centre), (centre[:, :2] - start) @ along
        facing = normals[:, :2] @ out
        ours = (deep > -0.01) & (deep < wall_m + 0.01) & (centre[:, 2] < roof_m) & ((name == STUCCO) | (name == "M_CN_Stone"))
        # a corner's block of wall belongs to both facades that meet there: each takes its own faces
        ours &= (np.abs(facing) > 0.5) | ((far > wall_m + 0.01) & (far < length - wall_m - 0.01))
        # the inner face is never seen while the building stands: every opening is lined
        inner = ours & (facing < -0.5) & (deep > wall_m - 1e-3)
        placed, cubes = [], []
        for name, m, tint in rows:
            if name.startswith(ROOF_FAMILIES) or depth(m[:3, 3]) > 0.8:
                continue
            if name.startswith("CNK_Decal_") and m[2, 3] + tap.meshes[name].bounds()[0][2] * m[2, 2] < 0.02:
                continue  # a stain that would run on into the ground
            if not name.startswith("primitive:"):
                placed.append((name, place @ m, tint))
                continue
            cube = self.masked(tap.meshes[name], False).transformed(m).coloured(1.0 if tint is None else tint[:3])
            normals, _ = cube.normals()
            # the face a cube turns to the wall it sits on is never seen
            against = (normals[:, :2] @ out < -0.5) & (depth(cube.v[cube.t].mean(1)) > -0.02)
            cubes.append(cube.keep(~against).transformed(place))
        openings = [(name, place @ m) for name, m, _ in tap.rows if name.startswith(self.OPENINGS) and depth(m[:3, 3]) < 0.8]
        # a room covers the opening it stands behind, top to bottom, under the floor above; an opening with none is bare
        ground_m, storey_m = self.GROUND_M, self.STOREY_M
        spans = []
        for name, m in openings:
            lo, hi = tap.meshes[name].bounds()
            spans.append((float((m[:2, 3] - run[0]) @ ((np.array(run[1]) - run[0]) / length)), m[2, 3] + lo[2] * m[2, 2], m[2, 3] + hi[2] * m[2, 2]))
        roomed = set()
        for k, (name, m, tint) in enumerate(placed):
            if name not in self.ROOM_MESHES:
                continue
            s_room = float((m[:2, 3] - run[0]) @ ((np.array(run[1]) - run[0]) / length))
            z0, z1 = max(m[2, 3], self.ROOM_STEP_M), m[2, 3] + m[2, 2]
            ceiling = (ground_m if z0 < ground_m - 1.0 else ground_m + storey_m * (math.floor((z0 - ground_m + 0.5) / storey_m) + 1)) - 0.03
            for i, (s_open, o0, o1) in enumerate(spans):
                if abs(s_open - s_room) < 0.6 and o0 < z1 and o1 > z0:
                    roomed.add(i)
                    z0, z1 = max(self.ROOM_STEP_M, min(z0, o0 - 0.03)), min(ceiling, max(z1, o1 + 0.05))
            m = m.copy()
            m[2, 3], m[2, 2] = z0, z1 - z0
            placed[k] = (name, m, tint)
        bare = [opening for i, opening in enumerate(openings) if i not in roomed]

        # a stain is on the wall: the graph hangs some where a door onto a balcony has taken the wall away
        def on_wall(name, m):
            if not name.startswith("CNK_Decal_"):
                return True
            lo, hi = tap.meshes[name].bounds()
            s_at = float((m[:2, 3] - run[0]) @ ((np.array(run[1]) - run[0]) / length))
            half, z0, z1 = max(-lo[0], hi[0]) * float(np.hypot(m[0, 0], m[1, 0])), m[2, 3] + lo[2] * m[2, 2], m[2, 3] + hi[2] * m[2, 2]
            for (o_name, o_m), (s_open, o0, o1) in zip(openings, spans):
                o_lo, o_hi = tap.meshes[o_name].bounds()
                o_half = max(-o_lo[0], o_hi[0]) * float(np.hypot(o_m[0, 0], o_m[1, 0]))
                if abs(s_at - s_open) < half + o_half and z0 < o1 and z1 > o0:
                    return False
            return True

        placed = [row for row in placed if on_wall(row[0], row[1])]
        direction = np.array([run[1][0] - run[0][0], run[1][1] - run[0][1]]) / length
        return SimpleNamespace(
            start=np.array(run[0], dtype=np.float64), along=direction, out=np.array([direction[1], -direction[0]]), length=length,
            wall=self.masked(own.keep(ours & ~inner), True).transformed(place), inner=self.masked(own.keep(inner), True).transformed(place),
            rows=placed, cubes=Soup.join(cubes), openings=openings, bare=bare)

    def roof_rows(self, graph_, part, floors, seed, roof, own):
        """A part's roof furniture: the graph's for a building of the part's size."""
        cx, cy, hx, hy = part
        inputs = dict(self.INPUTS, **own, **{"Width": 2.0 * hy, "Depth": 2.0 * hx, "Corner Margin": 1.0, "Floors": floors, "Seed": seed,
                                             "Shop Seed": 0, "Solar Heaters": roof[0], "Roof Props": roof[1], "Roof Vents": roof[2]})
        _, rows = graph_.tap(inputs)
        place = frame(math.pi / 2, cx + hx, cy - hy)
        return [(name, place @ m, tint) for name, m, tint in rows if name.startswith(ROOF_FAMILIES)]

    def assemble(self, graph_, template):
        """A template built: its outline, each run of it dressed (`facade`), its instances in
        its own frame, and what made them."""
        name, floors, parts_, seed, shop_seed, roof_, wall, own = template
        rects = list(parts_.values())
        loops = outline(rects)
        south, east = min(cy - hy for _, cy, _, hy in rects), max(cx + hx for cx, _, hx, _ in rects)
        _, roof_m, _, _ = self.heights(floors)
        runs, recipe, used = [], [], {}
        for run in (run for loop in loops for run in loop):
            side = self.facade_of(run, south, east)
            # each run of a compound is its own stretch of wall: another seed for each facade of a kind
            k = used[side] = used.get(side, -1) + 1
            inputs = self.run_inputs(math.dist(*run), side, floors, seed + k, shop_seed + k, own)
            runs.append(self.fit_rooms(self.facade(*graph_.tap(inputs), side, run, roof_m), rects))
            recipe.append({"from": list(run[0]), "to": list(run[1]), "facade": side, "Seed": seed + k, "Shop Seed": shop_seed + k,
                           "Corner Margin": inputs["Corner Margin"]})
        paint = linear_of(wall)
        for run in runs:  # a stain is the wall's own colour, darker
            run.rows = [(n, m, (*paint, 1.0) if n.startswith("CNK_Decal_") else tint) for n, m, tint in run.rows]
        on_roof = [row for k, part in enumerate(rects) for row in self.roof_rows(graph_, part, floors, seed + k, roof_, own)]
        return SimpleNamespace(
            name=name, floors=floors, parts=parts_, rects=rects, loops=loops, runs=runs, seed=seed, wall=wall, roof_rows=on_roof,
            rows=[row for run in runs for row in run.rows] + on_roof,
            recipe={"inputs": {k: (list(v) if isinstance(v, tuple) else v) for k, v in {**self.INPUTS, **own}.items()},
                    "facades": recipe, "roof": list(roof_)})


if __name__ == "__main__":
    China().main()
