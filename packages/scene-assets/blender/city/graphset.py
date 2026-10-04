"""The exporter every geometry-node apartment set shares: one source, one set.

A source script (`china.py`, `nyc.py`, `paris.py`) is a subclass of `GraphSet`
holding its graph's tables (inputs, kit families and their tiers, materials,
templates) and the few hooks that read its graph (`open_graph`, `assemble`).
Everything after the graph is read is here and is the same for every source:

- **A template is its parts**, boxes that abut. Only the outline of their union
  is built (`outline`): a source dresses each straight run of it with a facade
  of its graph, so a face where two parts join has no wall to hide, and a corner
  of the block is a corner of one building. A run is a `SimpleNamespace` in the
  template's frame: `start`, `along`, `out`, `length`, its `wall` and `inner`
  faces, its `rows` (kit name, 4x4, tint or None), its stretched `cubes`, the
  `openings` in it and those with no room (`bare`).
- **Kit modules** are the graph's own kit meshes. A row places one: position,
  yaw, scale per axis, and the tint the graph stored on the instance.
- **A template's shell** is what is made for it alone: its walls with every
  opening lined, the bands and parapet round the outline, a roof over every
  part, the stretched cubes, and at the two coarse tiers flat walls with what is
  left of the kit folded in.
- **What is not opaque** (README, "Surfaces that are not opaque" and
  "Interiors"). Glass is blended, one face a pane. Behind every window the
  graph stands a room box; ours is the same box showing a cell of the interior
  atlas, fitted to the plan so that no two rooms share space (`fit_rooms`). Thin
  bars are cutout sheets in the bars' own colour (`sheeted`).
- **Tiers.** A tier keeps features larger than its `FEATURE_M` (`detail.py`),
  and each kit family draws down to the tier its `FAMILIES` row names. At the
  two fine tiers a kit mesh is a row; at the two coarse ones it is folded into
  the shell, so a far building is one row.
- **Damage states.** The graphs have no damage inputs, so a destroyed building is
  made here from the intact one (`damage.py`): six floors or fewer collapse to a
  `ruin` inside the simulation's remains box, taller ones stand `gutted`.

A template's frame has its origin at the footprint's centre and its entrance
on -Y, the street side.

A source script runs as `bun run --cwd web asset -- blender <script> [out_dir]`;
`--dry` only prints.
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
import collapse  # noqa: E402
import damage  # noqa: E402
import detail  # noqa: E402
import graph  # noqa: E402
import parts  # noqa: E402
import textures  # noqa: E402
from facade import missing_openings, overlapping_openings  # noqa: E402
from graph import Soup  # noqa: E402

WHITE = (255, 255, 255)
# A masked surface takes its row's tint: where the graph tints an instance, the surfaces
# of it that take colour. The walls take the building's own colour this way.
MASK = "|tint"
# A surface drawn as a cutout sheet of bars in its own colour (`sheeted`): marked before the mask.
CUT = "|cut"
FACES = (  # name, facade, outward normal, the way its offsets run (templates.rs `Facade::axes`)
    ("east", "positive_x", (1.0, 0.0), (0.0, 1.0)), ("north", "positive_y", (0.0, 1.0), (-1.0, 0.0)),
    ("west", "negative_x", (-1.0, 0.0), (0.0, -1.0)), ("south", "negative_y", (0.0, -1.0), (1.0, 0.0)),
)


def script_args():
    """What follows the script on its command line (`asset blender` passes it after `--`)."""
    return [a for a in sys.argv[sys.argv.index("--") + 1:]] if "--" in sys.argv else []


def base_of(name):
    """The source material a surface name stands for, without its marks."""
    return name.split("|")[0]


def frame(angle, x, y):
    c, s = math.cos(angle), math.sin(angle)
    return np.array([[c, -s, 0.0, x], [s, c, 0.0, y], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]])


def quad(corners, material, colour=1.0):
    return damage.quad(corners, material, colour)


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


def linear_of(srgb):
    """An sRGB byte triple as linear floats."""
    return tuple(((c / 255 + 0.055) / 1.055) ** 2.4 if c / 255 > 0.04045 else c / 255 / 12.92 for c in srgb)


# ---------------------------------------------------------------- texture recipes every source may use
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
    """A wall after a fire: smoked grey all over, a little uneven, with a few patches
    where the face has spalled to the render under it. The marks that say fire are the
    fans above the openings, which are geometry: clouds of soot in a texture repeat as
    camouflage, and runs down a brown wall are the grain of a plank. Only the colour
    changes, so the burnt wall shares the intact wall's normal and roughness images."""
    soot = textures.smoothstep(0.25, 0.8, textures.fbm(1, 931, 3))
    spall = textures.smoothstep(0.86, 0.9, textures.fbm(5, 941, 4))
    out = textures.mix(albedo, (0.03, 0.028, 0.026), np.clip(0.52 + 0.18 * soot, 0.0, 0.94))
    render = np.array((0.085, 0.08, 0.075)) * (0.7 + 0.5 * textures.fbm(40, 943, 2))[..., None]
    return textures.mix(out, render, spall * 0.8), roughness


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


def lining(bounds, m, depth_m, material, grow=0.05):
    """The pane that stands in an opening: the bounds of what fills it (in its module's
    frame), across the wall, `depth_m` inside the wall's face (negative: in front of it)."""
    lo, hi = bounds
    corners = np.array([(lo[0] - grow, depth_m, lo[2] - grow), (hi[0] + grow, depth_m, lo[2] - grow),
                        (hi[0] + grow, depth_m, hi[2] + grow), (lo[0] - grow, depth_m, hi[2] + grow)])
    corners = corners @ m[:3, :3].T + m[:3, 3]
    corners[:, 2] = np.maximum(corners[:, 2], 0.0)
    return quad(corners, material)


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


class Graph:
    """The building, evaluated on demand and once per set of inputs."""

    def __init__(self, source):
        self.source = source
        self.ob, self.mod = source.open_graph()
        self.taps = {}

    def tap(self, inputs):
        key = json.dumps(inputs, sort_keys=True)
        if key not in self.taps:
            graph.set_inputs(self.mod, inputs)
            tap = graph.tap(self.ob, tinted=self.source.TAKES_COLOUR, shaped=self.source.SHAPED)
            self.taps[key] = (tap, self.source.drawn(tap))
        return self.taps[key]


class GraphSet:
    """One graph's apartment set. A subclass sets the tables below and the hooks
    `open_graph`, `assemble` and `recipes`; the rest is shared."""

    # ------------------------------------------------------------ what the subclass names
    SET = BLEND = SCRIPT = None
    ID_PREFIX = FAMILY = None  # "china-apartment-", "china"
    TEMPLATES = ()
    FIT = {"side_m": 1.5, "top_m": 3.5}
    BUDGET = (150_000, 50_000, 12_000, 2_000)
    BAY_M = 3.0
    # A building's levels: its ground floor's height, the floors over it, its walls' thickness, the parapet
    GROUND_M = STOREY_M = WALL_M = PARAPET_M = None
    SHAPED = False  # whether `graph.tap` names a generated primitive by its shape (`graph.tap`)
    # Whether a fighting bay with no opening over the eye and muzzle heights refuses the set (`check_bays`),
    # or is only printed: a set older than that check keeps its graph-to-bay lattice as its owner.
    BAYS_REFUSED = True

    # The smallest feature each tier keeps, in metres, and the thinnest a bar is drawn at it. The
    # tactical camera draws tier 0 at about 20 pixels a metre: a bar under 5 cm is under a pixel.
    FEATURE_M = (0.05, 0.25, 0.6, 1.5)
    BAR_M = (0.05, 0.125, 0.3, 0.75)
    # Kit family (a prefix of the source object's name) -> the tiers it draws at, as
    # the row's bits. A family not named here is not part of a building.
    FAMILIES = ()
    # Families whose finest mesh keeps a larger feature than the tier's.
    TIER0_FEATURE_M = ()
    ROW_TIERS = 0b0011  # drawn as rows; the coarser tiers are folded into the shell
    # What fills a wall opening: at the two coarse tiers the wall is flat and each is one quad on it.
    OPENINGS = ()
    # A box hung on the wall: at the two coarse tiers a hull of its bounds (`detail.hull`),
    # banded by height, then only its front.
    HULLS = ()
    HULL_BAND_M = (1.0, 1.3)
    # Room boxes: the graph's room mesh -> ours, ours -> its material, and material -> atlas sheet.
    ROOMS = {}
    ROOM_MESHES = {}
    ROOM_SHEETS = {}
    ROOM_CLEAR_M = 0.35  # a room stops this far short of the middle of the building and of a corner's other room
    ROOM_STEP_M = 0.12  # a ground-floor room's floor is this far over the ground, which would show through it
    # An opening with no room behind it (the entrance) is lined: a dark, matte pane stands in it
    # just inside the wall's thickness. A block is hollow: without one the eye goes in there and
    # out at the far side. A burnt block has no rooms, and every opening of it is lined.
    LINING_M = 0.28
    ON_BALCONY = ()  # what stands behind a glazed-in balcony (`GLAZED`): from far off its glass covers it
    GLAZED = None
    SHEETED = ()  # whose thin bars are drawn as a cutout sheet (`sheeted`)
    SHEET_BAR_M = 0.022  # a bar thinner than this, one of a row of them, is the sheet's
    KIT_PREFIX = ""  # what a kit object's name starts with, left off its module id
    DECAL = None  # the family of stains stood just off the wall
    ENTRANCE = None  # the family of the doors on the street

    # Materials: source material -> (ours, recipe or None for a flat colour, base colour (linear), roughness, metalness)
    SURFACES = {}
    COVERAGE = {}  # surfaces that are not opaque (`textures.surface`); a cutout's coverage is its recipe's image
    FITTED = frozenset()  # a surface whose image is fitted to the mesh it is on, not tiled in metres
    TAKES_COLOUR = frozenset()  # the source shader multiplies these by the kit's colour
    LEFT_OUT = frozenset()  # not in any mesh we draw
    TURNED = frozenset()  # the source turns this material's UVs a quarter
    BARS = None  # the recipe a cutout sheet of bars uses
    WALL = None  # the wall's own material: it takes the building's colour
    STONE = None  # the ground floor's plinth
    BAND = None  # the bands round the outline
    ROOF = None  # the roof's surface
    BURNT_ROOF = None  # what a burnt block's roof is, where it is not gone (the roof's own surface if None)
    GLASS = None  # the glass over a room; from tier 2 it is the dark pane `X_Pane`

    # ------------------------------------------------------------ damage (per source where it differs)
    ROOF_CELL_M = (3.0, 3.0, 6.0, 12.0)
    BURNT_ROOF_CELL_M = (1.0, 1.0, 2.0, 5.0)  # a burn has a ragged edge; a soft one is a cloud's shadow
    SMOKED_ROOF = (0.46, 0.66, 0.8)  # what multiplies the roof the fire has smoked
    BURNT_THROUGH = (0.55, 0.6)  # how hot (0 to 1) the fire was where the roof is gone, and where the slab is
    BURNT_STOREY_M = 2.9
    BURNT_WALL = 0.55  # how far the fire darkens a wall all over; the soot fans fade up to it
    BURNT_TRIM = 0.2  # and its bands, sills and plinth, which are pale and would stay clean
    BURNT_GREY = 0.9  # how far the wall's paint loses its colour
    # What a fire burns away, and how a fitting is left: `name+burnt` charred where it was,
    # `name+hanging` charred and torn half off the wall, `name+wreck` charred and thrown down.
    BURNS = frozenset()
    # How much of each family a fire leaves on the building, and as which variant. What is not named is gone.
    SURVIVES = ()
    FLAT_SOOT = True  # whether tier 2 has a soot fan over every opening, or the farthest tier's few
    FLAT_SOOT_SIDES = True  # whether tier 2's soot fans fade out to either side, or are the one quad over the head
    FAR_SOOT_SHARE = 0.25  # the share of the fans the farthest tier keeps
    BLOWN = 0.035  # the share of a facade's bays above the shops blown out to the floor slabs
    STUMP_STEP_M = 1.5  # a wall breaks off in columns this wide
    SPILL_M = 1.1  # how far the rubble runs out past the walls
    HEAP_CELL_M = (0.9, 1.8, 3.6, 7.2)
    # What lies in the heap, thrown down and charred: one to every `WRECK_M2` square metres of plan.
    WRECKS = ()
    WRECK_M2 = 20.0

    def __init__(self):
        self.materials = {}

    # ------------------------------------------------------------ hooks
    def open_graph(self):
        """The graph's object and modifier, with whatever must be muted to read its instances."""
        raise NotImplementedError

    def drawn(self, tap):
        """The instances a building of ours draws, from all the graph makes."""
        return tap.rows

    def assemble(self, graph_, template):
        """A template built: `name`, `floors`, `parts`, `rects`, `loops`, `runs`, `seed`, `wall`
        (its colour, sRGB), `roof_rows`, `rows` (every run's and the roof's) and `recipe`."""
        raise NotImplementedError

    def recipes(self):
        """Register the texture recipes the materials name."""
        raise NotImplementedError

    def bounds_of(self, meshes, name):
        """An opening's rectangle in its module's frame: what fills it."""
        return meshes[name].bounds()

    def storeys(self, floors):
        """How many of a building's floors stand in its walls (a mansard's does not)."""
        return floors

    def near_walls(self, run, tier):
        """A run's wall at one of the two fine tiers (its shell's own faces, openings cut)."""
        return run.wall

    def holed(self, start, along, out, length, floors, openings, meshes):
        """A run's wall as flat faces round its openings, each opening's reveal lined in the
        wall's own material back to what fills it: a wall of a few faces a bay, for tier 1
        where the graph's own wall is more than its budget."""
        _, roof_m, _, _ = self.heights(floors)
        at = lambda s, deep, z: (*(start + along * s - out * deep), z)
        bands = [0.0] + [self.GROUND_M + self.STOREY_M * k for k in range(floors) if self.GROUND_M + self.STOREY_M * k < roof_m - 1e-3] + [roof_m]
        faces = []
        # the faces between openings break on the bay lattice, so a bay blown out of a burnt block takes its own wall only
        bays = math.floor((length - 2.0) / self.BAY_M + 1e-9)
        cuts = [(length - self.BAY_M * bays) / 2 + self.BAY_M * k for k in range(bays + 1)]
        between = lambda a, b, z0, z1: [((p, z0), (q, z1)) for p, q in zip(*(lambda e: (e[:-1], e[1:]))(sorted({a, b, *(c for c in cuts if a < c < b)})))]
        for z0, z1 in zip(bands, bands[1:]):
            holes = []
            for n, m in openings:
                if not z0 - 0.01 < m[2, 3] < z1 - 0.01:
                    continue
                lo, hi = self.bounds_of(meshes, n)
                s = float((m[:2, 3] - start) @ along)
                holes.append((s + lo[0], s + hi[0], max(z0, m[2, 3] + lo[2]), min(z1, m[2, 3] + hi[2]), lo[1] if lo[1] > 0.05 else self.WALL_M / 2))
            s0 = 0.0
            for a, b, h0, h1, deep in sorted(holes):
                faces += between(s0, a, z0, z1)
                if h0 > z0 + 1e-3:
                    faces.append(((a, z0), (b, h0)))
                if h1 < z1 - 1e-3:
                    faces.append(((a, h1), (b, z1)))
                s0 = b
                for c0, c1, c2, c3 in (((a, 0, h0), (a, deep, h0), (a, deep, h1), (a, 0, h1)),  # the jambs, the sill, the head
                                       ((b, deep, h0), (b, 0, h0), (b, 0, h1), (b, deep, h1)),
                                       ((a, 0, h0), (b, 0, h0), (b, deep, h0), (a, deep, h0)),
                                       ((a, deep, h1), (b, deep, h1), (b, 0, h1), (a, 0, h1))):
                    faces.append((at(*c0), at(*c1), at(*c2), at(*c3)))
            faces += between(s0, length, z0, z1)
        soups = []
        for face in faces:
            if len(face) == 2:
                (a, z0), (b, z1) = face
                if b - a < 1e-3 or z1 - z0 < 1e-3:
                    continue
                face = (at(a, 0, z0), at(b, 0, z0), at(b, 0, z1), at(a, 0, z1))
            soups.append(quad(face, self.WALL + MASK))
        return Soup.join(soups)

    def far_panes(self, b, meshes, tier):
        """What the coarse tiers' flat walls show of the openings, where the rows' own panes do not."""
        return []

    def standing(self, run):
        """What of a run's wall a ruin cuts down to stumps."""
        return Soup.join([run.wall.coloured(0.72), run.inner.coloured(0.35)])

    def doors(self, b):
        """Where the entrances are, in the template's frame."""
        return [(m[0, 3], m[1, 3]) for n, m, _ in b.rows if n.startswith(self.ENTRANCE)]

    # ------------------------------------------------------------ materials
    def recipe_of(self, name):
        return self.BARS if CUT in name else self.SURFACES[base_of(name)][1]

    def see_through(self, name):
        """Whether a surface is a cutout or blended: no tier's simplifying may touch it."""
        return CUT in name or base_of(name) in self.COVERAGE

    def material_name(self, source):
        return self.SURFACES[base_of(source)][0] + ("_cut" if CUT in source else "") + ("_tint" if source.endswith(MASK) else "")

    def row_tiers(self, name):
        """The tiers a module is drawn at as rows; at the others it draws it is folded into the shell."""
        return self.ROW_TIERS

    def family_tiers(self, name):
        return next((tiers for prefix, tiers in self.FAMILIES if name.startswith(prefix)), 0)

    def masked(self, soup, tinted):
        """A kit mesh's surfaces as ours: what has no path left out, and the surfaces that
        take a row's tint marked (white where the tint will multiply)."""
        soup = soup.without(self.LEFT_OUT | {""})
        mats, colour = [], soup.c.copy()
        for i, name in enumerate(soup.mats):
            if tinted and base_of(name) in self.TAKES_COLOUR:
                colour[np.unique(soup.t[soup.m == i])] = 1.0
                name += MASK
            mats.append(name)
        return Soup(soup.v, colour, soup.t, soup.m, soup.s, mats)

    def sheeted(self, soup):
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
            if dims[order[0]] > 0.3 and dims[order[1]] < self.SHEET_BAR_M:
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

    # ------------------------------------------------------------ modules
    def module_tiers(self, name, soup):
        """The four meshes of a kit module, finest first; None where nothing is left to draw.
        A tier is never heavier than the one before it: where the rule would make it so, it
        repeats that one. A cutout or blended surface is one face already: the two fine tiers
        keep it as it is, and the two coarse ones, where a window is a dark pane on a flat
        wall, have none (glass there is that pane)."""
        if name in self.ROOM_MESHES:
            return [soup] * 4
        clear = np.array([self.see_through(m) for m in soup.mats], dtype=bool)[soup.m] if len(soup) else np.zeros(0, bool)
        solid, sheer = soup.keep(~clear), soup.keep(clear)
        glass = np.array([base_of(m) == self.GLASS for m in soup.mats], dtype=bool)[soup.m] if len(soup) else clear
        far = soup.keep(~clear | glass)  # what the coarse tiers are made from: glass, as a pane
        far = Soup(far.v, far.c, far.t, far.m, far.s, ["X_Pane" if base_of(m) == self.GLASS else m for m in far.mats])
        far = self.coarse(name, far)
        out = []
        for tier, g in enumerate(self.FEATURE_M):
            if tier == 0:
                g = next((own for prefix, own in self.TIER0_FEATURE_M if name.startswith(prefix)), g)
            if name.endswith("+wreck"):  # thrown down, it is neither an opening nor a balcony: a heap of bars and panels
                lod = Soup.join([detail.simplify(solid, g, self.BAR_M[tier]), *([sheer] if tier < 2 else [])])
            elif tier >= 2:
                lod = self.far_tier(name, far, soup, tier, g)
            else:
                lod = Soup.join([detail.simplify(solid, g, self.BAR_M[tier]), sheer])
            finer = next((l for l in reversed(out) if l is not None), None)
            if finer is not None and len(self.clean(lod)) > len(self.clean(finer)):
                lod = finer
            out.append(lod if len(lod) else None)
        return out

    def far_tier(self, name, far, soup, tier, g):
        """A module at a coarse tier, from `far` (its solid surfaces and its glass as a pane)."""
        if name.startswith(self.OPENINGS):
            front = front_material(far)  # the pane, not its frame: a far window is no bigger than a near one
            return detail.front_quad(far.keep(far.m == front), front, 0.03, far.mats)
        if name.startswith(self.HULLS):
            lod = detail.hull(far, self.HULL_BAND_M[tier - 2], caps=tier == 2, sides=tier == 2)
            if tier == 3 and len(lod):  # its front alone, laid on the wall: standing off it, it floats when seen from the side
                flat = lod.v.copy()
                flat[:, 1] = soup.bounds()[1][1] - 0.2
                lod = Soup(flat, lod.c, lod.t, lod.m, lod.s, lod.mats)
            return lod
        return detail.simplify(far, g, self.BAR_M[tier])

    def coarse(self, name, far):
        """What a module's two coarse tiers are made from (the shell draws the wall there)."""
        return far

    def module_id(self, name):
        return name.removeprefix(self.KIT_PREFIX).lower().replace("+", "_").replace(":", "_").replace("~", "_")

    def variant(self, soup, kind, seed):
        soup = damage.charred(soup, self.BURNS, seed)
        # burnt metal is sooted and dull: left a metal, it would mirror the sky
        soup = Soup(soup.v, soup.c, soup.t, soup.m, soup.s, ["X_Soot" + (CUT if CUT in name else "") if base_of(name) in self.SURFACES and self.SURFACES[base_of(name)][4] > 0 else name for name in soup.mats])
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

    # ------------------------------------------------------------ the shell
    def fire_at(self, x, y, seed):
        """How hot (0 to 1) the fire was under a burnt roof at a point."""
        return 0.72 * damage.noise(x, y, 7.0, seed + 9) + 0.28 * damage.noise(x, y, 1.7, seed + 10)

    def roof(self, rects, z, cell_m, seed, burnt=0.0, slab=True):
        """A roof over every part, as a grid whose vertices carry its weathering: a few
        large dark stains and a faint mottle, different on every template. `burnt` (what is
        left of its brightness) makes it the roof of a burnt block: smoked, and where the
        fire came up a hole through it, with the covering off the slab round the hole unless
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
                out.append(Soup(v, colour, tris, np.zeros(len(tris), int), np.zeros(len(tris), bool), [self.ROOF]))
                continue
            # Smoked all over. Where the fire came through, a hole (the storey under it shows, `burnt_storey`), and
            # round the hole the bare slab the covering has fallen from, grey with ash. A cell is one surface, and the
            # edges between them are hard: a dark patch with a soft edge is a tree's shadow.
            middle = (v[a] + v[a + nx + 2]) / 2
            surface = np.digitize(self.fire_at(middle[:, 0], middle[:, 1], seed), self.BURNT_THROUGH)
            surface = np.tile(surface if slab else np.where(surface == 1, 0, surface), 2)
            ash = (0.16 + 0.22 * damage.noise(x, y, 1.1, seed + 13))[:, None] * np.ones(3)
            corner = np.where((surface == 0)[:, None, None], (colour * np.asarray(self.SMOKED_ROOF) * burnt)[tris], ash[tris])
            out.append(Soup(v[tris].reshape(-1, 3), corner.reshape(-1, 3), np.arange(3 * len(tris)).reshape(-1, 3),
                            np.minimum(surface, 1), np.zeros(len(tris), bool), [self.BURNT_ROOF or self.ROOF, "X_Rubble"]).keep(surface != 2))
        return Soup.join(out)

    def intact_roof(self, b, roof_m, tier):
        """The roof the intact building's shell draws at a tier."""
        return self.roof(b.rects, roof_m + 0.04, self.ROOF_CELL_M[tier], b.seed)

    def burnt_storey(self, rects, loops, roof_m, material):
        """What shows through a hole in a burnt roof: the top storey's floor, black with what fell on it, and the
        inside of its walls."""
        z = roof_m - self.BURNT_STOREY_M
        floors = [quad([(cx - hx, cy - hy, z), (cx + hx, cy - hy, z), (cx + hx, cy + hy, z), (cx - hx, cy + hy, z)], "X_Rubble", 0.07)
                  for cx, cy, hx, hy in rects]
        return Soup.join([*floors, *(ring(loop, z, roof_m, 0.0, 0.32, material, inner=True, outer=False).coloured(0.2) for loop in loops)])

    def heights(self, floors):
        """A block's levels: its ground floor's head, its roof, its parapet's top, and what
        the simulation leaves of it when it collapses (`collapse.py`)."""
        ground_m = self.GROUND_M
        roof_m = ground_m + self.STOREY_M * (floors - 1)
        top_m = roof_m + self.PARAPET_M
        return ground_m, roof_m, top_m, collapse.ruin_height(top_m)

    def crown(self, loops, floors, tier, wall, shade=1.0, trim=1.0):
        """What follows the outline: the parapet and its coping, the belt over the shops and
        the band under the parapet, and at the fine tiers a band at every floor. `shade`
        darkens the parapet and `trim` the bands."""
        ground_m, roof_m, top_m, _ = self.heights(floors)
        wall_m, storey_m, band = self.WALL_M, self.STOREY_M, self.BAND
        walls, bands = [], []
        for loop in loops:
            walls.append(ring(loop, roof_m, top_m, 0.0, wall_m, wall, inner=True))
            bands.append(ring(loop, ground_m - 0.3, ground_m + 0.02, 0.14, 0.06, band))
            if tier < 3:
                bands += [ring(loop, top_m, top_m + 0.1, 0.07, wall_m + 0.07, band, inner=True),
                          ring(loop, roof_m - 0.3, roof_m + 0.02, 0.1, 0.06, band)]
            if tier < 2:
                bands += [ring(loop, ground_m + storey_m * k - 0.14, ground_m + storey_m * k, 0.05, 0.06, band)
                          for k in range(1, floors - 1)]
        walls, bands = Soup.join(walls), Soup.join(bands)
        return Soup.join([walls.coloured(shade) if shade != 1.0 else walls, bands.coloured(trim) if trim != 1.0 else bands])

    def flat_walls(self, loops, floors, wall, shade=1.0, trim=1.0):
        """The walls of the two coarse tiers: one quad of plinth and one of wall to a run."""
        ground_m, roof_m, _, _ = self.heights(floors)
        return Soup.join([quad([(ax, ay, z0), (bx, by, z0), (bx, by, z1), (ax, ay, z1)], material, colour)
                          for loop in loops for (ax, ay), (bx, by) in loop
                          for z0, z1, material, colour in ((0.0, ground_m, self.STONE, trim), (ground_m, roof_m, wall, shade))])

    def folded(self, rows, modules, tiers_of, tier, skip=()):
        """What is left of the rows' modules at a coarse tier, as meshes in the template's frame."""
        return [unmasked(modules[name][tier], None if tint is None else tint[:3]).transformed(m) for name, m, tint in rows
                if tiers_of[name] & ~self.row_tiers(name) & (1 << tier) and not name.startswith(skip)]

    def monotone(self, name, tiers):
        for tier in range(1, 4):
            if len(self.clean(tiers[tier])) > len(self.clean(tiers[tier - 1])):
                raise SystemExit(f"{name}: tier {tier} is heavier than tier {tier - 1}: {[len(self.clean(t)) for t in tiers]}")
        return tiers

    def intact_shell(self, b, modules, tiers_of, meshes):
        """A template's own mesh at each tier: the walls of every run with every opening
        lined, the bands, parapet and coping round the outline, a roof over every part, the
        cubes, and at the two coarse tiers flat walls with what is left of the kit folded in."""
        _, roof_m, _, _ = self.heights(b.floors)
        walls = [[self.near_walls(run, tier) for run in b.runs] for tier in (0, 1)]
        linings = [lining(self.bounds_of(meshes, name), m, self.LINING_M, "X_Void") for run in b.runs for name, m in run.bare]
        linings += [step for run in b.runs for step in run.steps]
        cubes = Soup.join([run.cubes for run in b.runs])
        # from far off a glazed-in balcony is its dark glass: what stands behind it is not folded in
        glazed = [m for name, m, _ in b.rows if name == self.GLAZED]
        behind = lambda m: any(math.hypot(m[0, 3] - g[0, 3], m[1, 3] - g[1, 3]) < 1.0 and abs(m[2, 3] - g[2, 3]) < 0.1 for g in glazed)
        far_rows = [row for row in b.rows if not (row[0].startswith(self.ON_BALCONY) and behind(row[1]))]
        out = []
        for tier, g in enumerate(self.FEATURE_M):
            body = [*walls[tier], *linings] if tier < 2 else [self.flat_walls(b.loops, b.floors, self.WALL + MASK), *self.far_panes(b, meshes, tier)]
            out.append(Soup.join([*body, self.crown(b.loops, b.floors, tier, self.WALL + MASK),
                                  self.intact_roof(b, roof_m, tier),
                                  cubes if tier == 0 else detail.simplify(cubes, g), *self.folded(far_rows, modules, tiers_of, tier)]))
        return self.monotone(b.name, out)

    # ------------------------------------------------------------ gutted
    def smoked(self, wall):
        """A wall's paint after the fire: most of its colour gone to grey."""
        grey = 0.2126 * wall[0] + 0.7152 * wall[1] + 0.0722 * wall[2]
        return tuple(int(round(c + (grey - c) * self.BURNT_GREY)) for c in wall)

    def survivor(self, rng, name):
        for prefix, share, kinds in self.SURVIVES:
            if name.startswith(prefix):
                return name + "+" + rng.choice(kinds) if rng.random() < share else None
        return None

    def gutted(self, b, meshes):
        """The block burnt out and standing: its rows, and a function that makes its shell
        from the kit's modules. Every opening is an empty dark hole with soot above most, a
        few bays are blown out to the floor slabs, the walls and roof are scorched."""
        rng = random.Random(b.seed * 1000 + 7)
        ground_m, roof_m, _, _ = self.heights(b.floors)
        storey_m, bay_m = self.STOREY_M, self.BAY_M
        burnt, char, concrete = "X_Burnt" + MASK, "X_Void", "X_Rubble"
        rows, walls, cubes = [], ([], []), []
        holes, far_holes, soot, flat_soot, far_soot = [], [], [], [], []  # for walls with openings, for flat walls, and the few fans the farthest tier keeps
        for run in b.runs:
            point = lambda s, deep, z, run=run: (*(run.start + run.along * s - run.out * deep), z)
            s_of = lambda xy, run=run: float((np.asarray(xy)[:2] - run.start) @ run.along)
            bays = math.floor((run.length - 2.0) / bay_m + 1e-9)
            margin = (run.length - bay_m * bays) / 2
            blown = set()
            storeys = self.storeys(b.floors)
            for _ in range(int(bays * (storeys - 1) * self.BLOWN + rng.random())):
                k, f = rng.randrange(bays), rng.randrange(1, storeys)
                blown |= {(k + dk, f + df) for dk in range(rng.choice((1, 1, 2))) for df in range(rng.choice((1, 1, 2)))
                          if k + dk < bays and f + df < storeys}
            gone = lambda s, z: (math.floor((s - margin) / bay_m), 0 if z < ground_m else 1 + math.floor((z - ground_m) / storey_m)) in blown

            def scorch(wall, gone=gone, s_of=s_of):
                """The wall cut where its bays are blown out, smoked, and its own colour after the fire."""
                centre = wall.v[wall.t].mean(1)
                wall = wall.keep(np.array([not gone(s_of(c), float(c[2])) for c in centre], dtype=bool))
                trim = np.zeros(len(wall.v), bool)
                trim[np.unique(wall.t[np.array(wall.mats)[wall.m] != self.WALL + MASK])] = True
                clear = np.median(wall.c[~trim], axis=0) * self.BURNT_WALL
                return Soup(wall.v, wall.c * np.where(trim, self.BURNT_TRIM, self.BURNT_WALL)[:, None], wall.t, wall.m, wall.s,
                            [burnt if name == self.WALL + MASK else name for name in wall.mats]), clear

            near, near1 = self.near_walls(run, 0), self.near_walls(run, 1)
            wall, clear = scorch(near)  # the burnt wall's own colour, which a fan fades to
            walls[0].append(wall)
            walls[1].append(wall if near1 is near else scorch(near1)[0])
            for k, f in sorted(blown):
                s0, z0 = margin + bay_m * k, ground_m + storey_m * (f - 1)
                s1, z1, deep = s0 + bay_m, z0 + storey_m, 3.2
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
                lo, hi = self.bounds_of(meshes, name)
                wide = math.hypot(m[0, 0], m[1, 0])
                s = s_of(m[:2, 3])
                spans.append((s + lo[0] * wide, s + hi[0] * wide, m[2, 3] + lo[2] * m[2, 2], m[2, 3] + hi[2] * m[2, 2]))
            spans += [(margin + bay_m * k, margin + bay_m * (k + 1), ground_m + storey_m * (f - 1), ground_m + storey_m * f) for k, f in blown]
            for name, m in run.openings:
                if gone(s_of(m[:2, 3]), m[2, 3] + 0.3):
                    continue
                holes.append(lining(self.bounds_of(meshes, name), m, self.LINING_M, char))
                if not name.startswith(self.ON_BALCONY):  # from far off a balcony covers the door onto it
                    far_holes.append(lining(self.bounds_of(meshes, name), m, -0.03, char, grow=0.0))
                if rng.random() < 0.85:
                    lo, hi = self.bounds_of(meshes, name)
                    wide = math.hypot(m[0, 0], m[1, 0])
                    s, half = s_of(m[:2, 3]) + (lo[0] + hi[0]) / 2 * wide, (hi[0] - lo[0]) / 2 * wide
                    z = m[2, 3] + hi[2] * m[2, 2] + 0.14
                    tall, spread = rng.uniform(1.4, 2.8), rng.uniform(0.25, 0.6)
                    above = [z0 for s0, s1, z0, _ in spans if z0 > z - 0.3 and s0 < s + half + spread and s1 > s - half - spread]
                    reach = min(roof_m - z, min(above, default=math.inf) - z - 0.08)
                    if reach < 0.25:
                        continue
                    dark = rng.uniform(0.03, 0.1) * self.BURNT_WALL
                    fan = ((point(s - half, -0.04, z), point(s + half, -0.04, z)), min(tall, reach), spread, burnt, dark)
                    # stopped under the opening above, it is still half as black there: the soot goes on up past that
                    # opening, and a column of windows is one black streak, not a shadow under each sill
                    held = lambda wall: dark + (wall - dark) * 0.45 if reach < tall else None
                    soot.append(damage.soot_fan(*fan, clear, top=held(clear)))
                    flat_soot.append(damage.soot_fan(*fan, self.BURNT_WALL, sides=self.FLAT_SOOT_SIDES, top=held(self.BURNT_WALL)))
                    if rng.random() < self.FAR_SOOT_SHARE:
                        far_soot.append(damage.soot_fan(*fan, self.BURNT_WALL, sides=False, top=held(self.BURNT_WALL)))
            if len(run.cubes):
                sooted = damage.charred(run.cubes, frozenset(), b.seed + 3, light=(0.03, 0.1))
                grey = sooted.c.mean(1, keepdims=True)  # a sign board's paint is gone with the rest
                sooted = Soup(sooted.v, grey + (sooted.c - grey) * 0.2, sooted.t, sooted.m, sooted.s, sooted.mats)
                centre = sooted.v[sooted.t].mean(1)
                cubes.append(sooted.keep(np.array([not gone(s_of(c), float(c[2])) for c in centre], dtype=bool)))
            for name, m, _ in run.rows:
                left = None if gone(s_of(m[:2, 3]), m[2, 3] + 0.3) else self.survivor(rng, name)
                if left:
                    rows.append((left, m, None))
        for name, m, _ in b.roof_rows:
            left = self.survivor(rng, name)
            # what stood where the roof is burnt through has gone down with it
            if left and self.fire_at(np.array([m[0, 3]]), np.array([m[1, 3]]), b.seed)[0] < self.BURNT_THROUGH[0] - 0.04:
                rows.append((left, m, None))

        def shell(modules, tiers_of):
            out, sooted = [], Soup.join(cubes)
            for tier, g in enumerate(self.FEATURE_M):
                body = [*walls[tier], *holes, *soot] if tier < 2 else \
                    [self.flat_walls(b.loops, b.floors, burnt, self.BURNT_WALL, self.BURNT_TRIM), *far_holes,
                     *(flat_soot if tier == 2 and self.FLAT_SOOT else far_soot)]
                out.append(Soup.join([*body, self.crown(b.loops, b.floors, tier, burnt, self.BURNT_WALL, self.BURNT_TRIM),
                                      self.roof(b.rects, roof_m + 0.04, self.BURNT_ROOF_CELL_M[tier], b.seed, burnt=0.85, slab=tier < 3),
                                      self.burnt_storey(b.rects, b.loops, roof_m, burnt),
                                      sooted if tier == 0 else detail.simplify(sooted, g),
                                      # at the coarse tiers an opening is its dark lining, whatever hangs in it
                                      *self.folded(rows, modules, tiers_of, tier, skip=self.OPENINGS)]))
            return self.monotone(b.name + " gutted", out)

        return rows, shell

    # ------------------------------------------------------------ ruin
    def ruin(self, b, meshes, source):
        """The block collapsed, inside the simulation's remains box (the parts' plan, at the
        ruin height): its rows, and a function that makes its shell. The walls stand as ragged
        stumps, the rubble of the same wall, roof and concrete is heaped over the plan and
        spills a little, each part to a height of its own, with the floors lying in it."""
        rng = random.Random(b.seed * 1000 + 13)
        ground_m, _, _, ruin_m = self.heights(b.floors)
        wall_m, step_m = self.WALL_M, self.STUMP_STEP_M
        burnt, concrete, tile = "X_Burnt" + MASK, "X_Rubble", self.ROOF
        high = [ruin_m * rng.uniform(0.5, 0.85) for _ in b.rects]
        height_at = lambda x, y: np.minimum(damage.heap_height(x, y, b.rects, high, self.SPILL_M, b.seed), ruin_m)
        stumps, cubes, coarse = [], [], {2: [], 3: []}
        for run in b.runs:
            tops = damage.profile(rng, run.length, step_m, 0.5, ruin_m)
            standing = self.standing(run)
            standing = Soup(standing.v, standing.c, standing.t, standing.m, standing.s,
                            [burnt if name == self.WALL + MASK else name for name in standing.mats])
            stumps += [damage.break_off(standing, run.start, run.along, step_m, tops),
                       damage.broken_edge(run.start, run.along, -run.out, wall_m, run.length, step_m, tops, concrete, 0.6)]
            if len(run.cubes):  # half the sign boards and surrounds are down in the heap; what hangs on is scorched
                piece = detail.islands(run.cubes)
                hangs = np.array([rng.random() < 0.5 for _ in range(piece.max() + 1)], dtype=bool)[piece]
                cubes.append(damage.break_off(damage.charred(run.cubes.keep(hangs), frozenset(), b.seed + 3, light=(0.12, 0.34)),
                                              run.start, run.along, step_m, tops))
            for tier, merge in ((2, 2), (3, 4)):  # the same stumps as flat faces, two and four columns to a face
                wide = step_m * merge
                flat_tops = [sum(tops[k:k + merge]) / len(tops[k:k + merge]) for k in range(0, len(tops), merge)]
                for k, top in enumerate(flat_tops):
                    s0, s1 = k * wide, min((k + 1) * wide, run.length)
                    at = lambda s, z, deep=0.0: (*(run.start + run.along * s - run.out * deep), z)
                    bands = [(0.0, min(top, ground_m), self.STONE, 0.72)] + ([(ground_m, top, burnt, 0.72)] if top > ground_m else [])
                    coarse[tier] += [quad([at(s0, z0), at(s1, z0), at(s1, z1), at(s0, z1)], material, shade) for z0, z1, material, shade in bands]
                    coarse[tier].append(quad([at(s1, 0.0, wall_m), at(s0, 0.0, wall_m), at(s0, top, wall_m), at(s1, top, wall_m)], burnt, 0.6))
                coarse[tier].append(damage.broken_edge(run.start, run.along, -run.out, wall_m, run.length, wide, flat_tops, concrete, 0.6, sides=tier == 2))

        names = [concrete, burnt, tile]
        # broken concrete, with plaster dust and smashed roof in it here and there
        mix = lambda x, y: np.digitize(damage.noise(x, y, 3.2, b.seed + 11), (0.68, 0.82))
        shade = lambda x, y, z: np.repeat((0.36 + 0.5 * damage.noise(x, y, 1.3, b.seed + 12) * (0.5 + 0.5 * damage.noise(x, y, 6.0, b.seed + 14)))[:, None], 3, 1)
        heap = lambda tier: damage.under(damage.heap(b.rects, high, self.SPILL_M, self.HEAP_CELL_M[tier], b.seed, (names, mix), shade), ruin_m)
        area = sum(4 * hx * hy for _, _, hx, hy in b.rects)
        fallen = damage.slabs(rng, b.rects, high, height_at, ruin_m, concrete, tile, (0.5, 0.8))
        chunks = [damage.scatter(random.Random(b.seed * 1000 + 17 + tier), b.rects, round(area / per), size, height_at, ruin_m,
                                 (concrete, burnt, concrete, concrete, tile), (0.4, 0.85))
                  for tier, (per, size) in enumerate(((3.0, 0.6), (9.0, 0.9), (30.0, 1.3)))]

        rows = []
        wrecks = [name for name in self.WRECKS if name in meshes]
        for _ in range(round(area / self.WRECK_M2) if wrecks else 0):
            name = wrecks[rng.randrange(len(wrecks))] + "+wreck"
            cx, cy, hx, hy = b.rects[rng.randrange(len(b.rects))]
            x, y = cx + rng.uniform(-hx + 0.8, hx - 0.8), cy + rng.uniform(-hy + 0.8, hy - 0.8)
            z = max(0.0, min(float(height_at(np.array([x]), np.array([y]))[0]) - 0.12, ruin_m - source(name).bounds()[1][2]))
            rows.append((name, frame(rng.uniform(0.0, math.tau), x, y) @ damage.tilted(0, 0, 0, 0, 0, z), None))

        def shell(modules, tiers_of):
            charred = Soup.join(cubes)
            out = [Soup.join([*stumps, charred, heap(0), chunks[0], fallen]),
                   Soup.join([*stumps, detail.simplify(charred, self.FEATURE_M[1]), heap(1), chunks[1], fallen]),
                   # what is thrown down is folded in at tier 2 and gone at tier 3
                   Soup.join([*coarse[2], detail.simplify(charred, self.FEATURE_M[2]), heap(2), chunks[2], fallen,
                              *self.folded([r for r in rows if modules[r[0]][2] is not None], modules, {name: 0b0100 for name in tiers_of}, 2)]),
                   Soup.join([*coarse[3], heap(3), fallen])]
            return self.monotone(b.name + " ruin", out)

        return rows, shell

    # ------------------------------------------------------------ the descriptor
    def floor_heights(self, floors):
        return [0.0] + [round(self.GROUND_M + self.STOREY_M * k, 3) for k in range(floors - 1)]

    def descriptor(self, name, floors, parts_, openings, doors):
        """The physical template. Every face of every part is cut into spans: joined where
        another part stands against it (no bays), exposed elsewhere, with the bay lattice
        checked against where the graph put its windows. `openings` and `doors` are
        template-space positions."""
        bay_m = self.BAY_M
        half_z = self.heights(floors)[2] / 2
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
            phase = round(min(found) % bay_m, 3) if found else bay_m / 2
            lattice = {round(phase + bay_m * k, 3) for k in range(-200, 200) if a < phase + bay_m * k < b}
            if found != lattice:
                raise SystemExit(f"{name} {edge['id']}: windows at {sorted(found)}, the bay lattice at {sorted(lattice)}")
            edge["bays"] = {"pitch_m": bay_m, "phase_m": phase}
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
            "id": f"{self.ID_PREFIX}{name}", "category": "urban_apartment", "regional_family": self.FAMILY,
            "parts": [{"id": part, "center": [float(cx), float(cy)], "yaw": 0.0, "half_extents": [float(hx), float(hy), half_z],
                       "base_z": 0.0} for part, (cx, cy, hx, hy) in parts_.items()],
            "floor_heights_m": self.floor_heights(floors),
            "entrances": entrances, "edges": edges, "joins": joins,
        }

    # ------------------------------------------------------------ the fighting bays
    def opening_spans(self, desc, b, meshes):
        """Every visible opening of the intact building on the descriptor's edges, as
        (edge, left, right, foot, head): the bounds of what fills it, on its wall."""
        out = []
        for run in b.runs:
            for name, m in run.openings:
                lo, hi = self.bounds_of(meshes, name)
                corners = np.array([(lo[0], 0.0, lo[2]), (hi[0], 0.0, hi[2])]) @ m[:3, :3].T + m[:3, 3]
                for edge in desc["edges"]:
                    part = next(p for p in desc["parts"] if p["id"] == edge["part"])
                    face = next(f for f in FACES if f[1] == edge["facade"])
                    (cx, cy), (hx, hy, _) = part["center"], part["half_extents"]
                    n, along = face[2], face[3]
                    reach = hx if n[0] else hy
                    off = (corners[:, 0] - cx) * along[0] + (corners[:, 1] - cy) * along[1]
                    plane = (corners[:, 0] - cx) * n[0] + (corners[:, 1] - cy) * n[1] - reach
                    a, b_ = edge["span_m"]
                    if edge["exposed"] and np.all(np.abs(plane) < 0.6) and a - 1e-6 < off.mean() < b_ + 1e-6:
                        out.append((edge["id"], float(off.min()), float(off.max()), float(corners[:, 2].min()), float(corners[:, 2].max())))
                        break
        return out

    def check_bays(self, desc, b, meshes):
        """Every declared fighting bay has a visible opening over the eye and muzzle heights
        (README, "Rules a set keeps"), and no two openings on a wall overlap."""
        with open(os.path.join(ROOT, "fixtures", "game.json")) as f:
            physics = json.load(f)["physics"]
        spans = self.opening_spans(desc, b, meshes)
        gaps = missing_openings(desc, spans, (physics["infantry_muzzle_m"], physics["infantry_eye_m"]))
        overlaps = overlapping_openings(spans)
        found = ([f"{desc['id']}: declared facade bays without visible openings: {[(e, float(o), f) for e, o, f in gaps]}"] if gaps else []) + \
            ([f"{desc['id']}: overlapping visible facade openings: {overlaps}"] if overlaps else [])
        for finding in found:
            if self.BAYS_REFUSED:
                raise SystemExit(finding)
            print("BAYS", finding)

    # ------------------------------------------------------------ the GLB
    def box_uv(self, soup):
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
        recipe = [self.recipe_of(name) for name in soup.mats]
        tile = np.array([textures.tile_of(r) if r else 1.0 for r in recipe])[soup.m][:, None]
        turned = np.array([base_of(name) in self.TURNED for name in soup.mats])[soup.m][:, None]
        uv = np.stack([np.where(turned, v, u) / tile, np.where(turned, u, v) / tile], -1)
        for i, name in enumerate(soup.mats):
            own = soup.m == i
            if not own.any():
                continue
            if base_of(name) in self.FITTED:  # one picture over the surface's own bounds, its top at the top
                q = p[own]
                lo, hi = q.reshape(-1, 3).min(0), q.reshape(-1, 3).max(0)
                uv[own] = np.stack([(q[..., 0] - lo[0]) / max(hi[0] - lo[0], 1e-9), (q[..., 2] - lo[2]) / max(hi[2] - lo[2], 1e-9)], -1)
            elif name in self.ROOM_SHEETS:  # the unit box unfolded round its back wall (`room_mesh`)
                q, n = p[own], normals[own]
                x, y, z = q[..., 0] + 0.5, q[..., 1], q[..., 2]
                face = np.abs(n).argmax(1)[:, None]
                low = (n[np.arange(len(n)), np.abs(n).argmax(1)] > 0)[:, None]  # the floor's and the left wall's normals point up and right
                uv[own] = np.stack([np.where(face == 0, np.where(low, y - 1.0, 2.0 - y), x),
                                    np.where(face == 2, np.where(low, y - 1.0, 2.0 - y), z)], -1)
        return uv

    def clean(self, soup):
        """Positions snapped to 0.1 mm, the triangles that snap flat removed, and only the
        materials still in use, in name order."""
        out = Soup(np.round(soup.v, 4), soup.c, soup.t, soup.m, soup.s, soup.mats)
        _, area = out.normals()
        out = out.keep(area > 1e-9)
        used = sorted({out.mats[i] for i in np.unique(out.m)}, key=self.material_name)
        slot = np.array([used.index(name) if name in used else -1 for name in out.mats])
        return Soup(out.v, out.c, out.t, slot[out.m], out.s, used)

    def material(self, source):
        name = self.material_name(source)
        if name not in self.materials:
            _, _, base, rough, metal = self.SURFACES[base_of(source)]
            recipe = self.recipe_of(source)
            if source in self.ROOM_SHEETS:
                self.materials[name] = parts.room(name, self.ROOM_SHEETS[source])
                return self.materials[name]
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
            coverage = ("cutout", 0.5) if CUT in source else self.COVERAGE.get(base_of(source))
            if coverage:
                textures.surface(m, coverage)
            self.materials[name] = m
        return self.materials[name]

    def mesh_object(self, name, soup, parent):
        soup = self.clean(soup)
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
            me.materials.append(self.material(source))
        me.uv_layers.new(name="UVMap").data.foreach_set("uv", self.box_uv(soup).astype(np.float32).ravel())
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

    def write_kit(self, path, kit):
        """`kit` is {module id: [four soups]}."""
        parts.reset()
        self.recipes()
        for name in sorted(kit):
            root = parts.empty(name)
            for tier, soup in enumerate(kit[name]):
                self.mesh_object(f"{name}_LOD{tier}", soup, root)
        parts.export(path, worn=False)
        steady_tangents(path)

    # ------------------------------------------------------------ rooms
    def fit_rooms(self, run, rects):
        """A run with its rooms no deeper than the plan has room for: short of the middle of
        the building behind the wall, and, near an end of the run, short of the room that
        stands behind the wall round the corner."""
        wall_m = self.WALL_M
        run.steps = []
        for k, (name, m, tint) in enumerate(run.rows):
            if name not in self.ROOM_MESHES:
                continue
            wide, deep = float(np.hypot(m[0, 0], m[1, 0])), float(np.hypot(m[0, 1], m[1, 1]))
            s = float((m[:2, 3] - run.start) @ run.along)
            if m[2, 3] < self.ROOM_STEP_M + 0.01:  # the threshold a ground-floor room stands on
                at = lambda along_m, z: (*(run.start + run.along * along_m - run.out * (wall_m - 0.01)), z)
                run.steps.append(quad([at(s - wide / 2, 0.0), at(s + wide / 2, 0.0), at(s + wide / 2, self.ROOM_STEP_M + 0.01),
                                       at(s - wide / 2, self.ROOM_STEP_M + 0.01)], "X_Void"))
            steps = np.arange(0.25, 12.0, 0.25)
            behind = run.start[None] + run.along[None] * s - run.out[None] * steps[:, None]
            outside = damage.distance_out(behind[:, 0], behind[:, 1], rects) > 0
            through = float(steps[np.argmax(outside)]) if outside.any() else 12.0
            corner = min(s - wide / 2, run.length - s - wide / 2)
            fits = max(0.6, min(deep, through / 2 - wall_m - self.ROOM_CLEAR_M, max(corner - wall_m - self.ROOM_CLEAR_M, 0.6) if corner < deep + wall_m else deep))
            if fits < deep:
                m = m.copy()
                m[:3, 1] *= fits / deep
                run.rows[k] = (name, m, tint)
        return run

    # ------------------------------------------------------------ main
    def kit_meshes(self, graph_):
        """Every kit mesh the taps instanced, as the exporter draws it."""
        meshes = {}
        for tap, _ in graph_.taps.values():
            meshes.update(tap.meshes)
        meshes = {name: self.sheeted(soup) if name.startswith(self.SHEETED) else soup for name, soup in meshes.items()}
        meshes.update({name: room_mesh(material) for name, material in self.ROOM_MESHES.items()})
        if self.DECAL:
            # a decal is a face on the wall's own plane: stood off it, the two do not fight for depth
            off = np.eye(4)
            off[1, 3] = -0.015
            meshes = {name: soup.transformed(off) if name.startswith(self.DECAL) else soup for name, soup in meshes.items()}
        return meshes

    def main(self):
        args = script_args()
        dry = "--dry" in args
        out_dir = next((a for a in args if not a.startswith("--")), None) or os.path.join(ROOT, "assets/source/city", self.SET)
        graph_ = Graph(self)
        version = ".".join(str(v) for v in bpy.app.version)
        built = [self.assemble(graph_, template) for template in self.TEMPLATES]
        meshes = self.kit_meshes(graph_)

        # whether the graph tints a mesh's instances, and every mesh a state places: a kit mesh, or a variant of one
        tinted, cache = {}, {}
        for b in built:
            for name, _, tint in b.rows:
                if tinted.setdefault(name, tint is not None) != (tint is not None):
                    raise SystemExit(f"{name} is instanced both with and without a tint")

        def source(name):
            if name not in cache:
                base, _, kind = name.partition("+")
                cache[name] = self.variant(self.masked(meshes[base], False), kind, sum(name.encode())) if kind else self.masked(meshes[base], tinted[base])
            return cache[name]

        # the states: a block of six floors or fewer collapses, a taller one burns out and stands
        states = []
        for b in built:
            state = {"intact": (b.rows, lambda modules, tiers_of, b=b: self.intact_shell(b, modules, tiers_of, meshes))}
            if collapse.damage_state(b.floors) == "ruin":
                state["ruin"] = self.ruin(b, meshes, source)
            else:
                state["gutted"] = self.gutted(b, meshes)
            states.append(state)

        used = sorted({name for state in states for rows, _ in state.values() for name, _, _ in rows})
        for name in used:
            if not self.family_tiers(name):
                raise SystemExit(f"{name} is in no family of FAMILIES")
        modules = {name: self.module_tiers(name, source(name)) for name in used}
        # a module is drawn at the tiers its family names, where it has anything left to draw
        tiers_of = {name: sum(1 << t for t in range(4) if self.family_tiers(name) >> t & 1 and modules[name][t] is not None)
                    for name in modules}
        # every module has four meshes: a tier it is not drawn at repeats the last one it is.
        # A fitting the fire leaves nothing of (a rack of cloth) is no module, and its rows are not drawn.
        kit = {}
        for name, lods in modules.items():
            if tiers_of[name] & 1:
                kit[self.module_id(name)] = [next(l for l in reversed(lods[:t + 1]) if l is not None) for t in range(4)]
            else:
                tiers_of[name] = 0

        templates = []
        for b, state in zip(built, states):
            placed = {}
            for which, (rows, shell) in state.items():
                module = b.name.replace("-", "_") + ("_shell" if which == "intact" else "_" + which)
                kit[module] = shell(modules, tiers_of)
                placed[which] = [(module, [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0], 0b1111, self.smoked(b.wall) if which == "gutted" else b.wall)]
                for n, m, tint in rows:
                    row = decompose(m)
                    if row is None or min(row[4:]) <= 0:
                        raise SystemExit(f"{n}: a row that tilts or mirrors needs a module variant")
                    if tiers_of[n] & self.row_tiers(n):
                        placed[which].append((self.module_id(n), row, tiers_of[n] & self.row_tiers(n), WHITE if tint is None else srgb_bytes(tint[:3])))
            openings = [(m[0, 3], m[1, 3]) for run in b.runs for _, m in run.openings]
            desc = self.descriptor(b.name, b.floors, b.parts, openings, self.doors(b))
            self.check_bays(desc, b, meshes)
            templates.append((desc, b, placed))

        kit = {name: [self.clean(s) for s in lods] for name, lods in kit.items()}
        names = sorted(kit)
        index = {name: i for i, name in enumerate(names)}
        triangles = {name: [len(s) for s in kit[name]] for name in names}
        doc_templates, table, over = [], [], []
        worst = np.zeros(2)
        for desc, b, placed in templates:
            boxes = np.array([[*p["center"], *p["half_extents"]] for p in desc["parts"]])
            _, _, top_m, ruin_m = self.heights(b.floors)
            packed, drawn_at = {}, {}
            for which, rows in placed.items():
                rows = packed[which] = sorted([index[name], *(round(float(v), 5) for v in row), tiers, *tint] for name, row, tiers, tint in rows)
                drawn_at[which] = [sum(triangles[names[r[0]]][t] for r in rows if r[8] >> t & 1) for t in range(4)]
                table.append((desc["id"], which, len(rows), drawn_at[which]))
                over += [f"{desc['id']} {which} draws {n} triangles at tier {t}, over {self.BUDGET[t]}"
                         for t, n in enumerate(drawn_at[which]) if n > self.BUDGET[t]]
                over += [f"{desc['id']} {which} draws {n} triangles at tier {t}, more than intact's {drawn_at['intact'][t]}"
                         for t, n in enumerate(drawn_at[which]) if n > drawn_at["intact"][t]]
                # the fit: every vertex inside some part grown by FIT's side, none below the ground, and none above what
                # the simulation leaves standing: the parts and FIT's top, or for a ruin its remains box
                ceiling = ruin_m if which == "ruin" else top_m + self.FIT["top_m"]
                for r in rows:
                    for t in range(4):
                        if r[8] >> t & 1:
                            v = kit[names[r[0]]][t].transformed(row_matrix(r[1:8])).v
                            side = np.maximum(np.abs(v[:, None, 0] - boxes[:, 0]) - boxes[:, 2],
                                              np.abs(v[:, None, 1] - boxes[:, 1]) - boxes[:, 3]).min(1).max()
                            if side > self.FIT["side_m"] or v[:, 2].max() > ceiling + 1e-3 or v[:, 2].min() < -1e-3:
                                raise SystemExit(f"{desc['id']} {which}: {names[r[0]]} at tier {t} reaches {side:.2f} m past the sides, "
                                                 f"up to {v[:, 2].max():.2f} m (the limit is {ceiling:.2f}) and {-v[:, 2].min():.3f} m below the ground")
                            if which != "ruin":
                                worst = np.maximum(worst, (side, v[:, 2].max() - top_m))
            doc_templates.append({"status": "release", "recipe": b.recipe, "descriptor": desc, "states": packed})

        doc = {"set": self.SET, "kit": f"city_kit_{self.SET}", "fit": self.FIT,
               "source": {"script": self.SCRIPT, "blend": self.BLEND, "blender": version},
               "modules": names, "templates": doc_templates}
        if not dry:
            os.makedirs(out_dir, exist_ok=True)
            with open(os.path.join(out_dir, "templates.json"), "w") as f:
                f.write(json.dumps(doc, separators=(",", ":")) + "\n")
            self.write_kit(os.path.join(out_dir, "kit.glb"), kit)

        print(f"KIT {len(names)} modules, triangles per tier {[sum(t[i] for t in triangles.values()) for i in range(4)]}")
        for name in names:
            print(f"MODULE {name:38s} {triangles[name]}")
        print(f"REACH past the sides {worst[0]:.2f} m, past the top {worst[1]:.2f} m (fit {self.FIT})")
        print(f"{'template':34s} {'state':7s} {'rows':>5s} " + " ".join(f"{'tier ' + str(t):>8s}" for t in range(4)))
        for name, which, count, drawn_at in table:
            print(f"{name:34s} {which:7s} {count:5d} " + " ".join(f"{n:8d}" for n in drawn_at))
        print(f"{'budget':34s} {'':7s} {'':5s} " + " ".join(f"{n:8d}" for n in self.BUDGET))
        if over:
            raise SystemExit("\n".join(over))
