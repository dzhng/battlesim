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
- **Tiers.** A tier keeps features larger than its `FEATURE_M` (`detail.py`),
  and each kit family draws down to the tier its `FAMILIES` row names. At the
  two fine tiers a kit mesh is a row; at the two coarse ones it is folded into
  the shell, so a far building is one row.

A template's frame has its origin at the footprint's centre and its entrance
on -Y, the street side: the southmost run takes the graph's entrance facade.
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
# not its defaults. Bays are 3 m, and floors 3 m over a 3.2 m ground floor (S2).
# What is switched off has no place in a building of ours: the street (L5), the
# rooftop sign (unique text and a cable mesh), lit rooms and anything that glows,
# the shops' pavement clutter, and the rain-streak decals (alpha-blended; the
# grime is in the textures instead). The drainpipe input does nothing: in the
# vendored file the nodes that instance the pipes point at no object.
INPUTS = {
    "Ground Floor Height": 3.2, "Floor Height": 3.0, "Bay Width": 3.0, "Wall Thickness": 0.3,
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
    # four 12 m wings round a 17 x 14 m court: 408 bays, so plainer than a slab to stay in a template's budget
    ("block-court-6f", 6, {"south": (0, -13, 20.5, 6), "north": (0, 13, 20.5, 6), "west": (-14.5, 0, 6, 7),
                           "east": (14.5, 0, 6, 7)}, 17, 37, (2, 3, 2), (214, 206, 196),
     {"Balcony Probability": 0.18, "Security Grille Probability": 0.25, "AC Unit Probability": 0.22, "Laundry Probability": 0.2,
      "Lantern Probability": 0.2}),
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
# A window modelled with a leaf open shows the room behind it. Until rooms are drawn
# (C26) a dark pane stands in the opening behind the leaf, or the eye goes through the block.
OPEN_LEAVES = ("CNK_Win_02_Casement",)
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


def ring(loop, z0, z1, out_m, in_m, material, inner=False):
    """A band round a loop of the outline: `out_m` proud of the wall and `in_m` into it,
    from `z0` to `z1`, mitred at every corner. Its outer face and its top; with `inner`
    its inner face too (a parapet seen from the roof)."""
    normals = [right_of(a, b) for a, b in loop]
    moved = lambda k, d: (loop[k][0][0] + d * (normals[k - 1][0] + normals[k][0]),
                          loop[k][0][1] + d * (normals[k - 1][1] + normals[k][1]))
    faces = []
    for k in range(len(loop)):
        n = (k + 1) % len(loop)
        a, b, c, d = moved(k, out_m), moved(n, out_m), moved(n, -in_m), moved(k, -in_m)
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
    """One facade of an evaluated building, moved onto a run of the outline: its wall
    (the faces we can see of it), its instances, and its stretched cubes as one mesh."""
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
    # the inner face is never seen: every opening is closed by a module
    ours &= ~((facing < -0.5) & (deep > wall_m - 1e-3))
    wall = masked(own.keep(ours), True).transformed(place)
    placed, cubes = [], []
    for name, m, tint in rows:
        if name.startswith(ROOF_FAMILIES) or depth(m[:3, 3]) > 0.8:
            continue
        if not name.startswith("primitive:"):
            placed.append((name, place @ m, tint))
            continue
        cube = masked(tap.meshes[name], False).transformed(m).coloured(1.0 if tint is None else tint[:3])
        normals, _ = cube.normals()
        # the face a cube turns to the wall it sits on is never seen
        against = (normals[:, :2] @ out < -0.5) & (depth(cube.v[cube.t].mean(1)) > -0.02)
        cubes.append(cube.keep(~against).transformed(place))
    return wall, placed, Soup.join(cubes)


def roof_rows(graph_, part, floors, seed, roof, own):
    """A part's roof furniture: the graph's for a building of the part's size."""
    cx, cy, hx, hy = part
    inputs = dict(INPUTS, **own, **{"Width": 2.0 * hy, "Depth": 2.0 * hx, "Corner Margin": 1.0, "Floors": floors, "Seed": seed,
                                    "Shop Seed": 0, "Solar Heaters": roof[0], "Roof Props": roof[1], "Roof Vents": roof[2]})
    _, rows = graph_.tap(inputs)
    place = frame(math.pi / 2, cx + hx, cy - hy)
    return [(name, place @ m, tint) for name, m, tint in rows if name.startswith(ROOF_FAMILIES)]


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
    if name.startswith(OPEN_LEAVES):
        soup = Soup.join([soup, detail.front_quad(soup, soup.mats.index("M_CN_Glass"), -0.15, soup.mats)])
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


def shell_tiers(walls, cubes, rows, loops, rects, modules, tiers_of, floors):
    """A template's own mesh at each tier: the walls of every run, the bands, parapet and
    coping round the outline, a roof over every part, the cubes, and at the two coarse
    tiers flat walls with what is left of the kit folded in."""
    ground_m, storey_m, wall_m = INPUTS["Ground Floor Height"], INPUTS["Floor Height"], INPUTS["Wall Thickness"]
    roof_m = ground_m + storey_m * (floors - 1)
    top_m = roof_m + INPUTS["Parapet Height"]
    band = "M_CN_Band"
    crown, belts, floor_bands, flat = [], [], [], []
    for loop in loops:
        crown += [ring(loop, roof_m, top_m, 0.0, wall_m, STUCCO + MASK, inner=True),
                  ring(loop, top_m, top_m + 0.1, 0.07, wall_m + 0.07, band, inner=True)]
        belts += [ring(loop, ground_m - 0.3, ground_m + 0.02, 0.14, 0.06, band), ring(loop, roof_m - 0.3, roof_m + 0.02, 0.1, 0.06, band)]
        floor_bands += [ring(loop, ground_m + storey_m * k - 0.14, ground_m + storey_m * k, 0.05, 0.06, band)
                        for k in range(1, floors - 1)]
        for (ax, ay), (bx, by) in loop:
            flat += [quad([(ax, ay, z0), (bx, by, z0), (bx, by, z1), (ax, ay, z1)], material)
                     for z0, z1, material in ((0.0, ground_m, "M_CN_Stone"), (ground_m, roof_m, STUCCO + MASK))]
    z = roof_m + 0.04
    roofs = [quad([(cx - hx, cy - hy, z), (cx + hx, cy - hy, z), (cx + hx, cy + hy, z), (cx - hx, cy + hy, z)], "M_CN_RoofTile")
             for cx, cy, hx, hy in rects]
    out = []
    for tier, g in enumerate(FEATURE_M):
        folded = [unmasked(modules[name][tier], None if tint is None else tint[:3]).transformed(m) for name, m, tint in rows
                  if tiers_of[name] & ~ROW_TIERS & (1 << tier)]
        body = [*walls, *floor_bands] if tier < 2 else flat
        out.append(Soup.join([*body, *crown, *(belts if tier < 3 else belts[::2]), *roofs,
                              cubes if tier == 0 else detail.simplify(cubes, g), *folded]))
    for tier in range(1, 4):
        if len(clean(out[tier])) > len(clean(out[tier - 1])):
            raise SystemExit(f"a shell's tier {tier} is heavier than its tier {tier - 1}")
    return out


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
def assemble(graph_, template):
    """A template's outline, the walls and cubes of its runs, its instances in its own
    frame, and what made them."""
    name, floors, parts_, seed, shop_seed, roof, wall, own = template
    rects = list(parts_.values())
    loops = outline(rects)
    south, east = min(cy - hy for _, cy, _, hy in rects), max(cx + hx for cx, _, hx, _ in rects)
    roof_m = INPUTS["Ground Floor Height"] + INPUTS["Floor Height"] * (floors - 1)
    walls, cubes, rows, recipe, used = [], [], [], [], {}
    for run in (run for loop in loops for run in loop):
        side = facade_of(run, south, east)
        # each run of a compound is its own stretch of wall: another seed for each facade of a kind
        k = used[side] = used.get(side, -1) + 1
        inputs = run_inputs(math.dist(*run), side, floors, seed + k, shop_seed + k, own)
        wall_, placed, cubes_ = facade(*graph_.tap(inputs), side, run, roof_m)
        walls.append(wall_)
        cubes.append(cubes_)
        rows += placed
        recipe.append({"from": list(run[0]), "to": list(run[1]), "facade": side, "Seed": seed + k, "Shop Seed": shop_seed + k,
                       "Corner Margin": inputs["Corner Margin"]})
    for k, part in enumerate(rects):
        rows += roof_rows(graph_, part, floors, seed + k, roof, own)
    return loops, walls, Soup.join(cubes), rows, {"inputs": {k: (list(v) if isinstance(v, tuple) else v) for k, v in {**INPUTS, **own}.items()},
                                "facades": recipe, "roof": list(roof)}


def main():
    graph_ = Graph()
    version = ".".join(str(v) for v in bpy.app.version)
    built = [assemble(graph_, template) for template in TEMPLATES]

    # the kit: every mesh a template instances, and whether the graph tints its instances
    sources, tinted, meshes = {}, {}, {}
    for tap, _ in graph_.taps.values():
        meshes.update(tap.meshes)
    for _, _, _, rows, _ in built:
        for name, _, tint in rows:
            if not family_tiers(name):
                raise SystemExit(f"{name} is in no family of FAMILIES")
            if tinted.setdefault(name, tint is not None) != (tint is not None):
                raise SystemExit(f"{name} is instanced both with and without a tint")
            sources[name] = meshes[name]
    modules = {name: module_tiers(name, masked(sources[name], tinted[name])) for name in sorted(sources)}
    # a module is drawn at the tiers its family names, where it has anything left to draw
    tiers_of = {name: sum(1 << t for t in range(4) if family_tiers(name) >> t & 1 and modules[name][t] is not None)
                for name in modules}
    # every module has four meshes: a tier it is not drawn at repeats the last one it is
    kit = {}
    for name, lods in modules.items():
        kit[module_id(name)] = [next(l for l in reversed(lods[:t + 1]) if l is not None) for t in range(4)]

    templates = []
    for (name, floors, parts_, _, _, _, wall, _), (loops, walls, cubes, rows, recipe) in zip(TEMPLATES, built):
        shell = name.replace("-", "_") + "_shell"
        kit[shell] = shell_tiers(walls, cubes, rows, loops, list(parts_.values()), modules, tiers_of, floors)
        placed = [(shell, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 0b1111, wall)]
        openings, doors = [], []
        for n, m, tint in rows:
            row = decompose(m)
            if row is None or min(row[4:]) <= 0:
                raise SystemExit(f"{n}: a row that tilts or mirrors needs a module variant, and the China graph has none")
            if n.startswith(OPENINGS):
                openings.append((m[0, 3], m[1, 3]))
            if n.startswith("CNK_Ent_"):
                doors.append((m[0, 3], m[1, 3]))
            if tiers_of[n] & ROW_TIERS:
                placed.append((module_id(n), row, tiers_of[n] & ROW_TIERS, WHITE if tint is None else srgb_bytes(tint[:3])))
        templates.append((descriptor(name, floors, parts_, openings, doors), recipe, placed))

    kit = {name: [clean(s) for s in lods] for name, lods in kit.items()}
    names = sorted(kit)
    index = {name: i for i, name in enumerate(names)}
    triangles = {name: [len(s) for s in kit[name]] for name in names}
    doc_templates, table = [], []
    worst = np.zeros(2)
    for desc, recipe, placed in templates:
        rows = sorted([index[name], *(round(float(v), 5) for v in row), tiers, *tint] for name, row, tiers, tint in placed)
        table.append((desc["id"], len(rows), [sum(triangles[names[r[0]]][t] for r in rows if r[8] >> t & 1) for t in range(4)]))
        # the fit: every vertex inside some part grown by FIT, and none below the ground, at every tier a row draws at
        boxes = np.array([[*p["center"], *p["half_extents"]] for p in desc["parts"]])
        for r in rows:
            for t in range(4):
                if r[8] >> t & 1:
                    v = kit[names[r[0]]][t].transformed(row_matrix(r[1:8])).v
                    side = np.maximum(np.abs(v[:, None, 0] - boxes[:, 0]) - boxes[:, 2],
                                      np.abs(v[:, None, 1] - boxes[:, 1]) - boxes[:, 3]).min(1).max()
                    reach = np.array([side, v[:, 2].max() - 2 * boxes[0, 4]])
                    if side > FIT["side_m"] or reach[1] > FIT["top_m"] or v[:, 2].min() < -1e-3:
                        raise SystemExit(f"{desc['id']}: {names[r[0]]} at tier {t} reaches {side:.2f} m past the sides, "
                                         f"{reach[1]:.2f} m past the top and {-v[:, 2].min():.3f} m below the ground")
                    worst = np.maximum(worst, reach)
        doc_templates.append({"status": "release", "recipe": recipe, "descriptor": desc, "states": {"intact": rows}})

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
    print(f"REACH past the sides {worst[0]:.2f} m, past the top {worst[1]:.2f} m (fit {FIT})")
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
