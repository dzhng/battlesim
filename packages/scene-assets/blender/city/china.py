"""The China apartment blocks: one shared kit and five templates.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/china.py

writes `assets/source/city/china_apartments/kit.glb` and `templates.json` (the
format is in this folder's readme) and prints what each template draws at each
tier; with `-- --dry` it only prints. It needs the ambientCG sets in the pack
cache (`packs.py fetch ambientcg`).

The source is the vendored `CN_ApartmentBuilding.blend`, a geometry-nodes
building, evaluated once per template with its two closing Realize Instances
nodes muted in memory so its instances can be read (`graph.py`):

- **Kit modules** are the graph's own kit meshes (a window, a balcony, an air
  conditioner). A row places one: position, yaw, scale per axis, and the tint
  the graph stored on the instance.
- **A template's shell** is everything the graph generated for that recipe
  alone: the walls with their window openings, bands, parapet and roof, and the
  unit cubes it stretches into window surrounds and sign boards. A stretched
  cube cannot carry a baked texture, so they are folded into the shell with UVs
  in metres.
- **Tiers.** A tier keeps features larger than its `FEATURE_M` (`detail.py`),
  and each kit family draws down to the tier its `FAMILIES` row names. At the
  two fine tiers a kit mesh is a row; at the two coarse ones it is folded into
  the shell, so a far building is one row.

The graph faces its entrance along its Depth side. A template's frame has its
origin at the footprint's centre, its long side along X and the entrance on -Y,
so a recipe's Width is the template's short side.
"""
import json
import math
import os
import sys

import bpy
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
BLENDER = os.path.dirname(HERE)
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(BLENDER)))
sys.path.insert(0, HERE)
sys.path.insert(0, BLENDER)
import ambientcg  # noqa: E402
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
# not its defaults. Sides are 3n + 2 m with 3 m bays, and floors 3 m over a 3.2 m
# ground floor (S2). What is switched off has no place in a building of ours:
# the street (L5), the rooftop sign (unique text and a cable mesh), lit rooms and
# anything that glows, the shops' pavement clutter, and the rain-streak decals
# (alpha-blended; the grime is in the textures instead). The drainpipe input does
# nothing: in the vendored file the nodes that instance the pipes point at no object.
INPUTS = {
    "Ground Floor Height": 3.2, "Floor Height": 3.0, "Bay Width": 3.0, "Corner Margin": 1.0, "Wall Thickness": 0.3,
    "Detail Level": "LOD0", "Facade Finish": "Stucco", "Wall Tint": (1.0, 1.0, 1.0, 1.0), "Stone Ground Floor": True,
    "Floor Bands": True, "Parapet Height": 1.1, "Weathering": 0.0,
    "Window Width": 1.5, "Window Height": 1.5, "Sill Height": 0.9, "Small Window Probability": 0.15,
    "Curtain Probability": 0.75, "Lit Window Probability": 0.0,
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
# (kind, long side, short side, floors, seed, shop seed, roof furniture (solar, props, vents), wall colour sRGB)
TEMPLATES = (
    ("slab", 35, 11, 4, 11, 31, (3, 5, 3), (233, 229, 218)),
    ("slab", 47, 11, 5, 12, 32, (4, 7, 4), (226, 214, 184)),
    ("slab", 59, 14, 6, 13, 33, (6, 10, 6), (200, 208, 200)),
    ("slab", 53, 14, 8, 14, 34, (5, 9, 5), (224, 204, 192)),
    ("point", 20, 20, 7, 15, 35, (3, 5, 3), (204, 211, 219)),
)
FIT = {"side_m": 1.5, "top_m": 3.5}
BUDGET = (150_000, 50_000, 12_000, 2_000)
BAY_M = 3.0
WHITE = (255, 255, 255)

# ---------------------------------------------------------------- tiers
# The smallest feature each tier keeps, in metres.
FEATURE_M = (0.05, 0.2, 0.6, 1.5)
# Kit family (a prefix of the source object's name) -> the tiers it draws at, as
# the row's bits. A family not named here is not part of a building.
FAMILIES = (
    ("CNK_Win_", 0b1111), ("CNK_Shop_", 0b1111), ("CNK_Ent_", 0b1111), ("CNK_Balc_", 0b1111),
    ("CNK_RoofBulk_", 0b1111),
    ("CNK_AC_", 0b0111), ("CNK_Awn_", 0b0111), ("CNK_RoofProp_", 0b0111), ("CNK_Laundry", 0b0111),
    ("CNK_Grille_", 0b0011), ("CNK_RoofSmall_", 0b0011),
    ("CNK_Text", 0b0001), ("CNK_Lant_", 0b0001),
    ("CNK_Int_", 0b0001), ("CNK_ShopInt_", 0b0001),
)
ROW_TIERS = 0b0011  # drawn as rows; the coarser tiers are folded into the shell
# What fills a wall opening: at the two coarse tiers the wall is flat and each is one quad on it.
OPENINGS = ("CNK_Win_", "CNK_Shop_", "CNK_Ent_")
# A box hung on the wall: at the two coarse tiers a hull of its bounds (`detail.hull`),
# banded by height, then only its front and sides.
HULLS = ("CNK_Balc_",)
HULL_BAND_M = 1.0
# Behind glass. Glass is opaque until the renderer has a glass path (C25), so these are
# drawn only where the front is open: behind `OPEN_FRONTS`.
BEHIND_GLASS = ("CNK_Int_", "CNK_ShopInt_", "CNK_Curt_")
OPEN_FRONTS = ("CNK_Shop_04_OpenStall",)
# Inside a glazed-in balcony, hidden for the same reason: its laundry and the door onto it.
ON_BALCONY = ("CNK_LaundryBalc", "CNK_Win_05_BalcDoor")
GLAZED_BALCONY = "CNK_Balc_01_Enclosed"
# No path in the renderer until cutout (C24): the rain-streak decals, and the pot plants,
# which are leaf cards over a pot.
NO_PATH = ("CNK_Decal_", "CNK_Plant_")
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
    "M_CN_Stone": ("cn_stone", "cn_stone", (0.95, 0.95, 0.96), 1.0, 0.0),
    "M_CN_RoofTile": ("cn_roof_tile", "cn_roof_tile", (0.8, 0.7, 0.66), 1.0, 0.0),
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
    # glass has no path yet: opaque, dark and glossy, as the village's windows are
    "M_CN_Glass": ("cn_glass", None, (0.015, 0.02, 0.025), 0.1, 0.0),
    "M_CN_GlassFrosted": ("cn_glass_frosted", None, (0.3, 0.35, 0.36), 0.4, 0.0),
    "M_CN_PVC": ("cn_pvc", None, (0.38, 0.52, 0.5), 0.3, 0.0),
    "M_CN_SolarTube": ("cn_solar_tube", None, (0.03, 0.05, 0.09), 0.15, 0.3),
    "M_CN_SignText": ("cn_sign_text", None, (1.0, 1.0, 1.0), 0.4, 0.0),
    # a lamp, unlit: nothing glows
    "M_CN_Emissive": ("cn_lamp", None, (0.8, 0.78, 0.72), 0.3, 0.0),
}
# The source shader multiplies these by the kit's colour attribute (`col=True` in its material script).
TAKES_COLOUR = frozenset((
    STUCCO, "M_CN_Aluminum", "M_CN_SteelBlack", "M_CN_PaintWhite", "M_CN_Shutter", "M_CN_Plastic", "M_CN_PlasticDirty",
    "M_CN_Fabric", "M_CN_Wood", "M_CN_IntWall", "M_CN_SignBoard", "M_CN_SignText", "M_CN_Emissive",
))
# Alpha-tested cards and decals: no path in the renderer yet (C24).
LEFT_OUT = frozenset(("M_CN_Decal", "M_CN_Leaves", "M_CN_Bark"))
# The source turns this material's UVs a quarter (its ridges run across).
TURNED = frozenset(("M_CN_Shutter",))
# A masked surface takes its row's tint: where the graph tints an instance, the surfaces
# of it that take colour. The walls take the building's own colour this way.
MASK = "|tint"


def grime(streak, blotch, colour=(0.2, 0.18, 0.15)):
    """Dirt burnt into a wall texture: damp blotches, and rain streaks down it."""

    def burn(albedo, roughness):
        blotches = textures.smoothstep(0.45, 0.85, textures.fbm(3, 911, 4))
        runs = textures.smoothstep(0.55, 0.9, textures.fbm((40, 2), 913, 3)) * (0.5 + 0.5 * textures.fbm(5, 915, 3))
        dirt = np.clip(blotch * blotches + streak * runs, 0.0, 0.9)
        return textures.mix(albedo, np.asarray(colour) * albedo.mean((0, 1)) / max(albedo.mean(), 1e-6), dirt), \
            np.clip(roughness + 0.25 * dirt, 0.0, 1.0)

    return burn


def recipes():
    # the stucco covers 4.8 m (the set twice each way) so its streaks do not repeat bay to bay
    ambientcg.bake("cn_stucco", "PaintedPlaster017", 2.4, repeat=2, tint=(1.0, 0.985, 0.955), rough=(0.9, 0.08),
                   normal=0.55, grime=grime(0.35, 0.3))
    ambientcg.bake("cn_concrete", "Concrete034", 2.0, rough=(1.0, 0.05), normal=0.6, grime=grime(0.35, 0.3))
    ambientcg.bake("cn_stone", "Tiles138", 1.2, normal=0.8, grime=grime(0.0, 0.25))
    # the roof is the largest surface from above: four repeats, so its dirt does not chequer
    ambientcg.bake("cn_roof_tile", "Tiles047", 1.2, repeat=4, rough=(1.0, 0.05), normal=0.8,
                   grime=grime(0.0, 0.6, (0.12, 0.12, 0.1)))
    ambientcg.bake("cn_metal", "Metal009", 1.0, rough=(1.0, 0.05), normal=0.4)
    ambientcg.bake("cn_corrugated", "CorrugatedSteel005", 1.0)
    ambientcg.bake("cn_plastic", "Plastic010", 1.0, rough=(0.8, 0.12), normal=0.3)
    ambientcg.bake("cn_fabric", "Fabric036", 0.6, rough=(1.0, 0.1), normal=0.6)
    ambientcg.bake("cn_wood", "WoodFloor041", 1.5, rough=(1.0, 0.05), normal=0.6)


def material_name(source):
    masked = source.endswith(MASK)
    return SURFACES[source.removesuffix(MASK)][0] + ("_tint" if masked else "")


# ---------------------------------------------------------------- the graph
def family_tiers(name):
    return next((tiers for prefix, tiers in FAMILIES if name.startswith(prefix)), 0)


def recipe_inputs(long_m, short_m, floors, seed, shop_seed, roof):
    return dict(INPUTS, Width=float(short_m), Depth=float(long_m), Floors=floors, Seed=seed,
                **{"Shop Seed": shop_seed, "Solar Heaters": roof[0], "Roof Props": roof[1], "Roof Vents": roof[2]})


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


def template_frame(long_m, short_m):
    """Graph space (a corner at the origin, the entrance on x = 0) to the template's."""
    return np.array([[0.0, -1.0, 0.0, long_m / 2], [1.0, 0.0, 0.0, -short_m / 2], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])


# ---------------------------------------------------------------- modules
def masked(soup, tinted):
    """A kit mesh's surfaces as ours: what has no path left out, and the surfaces that
    take a row's tint marked (white where the tint will multiply)."""
    soup = soup.without(LEFT_OUT | {""})
    mats, colour = [], soup.c.copy()
    for i, name in enumerate(soup.mats):
        if tinted and name in TAKES_COLOUR:
            colour[np.unique(soup.t[soup.m == i])] = 1.0
            name += MASK
        mats.append(name)
    return Soup(soup.v, colour, soup.t, soup.m, soup.s, mats)


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
    repeats that one."""
    out = []
    for tier, g in enumerate(FEATURE_M):
        if tier >= 2 and name.startswith(OPENINGS):
            front = front_material(soup)  # the pane, not its frame: a far window is no bigger than a near one
            lod = detail.front_quad(soup.keep(soup.m == front), front, 0.03, soup.mats)
        elif tier >= 2 and name.startswith(HULLS):
            lod = detail.hull(soup, HULL_BAND_M if tier == 2 else 100.0, caps=tier == 2)
        else:
            lod = detail.simplify(soup, g)
        finer = next((l for l in reversed(out) if l is not None), None)
        if finer is not None and len(clean(lod)) > len(clean(finer)):
            lod = finer
        out.append(lod if len(lod) else None)
    return out


def module_id(name):
    return name.removeprefix("CNK_").lower()


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


def near(a, b, reach):
    return math.hypot(a[0, 3] - b[0, 3], a[1, 3] - b[1, 3]) < reach and abs(a[2, 3] - b[2, 3]) < 0.1


def drawn(rows):
    """The instances a building of ours draws, from all the graph makes."""
    fronts = [m for name, m, _ in rows if name.startswith(OPEN_FRONTS)]
    glazed = [m for name, m, _ in rows if name == GLAZED_BALCONY]
    out = []
    for name, m, tint in rows:
        if name.startswith(NO_PATH):
            continue
        if name.startswith(BEHIND_GLASS) and not any(near(m, f, 0.5) for f in fronts):
            continue
        if name.startswith(ON_BALCONY) and any(near(m, b, 1.0) for b in glazed):
            continue
        out.append((SIGN_SWAPS.get(name, name), m, tint))
    return out


# ---------------------------------------------------------------- the shell
def quad(corners, material):
    return Soup(np.array(corners, dtype=np.float64), np.ones((4, 3)), [(0, 1, 2), (0, 2, 3)], [0, 0], [False, False], [material])


def massing(short_m, long_m, ground_m, top_m, wall_m):
    """The flat walls of the coarsest tier in graph space: stone to the ground floor's
    head, stucco above to the parapet's top, and the parapet's inner faces."""
    w, d = short_m, long_m
    ring = [(0.0, 0.0), (w, 0.0), (w, d), (0.0, d)]
    inner = [(wall_m, wall_m), (w - wall_m, wall_m), (w - wall_m, d - wall_m), (wall_m, d - wall_m)]
    roof = top_m - INPUTS["Parapet Height"]
    out = []
    for i in range(4):
        (ax, ay), (bx, by) = ring[i], ring[(i + 1) % 4]
        for z0, z1, material in ((0.0, ground_m, "M_CN_Stone"), (ground_m, top_m, STUCCO + MASK)):
            out.append(quad([(ax, ay, z0), (bx, by, z0), (bx, by, z1), (ax, ay, z1)], material))
        (ax, ay), (bx, by) = inner[i], inner[(i + 1) % 4]
        out.append(quad([(bx, by, roof), (ax, ay, roof), (ax, ay, top_m), (bx, by, top_m)], STUCCO + MASK))
    return Soup.join(out)


def shell_tiers(tap, rows, modules, tiers_of, dims, tint_of):
    """A template's own mesh at each tier, in graph space."""
    short_m, long_m, ground_m, roof_m, top_m = dims
    wall_m = INPUTS["Wall Thickness"]
    # the graph tints the walls and the parapet; the overhead cables are 2 cm tubes, 8,500 triangles
    own = masked(tap.own, True).without({"M_CN_Rubber"})
    # The walls' inner faces are never seen: every opening is closed by a module.
    normals, _ = own.normals()
    centre = own.v[own.t].mean(1)
    inset = np.minimum.reduce([centre[:, 0], short_m - centre[:, 0], centre[:, 1], long_m - centre[:, 1]])
    hidden = (np.abs(inset - wall_m) < 1e-3) & (np.abs(normals[:, 2]) < 0.1) & (centre[:, 2] < roof_m)
    own = own.keep(~hidden)
    cubes = Soup.join([masked(tap.meshes[name], False).transformed(m).coloured(1.0 if tint is None else tint[:3])
                       for name, m, tint in rows if name.startswith("primitive:")])
    band = own.keep(np.array(own.material_names()) == "M_CN_Band")
    flat = Soup.join([massing(short_m, long_m, ground_m, top_m, wall_m),
                      own.keep(np.array(own.material_names()) == "M_CN_RoofTile"), detail.simplify(band, FEATURE_M[3])])
    out = []
    for tier, g in enumerate(FEATURE_M):
        folded = [unmasked(modules[name][tier], tint_of(tint)).transformed(m) for name, m, tint in rows
                  if not name.startswith("primitive:") and tiers_of[name] & ~ROW_TIERS & (1 << tier)]
        out.append(Soup.join([flat if tier >= 2 else own, cubes if tier == 0 else detail.simplify(cubes, g), *folded]))
    for tier in range(1, 4):
        if len(clean(out[tier])) > len(clean(out[tier - 1])):
            raise SystemExit(f"a shell's tier {tier} is heavier than its tier {tier - 1}")
    return out


# ---------------------------------------------------------------- the descriptor
EDGES = (  # id, facade, the axis its offsets run along (templates.rs `Facade::axes`), which half extent it spans
    ("body-east", "positive_x", (0.0, 1.0), 1), ("body-north", "positive_y", (-1.0, 0.0), 0),
    ("body-west", "negative_x", (0.0, -1.0), 1), ("body-south", "negative_y", (1.0, 0.0), 0),
)


def descriptor(kind, long_m, short_m, floors, half, openings, doors):
    """The physical template, with each facade's bay lattice checked against where the
    graph put its windows. `openings` and `doors` are template-space positions."""
    edges = []
    offsets = {edge[0]: set() for edge in EDGES}
    for x, y in openings:
        reach = [half[0] - x, half[1] - y, half[0] + x, half[1] + y]  # distance inside each face, in EDGES order
        i = int(np.argmin(reach))
        offsets[EDGES[i][0]].add(round(x * EDGES[i][2][0] + y * EDGES[i][2][1], 3))
    for name, facade, _, axis in EDGES:
        span = half[axis]
        count = round((2 * span - 2) / BAY_M)
        phase = 0.0 if count % 2 else BAY_M / 2
        lattice = {round(phase + BAY_M * k, 3) for k in range(-count, count + 1) if abs(phase + BAY_M * k) < span}
        if offsets[name] != lattice:
            raise SystemExit(f"{name}: windows at {sorted(offsets[name])}, the bay lattice at {sorted(lattice)}")
        edges.append({"id": name, "part": "body", "facade": facade, "span_m": [-span, span], "exposed": True,
                      "bays": {"pitch_m": BAY_M, "phase_m": phase}})
    entrances = []
    for k, (x, y) in enumerate(doors):
        if abs(y + half[1]) > 1e-3:
            raise SystemExit(f"the entrance at {x}, {y} is not on the street side")
        entrances.append({"id": f"door-{k}", "edge": "body-south", "offset_m": round(x, 3)})
    ground, storey = INPUTS["Ground Floor Height"], INPUTS["Floor Height"]
    return {
        "id": f"china-apartment-{kind}-{long_m}x{short_m}-{floors}f", "category": "urban_apartment", "regional_family": "china",
        "parts": [{"id": "body", "center": [0.0, 0.0], "yaw": 0.0, "half_extents": [float(h) for h in half], "base_z": 0.0}],
        "floor_heights_m": [0.0] + [round(ground + storey * k, 3) for k in range(floors - 1)],
        "entrances": entrances, "edges": edges, "joins": [],
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
    recipe = [SURFACES[name.removesuffix(MASK)][1] for name in soup.mats]
    tile = np.array([textures.tile_of(r) if r else 1.0 for r in recipe])[soup.m][:, None]
    turned = np.array([name.removesuffix(MASK) in TURNED for name in soup.mats])[soup.m][:, None]
    return np.stack([np.where(turned, v, u) / tile, np.where(turned, u, v) / tile], -1)


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
        _, recipe, base, rough, metal = SURFACES[source.removesuffix(MASK)]
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


# ---------------------------------------------------------------- main
def main():
    ob, mod = open_graph()
    version = ".".join(str(v) for v in bpy.app.version)
    taps = []
    for kind, long_m, short_m, floors, seed, shop_seed, roof, wall in TEMPLATES:
        inputs = recipe_inputs(long_m, short_m, floors, seed, shop_seed, roof)
        graph.set_inputs(mod, inputs)
        tap = graph.tap(ob, tinted=TAKES_COLOUR)
        taps.append((inputs, tap, drawn(tap.rows)))

    # the kit: every mesh a template instances, and whether the graph tints its instances
    sources, tinted = {}, {}
    for _, tap, rows in taps:
        for name, _, tint in rows:
            if name.startswith("primitive:"):
                continue
            if not family_tiers(name):
                raise SystemExit(f"{name} is in no family of FAMILIES")
            if tinted.setdefault(name, tint is not None) != (tint is not None):
                raise SystemExit(f"{name} is instanced both with and without a tint")
            sources[name] = tap.meshes[name]
    modules = {name: module_tiers(name, masked(sources[name], tinted[name])) for name in sorted(sources)}
    # a module is drawn at the tiers its family names, where it has anything left to draw
    tiers_of = {name: sum(1 << t for t in range(4) if family_tiers(name) >> t & 1 and modules[name][t] is not None)
                for name in modules}
    # every module has four meshes: a tier it is not drawn at repeats the last one it is
    kit = {}
    for name, lods in modules.items():
        kit[module_id(name)] = [next(l for l in reversed(lods[:t + 1]) if l is not None) for t in range(4)]

    templates, table = [], []
    for (kind, long_m, short_m, floors, _, _, _, wall), (inputs, tap, rows) in zip(TEMPLATES, taps):
        ground_m = inputs["Ground Floor Height"]
        roof_m = ground_m + inputs["Floor Height"] * (floors - 1)
        top_m = roof_m + inputs["Parapet Height"]
        frame = template_frame(long_m, short_m)
        half = (long_m / 2, short_m / 2, top_m / 2)
        shell = f"{kind}_{long_m}x{short_m}_{floors}f_shell"
        tint_of = lambda tint: None if tint is None else tint[:3]
        kit[shell] = [s.transformed(frame) for s in
                      shell_tiers(tap, rows, modules, tiers_of, (short_m, long_m, ground_m, roof_m, top_m), tint_of)]
        placed = [(shell, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 0b1111, wall)]
        openings, doors = [], []
        for name, m, tint in rows:
            if name.startswith("primitive:"):
                continue
            m = frame @ m
            row = decompose(m)
            if row is None or min(row[4:]) <= 0:
                raise SystemExit(f"{name}: a row that tilts or mirrors needs a module variant, and the China graph has none")
            if name.startswith(OPENINGS):
                openings.append((m[0, 3], m[1, 3]))
            if name.startswith("CNK_Ent_"):
                doors.append((m[0, 3], m[1, 3]))
            if tiers_of[name] & ROW_TIERS:
                placed.append((module_id(name), row, tiers_of[name] & ROW_TIERS, WHITE if tint is None else srgb_bytes(tint[:3])))
        templates.append((kind, long_m, short_m, floors, half, inputs, placed, openings, doors))

    kit = {name: [clean(s) for s in lods] for name, lods in kit.items()}
    names = sorted(kit)
    index = {name: i for i, name in enumerate(names)}
    triangles = {name: [len(s) for s in kit[name]] for name in names}
    doc_templates = []
    lo, hi = np.full(3, np.inf), np.full(3, -np.inf)
    for kind, long_m, short_m, floors, half, inputs, placed, openings, doors in templates:
        rows = sorted([index[name], *(round(float(v), 5) for v in row), tiers, *tint] for name, row, tiers, tint in placed)
        drawn_at = [sum(triangles[names[r[0]]][t] for r in rows if r[8] >> t & 1) for t in range(4)]
        desc = descriptor(kind, long_m, short_m, floors, half, openings, doors)
        table.append((desc["id"], len(rows), drawn_at))
        # the fit: nothing past the part's faces by more than FIT, at any tier
        for r in rows:
            m = row_matrix(r[1:8])
            for t in range(4):
                if r[8] >> t & 1:
                    b = np.array(kit[names[r[0]]][t].transformed(m).bounds())
                    reach = [max(-b[0, 0] - half[0], b[1, 0] - half[0], -b[0, 1] - half[1], b[1, 1] - half[1]),
                             b[1, 2] - 2 * half[2], -b[0, 2]]
                    if reach[0] > FIT["side_m"] or reach[1] > FIT["top_m"] or reach[2] > 1e-3:
                        raise SystemExit(f"{desc['id']}: {names[r[0]]} at tier {t} reaches {reach[0]:.2f} m past the sides, "
                                         f"{reach[1]:.2f} m past the top and {reach[2]:.3f} m below the ground")
                    lo, hi = np.minimum(lo, reach), np.maximum(hi, reach)
        doc_templates.append({
            "status": "release",
            "recipe": {k: (list(v) if isinstance(v, tuple) else v) for k, v in inputs.items()},
            "descriptor": desc,
            "states": {"intact": rows},
        })

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
        print(f"MODULE {name:34s} {triangles[name]}")
    print(f"REACH past the sides {hi[0]:.2f} m, past the top {hi[1]:.2f} m (fit {FIT})")
    print(f"{'template':40s} {'rows':>6s} " + " ".join(f"{'tier ' + str(t):>9s}" for t in range(4)))
    over = []
    for name, count, drawn_at in table:
        print(f"{name:40s} {count:6d} " + " ".join(f"{n:9d}" for n in drawn_at))
        over += [f"{name} draws {n} triangles at tier {t}, over {BUDGET[t]}" for t, n in enumerate(drawn_at) if n > BUDGET[t]]
    print(f"{'budget':40s} {'':6s} " + " ".join(f"{n:9d}" for n in BUDGET))
    if over:
        raise SystemExit("\n".join(over))


if __name__ == "__main__":
    main()
