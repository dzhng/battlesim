"""The China apartment blocks: one shared kit and its templates.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/china.py

writes `assets/source/city/china_apartments/kit.glb` and `templates.json` (the
format is in this folder's readme) and prints what each template draws at each
tier; with `-- --dry` it only prints. It needs the ambientCG sets in the pack
cache (`packs.py fetch ambientcg`).

The source is the vendored `CN_ApartmentBuilding.blend`, a geometry-nodes
building, evaluated with its two closing Realize Instances nodes muted in
memory so its instances can be read (`graph.py`).

- **A template is its parts**, boxes that abut (a slab is one; a U is three).
  Only the outline of their union is built: each straight run of it is one
  facade of the graph, evaluated at that run's length, so a face where two
  parts join has no wall, window, pier or cornice to hide, and a corner of the
  block is a corner of one building. The bands, the parapet and its coping
  follow the outline, and each part's roof and roof furniture is the graph's
  for a building of its size.
- **Kit modules** are the graph's own kit meshes (a window, a balcony, an air
  conditioner). A row places one: position, yaw, scale per axis, and the tint
  the graph stored on the instance.
- **A template's shell** is what is made for it alone: the walls with their
  window openings, the bands, parapet and roof, and the unit cubes the graph
  stretches into window surrounds and sign boards. A stretched cube cannot
  carry a baked texture, so they are folded into the shell with UVs in metres.
- **What is not opaque** (README, "Surfaces that are not opaque" and
  "Interiors"). Glass is blended, one face a pane as the graph models it.
  Behind every window and shop front the graph stands a room box; ours is the
  same box showing a cell of the interior atlas, a row a window, fitted to the
  plan so that no two rooms share space. A cage's, a grille's and a railing's
  thin bars are cutout sheets in the bars' own colour (`sheeted`), and the
  graph's rain stains and leaf cards are cutouts of our own images.
- **Tiers.** A tier keeps features larger than its `FEATURE_M` (`detail.py`),
  and each kit family draws down to the tier its `FAMILIES` row names. At the
  two fine tiers a kit mesh is a row; at the two coarse ones it is folded into
  the shell, so a far building is one row. Rooms, glass and cutouts are the
  fine tiers'; from tier 2 a window is a dark pane on a flat wall.
- **Damage states.** The graph has no damage inputs, so a destroyed building is
  made here from the intact one (`damage.py`). Six floors or fewer collapse to a
  `ruin` inside the simulation's remains box: the same walls cut to ragged
  stumps, a heap of their rubble over the plan, fallen floors, fittings thrown
  down. Taller ones stand `gutted`: every opening empty and sooted, a few bays
  blown out to the floor slabs, the fittings charred or gone, no glass and no
  room. Each state has a
  shell of its own and is one row from far off, like the intact building.

A template's frame has its origin at the footprint's centre and its entrance
on -Y, the street side: the southmost run takes the graph's entrance facade.
"""
import json
import math
import os
import random
import sys
from types import SimpleNamespace

import bpy
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
BLENDER = os.path.dirname(HERE)
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BLENDER)))
sys.path.insert(0, HERE)
sys.path.insert(0, BLENDER)
import ambientcg  # noqa: E402
import collapse  # noqa: E402
import damage  # noqa: E402
import detail  # noqa: E402
import graph  # noqa: E402
import parts  # noqa: E402
import textures  # noqa: E402
from graph import Soup  # noqa: E402

SET = "china_apartments"
BLEND = "vendor/procedural-buildings/CN_ApartmentBuilding.blend"
OUT = os.path.join(ROOT, "assets/source/city", SET)

# ---------------------------------------------------------------- the recipes
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
FIT = {"side_m": 1.5, "top_m": 3.5}
BUDGET = (150_000, 50_000, 12_000, 2_000)
BAY_M = 3.0
WHITE = (255, 255, 255)

# ---------------------------------------------------------------- tiers
# The smallest feature each tier keeps, in metres, and the thinnest a bar is drawn at it. The
# tactical camera draws tier 0 at about 20 pixels a metre: a bar under 5 cm is under a pixel.
FEATURE_M = (0.05, 0.25, 0.6, 1.5)
BAR_M = (0.05, 0.125, 0.3, 0.75)
# Kit family (a prefix of the source object's name) -> the tiers it draws at, as
# the row's bits. A family not named here is not part of a building.
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
ROW_TIERS = 0b0011  # drawn as rows; the coarser tiers are folded into the shell
# What fills a wall opening: at the two coarse tiers the wall is flat and each is one quad on it.
OPENINGS = ("CNK_Win_", "CNK_Shop_", "CNK_Ent_")
# A box hung on the wall: at the two coarse tiers a hull of its bounds (`detail.hull`),
# banded by height, then only its front.
HULLS = ("CNK_Balc_",)
HULL_BAND_M = (1.0, 1.3)  # at tiers 2 and 3: a glazed balcony keeps its pale parapet under its dark glass
# Behind every window and shop front the graph stands a room: a unit box scaled to the bay,
# the floor and the depth the building has there. Ours is the same box, open to the wall,
# showing a cell of the interior atlas (README, "Interiors"): rows, at the two fine tiers.
ROOMS = {"CNK_Int_00_Room": "CNK_Room_Home", "CNK_Int_01_RoomB": "CNK_Room_Home", "CNK_Int_04_Shop": "CNK_Room_Shop"}
ROOM_SHEETS = {"X_Room": "rooms", "X_ShopRoom": "shops"}
ROOM_MESHES = {"CNK_Room_Home": "X_Room", "CNK_Room_Shop": "X_ShopRoom"}
ROOM_CLEAR_M = 0.35  # a room stops this far short of the middle of the building and of a corner's other room
ROOM_STEP_M = 0.12  # a ground-floor room's floor is this far over the ground, which would show through it
# An opening with no room behind it (the entrance) is lined: a dark, matte pane stands in it
# just inside the wall's thickness. A block is hollow: without one the eye goes in there and
# out at the far side. A burnt block has no rooms, and every opening of it is lined.
LINING_M = 0.28
# The graph's own furnished shop interiors: the atlas's shops stand for them.
INSIDE = ("CNK_ShopInt_",)
# Inside a glazed-in balcony: its laundry and the door onto it, seen through its glass.
ON_BALCONY = ("CNK_LaundryBalc", "CNK_Win_05_BalcDoor")
# Whose thin bars are drawn as a cutout sheet (`sheeted`): a cage's, a flat grille's, a railing's.
SHEETED = ("CNK_Grille_", "CNK_Balc_00_OpenRail")
SHEET_BAR_M = 0.022  # a bar thinner than this, one of a row of them, is the sheet's
# Sign text is the kit's own generic shop words (tea, pharmacy, fast food). This one names a real city.
SIGN_SWAPS = {"CNK_Text_01": "CNK_Text_09"}

# ---------------------------------------------------------------- materials
# The graph's 42 materials read 26 ambientCG sets at 1K and 2K. Ours share nine
# recipes at the size every texture in the game has; a material is a recipe at
# its own base colour, roughness and metalness.
STUCCO = "M_CN_WallStucco"
# source material -> (ours, recipe or None for a flat colour, base colour (linear), roughness, metalness)
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
# Surfaces that are not opaque (`textures.surface`). A cutout's coverage is its recipe's image.
COVERAGE = {"M_CN_Glass": ("blended", 0.35), "M_CN_Decal": ("cutout", 0.5), "M_CN_Leaves": ("cutout", 0.5)}
# A surface whose image is fitted to the mesh it is on (a decal is one picture), not tiled in metres.
FITTED = frozenset(("M_CN_Decal",))
# The source shader multiplies these by the kit's colour attribute (`col=True` in its material script).
TAKES_COLOUR = frozenset((
    STUCCO, "M_CN_Aluminum", "M_CN_SteelBlack", "M_CN_PaintWhite", "M_CN_Shutter", "M_CN_Plastic", "M_CN_PlasticDirty",
    "M_CN_Fabric", "M_CN_Wood", "M_CN_IntWall", "M_CN_SignBoard", "M_CN_SignText", "M_CN_Emissive",
    "M_CN_Decal",  # ours: a stain's row carries its wall's colour (`assemble`)
))
# Not in any mesh we draw.
LEFT_OUT = frozenset(("M_CN_Bark",))
# The source turns this material's UVs a quarter (its ridges run across).
TURNED = frozenset(("M_CN_Shutter",))
# A masked surface takes its row's tint: where the graph tints an instance, the surfaces
# of it that take colour. The walls take the building's own colour this way.
MASK = "|tint"
# A surface drawn as a cutout sheet of bars in its own colour (`sheeted`): marked before the mask.
CUT = "|cut"
BARS = "cn_bars"


def base_of(name):
    """The source material a surface name stands for, without its marks."""
    return name.split("|")[0]


def recipe_of(name):
    return BARS if CUT in name else SURFACES[base_of(name)][1]


def see_through(name):
    """Whether a surface is a cutout or blended: no tier's simplifying may touch it."""
    return CUT in name or base_of(name) in COVERAGE


def grime(streak, blotch, colour=(0.2, 0.18, 0.15)):
    """Dirt burnt into a wall texture: damp blotches, and rain streaks down it."""

    def burn(albedo, roughness):
        blotches = textures.smoothstep(0.45, 0.85, textures.fbm(3, 911, 4))
        runs = textures.smoothstep(0.55, 0.9, textures.fbm((40, 2), 913, 3)) * (0.5 + 0.5 * textures.fbm(5, 915, 3))
        dirt = np.clip(blotch * blotches + streak * runs, 0.0, 0.9)
        return textures.mix(albedo, np.asarray(colour) * albedo.mean((0, 1)) / max(albedo.mean(), 1e-6), dirt), \
            np.clip(roughness + 0.25 * dirt, 0.0, 1.0)

    return burn


def toned(mean, keep, level_cycles=0):
    """A set brought to a colour of our own: its hue kept by `keep` (0 is grey) and its
    mean albedo set to `mean` (linear). `level_cycles` levels its brightness over
    anything longer than that many cycles to the tile: a set lighter at one corner
    than another, repeated, is a chequer from the air. Nothing is added that would
    repeat: a surface this recipe covers by the hundred square metres gets its dirt
    from its mesh."""

    def burn(albedo, roughness):
        grey = (albedo @ np.array([0.2126, 0.7152, 0.0722]))[..., None]
        if level_cycles:
            spectrum = np.fft.fft2(grey[..., 0])
            fy, fx = np.meshgrid(np.fft.fftfreq(grey.shape[0]) * grey.shape[0], np.fft.fftfreq(grey.shape[1]) * grey.shape[1], indexing="ij")
            slow = np.hypot(fx, fy) <= level_cycles
            slow[0, 0] = False
            level = np.fft.ifft2(np.where(slow, spectrum, 0)).real[..., None]
            albedo = albedo * np.clip((grey - level) / np.maximum(grey, 1e-6), 0.0, 4.0)
            grey = (albedo @ np.array([0.2126, 0.7152, 0.0722]))[..., None]
        out = grey + (albedo - grey) * keep
        return np.clip(out * (np.asarray(mean) / out.mean((0, 1))), 0.0, 1.0), roughness

    return burn


def scorched(albedo, roughness):
    """Painted plaster after a fire: smoked grey all over, a little uneven, with a few
    patches where the plaster has spalled to the render under it. The marks that say
    fire are the fans above the openings, which are geometry: clouds of soot in a
    texture repeat as camouflage, and runs down a brown wall are the grain of a plank.
    Only the colour changes, so the burnt wall shares the intact wall's normal and
    roughness images."""
    soot = textures.smoothstep(0.25, 0.8, textures.fbm(1, 931, 3))
    spall = textures.smoothstep(0.86, 0.9, textures.fbm(5, 941, 4))
    out = textures.mix(albedo, (0.03, 0.028, 0.026), np.clip(0.52 + 0.18 * soot, 0.0, 0.94))
    render = np.array((0.085, 0.08, 0.075)) * (0.7 + 0.5 * textures.fbm(40, 943, 2))[..., None]
    return textures.mix(out, render, spall * 0.8), roughness


def recipes():
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
    textures.recipe(BARS, tile=0.5)(bars)
    textures.recipe("cn_streak", tile=1.0)(streak)
    textures.recipe("cn_leaves", tile=0.45)(leaves)


def bars():
    """A cage's or a railing's bars: round bars 12.5 cm apart between flat rails half a
    metre apart, as the shared `grille` recipe has them, but in no colour of its own
    (white: each shows its own paint or metal) and over half a metre, so a bar is twelve
    texels across. At the shared recipe's one metre a bar seen from 30 m had a saw's edge."""
    size = textures.SIZE
    yy, xx = np.mgrid[0:size, 0:size].astype(float) / size
    px = 1.0 / size
    bar_r, rail_r = 0.012 / 0.5, 0.02 / 0.5  # half widths, in tiles
    bx = np.abs((xx * 4) % 1.0 - 0.5) / 4  # to the nearest bar's axis
    ry = np.abs(yy % 1.0 - 0.5)  # to the rail's
    bar = textures.smoothstep(bar_r + px / 2, bar_r - px / 2, bx)
    rail = textures.smoothstep(rail_r + px / 2, rail_r - px / 2, ry)
    height = np.maximum(np.sqrt(np.clip(1.0 - (bx / bar_r) ** 2, 0, 1)) * 3.0, rail * 2.0)
    ones = np.ones((size, size))
    return textures.Baked(np.ones((size, size, 3)), 1.0, textures.normals_from_height(textures.blur(height), 1.0), 1.0, ones, ones,
                          coverage=np.maximum(bar, rail))


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


def material_name(source):
    return SURFACES[base_of(source)][0] + ("_cut" if CUT in source else "") + ("_tint" if source.endswith(MASK) else "")


# ---------------------------------------------------------------- the graph
def family_tiers(name):
    return next((tiers for prefix, tiers in FAMILIES if name.startswith(prefix)), 0)


def open_graph():
    graph.open_blend(os.path.join(BLENDER, BLEND))
    groups = bpy.data.node_groups
    # in memory only: the building as instances, and the tint the graph stores on each
    groups["CN_Facade"].nodes["Realize Instances.001"].mute = True
    groups["CN_Building"].nodes["Realize Instances.003"].mute = True
    for node in groups["CN_Finalize"].nodes:
        if node.bl_idname == "GeometryNodeRemoveAttribute":
            node.mute = True
    return graph.modifier("CN_CornerApartment", "CN_Building")


class Graph:
    """The building, evaluated on demand and once per set of inputs."""

    def __init__(self):
        self.ob, self.mod = open_graph()
        self.taps = {}

    def tap(self, inputs):
        key = json.dumps(inputs, sort_keys=True)
        if key not in self.taps:
            graph.set_inputs(self.mod, inputs)
            tap = graph.tap(self.ob, tinted=TAKES_COLOUR)
            self.taps[key] = (tap, drawn(tap.rows))
        return self.taps[key]


def frame(angle, x, y):
    c, s = math.cos(angle), math.sin(angle)
    return np.array([[c, -s, 0.0, x], [s, c, 0.0, y], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])


# ---------------------------------------------------------------- the outline
def outline(rects):
    """The boundary of a union of abutting rectangles, as loops of straight runs
    (start, end) walked with the inside on the left, so outside is to the right. The
    outer loop comes first; a courtyard is a loop of its own."""
    xs = sorted({v for cx, _, hx, _ in rects for v in (cx - hx, cx + hx)})
    ys = sorted({v for _, cy, _, hy in rects for v in (cy - hy, cy + hy)})
    inside = lambda i, j: 0 <= i < len(xs) - 1 and 0 <= j < len(ys) - 1 and any(
        abs((xs[i] + xs[i + 1]) / 2 - cx) < hx and abs((ys[j] + ys[j + 1]) / 2 - cy) < hy for cx, cy, hx, hy in rects)
    step = {}
    for i in range(len(xs) - 1):
        for j in range(len(ys) - 1):
            if not inside(i, j):
                continue
            x0, x1, y0, y1 = xs[i], xs[i + 1], ys[j], ys[j + 1]
            for free, a, b in ((inside(i, j - 1), (x0, y0), (x1, y0)), (inside(i + 1, j), (x1, y0), (x1, y1)),
                               (inside(i, j + 1), (x1, y1), (x0, y1)), (inside(i - 1, j), (x0, y1), (x0, y0))):
                if not free:
                    if a in step:
                        raise SystemExit("parts that touch only at a corner have no outline")
                    step[a] = b
    loops = []
    while step:
        start = min(step)
        corners, at = [start], step.pop(start)
        while at != start:
            corners.append(at)
            at = step.pop(at)
        turns = [c for k, c in enumerate(corners)
                 if (c[0] - corners[k - 1][0]) * (corners[(k + 1) % len(corners)][1] - c[1])
                 != (c[1] - corners[k - 1][1]) * (corners[(k + 1) % len(corners)][0] - c[0])]
        loops.append([(turns[k], turns[(k + 1) % len(turns)]) for k in range(len(turns))])
    return loops


def right_of(a, b):
    """The unit normal to the right of the way from `a` to `b`: outward, on an outline."""
    length = math.hypot(b[0] - a[0], b[1] - a[1])
    return ((b[1] - a[1]) / length, -(b[0] - a[0]) / length)


def ring(loop, z0, z1, out_m, in_m, material, inner=False, outer=True):
    """A band round a loop of the outline: `out_m` proud of the wall and `in_m` into it,
    from `z0` to `z1`, mitred at every corner. Its outer face and its top; with `inner`
    its inner face too (a parapet seen from the roof), and without `outer` that alone
    (a wall seen from inside)."""
    normals = [right_of(a, b) for a, b in loop]
    moved = lambda k, d: (loop[k][0][0] + d * (normals[k - 1][0] + normals[k][0]),
                          loop[k][0][1] + d * (normals[k - 1][1] + normals[k][1]))
    faces = []
    for k in range(len(loop)):
        n = (k + 1) % len(loop)
        a, b, c, d = moved(k, out_m), moved(n, out_m), moved(n, -in_m), moved(k, -in_m)
        if outer:
            faces.append([(*a, z0), (*b, z0), (*b, z1), (*a, z1)])
            faces.append([(*a, z1), (*b, z1), (*c, z1), (*d, z1)])
        if inner:
            faces.append([(*c, z0), (*d, z0), (*d, z1), (*c, z1)])
    return Soup.join([quad(face, material) for face in faces])


# ---------------------------------------------------------------- facades and roofs
# The graph's four facades, each as the corner it starts at and the corner it ends at
# (in units of its Width along x and its Depth along y), walked with the street on the
# right. Facade 1 has the entrance and shops, 0 shops, 2 and 3 are backs.
FACADES = {0: ((0, 0), (1, 0)), 1: ((0, 1), (0, 0)), 2: ((1, 0), (1, 1)), 3: ((1, 1), (0, 1))}
ACROSS_M = 12.0  # the other side of a building evaluated for one facade; a facade does not depend on it
ROOF_FAMILIES = ("CNK_RoofBulk_", "CNK_RoofProp_", "CNK_RoofSmall_")


def facade_of(run, south, east):
    """Which of the graph's facades dresses a run of the outline: the entrance facade on
    the street (the southmost runs), shops on the east end, backs everywhere else."""
    (ax, ay), (bx, by) = run
    n = right_of(*run)
    if n[1] < -0.5:
        return 1 if ay == south else 2
    if n[0] > 0.5:
        return 0 if ax == east else 3
    return 2 if n[1] > 0.5 else 3


def run_inputs(length, side, floors, seed, shop_seed, own):
    """The inputs that make `side` a facade `length` long. Its corner piers are 1 m when
    the length is 3n + 2 and 1.5 m when it is 3n; no other length keeps windows off a corner
    and on every point of the lattice."""
    bays = math.floor((length - 2.0) / BAY_M + 1e-9)
    margin = (length - BAY_M * bays) / 2
    if margin > 1.5 + 1e-9:
        raise SystemExit(f"a {length} m run needs {margin} m corner piers: make it 3n or 3n + 2 metres")
    size = {"Width": length, "Depth": ACROSS_M} if side in (0, 3) else {"Width": ACROSS_M, "Depth": length}
    return dict(INPUTS, **own, **size, **{"Corner Margin": margin, "Floors": floors, "Seed": seed, "Shop Seed": shop_seed,
                                          "Solar Heaters": 0, "Roof Props": 0, "Roof Vents": 0})


def facade(tap, rows, side, run, roof_m):
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

    wall_m = INPUTS["Wall Thickness"]
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
        cube = masked(tap.meshes[name], False).transformed(m).coloured(1.0 if tint is None else tint[:3])
        normals, _ = cube.normals()
        # the face a cube turns to the wall it sits on is never seen
        against = (normals[:, :2] @ out < -0.5) & (depth(cube.v[cube.t].mean(1)) > -0.02)
        cubes.append(cube.keep(~against).transformed(place))
    openings = [(name, place @ m) for name, m, _ in tap.rows if name.startswith(OPENINGS) and depth(m[:3, 3]) < 0.8]
    # a room covers the opening it stands behind, top to bottom, under the floor above; an opening with none is bare
    ground_m, storey_m = INPUTS["Ground Floor Height"], INPUTS["Floor Height"]
    spans = []
    for name, m in openings:
        lo, hi = tap.meshes[name].bounds()
        spans.append((float((m[:2, 3] - run[0]) @ ((np.array(run[1]) - run[0]) / length)), m[2, 3] + lo[2] * m[2, 2], m[2, 3] + hi[2] * m[2, 2]))
    roomed = set()
    for k, (name, m, tint) in enumerate(placed):
        if name not in ROOM_MESHES:
            continue
        s_room = float((m[:2, 3] - run[0]) @ ((np.array(run[1]) - run[0]) / length))
        z0, z1 = max(m[2, 3], ROOM_STEP_M), m[2, 3] + m[2, 2]
        ceiling = (ground_m if z0 < ground_m - 1.0 else ground_m + storey_m * (math.floor((z0 - ground_m + 0.5) / storey_m) + 1)) - 0.03
        for i, (s_open, o0, o1) in enumerate(spans):
            if abs(s_open - s_room) < 0.6 and o0 < z1 and o1 > z0:
                roomed.add(i)
                z0, z1 = max(ROOM_STEP_M, min(z0, o0 - 0.03)), min(ceiling, max(z1, o1 + 0.05))
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
        wall=masked(own.keep(ours & ~inner), True).transformed(place), inner=masked(own.keep(inner), True).transformed(place),
        rows=placed, cubes=Soup.join(cubes), openings=openings, bare=bare)


def roof_rows(graph_, part, floors, seed, roof, own):
    """A part's roof furniture: the graph's for a building of the part's size."""
    cx, cy, hx, hy = part
    inputs = dict(INPUTS, **own, **{"Width": 2.0 * hy, "Depth": 2.0 * hx, "Corner Margin": 1.0, "Floors": floors, "Seed": seed,
                                    "Shop Seed": 0, "Solar Heaters": roof[0], "Roof Props": roof[1], "Roof Vents": roof[2]})
    _, rows = graph_.tap(inputs)
    place = frame(math.pi / 2, cx + hx, cy - hy)
    return [(name, place @ m, tint) for name, m, tint in rows if name.startswith(ROOF_FAMILIES)]


def lining(meshes, name, m, depth_m, material, grow=0.05):
    """The pane that stands in an opening: the bounds of what fills it, across the wall,
    `depth_m` inside the wall's face (negative: in front of it)."""
    lo, hi = meshes[name].bounds()
    corners = np.array([(lo[0] - grow, depth_m, lo[2] - grow), (hi[0] + grow, depth_m, lo[2] - grow),
                        (hi[0] + grow, depth_m, hi[2] + grow), (lo[0] - grow, depth_m, hi[2] + grow)])
    corners = corners @ m[:3, :3].T + m[:3, 3]
    corners[:, 2] = np.maximum(corners[:, 2], 0.0)
    return quad(corners, material)


# ---------------------------------------------------------------- modules
def masked(soup, tinted):
    """A kit mesh's surfaces as ours: what has no path left out, and the surfaces that
    take a row's tint marked (white where the tint will multiply)."""
    soup = soup.without(LEFT_OUT | {""})
    mats, colour = [], soup.c.copy()
    for i, name in enumerate(soup.mats):
        if tinted and base_of(name) in TAKES_COLOUR:
            colour[np.unique(soup.t[soup.m == i])] = 1.0
            name += MASK
        mats.append(name)
    return Soup(soup.v, colour, soup.t, soup.m, soup.s, mats)


def sheeted(soup):
    """A kit mesh with its rows of thin bars drawn as cutout sheets: a cage's bars are
    1.3 cm across, under a pixel at the battle's camera, and a hundred of them are
    stipple where one face of the grille recipe thins evenly with distance. Bars of one
    material that stand in one plane, three or more of them, become one face over the
    plane they fill; a thin bar lying in such a face is the recipe's rail. The frame they
    hang in stays geometry."""
    labels = detail.islands(soup)
    thin = {}
    for label in range(labels.max() + 1):
        part = soup.keep(labels == label)
        lo, hi = part.bounds()
        dims = hi - lo
        order = np.argsort(-dims, kind="stable")
        if dims[order[0]] > 0.3 and dims[order[1]] < SHEET_BAR_M:
            thin[label] = (int(part.m[0]), int(order[0]), (lo + hi) / 2, lo, hi, part.c.mean(0))
    # the plane a bar stands in: for each axis across it, the bars of its material and direction at the same place
    groups = {}
    for label, (mat, long_axis, mid, lo, hi, _) in thin.items():
        for across in range(3):
            if across != long_axis:
                groups.setdefault((mat, long_axis, across, round(float(mid[across]), 2)), []).append(label)
    sheets, taken = [], set()
    for key in sorted(groups, key=lambda k: (-len(groups[k]), k)):
        mat, long_axis, across, at = key
        members = [label for label in groups[key] if label not in taken]
        if len(members) < 3:
            continue
        lo = np.min([thin[label][3] for label in members], 0)
        hi = np.max([thin[label][4] for label in members], 0)
        spread = 3 - long_axis - across
        if hi[spread] - lo[spread] < 0.25:
            continue
        taken.update(members)
        corners = np.zeros((4, 3))
        corners[:, across] = at
        corners[:, long_axis] = (lo[long_axis], hi[long_axis], hi[long_axis], lo[long_axis])
        corners[:, spread] = (lo[spread], lo[spread], hi[spread], hi[spread])
        colour = np.mean([thin[label][5] for label in members], 0)
        sheets.append((mat, across, at, lo, hi, quad(corners, soup.mats[mat] + CUT, colour)))
    for label, (mat, long_axis, mid, lo, hi, _) in thin.items():  # the rails
        if label not in taken and any(m == mat and abs(mid[across] - at) < 0.02 and np.all(lo >= slo - 0.02) and np.all(hi <= shi + 0.02)
                                      for m, across, at, slo, shi, _ in sheets):
            taken.add(label)
    if not sheets:
        return soup
    return Soup.join([soup.keep(~np.isin(labels, sorted(taken))), *(sheet[5] for sheet in sheets)])


def room_mesh(material):
    """A room: the unit box the graph scales to a bay, a floor and a depth, open to the
    wall at y = 0 and running in along +Y, its five faces turned inward. Its UVs are the
    box unfolded round its back wall (`box_uv`), as `parts.room_box` writes them."""
    x0, x1 = -0.5, 0.5
    faces = [[(x0, 1, 0), (x1, 1, 0), (x1, 1, 1), (x0, 1, 1)],  # back
             [(x0, 0, 0), (x1, 0, 0), (x1, 1, 0), (x0, 1, 0)],  # floor
             [(x0, 1, 1), (x1, 1, 1), (x1, 0, 1), (x0, 0, 1)],  # ceiling
             [(x0, 0, 0), (x0, 1, 0), (x0, 1, 1), (x0, 0, 1)],  # left
             [(x1, 1, 0), (x1, 0, 0), (x1, 0, 1), (x1, 1, 1)]]  # right
    return Soup.join([quad(face, material) for face in faces])


def unmasked(soup, tint):
    """A masked mesh with a row's tint baked into its colour, for folding into a shell
    (whose own row carries the wall's colour)."""
    if tint is None:
        return soup
    mats, colour = [], soup.c.copy()
    for i, name in enumerate(soup.mats):
        if name.endswith(MASK):
            colour[np.unique(soup.t[soup.m == i])] *= tint
            name = name.removesuffix(MASK)
        mats.append(name)
    return Soup(soup.v, colour, soup.t, soup.m, soup.s, mats)


def front_material(soup):
    """The material most of a module's street-facing area is (local -Y is outward)."""
    normals, area = soup.normals()
    facing = area * np.clip(-normals[:, 1], 0.0, 1.0)
    return int(np.argmax(np.bincount(soup.m, weights=facing, minlength=len(soup.mats))))


def module_tiers(name, soup):
    """The four meshes of a kit module, finest first; None where nothing is left to draw.
    A tier is never heavier than the one before it: where the rule would make it so, it
    repeats that one. A cutout or blended surface is one face already: the two fine tiers
    keep it as it is, and the two coarse ones, where a window is a dark pane on a flat
    wall, have none (glass there is that pane)."""
    if name in ROOM_MESHES:
        return [soup] * 4
    clear = np.array([see_through(m) for m in soup.mats], dtype=bool)[soup.m] if len(soup) else np.zeros(0, bool)
    solid, sheer = soup.keep(~clear), soup.keep(clear)
    glass = np.array([base_of(m) == "M_CN_Glass" for m in soup.mats], dtype=bool)[soup.m] if len(soup) else clear
    far = soup.keep(~clear | glass)  # what the coarse tiers are made from: glass, as a pane
    far = Soup(far.v, far.c, far.t, far.m, far.s, ["X_Pane" if base_of(m) == "M_CN_Glass" else m for m in far.mats])
    out = []
    for tier, g in enumerate(FEATURE_M):
        if tier == 0:
            g = next((own for prefix, own in TIER0_FEATURE_M if name.startswith(prefix)), g)
        if name.endswith("+wreck"):  # thrown down, it is neither an opening nor a balcony: a heap of bars and panels
            lod = Soup.join([detail.simplify(solid, g, BAR_M[tier]), *([sheer] if tier < 2 else [])])
        elif tier >= 2 and name.startswith(OPENINGS):
            front = front_material(far)  # the pane, not its frame: a far window is no bigger than a near one
            lod = detail.front_quad(far.keep(far.m == front), front, 0.03, far.mats)
        elif tier >= 2 and name.startswith(HULLS):
            lod = detail.hull(far, HULL_BAND_M[tier - 2], caps=tier == 2, sides=tier == 2)
            if tier == 3 and len(lod):  # its front alone, laid on the wall: standing off it, it floats when seen from the side
                flat = lod.v.copy()
                flat[:, 1] = soup.bounds()[1][1] - 0.2
                lod = Soup(flat, lod.c, lod.t, lod.m, lod.s, lod.mats)
        elif tier >= 2:
            lod = detail.simplify(far, g, BAR_M[tier])
        else:
            lod = Soup.join([detail.simplify(solid, g, BAR_M[tier]), sheer])
        finer = next((l for l in reversed(out) if l is not None), None)
        if finer is not None and len(clean(lod)) > len(clean(finer)):
            lod = finer
        out.append(lod if len(lod) else None)
    return out


def module_id(name):
    return name.removeprefix("CNK_").lower().replace("+", "_")


# What a fire burns away, and how a fitting is left: `name+burnt` charred where it was,
# `name+hanging` charred and torn half off the wall, `name+wreck` charred and thrown down.
BURNS = frozenset(("M_CN_Glass", "M_CN_GlassFrosted", "M_CN_PVC", "M_CN_Fabric", "M_CN_Lantern", "M_CN_RedPaper"))


def variant(soup, kind, seed):
    soup = damage.charred(soup, BURNS, seed)
    # burnt metal is sooted and dull: left a metal, it would mirror the sky
    soup = Soup(soup.v, soup.c, soup.t, soup.m, soup.s, ["X_Soot" + (CUT if CUT in name else "") if base_of(name) in SURFACES and SURFACES[base_of(name)][4] > 0 else name for name in soup.mats])
    if not len(soup) or kind == "burnt":
        return soup
    lo, hi = soup.bounds()
    if kind == "hanging":  # swung out from its top edge on the wall, one end dropped
        pivot = np.eye(4)
        pivot[:3, 3] = (0.0, 0.0, hi[2])
        back = np.eye(4)
        back[:3, 3] = (0.0, 0.0, -hi[2])
        return soup.transformed(pivot @ damage.tilted(0.0, 0.18, -0.42, 0.0, 0.0, -0.25) @ back)
    rng = random.Random(seed)
    lying = soup.transformed(damage.tilted(0.0, rng.uniform(-0.35, 0.35), rng.choice((-1, 1)) * rng.uniform(1.0, 1.45), 0, 0, 0))
    lo, hi = lying.bounds()
    rest = np.eye(4)
    rest[:3, 3] = (-(lo[0] + hi[0]) / 2, -(lo[1] + hi[1]) / 2, -lo[2])
    return lying.transformed(rest)


# ---------------------------------------------------------------- rows
def decompose(matrix):
    """A 4x4 as a row's (x, y, z, yaw, sx, sy, sz), or None when it also tilts or mirrors."""
    a = matrix[:3, :3]
    scale = np.linalg.norm(a, axis=0)
    yaw = math.atan2(a[1, 0], a[0, 0])
    c, s = math.cos(yaw), math.sin(yaw)
    rebuilt = np.array([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]) * scale
    if np.abs(rebuilt - a).max() > 1e-4:
        return None
    return [*matrix[:3, 3], yaw, *scale]


def row_matrix(row):
    x, y, z, yaw, sx, sy, sz = row
    c, s = math.cos(yaw), math.sin(yaw)
    m = np.eye(4)
    m[:3, :3] = np.array([[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]) * (sx, sy, sz)
    m[:3, 3] = (x, y, z)
    return m


def srgb_bytes(linear):
    c = np.clip(np.asarray(linear, dtype=np.float64), 0.0, 1.0)
    return [int(round(v * 255)) for v in np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)]


def drawn(rows):
    """The instances a building of ours draws, from all the graph makes: its room boxes
    as ours, and not its furnished shop interiors."""
    out = []
    for name, m, tint in rows:
        if name.startswith(INSIDE):
            continue
        if name in ROOMS:
            out.append((ROOMS[name], m, None))
        elif name.startswith("CNK_Curt_") and tint is not None:  # the graph's are flags; cloth behind glass has faded
            grey = sum(tint[:3]) / 3
            out.append((name, m, (*(0.85 * (grey + (c - grey) * 0.45) for c in tint[:3]), 1.0)))
        else:
            out.append((SIGN_SWAPS.get(name, name), m, tint))
    return out


# ---------------------------------------------------------------- the shell
def quad(corners, material, colour=1.0):
    return damage.quad(corners, material, colour)


ROOF_CELL_M = (3.0, 3.0, 6.0, 12.0)
BURNT_ROOF_CELL_M = (1.0, 1.0, 2.0, 5.0)  # a burn has a ragged edge; a soft one is a cloud's shadow
SMOKED_ROOF = (0.46, 0.66, 0.8)  # what multiplies the clay of a roof the fire has smoked
BURNT_THROUGH = (0.55, 0.6)  # how hot (0 to 1) the fire was where the tiles are gone, and where the slab is


def fire_at(x, y, seed):
    """How hot (0 to 1) the fire was under a burnt roof at a point."""
    return 0.72 * damage.noise(x, y, 7.0, seed + 9) + 0.28 * damage.noise(x, y, 1.7, seed + 10)


def roof(rects, z, cell_m, seed, burnt=0.0, slab=True):
    """A tiled roof over every part, as a grid whose vertices carry its weathering: a few
    large dark stains and a faint mottle, different on every template. `burnt` (what is
    left of its brightness) makes it the roof of a burnt block: smoked, and where the
    fire came up a hole through it, with the tiles off the slab round the hole unless
    `slab` is false (the farthest tier, whose cells are larger than that rim)."""
    out = []
    for cx, cy, hx, hy in rects:
        nx, ny = max(1, round(2 * hx / cell_m)), max(1, round(2 * hy / cell_m))
        x, y = np.meshgrid(np.linspace(cx - hx, cx + hx, nx + 1), np.linspace(cy - hy, cy + hy, ny + 1))
        x, y = x.ravel(), y.ravel()
        if burnt:  # off the grid, inside the roof's edge: a burn's edge follows no cell
            inside = (np.abs(x - cx) < hx - 1e-6) & (np.abs(y - cy) < hy - 1e-6)
            x = x + inside * 0.5 * cell_m * (damage.noise(x, y, 0.4 * cell_m, seed + 11) - 0.5)
            y = y + inside * 0.5 * cell_m * (damage.noise(x, y, 0.4 * cell_m, seed + 12) - 0.5)
        stain = textures.smoothstep(0.6, 0.85, damage.noise(x, y, 11.0, seed)) * (0.5 + 0.5 * damage.noise(x, y, 4.0, seed + 1))
        shade = (1.0 - 0.38 * stain) * (0.86 + 0.14 * damage.noise(x, y, 5.0, seed + 2))
        colour = np.repeat(shade[:, None], 3, 1)
        a = (np.arange(ny)[:, None] * (nx + 1) + np.arange(nx)[None, :]).ravel()
        tris = np.concatenate([np.stack([a, a + 1, a + nx + 2], 1), np.stack([a, a + nx + 2, a + nx + 1], 1)])
        v = np.stack([x, y, np.full(len(x), z)], 1)
        if not burnt:
            out.append(Soup(v, colour, tris, np.zeros(len(tris), int), np.zeros(len(tris), bool), ["M_CN_RoofTile"]))
            continue
        # Smoked all over, its clay gone to a brown grey. Where the fire came through, a hole (the storey under it
        # shows, `burnt_storey`), and round the hole the bare slab the tiles have fallen from, grey with ash. A cell
        # is one surface, and the edges between them are hard: a dark patch with a soft edge is a tree's shadow.
        middle = (v[a] + v[a + nx + 2]) / 2
        surface = np.digitize(fire_at(middle[:, 0], middle[:, 1], seed), BURNT_THROUGH)
        surface = np.tile(surface if slab else np.where(surface == 1, 0, surface), 2)
        ash = (0.16 + 0.22 * damage.noise(x, y, 1.1, seed + 13))[:, None] * np.ones(3)
        corner = np.where((surface == 0)[:, None, None], (colour * np.asarray(SMOKED_ROOF) * burnt)[tris], ash[tris])
        out.append(Soup(v[tris].reshape(-1, 3), corner.reshape(-1, 3), np.arange(3 * len(tris)).reshape(-1, 3),
                        np.minimum(surface, 1), np.zeros(len(tris), bool), ["M_CN_RoofTile", "X_Rubble"]).keep(surface != 2))
    return Soup.join(out)


BURNT_STOREY_M = 2.9


def burnt_storey(rects, loops, roof_m, material):
    """What shows through a hole in a burnt roof: the top storey's floor, black with what fell on it, and the
    inside of its walls."""
    z = roof_m - BURNT_STOREY_M
    floors = [quad([(cx - hx, cy - hy, z), (cx + hx, cy - hy, z), (cx + hx, cy + hy, z), (cx - hx, cy + hy, z)], "X_Rubble", 0.07)
              for cx, cy, hx, hy in rects]
    return Soup.join([*floors, *(ring(loop, z, roof_m, 0.0, 0.32, material, inner=True, outer=False).coloured(0.2) for loop in loops)])


def heights(floors):
    """A block's levels: its ground floor's head, its roof, its parapet's top, and what
    the simulation leaves of it when it collapses (`collapse.py`)."""
    ground_m = INPUTS["Ground Floor Height"]
    roof_m = ground_m + INPUTS["Floor Height"] * (floors - 1)
    top_m = roof_m + INPUTS["Parapet Height"]
    return ground_m, roof_m, top_m, collapse.ruin_height(top_m)


def crown(loops, floors, tier, stucco, shade=1.0, trim=1.0):
    """What follows the outline: the parapet and its coping, the belt over the shops and
    the band under the parapet, and at the fine tiers a band at every floor. `shade`
    darkens the parapet and `trim` the bands."""
    ground_m, roof_m, top_m, _ = heights(floors)
    wall_m, storey_m, band = INPUTS["Wall Thickness"], INPUTS["Floor Height"], "M_CN_Band"
    walls, bands = [], []
    for loop in loops:
        walls.append(ring(loop, roof_m, top_m, 0.0, wall_m, stucco, inner=True))
        bands.append(ring(loop, ground_m - 0.3, ground_m + 0.02, 0.14, 0.06, band))
        if tier < 3:
            bands += [ring(loop, top_m, top_m + 0.1, 0.07, wall_m + 0.07, band, inner=True),
                      ring(loop, roof_m - 0.3, roof_m + 0.02, 0.1, 0.06, band)]
        if tier < 2:
            bands += [ring(loop, ground_m + storey_m * k - 0.14, ground_m + storey_m * k, 0.05, 0.06, band)
                      for k in range(1, floors - 1)]
    walls, bands = Soup.join(walls), Soup.join(bands)
    return Soup.join([walls.coloured(shade) if shade != 1.0 else walls, bands.coloured(trim) if trim != 1.0 else bands])


def flat_walls(loops, floors, stucco, shade=1.0, trim=1.0):
    """The walls of the two coarse tiers: one quad of stone and one of stucco to a run."""
    ground_m, roof_m, _, _ = heights(floors)
    return Soup.join([quad([(ax, ay, z0), (bx, by, z0), (bx, by, z1), (ax, ay, z1)], material, colour)
                      for loop in loops for (ax, ay), (bx, by) in loop
                      for z0, z1, material, colour in ((0.0, ground_m, "M_CN_Stone", trim), (ground_m, roof_m, stucco, shade))])


def folded(rows, modules, tiers_of, tier, skip=()):
    """What is left of the rows' modules at a coarse tier, as meshes in the template's frame."""
    return [unmasked(modules[name][tier], None if tint is None else tint[:3]).transformed(m) for name, m, tint in rows
            if tiers_of[name] & ~ROW_TIERS & (1 << tier) and not name.startswith(skip)]


def monotone(name, tiers):
    for tier in range(1, 4):
        if len(clean(tiers[tier])) > len(clean(tiers[tier - 1])):
            raise SystemExit(f"{name}: tier {tier} is heavier than tier {tier - 1}")
    return tiers


def intact_shell(b, modules, tiers_of, meshes):
    """A template's own mesh at each tier: the walls of every run with every opening
    lined, the bands, parapet and coping round the outline, a roof over every part, the
    cubes, and at the two coarse tiers flat walls with what is left of the kit folded in."""
    _, roof_m, _, _ = heights(b.floors)
    walls = [run.wall for run in b.runs]
    linings = [lining(meshes, name, m, LINING_M, "X_Void") for run in b.runs for name, m in run.bare]
    linings += [step for run in b.runs for step in run.steps]
    cubes = Soup.join([run.cubes for run in b.runs])
    # from far off a glazed-in balcony is its dark glass: the door and the washing behind it are not folded in
    glazed = [m for name, m, _ in b.rows if name == "CNK_Balc_01_Enclosed"]
    behind = lambda m: any(math.hypot(m[0, 3] - g[0, 3], m[1, 3] - g[1, 3]) < 1.0 and abs(m[2, 3] - g[2, 3]) < 0.1 for g in glazed)
    far_rows = [row for row in b.rows if not (row[0].startswith(ON_BALCONY) and behind(row[1]))]
    out = []
    for tier, g in enumerate(FEATURE_M):
        body = [*walls, *linings] if tier < 2 else [flat_walls(b.loops, b.floors, STUCCO + MASK)]
        out.append(Soup.join([*body, crown(b.loops, b.floors, tier, STUCCO + MASK), roof(b.rects, roof_m + 0.04, ROOF_CELL_M[tier], b.seed),
                              cubes if tier == 0 else detail.simplify(cubes, g), *folded(far_rows, modules, tiers_of, tier)]))
    return monotone(b.name, out)


# ---------------------------------------------------------------- gutted
BURNT_WALL = 0.55  # how far the fire darkens a wall all over; the soot fans fade up to it
BURNT_TRIM = 0.2  # and its bands, sills and plinth, which are pale and would stay clean
BURNT_GREY = 0.9  # how far the wall's paint loses its colour
# How much of each family a fire leaves on the building, and as which variant. What is not named is gone.
SURVIVES = (
    ("CNK_Balc_", 1.0, ("burnt",)), ("CNK_Win_", 0.25, ("burnt",)), ("CNK_Grille_", 0.5, ("hanging", "burnt")),
    ("CNK_AC_", 0.45, ("burnt",)), ("CNK_Shop_02_", 1.0, ("burnt",)), ("CNK_Shop_03_", 1.0, ("burnt",)),
    ("CNK_Ent_", 1.0, ("burnt",)), ("CNK_RoofBulk_", 1.0, ("burnt",)), ("CNK_RoofProp_", 0.6, ("burnt",)),
    ("CNK_RoofSmall_", 0.5, ("burnt",)),
)
BLOWN = 0.035  # the share of a facade's bays above the shops blown out to the floor slabs


def smoked(wall):
    """A wall's paint after the fire: most of its colour gone to grey."""
    grey = 0.2126 * wall[0] + 0.7152 * wall[1] + 0.0722 * wall[2]
    return tuple(int(round(c + (grey - c) * BURNT_GREY)) for c in wall)


def survivor(rng, name):
    for prefix, share, kinds in SURVIVES:
        if name.startswith(prefix):
            return name + "+" + rng.choice(kinds) if rng.random() < share else None
    return None


def gutted(b, meshes):
    """The block burnt out and standing: its rows, and a function that makes its shell
    from the kit's modules. Every opening is an empty dark hole with soot above most, a
    few bays are blown out to the floor slabs, the walls and roof are scorched."""
    rng = random.Random(b.seed * 1000 + 7)
    ground_m, roof_m, _, _ = heights(b.floors)
    storey_m = INPUTS["Floor Height"]
    burnt, char, concrete = "X_Burnt" + MASK, "X_Void", "X_Rubble"
    rows, walls, cubes = [], [], []
    holes, far_holes, soot, flat_soot, far_soot = [], [], [], [], []  # for walls with openings, for flat walls, and the few fans the farthest tier keeps
    for run in b.runs:
        point = lambda s, deep, z, run=run: (*(run.start + run.along * s - run.out * deep), z)
        s_of = lambda xy, run=run: float((np.asarray(xy)[:2] - run.start) @ run.along)
        bays = math.floor((run.length - 2.0) / BAY_M + 1e-9)
        margin = (run.length - BAY_M * bays) / 2
        blown = set()
        for _ in range(int(bays * (b.floors - 1) * BLOWN + rng.random())):
            k, f = rng.randrange(bays), rng.randrange(1, b.floors)
            blown |= {(k + dk, f + df) for dk in range(rng.choice((1, 1, 2))) for df in range(rng.choice((1, 1, 2)))
                      if k + dk < bays and f + df < b.floors}
        gone = lambda s, z: (math.floor((s - margin) / BAY_M), 0 if z < ground_m else 1 + math.floor((z - ground_m) / storey_m)) in blown

        centre = run.wall.v[run.wall.t].mean(1)
        wall = run.wall.keep(np.array([not gone(s_of(c), float(c[2])) for c in centre], dtype=bool))
        trim = np.zeros(len(wall.v), bool)
        trim[np.unique(wall.t[np.array(wall.mats)[wall.m] != STUCCO + MASK])] = True
        clear = np.median(wall.c[~trim], axis=0) * BURNT_WALL  # the burnt wall's own colour, which a fan fades to
        walls.append(Soup(wall.v, wall.c * np.where(trim, BURNT_TRIM, BURNT_WALL)[:, None], wall.t, wall.m, wall.s,
                          [burnt if name == STUCCO + MASK else name for name in wall.mats]))
        for k, f in sorted(blown):
            s0, z0 = margin + BAY_M * k, ground_m + storey_m * (f - 1)
            s1, z1, deep = s0 + BAY_M, z0 + storey_m, 3.2
            edge = quad([point(s0, -0.05, z0 - 0.22), point(s1, -0.05, z0 - 0.22), point(s1, -0.05, z0), point(s0, -0.05, z0)], concrete, 0.22)
            holes += [edge,
                      quad([point(s0, 0, z0), point(s1, 0, z0), point(s1, deep, z0), point(s0, deep, z0)], concrete, 0.13),
                      quad([point(s0, deep, z0), point(s1, deep, z0), point(s1, deep, z1), point(s0, deep, z1)], char),
                      quad([point(s0, 0, z0), point(s0, deep, z0), point(s0, deep, z1), point(s0, 0, z1)], char),
                      quad([point(s1, deep, z0), point(s1, 0, z0), point(s1, 0, z1), point(s1, deep, z1)], char)]
            far_holes += [edge, quad([point(s0, -0.03, z0), point(s1, -0.03, z0), point(s1, -0.03, z1), point(s0, -0.03, z1)], char)]
        # where each opening is on the run (along it, and up): soot rises to the next one above and no further
        spans = []
        for name, m in run.openings:
            lo, hi = meshes[name].bounds()
            wide = math.hypot(m[0, 0], m[1, 0])
            s = s_of(m[:2, 3])
            spans.append((s + lo[0] * wide, s + hi[0] * wide, m[2, 3] + lo[2] * m[2, 2], m[2, 3] + hi[2] * m[2, 2]))
        spans += [(margin + BAY_M * k, margin + BAY_M * (k + 1), ground_m + storey_m * (f - 1), ground_m + storey_m * f) for k, f in blown]
        for name, m in run.openings:
            if gone(s_of(m[:2, 3]), m[2, 3] + 0.3):
                continue
            holes.append(lining(meshes, name, m, LINING_M, char))
            if not name.startswith(ON_BALCONY):  # from far off a balcony covers the door onto it
                far_holes.append(lining(meshes, name, m, -0.03, char, grow=0.0))
            if rng.random() < 0.85:
                lo, hi = meshes[name].bounds()
                wide = math.hypot(m[0, 0], m[1, 0])
                s, half = s_of(m[:2, 3]) + (lo[0] + hi[0]) / 2 * wide, (hi[0] - lo[0]) / 2 * wide
                z = m[2, 3] + hi[2] * m[2, 2] + 0.14
                tall, spread = rng.uniform(1.4, 2.8), rng.uniform(0.25, 0.6)
                above = [z0 for s0, s1, z0, _ in spans if z0 > z - 0.3 and s0 < s + half + spread and s1 > s - half - spread]
                reach = min(roof_m - z, min(above, default=math.inf) - z - 0.08)
                if reach < 0.25:
                    continue
                dark = rng.uniform(0.03, 0.1) * BURNT_WALL
                fan = ((point(s - half, -0.04, z), point(s + half, -0.04, z)), min(tall, reach), spread, burnt, dark)
                # stopped under the opening above, it is still half as black there: the soot goes on up past that
                # opening, and a column of windows is one black streak, not a shadow under each sill
                held = lambda wall: dark + (wall - dark) * 0.45 if reach < tall else None
                soot.append(damage.soot_fan(*fan, clear, top=held(clear)))
                flat_soot.append(damage.soot_fan(*fan, BURNT_WALL, top=held(BURNT_WALL)))
                if rng.random() < 0.25:
                    far_soot.append(damage.soot_fan(*fan, BURNT_WALL, sides=False, top=held(BURNT_WALL)))
        if len(run.cubes):
            sooted = damage.charred(run.cubes, frozenset(), b.seed + 3, light=(0.03, 0.1))
            grey = sooted.c.mean(1, keepdims=True)  # a sign board's paint is gone with the rest
            sooted = Soup(sooted.v, grey + (sooted.c - grey) * 0.2, sooted.t, sooted.m, sooted.s, sooted.mats)
            centre = sooted.v[sooted.t].mean(1)
            cubes.append(sooted.keep(np.array([not gone(s_of(c), float(c[2])) for c in centre], dtype=bool)))
        for name, m, _ in run.rows:
            left = None if gone(s_of(m[:2, 3]), m[2, 3] + 0.3) else survivor(rng, name)
            if left:
                rows.append((left, m, None))
    for name, m, _ in b.roof_rows:
        left = survivor(rng, name)
        # what stood where the roof is burnt through has gone down with it
        if left and fire_at(np.array([m[0, 3]]), np.array([m[1, 3]]), b.seed)[0] < BURNT_THROUGH[0] - 0.04:
            rows.append((left, m, None))

    def shell(modules, tiers_of):
        out, sooted = [], Soup.join(cubes)
        for tier, g in enumerate(FEATURE_M):
            body = [*walls, *holes, *soot] if tier < 2 else \
                [flat_walls(b.loops, b.floors, burnt, BURNT_WALL, BURNT_TRIM), *far_holes, *(flat_soot if tier == 2 else far_soot)]
            out.append(Soup.join([*body, crown(b.loops, b.floors, tier, burnt, BURNT_WALL, BURNT_TRIM),
                                  roof(b.rects, roof_m + 0.04, BURNT_ROOF_CELL_M[tier], b.seed, burnt=0.85, slab=tier < 3),
                                  burnt_storey(b.rects, b.loops, roof_m, burnt),
                                  sooted if tier == 0 else detail.simplify(sooted, g),
                                  # at the coarse tiers an opening is its dark lining, whatever hangs in it
                                  *folded(rows, modules, tiers_of, tier, skip=OPENINGS)]))
        return monotone(b.name + " gutted", out)

    return rows, shell


# ---------------------------------------------------------------- ruin
STUMP_STEP_M = 1.5  # a wall breaks off in columns this wide
SPILL_M = 1.1  # how far the rubble runs out past the walls
HEAP_CELL_M = (0.9, 1.8, 3.6, 7.2)
# What lies in the heap, thrown down and charred: one to every 20 square metres of plan.
WRECKS = ("CNK_AC_00_Std", "CNK_AC_02_Old", "CNK_Grille_00_CageSS", "CNK_Grille_01_CageBlack", "CNK_Balc_00_OpenRail",
          "CNK_Win_00_Slide2", "CNK_Shop_02_ShutterDown", "CNK_Laundry_01_Bamboo")
WRECK_M2 = 20.0


def ruin(b, meshes, source):
    """The block collapsed, inside the simulation's remains box (the parts' plan, at the
    ruin height): its rows, and a function that makes its shell. The walls stand as ragged
    stumps, the rubble of the same stucco, tile and concrete is heaped over the plan and
    spills a little, each part to a height of its own, with the floors lying in it."""
    rng = random.Random(b.seed * 1000 + 13)
    ground_m, _, _, ruin_m = heights(b.floors)
    wall_m = INPUTS["Wall Thickness"]
    burnt, concrete, tile = "X_Burnt" + MASK, "X_Rubble", "M_CN_RoofTile"
    high = [ruin_m * rng.uniform(0.5, 0.85) for _ in b.rects]
    height_at = lambda x, y: np.minimum(damage.heap_height(x, y, b.rects, high, SPILL_M, b.seed), ruin_m)
    stumps, cubes, coarse = [], [], {2: [], 3: []}
    for run in b.runs:
        tops = damage.profile(rng, run.length, STUMP_STEP_M, 0.5, ruin_m)
        standing = Soup.join([run.wall.coloured(0.72), run.inner.coloured(0.35)])
        standing = Soup(standing.v, standing.c, standing.t, standing.m, standing.s,
                        [burnt if name == STUCCO + MASK else name for name in standing.mats])
        stumps += [damage.break_off(standing, run.start, run.along, STUMP_STEP_M, tops),
                   damage.broken_edge(run.start, run.along, -run.out, wall_m, run.length, STUMP_STEP_M, tops, concrete, 0.6)]
        if len(run.cubes):  # half the sign boards and surrounds are down in the heap; what hangs on is scorched
            piece = detail.islands(run.cubes)
            hangs = np.array([rng.random() < 0.5 for _ in range(piece.max() + 1)], dtype=bool)[piece]
            cubes.append(damage.break_off(damage.charred(run.cubes.keep(hangs), frozenset(), b.seed + 3, light=(0.12, 0.34)),
                                          run.start, run.along, STUMP_STEP_M, tops))
        for tier, merge in ((2, 2), (3, 4)):  # the same stumps as flat faces, two and four columns to a face
            wide = STUMP_STEP_M * merge
            flat_tops = [sum(tops[k:k + merge]) / len(tops[k:k + merge]) for k in range(0, len(tops), merge)]
            for k, top in enumerate(flat_tops):
                s0, s1 = k * wide, min((k + 1) * wide, run.length)
                at = lambda s, z, deep=0.0: (*(run.start + run.along * s - run.out * deep), z)
                bands = [(0.0, min(top, ground_m), "M_CN_Stone", 0.72)] + ([(ground_m, top, burnt, 0.72)] if top > ground_m else [])
                coarse[tier] += [quad([at(s0, z0), at(s1, z0), at(s1, z1), at(s0, z1)], material, shade) for z0, z1, material, shade in bands]
                coarse[tier].append(quad([at(s1, 0.0, wall_m), at(s0, 0.0, wall_m), at(s0, top, wall_m), at(s1, top, wall_m)], burnt, 0.6))
            coarse[tier].append(damage.broken_edge(run.start, run.along, -run.out, wall_m, run.length, wide, flat_tops, concrete, 0.6, sides=tier == 2))

    names = [concrete, burnt, tile]
    # broken concrete, with plaster dust and smashed tile in it here and there
    mix = lambda x, y: np.digitize(damage.noise(x, y, 3.2, b.seed + 11), (0.68, 0.82))
    shade = lambda x, y, z: np.repeat((0.36 + 0.5 * damage.noise(x, y, 1.3, b.seed + 12) * (0.5 + 0.5 * damage.noise(x, y, 6.0, b.seed + 14)))[:, None], 3, 1)
    heap = lambda tier: damage.under(damage.heap(b.rects, high, SPILL_M, HEAP_CELL_M[tier], b.seed, (names, mix), shade), ruin_m)
    area = sum(4 * hx * hy for _, _, hx, hy in b.rects)
    fallen = damage.slabs(rng, b.rects, high, height_at, ruin_m, concrete, tile, (0.5, 0.8))
    chunks = [damage.scatter(random.Random(b.seed * 1000 + 17 + tier), b.rects, round(area / per), size, height_at, ruin_m,
                             (concrete, burnt, concrete, concrete, tile), (0.4, 0.85))
              for tier, (per, size) in enumerate(((3.0, 0.6), (9.0, 0.9), (30.0, 1.3)))]

    rows = []
    wrecks = [name for name in WRECKS if name in meshes]
    for _ in range(round(area / WRECK_M2)):
        name = wrecks[rng.randrange(len(wrecks))] + "+wreck"
        cx, cy, hx, hy = b.rects[rng.randrange(len(b.rects))]
        x, y = cx + rng.uniform(-hx + 0.8, hx - 0.8), cy + rng.uniform(-hy + 0.8, hy - 0.8)
        z = max(0.0, min(float(height_at(np.array([x]), np.array([y]))[0]) - 0.12, ruin_m - source(name).bounds()[1][2]))
        rows.append((name, frame(rng.uniform(0.0, math.tau), x, y) @ damage.tilted(0, 0, 0, 0, 0, z), None))

    def shell(modules, tiers_of):
        charred = Soup.join(cubes)
        out = [Soup.join([*stumps, charred, heap(0), chunks[0], fallen]),
               Soup.join([*stumps, detail.simplify(charred, FEATURE_M[1]), heap(1), chunks[1], fallen]),
               # what is thrown down is folded in at tier 2 and gone at tier 3
               Soup.join([*coarse[2], detail.simplify(charred, FEATURE_M[2]), heap(2), chunks[2], fallen,
                          *folded([r for r in rows if modules[r[0]][2] is not None], modules, {name: 0b0100 for name in tiers_of}, 2)]),
               Soup.join([*coarse[3], heap(3), fallen])]
        return monotone(b.name + " ruin", out)

    return rows, shell


# ---------------------------------------------------------------- the descriptor
FACES = (  # name, facade, outward normal, the way its offsets run (templates.rs `Facade::axes`)
    ("east", "positive_x", (1.0, 0.0), (0.0, 1.0)), ("north", "positive_y", (0.0, 1.0), (-1.0, 0.0)),
    ("west", "negative_x", (-1.0, 0.0), (0.0, -1.0)), ("south", "negative_y", (0.0, -1.0), (1.0, 0.0)),
)


def descriptor(name, floors, parts_, openings, doors):
    """The physical template. Every face of every part is cut into spans: joined where
    another part stands against it (no bays), exposed elsewhere, with the bay lattice
    checked against where the graph put its windows. `openings` and `doors` are
    template-space positions."""
    ground, storey = INPUTS["Ground Floor Height"], INPUTS["Floor Height"]
    half_z = (ground + storey * (floors - 1) + INPUTS["Parapet Height"]) / 2
    edges, spans = [], []  # spans: (edge, part, face index, start, end, the offsets of its openings)
    for part, (cx, cy, hx, hy) in parts_.items():
        for f, (face, facade_, n, along) in enumerate(FACES):
            reach, half = (hx, hy) if n[0] else (hy, hx)
            line = cx * n[0] + cy * n[1] + reach
            joined = []
            for other, (ox, oy, ohx, ohy) in parts_.items():
                oreach, ohalf = (ohx, ohy) if n[0] else (ohy, ohx)
                if other != part and abs(ox * n[0] + oy * n[1] - oreach - line) < 1e-9:
                    mid = (ox - cx) * along[0] + (oy - cy) * along[1]
                    lo, hi = max(-half, mid - ohalf), min(half, mid + ohalf)
                    if hi - lo > 1e-9:
                        joined.append((lo, hi))
            cuts = sorted({-half, half, *(v for span in joined for v in span)})
            pieces = [(a, b, any(lo <= a and b <= hi for lo, hi in joined)) for a, b in zip(cuts, cuts[1:])]
            # numbered the way the world's axis runs
            pieces.sort(key=lambda piece: piece[0] * (along[0] + along[1]))
            for k, (a, b, inner) in enumerate(pieces):
                edge = {"id": f"{part}-{face}" + (f"-{k}" if len(pieces) > 1 else ""), "part": part, "facade": facade_,
                        "span_m": [float(a), float(b)], "exposed": not inner, "bays": None}
                edges.append(edge)
                spans.append((edge, part, f, a, b, set()))

    def at(part, f, offset):
        cx, cy, hx, hy = parts_[part]
        n, along = FACES[f][2], FACES[f][3]
        reach = hx if n[0] else hy
        return (round(cx + n[0] * reach + along[0] * offset, 6), round(cy + n[1] * reach + along[1] * offset, 6))

    def on_edge(x, y):
        for edge, part, f, a, b, found in spans:
            cx, cy, hx, hy = parts_[part]
            n, along = FACES[f][2], FACES[f][3]
            offset = (x - cx) * along[0] + (y - cy) * along[1]
            if abs((x - cx) * n[0] + (y - cy) * n[1] - (hx if n[0] else hy)) < 1e-3 and a + 1e-6 < offset < b - 1e-6:
                if not edge["exposed"]:
                    raise SystemExit(f"{name}: an opening at {x}, {y} is on the joined face {edge['id']}")
                return edge, round(offset, 3), found
        raise SystemExit(f"{name}: an opening at {x}, {y} is on no edge")

    for x, y in openings:
        on_edge(x, y)[2].add(on_edge(x, y)[1])
    for edge, part, f, a, b, found in spans:
        if not edge["exposed"]:
            continue
        phase = round(min(found) % BAY_M, 3) if found else BAY_M / 2
        lattice = {round(phase + BAY_M * k, 3) for k in range(-200, 200) if a < phase + BAY_M * k < b}
        if found != lattice:
            raise SystemExit(f"{name} {edge['id']}: windows at {sorted(found)}, the bay lattice at {sorted(lattice)}")
        edge["bays"] = {"pitch_m": BAY_M, "phase_m": phase}
    joins = []
    for edge, part, f, a, b, _ in spans:
        for other, opart, of, oa, ob, _ in spans:
            if not edge["exposed"] and not other["exposed"] and edge["id"] < other["id"] \
                    and at(part, f, a) == at(opart, of, ob) and at(part, f, b) == at(opart, of, oa):
                joins.append({"id": f"join-{len(joins)}", "edges": [edge["id"], other["id"]]})
    entrances = []
    street = min(cy - hy for _, cy, _, hy in parts_.values())
    for k, (x, y) in enumerate(sorted(doors)):
        edge, offset, _ = on_edge(x, y)
        if edge["facade"] != "negative_y" or abs(y - street) > 1e-3:
            raise SystemExit(f"{name}: the entrance at {x}, {y} is not on the street side")
        entrances.append({"id": f"door-{k}", "edge": edge["id"], "offset_m": offset})
    return {
        "id": f"china-apartment-{name}", "category": "urban_apartment", "regional_family": "china",
        "parts": [{"id": part, "center": [float(cx), float(cy)], "yaw": 0.0, "half_extents": [float(hx), float(hy), half_z],
                   "base_z": 0.0} for part, (cx, cy, hx, hy) in parts_.items()],
        "floor_heights_m": [0.0] + [round(ground + storey * k, 3) for k in range(floors - 1)],
        "entrances": entrances, "edges": edges, "joins": joins,
    }


# ---------------------------------------------------------------- the GLB
def box_uv(soup):
    """UVs in metres over each material's tile, one per triangle corner: each face
    projected along its dominant axis, as `textures.box_uv` does."""
    normals, _ = soup.normals()
    axis = np.abs(normals).argmax(1)
    p = soup.v[soup.t]  # (m, 3, 3)
    a = np.where(axis == 0, 1, 0)
    b = np.where(axis == 2, 1, 2)
    rows = np.arange(len(soup.t))
    sign = np.where(normals[rows, axis] < 0, -1.0, 1.0) * np.where(axis == 1, -1.0, 1.0)
    u = sign[:, None] * p[rows, :, a]
    v = p[rows, :, b]
    recipe = [recipe_of(name) for name in soup.mats]
    tile = np.array([textures.tile_of(r) if r else 1.0 for r in recipe])[soup.m][:, None]
    turned = np.array([base_of(name) in TURNED for name in soup.mats])[soup.m][:, None]
    uv = np.stack([np.where(turned, v, u) / tile, np.where(turned, u, v) / tile], -1)
    for i, name in enumerate(soup.mats):
        own = soup.m == i
        if not own.any():
            continue
        if base_of(name) in FITTED:  # one picture over the surface's own bounds, its top at the top
            q = p[own]
            lo, hi = q.reshape(-1, 3).min(0), q.reshape(-1, 3).max(0)
            uv[own] = np.stack([(q[..., 0] - lo[0]) / max(hi[0] - lo[0], 1e-9), (q[..., 2] - lo[2]) / max(hi[2] - lo[2], 1e-9)], -1)
        elif name in ROOM_SHEETS:  # the unit box unfolded round its back wall (`room_mesh`)
            q, n = p[own], normals[own]
            x, y, z = q[..., 0] + 0.5, q[..., 1], q[..., 2]
            face = np.abs(n).argmax(1)[:, None]
            low = (n[np.arange(len(n)), np.abs(n).argmax(1)] > 0)[:, None]  # the floor's and the left wall's normals point up and right
            uv[own] = np.stack([np.where(face == 0, np.where(low, y - 1.0, 2.0 - y), x),
                                np.where(face == 2, np.where(low, y - 1.0, 2.0 - y), z)], -1)
    return uv


def clean(soup):
    """Positions snapped to 0.1 mm, the triangles that snap flat removed, and only the
    materials still in use, in name order."""
    out = Soup(np.round(soup.v, 4), soup.c, soup.t, soup.m, soup.s, soup.mats)
    _, area = out.normals()
    out = out.keep(area > 1e-9)
    used = sorted({out.mats[i] for i in np.unique(out.m)}, key=material_name)
    slot = np.array([used.index(name) if name in used else -1 for name in out.mats])
    return Soup(out.v, out.c, out.t, slot[out.m], out.s, used)


MATERIALS = {}


def material(source):
    name = material_name(source)
    if name not in MATERIALS:
        _, _, base, rough, metal = SURFACES[base_of(source)]
        recipe = recipe_of(source)
        if source in ROOM_SHEETS:
            MATERIALS[name] = parts.room(name, ROOM_SHEETS[source])
            return MATERIALS[name]
        m = bpy.data.materials.new(name)
        m.use_nodes = True
        bsdf = m.node_tree.nodes["Principled BSDF"]
        bsdf.inputs["Base Color"].default_value = (*base, 1.0)
        bsdf.inputs["Roughness"].default_value = rough
        bsdf.inputs["Metallic"].default_value = metal
        if source.endswith(MASK):
            m["tint"] = 1.0
        if recipe:
            parts.TEXTURED[name] = recipe
        coverage = ("cutout", 0.5) if CUT in source else COVERAGE.get(base_of(source))
        if coverage:
            textures.surface(m, coverage)
        MATERIALS[name] = m
    return MATERIALS[name]


def mesh_object(name, soup, parent):
    soup = clean(soup)
    me = bpy.data.meshes.new(name)
    k = len(soup.t)
    me.vertices.add(len(soup.v))
    me.vertices.foreach_set("co", soup.v.astype(np.float32).ravel())
    me.loops.add(3 * k)
    me.loops.foreach_set("vertex_index", soup.t.astype(np.int32).ravel())
    me.polygons.add(k)
    me.polygons.foreach_set("loop_start", np.arange(k, dtype=np.int32) * 3)
    me.polygons.foreach_set("material_index", soup.m.astype(np.int32))
    me.polygons.foreach_set("use_smooth", soup.s)
    for source in soup.mats:
        me.materials.append(material(source))
    me.uv_layers.new(name="UVMap").data.foreach_set("uv", box_uv(soup).astype(np.float32).ravel())
    colour = me.color_attributes.new("Color", "FLOAT_COLOR", "POINT")
    # alpha is wear in our bundles: none
    colour.data.foreach_set("color", np.concatenate([np.clip(soup.c, 0.0, 1.0), np.zeros((len(soup.v), 1))], 1)
                            .astype(np.float32).ravel())
    me.color_attributes.active_color = colour
    me.update()
    if me.validate():
        raise SystemExit(f"{name}: Blender had to repair the mesh")
    o = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(o)
    o.parent = parent
    return o


def write_kit(path, kit):
    """`kit` is {module id: [four soups]}."""
    parts.reset()
    recipes()
    for name in sorted(kit):
        root = parts.empty(name)
        for tier, soup in enumerate(kit[name]):
            mesh_object(f"{name}_LOD{tier}", soup, root)
    parts.export(path, worn=False)
    steady_tangents(path)


def steady_tangents(path):
    """Write every vertex's tangent again, from the first triangle that uses it: along
    that triangle's u, square to the vertex's normal. The exporter averages a smooth
    vertex's tangent over its faces in whatever order its threads finish, and rounds the
    sum, so one run in three wrote a rubble heap's tangent a ten-thousandth apart from the
    last. A flat face's tangent comes out as the exporter's own."""
    import struct

    data = bytearray(open(path, "rb").read())
    length = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(bytes(data[20:20 + length]))
    start = 20 + length + 8
    kind = {5121: "u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}

    def where(index):
        accessor = doc["accessors"][index]
        view = doc["bufferViews"][accessor["bufferView"]]
        if "byteStride" in view:
            raise SystemExit("steady_tangents: an interleaved buffer view")
        return start + view.get("byteOffset", 0) + accessor.get("byteOffset", 0), accessor["count"], width[accessor["type"]], \
            kind[accessor["componentType"]]

    def read(index):
        at, count, n, dtype = where(index)
        return np.frombuffer(bytes(data[at:at + count * n * np.dtype(dtype).itemsize]), dtype).reshape(count, n)

    for mesh in doc["meshes"]:
        for prim in mesh["primitives"]:
            attributes = prim["attributes"]
            if "TANGENT" not in attributes:
                continue
            p, n, uv = (read(attributes[k]).astype(np.float64) for k in ("POSITION", "NORMAL", "TEXCOORD_0"))
            t = read(prim["indices"]).astype(np.int64).reshape(-1, 3)
            e1, e2 = p[t[:, 1]] - p[t[:, 0]], p[t[:, 2]] - p[t[:, 0]]
            d1, d2 = uv[t[:, 1]] - uv[t[:, 0]], uv[t[:, 2]] - uv[t[:, 0]]
            det = d1[:, 0] * d2[:, 1] - d2[:, 0] * d1[:, 1]
            flat = np.abs(det) <= 1e-12
            safe = np.where(flat, 1.0, det)[:, None]
            along_u = (e1 * d2[:, 1:2] - e2 * d1[:, 1:2]) / safe
            along_v = (e2 * d1[:, 0:1] - e1 * d2[:, 0:1]) / safe
            first = np.full(len(p), len(t), dtype=np.int64)
            for corner in range(3):
                np.minimum.at(first, t[:, corner], np.arange(len(t)))
            first = np.minimum(first, len(t) - 1)
            tangent = along_u[first] - n * (n * along_u[first]).sum(1, keepdims=True)
            size = np.linalg.norm(tangent, axis=1, keepdims=True)
            lost = flat[first] | (size[:, 0] < 1e-9)
            # no u to follow (a triangle with no area in the texture): any direction square to the normal
            axis = np.eye(3)[np.abs(n).argmin(1)]
            spare = axis - n * (n * axis).sum(1, keepdims=True)
            tangent = np.where(lost[:, None], spare, tangent)
            tangent /= np.linalg.norm(tangent, axis=1, keepdims=True)
            # glTF's v runs down the image: the exporter's sign is the opposite of this one
            sign = np.where(lost, 1.0, -np.sign((np.cross(n, tangent) * along_v[first]).sum(1)))
            sign = np.where(sign == 0.0, 1.0, sign)
            out = np.concatenate([tangent, sign[:, None]], 1).astype("<f4")
            at, count, _, _ = where(attributes["TANGENT"])
            data[at:at + count * 16] = out.tobytes()
    open(path, "wb").write(bytes(data))


# ---------------------------------------------------------------- main
def fit_rooms(run, rects):
    """A run with its rooms no deeper than the plan has room for: short of the middle of
    the building behind the wall, and, near an end of the run, short of the room that
    stands behind the wall round the corner."""
    wall_m = INPUTS["Wall Thickness"]
    run.steps = []
    for k, (name, m, tint) in enumerate(run.rows):
        if name not in ROOM_MESHES:
            continue
        wide, deep = float(np.hypot(m[0, 0], m[1, 0])), float(np.hypot(m[0, 1], m[1, 1]))
        s = float((m[:2, 3] - run.start) @ run.along)
        if m[2, 3] < ROOM_STEP_M + 0.01:  # the threshold a ground-floor room stands on
            at = lambda along_m, z: (*(run.start + run.along * along_m - run.out * (wall_m - 0.01)), z)
            run.steps.append(quad([at(s - wide / 2, 0.0), at(s + wide / 2, 0.0), at(s + wide / 2, ROOM_STEP_M + 0.01),
                                   at(s - wide / 2, ROOM_STEP_M + 0.01)], "X_Void"))
        steps = np.arange(0.25, 12.0, 0.25)
        behind = run.start[None] + run.along[None] * s - run.out[None] * steps[:, None]
        outside = damage.distance_out(behind[:, 0], behind[:, 1], rects) > 0
        through = float(steps[np.argmax(outside)]) if outside.any() else 12.0
        corner = min(s - wide / 2, run.length - s - wide / 2)
        fits = max(0.6, min(deep, through / 2 - wall_m - ROOM_CLEAR_M, max(corner - wall_m - ROOM_CLEAR_M, 0.6) if corner < deep + wall_m else deep))
        if fits < deep:
            m = m.copy()
            m[:3, 1] *= fits / deep
            run.rows[k] = (name, m, tint)
    return run


def assemble(graph_, template):
    """A template built: its outline, each run of it dressed (`facade`), its instances in
    its own frame, and what made them."""
    name, floors, parts_, seed, shop_seed, roof_, wall, own = template
    rects = list(parts_.values())
    loops = outline(rects)
    south, east = min(cy - hy for _, cy, _, hy in rects), max(cx + hx for cx, _, hx, _ in rects)
    _, roof_m, _, _ = heights(floors)
    runs, recipe, used = [], [], {}
    for run in (run for loop in loops for run in loop):
        side = facade_of(run, south, east)
        # each run of a compound is its own stretch of wall: another seed for each facade of a kind
        k = used[side] = used.get(side, -1) + 1
        inputs = run_inputs(math.dist(*run), side, floors, seed + k, shop_seed + k, own)
        runs.append(fit_rooms(facade(*graph_.tap(inputs), side, run, roof_m), rects))
        recipe.append({"from": list(run[0]), "to": list(run[1]), "facade": side, "Seed": seed + k, "Shop Seed": shop_seed + k,
                       "Corner Margin": inputs["Corner Margin"]})
    paint = tuple(((c / 255 + 0.055) / 1.055) ** 2.4 if c / 255 > 0.04045 else c / 255 / 12.92 for c in wall)
    for run in runs:  # a stain is the wall's own colour, darker
        run.rows = [(n, m, (*paint, 1.0) if n.startswith("CNK_Decal_") else tint) for n, m, tint in run.rows]
    on_roof = [row for k, part in enumerate(rects) for row in roof_rows(graph_, part, floors, seed + k, roof_, own)]
    return SimpleNamespace(
        name=name, floors=floors, parts=parts_, rects=rects, loops=loops, runs=runs, seed=seed, wall=wall, roof_rows=on_roof,
        rows=[row for run in runs for row in run.rows] + on_roof,
        recipe={"inputs": {k: (list(v) if isinstance(v, tuple) else v) for k, v in {**INPUTS, **own}.items()},
                "facades": recipe, "roof": list(roof_)})


def main():
    graph_ = Graph()
    version = ".".join(str(v) for v in bpy.app.version)
    built = [assemble(graph_, template) for template in TEMPLATES]
    meshes = {}
    for tap, _ in graph_.taps.values():
        meshes.update(tap.meshes)
    meshes = {name: sheeted(soup) if name.startswith(SHEETED) else soup for name, soup in meshes.items()}
    meshes.update({name: room_mesh(material) for name, material in ROOM_MESHES.items()})
    # a decal is a face on the wall's own plane: stood off it, the two do not fight for depth
    off = np.eye(4)
    off[1, 3] = -0.015
    meshes = {name: soup.transformed(off) if name.startswith("CNK_Decal_") else soup for name, soup in meshes.items()}

    # whether the graph tints a mesh's instances, and every mesh a state places: a kit mesh, or a variant of one
    tinted, cache = {}, {}
    for b in built:
        for name, _, tint in b.rows:
            if tinted.setdefault(name, tint is not None) != (tint is not None):
                raise SystemExit(f"{name} is instanced both with and without a tint")

    def source(name):
        if name not in cache:
            base, _, kind = name.partition("+")
            cache[name] = variant(masked(meshes[base], False), kind, sum(name.encode())) if kind else masked(meshes[base], tinted[base])
        return cache[name]

    # the states: a block of six floors or fewer collapses, a taller one burns out and stands
    states = []
    for b in built:
        state = {"intact": (b.rows, lambda modules, tiers_of, b=b: intact_shell(b, modules, tiers_of, meshes))}
        if collapse.damage_state(b.floors) == "ruin":
            state["ruin"] = ruin(b, meshes, source)
        else:
            state["gutted"] = gutted(b, meshes)
        states.append(state)

    used = sorted({name for state in states for rows, _ in state.values() for name, _, _ in rows})
    for name in used:
        if not family_tiers(name):
            raise SystemExit(f"{name} is in no family of FAMILIES")
    modules = {name: module_tiers(name, source(name)) for name in used}
    # a module is drawn at the tiers its family names, where it has anything left to draw
    tiers_of = {name: sum(1 << t for t in range(4) if family_tiers(name) >> t & 1 and modules[name][t] is not None)
                for name in modules}
    # every module has four meshes: a tier it is not drawn at repeats the last one it is.
    # A fitting the fire leaves nothing of (a rack of cloth) is no module, and its rows are not drawn.
    kit = {}
    for name, lods in modules.items():
        if tiers_of[name] & 1:
            kit[module_id(name)] = [next(l for l in reversed(lods[:t + 1]) if l is not None) for t in range(4)]
        else:
            tiers_of[name] = 0

    templates = []
    for b, state in zip(built, states):
        placed = {}
        for which, (rows, shell) in state.items():
            module = b.name.replace("-", "_") + ("_shell" if which == "intact" else "_" + which)
            kit[module] = shell(modules, tiers_of)
            placed[which] = [(module, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 0b1111, smoked(b.wall) if which == "gutted" else b.wall)]
            for n, m, tint in rows:
                row = decompose(m)
                if row is None or min(row[4:]) <= 0:
                    raise SystemExit(f"{n}: a row that tilts or mirrors needs a module variant")
                if tiers_of[n] & ROW_TIERS:
                    placed[which].append((module_id(n), row, tiers_of[n] & ROW_TIERS, WHITE if tint is None else srgb_bytes(tint[:3])))
        openings = [(m[0, 3], m[1, 3]) for run in b.runs for _, m in run.openings]
        doors = [(m[0, 3], m[1, 3]) for n, m, _ in b.rows if n.startswith("CNK_Ent_")]
        templates.append((descriptor(b.name, b.floors, b.parts, openings, doors), b, placed))

    kit = {name: [clean(s) for s in lods] for name, lods in kit.items()}
    names = sorted(kit)
    index = {name: i for i, name in enumerate(names)}
    triangles = {name: [len(s) for s in kit[name]] for name in names}
    doc_templates, table, over = [], [], []
    worst = np.zeros(2)
    for desc, b, placed in templates:
        boxes = np.array([[*p["center"], *p["half_extents"]] for p in desc["parts"]])
        _, _, top_m, ruin_m = heights(b.floors)
        packed, drawn_at = {}, {}
        for which, rows in placed.items():
            rows = packed[which] = sorted([index[name], *(round(float(v), 5) for v in row), tiers, *tint] for name, row, tiers, tint in rows)
            drawn_at[which] = [sum(triangles[names[r[0]]][t] for r in rows if r[8] >> t & 1) for t in range(4)]
            table.append((desc["id"], which, len(rows), drawn_at[which]))
            over += [f"{desc['id']} {which} draws {n} triangles at tier {t}, over {BUDGET[t]}"
                     for t, n in enumerate(drawn_at[which]) if n > BUDGET[t]]
            over += [f"{desc['id']} {which} draws {n} triangles at tier {t}, more than intact's {drawn_at['intact'][t]}"
                     for t, n in enumerate(drawn_at[which]) if n > drawn_at["intact"][t]]
            # the fit: every vertex inside some part grown by FIT's side, none below the ground, and none above what
            # the simulation leaves standing: the parts and FIT's top, or for a ruin its remains box
            ceiling = ruin_m if which == "ruin" else top_m + FIT["top_m"]
            for r in rows:
                for t in range(4):
                    if r[8] >> t & 1:
                        v = kit[names[r[0]]][t].transformed(row_matrix(r[1:8])).v
                        side = np.maximum(np.abs(v[:, None, 0] - boxes[:, 0]) - boxes[:, 2],
                                          np.abs(v[:, None, 1] - boxes[:, 1]) - boxes[:, 3]).min(1).max()
                        if side > FIT["side_m"] or v[:, 2].max() > ceiling + 1e-3 or v[:, 2].min() < -1e-3:
                            raise SystemExit(f"{desc['id']} {which}: {names[r[0]]} at tier {t} reaches {side:.2f} m past the sides, "
                                             f"up to {v[:, 2].max():.2f} m (the limit is {ceiling:.2f}) and {-v[:, 2].min():.3f} m below the ground")
                        if which != "ruin":
                            worst = np.maximum(worst, (side, v[:, 2].max() - top_m))
        doc_templates.append({"status": "release", "recipe": b.recipe, "descriptor": desc, "states": packed})

    doc = {"set": SET, "kit": f"city_kit_{SET}", "fit": FIT,
           "source": {"script": "china.py", "blend": BLEND, "blender": version},
           "modules": names, "templates": doc_templates}
    if "--dry" not in sys.argv:
        os.makedirs(OUT, exist_ok=True)
        with open(os.path.join(OUT, "templates.json"), "w") as f:
            f.write(json.dumps(doc, separators=(",", ":")) + "\n")
        write_kit(os.path.join(OUT, "kit.glb"), kit)

    print(f"KIT {len(names)} modules, triangles per tier {[sum(t[i] for t in triangles.values()) for i in range(4)]}")
    for name in names:
        print(f"MODULE {name:38s} {triangles[name]}")
    print(f"REACH past the sides {worst[0]:.2f} m, past the top {worst[1]:.2f} m (fit {FIT})")
    print(f"{'template':34s} {'state':7s} {'rows':>5s} " + " ".join(f"{'tier ' + str(t):>8s}" for t in range(4)))
    for name, which, count, drawn_at in table:
        print(f"{name:34s} {which:7s} {count:5d} " + " ".join(f"{n:8d}" for n in drawn_at))
    print(f"{'budget':34s} {'':7s} {'':5s} " + " ".join(f"{n:8d}" for n in BUDGET))
    if over:
        raise SystemExit("\n".join(over))


if __name__ == "__main__":
    main()
