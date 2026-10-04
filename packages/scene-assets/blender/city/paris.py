"""The Paris apartment blocks: Haussmann blocks from the vendored French graph.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/city/paris.py [out_dir] [--dry]

writes `assets/source/city/apartments_paris/kit.glb` and `templates.json` (the
format is in this folder's readme), or the two files into `out_dir`; with `--dry`
it only prints. It needs the ambientCG sets in the pack cache.

The source is the vendored `FrenchBuilding.blend`: a bay instancer that lays
3 m modules (a ground-floor shop window or door, an upper window, a window with
a balcony, a dormer, a cornice piece) round a block `Bays X` by `Bays Y` bays
with 1 m corner pieces, `Floors` storeys over a 4.2 m ground floor, under a zinc
mansard. Its modules carry their own stretch of ashlar wall, so here, unlike the
China graph, the walls are rows: the shell is the mansard's zinc top and, at the
two coarse tiers, flat walls with the modules' windows and balconies folded on.
The graph's separate room and curtain objects (`FR_Rooms`, `FR_Curtains`) read
an interior photo atlas we do not ship; our room boxes stand behind every window
instead (`graphset.py`, "Interiors").

What every graph set shares is `graphset.py`; this is the French graph's part.
A template is one block: the graph builds a rectangle with all four sides
dressed, so a template here is a single part, and its floors count the mansard
storey (its dormers are windows, the attic is lived in). A block's run is
3n + 2 m, every bay on the 3 m lattice.
"""
import os
import sys
from types import SimpleNamespace

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ambientcg  # noqa: E402
import collapse  # noqa: E402
import damage  # noqa: E402
import detail  # noqa: E402
import graph  # noqa: E402
import textures  # noqa: E402
from graph import Soup  # noqa: E402
from filters import grime, scorched, toned  # noqa: E402
from graphset import MASK, GraphSet, base_of, outline, right_of  # noqa: E402

STONE, TRIM, ZINC, IRON, FRAME, GLASS, DOOR = "FR_Stone", "FR_StoneTrim", "FR_Zinc", "FR_Iron", "FR_WindowFrame", "FR_Glass", "FR_DoorWood"
POT = "FR_Pot"  # ours: a chimney pot's clay
WALLS = frozenset((STONE, TRIM))  # what a module's own stretch of wall is made of
FILLS = frozenset((FRAME, GLASS, DOOR))  # what fills its opening
# Modules with a window or a door in them, and which of those have a room behind them.
WINDOWED = ("G0_", "G1_", "U0_", "U1_", "U2_", "R0_")
ROOMED = {"G0_": "FR_Room_Shop", "G1_": "FR_Room_Shop", "U0_": "FR_Room_Home", "U1_": "FR_Room_Home", "U2_": "FR_Room_Home"}
# Modules whose trim is the read of the block from far off: the cornice, a dormer's surround.
TRIM_KEPT = ("T0_", "T1_", "R0_", "R1_", "T2_")
ROOM_DEPTH_M = 4.0
# The mansard's slope, as its dormer modules lay it: from the cornice's top, out past the wall, up and in.
MANSARD_FOOT_M, MANSARD_SLOPE_M, MANSARD_OUT_M, MANSARD_IN_M = 0.55, 3.0, 0.24, 1.0
CHIMNEY_M = 1.1  # a stack's height over its foot in the slope's top, pots on it


def uprights():
    """A railing's uprights 12.5 cm apart, and nothing across them: its rails are geometry."""
    size = textures.SIZE
    xx = np.mgrid[0:size, 0:size][1].astype(float) / size
    px, bar_r = 1.0 / size, 0.012 / 0.5
    bx = np.abs((xx * 4) % 1.0 - 0.5) / 4
    bar = textures.smoothstep(bar_r + px / 2, bar_r - px / 2, bx)
    height = np.sqrt(np.clip(1.0 - (bx / bar_r) ** 2, 0, 1)) * 3.0
    ones = np.ones((size, size))
    return textures.Baked(np.ones((size, size, 3)), 1.0, textures.normals_from_height(textures.blur(height), 1.0), 1.0, ones, ones,
                          coverage=bar)


def ashlar(albedo, roughness):
    """Cut limestone: courses 45 cm high, blocks 90 cm long and staggered a half each
    course, the joints thin and a shade darker; each block a hair off its neighbours."""
    size = albedo.shape[0]
    yy, xx = np.mgrid[0:size, 0:size].astype(float) / size  # the tile is 1.8 m
    course = np.floor(yy * 4)
    u = (xx * 2 + 0.5 * (course % 2)) % 1.0
    v = (yy * 4) % 1.0
    px = 1.0 / size
    joint = np.maximum(textures.smoothstep(1.6 * px * 4, 0.4 * px * 4, np.minimum(v, 1 - v)),
                       textures.smoothstep(1.6 * px * 2, 0.4 * px * 2, np.minimum(u, 1 - u)))
    block = np.floor(xx * 2 + 0.5 * (course % 2)) + 7 * course
    tone = 0.96 + 0.08 * ((block * 2654435761) % 97 / 96.0)
    out = albedo * tone[..., None] * (1.0 - 0.35 * joint[..., None])
    return out, np.clip(roughness + 0.1 * joint, 0.0, 1.0)


def stone(mean):
    """Limestone of a mean colour, with the ashlar's courses cut in."""
    tone = toned(mean, 0.35, level_cycles=6)

    def burn(albedo, roughness):
        return ashlar(*tone(albedo, roughness))

    return burn


def seams(albedo, roughness):
    """Zinc in standing seams half a metre apart, its sheets weathered a little apart."""
    albedo, roughness = toned((0.16, 0.175, 0.19), 0.15)(albedo, roughness)
    size = albedo.shape[0]
    xx = np.mgrid[0:size, 0:size][1].astype(float) / size  # the tile is 2 m: four sheets
    u = (xx * 4) % 1.0
    seam = textures.smoothstep(0.03, 0.0, np.minimum(u, 1 - u))
    sheet = np.floor(xx * 4)
    out = albedo * (0.94 + 0.12 * (sheet % 3) / 2)[..., None] * (1.0 + 0.35 * seam[..., None])
    return np.clip(out, 0.0, 1.0), roughness


class Paris(GraphSet):
    SET = "apartments_paris"
    BLEND = "vendor/procedural-buildings/FrenchBuilding.blend"
    SCRIPT = "paris.py"
    ID_PREFIX, FAMILY = "paris-apartment-", "paris"
    # The graph's inputs, every one set. Balconies stand out from the wall ("Exterior"): the
    # continuous iron balconies of the second and fifth floors are the block's signature.
    INPUTS = {"Dormers": True, "Balcony Type": "Exterior", "Detail Depth": 1.0}
    GROUND_M, STOREY_M, WALL_M = 4.2, 3.2, 0.4
    MANSARD_M = 4.45  # the cornice to the top of the mansard's zinc
    PARAPET_M = MANSARD_M
    # A template: its id's tail, its floors (the mansard's among them), its bays along x and y,
    # the graph's detail seed, how the wall details repeat up the block and which they are.
    # The ornamented panels are a 1,900-triangle relief a bay: none of these blocks has them. A bay of a
    # Haussmann front, its carved surrounds and iron, is three times a China bay's triangles, so the tall
    # blocks are shorter than China's to stay in the template budget.
    TEMPLATES = (
        ("slab-35x11-4f", 4, 11, 3, 1, "Same on All Floors", "Refends (Banded Ashlar)"),
        ("slab-47x11-5f", 5, 15, 3, 2, "Alternate per Floor", "Pilasters"),
        ("slab-47x14-6f", 6, 15, 4, 3, "Off", "Pilasters"),
        ("slab-35x14-8f", 8, 11, 4, 4, "Off", "Pilasters"),
        ("point-20x20-7f", 7, 6, 6, 5, "Alternate per Floor", "Refends (Banded Ashlar)"),
    )
    FIT = {"side_m": 1.5, "top_m": 0.5}

    KIT_GROUPS = (
        ("G0_", 0b1111), ("G1_", 0b1111), ("U0_", 0b1111), ("U1_", 0b1111), ("U2_", 0b1111), ("C0_", 0b1111),
        ("C1_", 0b1111), ("C2_", 0b1111), ("R0_", 0b1111), ("R1_", 0b1111), ("T0_", 0b1111), ("T1_", 0b1111),
        ("T2_", 0b1111), ("D0_", 0b0001), ("D1_", 0b0001), ("D2_", 0b0001), ("FR_Room_", 0b0011), ("FR_Railing", 0b1111),
    )
    ROOM_MESHES = {"FR_Room_Home": "X_Room", "FR_Room_Shop": "X_ShopRoom"}
    ROOM_SHEETS = {"X_Room": "rooms", "X_ShopRoom": "shops"}
    SHEETED = ("U1_", "U2_", "C2_", "G0_", "G1_")
    # a railing's rails stay its own bars: the recipe's rails crossed its uprights at heights of their own
    SHEET_RAILS = False
    SHEET_BAR_M = 0.03
    ENTRANCE = "G1_"

    WALL, STONE, BAND, ROOF, GLASS = STONE, STONE, TRIM, ZINC, GLASS
    SURFACES = {
        STONE: ("fr_stone", "fr_ashlar", (1.0, 1.0, 1.0), 1.0, 0.0),
        TRIM: ("fr_trim", "fr_trim", (1.0, 1.0, 1.0), 1.0, 0.0),
        # weathered zinc is dull: as a metal it mirrored the sky, and a slope facing the sun read as white
        ZINC: ("fr_zinc", "fr_zinc", (1.0, 1.0, 1.0), 0.75, 0.15),
        IRON: ("fr_iron", "fr_metal", (0.035, 0.037, 0.04), 0.7, 0.5),
        FRAME: ("fr_frame", "fr_paint", (0.62, 0.6, 0.55), 0.6, 0.0),
        DOOR: ("fr_door", "fr_wood", (0.14, 0.28, 0.2), 0.6, 0.0),
        POT: ("fr_pot", None, (0.36, 0.13, 0.07), 0.9, 0.0),
        GLASS: ("fr_glass", None, (0.05, 0.075, 0.09), 0.08, 0.0),
        "X_Pane": ("fr_pane", None, (0.06, 0.066, 0.068), 0.25, 0.0),
        "X_Void": ("fr_void", None, (0.005, 0.005, 0.006), 1.0, 0.0),
        "X_Burnt": ("fr_stone_burnt", "fr_burnt", (1.0, 1.0, 1.0), 1.0, 0.0),
        "X_Rubble": ("fr_rubble", "fr_concrete", (0.6, 0.57, 0.52), 1.0, 0.0),
        "X_Soot": ("fr_soot", "fr_concrete", (0.5, 0.48, 0.46), 1.0, 0.0),
        "X_Room": ("fr_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
        "X_ShopRoom": ("fr_shop_room", None, (0.03, 0.03, 0.03), 1.0, 0.0),
    }
    COVERAGE = {GLASS: ("blended", 0.35)}
    BARS = "fr_bars"
    BURNS = frozenset((GLASS, FRAME, DOOR))
    # A fire takes the mansard down with the roof; the masonry stands, smoked, its iron and carved stone charred on it.
    SURVIVES = (
        ("G0_", 1.0, ("burnt",)), ("G1_", 1.0, ("burnt",)), ("U0_", 1.0, ("burnt",)), ("U1_", 1.0, ("burnt",)),
        ("U2_", 1.0, ("burnt",)), ("C0_", 1.0, ("burnt",)), ("C1_", 1.0, ("burnt",)), ("C2_", 1.0, ("burnt",)),
        ("D0_", 1.0, ("burnt",)), ("D1_", 1.0, ("burnt",)),
    )
    BURNT_ROOF = "X_Rubble"  # the top floor's slab: the zinc went with the mansard
    SMOKED_ROOF = (0.55, 0.53, 0.5)  # smoke on a grey slab, not on clay
    BURNT_ROOF_CELL_M = (1.0, 1.0, 2.0, 5.0)
    BURNT_WALL = 0.42  # pale limestone smoked: it must fall further than a painted wall to read as burnt
    WRECKS = ("FR_Railing",)

    def recipes(self):
        # Paris limestone: a warm cream, its courses cut in; the mouldings a shade paler and uncut
        ambientcg.bake("fr_ashlar", "Concrete034", 1.8, rough=(0.95, 0.05), normal=0.45, grime=stone((0.5, 0.45, 0.36)))
        ambientcg.bake("fr_trim", "Concrete034", 1.2, rough=(0.95, 0.05), normal=0.35, grime=toned((0.55, 0.5, 0.41), 0.35, level_cycles=6))
        ambientcg.bake("fr_burnt", "Concrete034", 1.8, rough=(0.95, 0.05), normal=0.45,
                       grime=lambda a, r: scorched(*stone((0.5, 0.45, 0.36))(a, r)))
        ambientcg.bake("fr_zinc", "Metal009", 2.0, rough=(0.9, 0.1), normal=0.3, grime=seams)
        ambientcg.bake("fr_metal", "Metal009", 1.0, rough=(1.0, 0.05), normal=0.4)
        ambientcg.bake("fr_paint", "Plastic010", 1.0, rough=(0.8, 0.12), normal=0.3)
        ambientcg.bake("fr_wood", "WoodFloor041", 1.5, rough=(1.0, 0.05), normal=0.6)
        ambientcg.bake("fr_concrete", "Concrete034", 2.0, rough=(1.0, 0.05), normal=0.6, grime=grime(0.35, 0.3))
        textures.recipe(self.BARS, tile=0.5)(uprights)

    # ------------------------------------------------------------ the graph
    def open_graph(self):
        graph.open_blend(os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), self.BLEND))
        return graph.modifier("FR_Building", "FR_Procedural_Building")

    def bounds_of(self, meshes, name):
        """What fills a module's opening (its frame, glass and door), not its stretch of wall."""
        if name not in self._bounds:
            soup = meshes[name]
            fill = soup.keep(np.isin(np.array(soup.mats)[soup.m], sorted(FILLS))) if name.startswith(WINDOWED) else soup
            self._bounds[name] = fill.bounds()
        return self._bounds[name]

    def __init__(self):
        super().__init__()
        self._bounds = {}

    def kit_meshes(self, graph_):
        """The modules without their flat ashlar, which is the shell's (`assemble`): a row is
        what stands in and on the wall, its frames, glass, iron and carved trim."""
        meshes = super().kit_meshes(graph_)
        # what a ruin's heap holds: a balcony's railing, torn off
        if "U2_Upper_ExtBalcony" not in meshes:
            raise SystemExit("no template has a continuous balcony, whose railing a ruin's heap holds (WRECKS)")
        railing = meshes["U2_Upper_ExtBalcony"]
        meshes["FR_Railing"] = railing.keep(np.array(railing.mats)[railing.m] == IRON)
        meshes = {name: soup.without({STONE}) for name, soup in meshes.items()}
        # a dormer is clad in zinc but for its stone front: cream cheeks and roof stood on the cornice like battlements from above
        dormer = meshes["R0_Mansard_Dormer"]
        normals, _ = dormer.normals()
        clad = (np.array(dormer.mats)[dormer.m] == TRIM) & (normals[:, 1] > -0.5)
        meshes["R0_Mansard_Dormer"] = Soup(dormer.v, dormer.c, dormer.t,
                                           np.where(clad, len(dormer.mats), dormer.m), dormer.s, [*dormer.mats, ZINC])
        return meshes

    # ------------------------------------------------------------ levels
    def heights(self, floors):
        """The ground floor's head, the cornice (the walls' top), the mansard's top, and the remains."""
        ground_m = self.GROUND_M
        roof_m = ground_m + self.STOREY_M * (floors - 2)
        top_m = roof_m + self.MANSARD_M
        return ground_m, roof_m, top_m, collapse.ruin_height(top_m)

    def storeys(self, floors):
        return floors - 1

    def crown(self, loops, floors, tier, wall, shade=1.0, trim=1.0):
        return Soup.empty()  # the cornice is the graph's own modules

    def intact_roof(self, b, roof_m, tier):
        """The mansard's zinc top; at the coarse tiers its slopes too, which the dormers carry
        near (`coarse`)."""
        if tier < 2:
            return b.top
        z0, z1 = roof_m + MANSARD_FOOT_M, roof_m + MANSARD_FOOT_M + MANSARD_SLOPE_M
        loop = b.loops[0]
        normals = [right_of(a, c) for a, c in loop]
        at = lambda k, d, z: (loop[k][0][0] + d * (normals[k - 1][0] + normals[k][0]),
                              loop[k][0][1] + d * (normals[k - 1][1] + normals[k][1]), z)
        slopes = [damage.quad([at(k, MANSARD_OUT_M, z0), at((k + 1) % len(loop), MANSARD_OUT_M, z0),
                        at((k + 1) % len(loop), -MANSARD_IN_M, z1), at(k, -MANSARD_IN_M, z1)], ZINC) for k in range(len(loop))]
        return Soup.join([b.top, *slopes])

    def far_tier(self, name, far, soup, tier, g):
        """At the coarse tiers the shell's flat wall is the wall: a module is what stands on it.
        Its glass is a pane over the opening's bounds, its iron a hull, the rest simplified."""
        mats = np.array(far.mats)[far.m] if len(far) else np.array([])
        pane = far.keep(np.isin(mats, ["X_Pane", FRAME, DOOR]))
        # a balcony's iron, its railing sheets among it: its bars are cutouts the far tiers' `far` leaves out
        iron = soup.keep(np.array([base_of(n) == IRON for n in soup.mats])[soup.m]) if len(soup) else soup
        rest = far.keep(~np.isin(mats, ["X_Pane", FRAME, DOOR, IRON]))
        out = []
        if len(pane):
            front = pane.mats.index("X_Pane") if "X_Pane" in pane.mats and (np.array(pane.mats)[pane.m] == "X_Pane").any() else int(pane.m[0])
            out.append(detail.front_quad(pane, front, 0.03, pane.mats))
        if len(iron):
            out.append(detail.hull(iron, 1.0, caps=tier == 2, sides=tier == 2))
        if name.startswith("R") and tier == 3:  # a far dormer is its window on the slope: its surround would stand up as a tooth
            rest = Soup.empty()
        if len(rest):  # at the farthest tier carved stone is a band of its colour on the wall
            out.append(detail.simplify(rest, g, self.BAR_M[tier]) if tier == 2 else detail.hull(rest, 1.5, caps=False, sides=False))
        return Soup.join(out)

    def row_tiers(self, name):
        """The cornice, the mansard and the carved panels are the shell's from tier 1: one
        row draws the block's outline and its roof."""
        return 0b0001 if name.startswith(TRIM_KEPT + ("D",)) else self.ROW_TIERS

    def coarse(self, name, far):
        """The carved trim round a window is the flat wall at the coarse tiers, but for the
        cornice's and a dormer's, which are the block's outline from afar; the mansard's zinc is
        the shell's slope there (`intact_roof`)."""
        if not len(far):
            return far
        if name.startswith(TRIM_KEPT):
            return far.without({ZINC})
        # a balcony's deck is carved stone, out past the wall at its floor: without it a far balcony is a railing in the air
        centre = far.v[far.t].mean(1)
        trim = np.array([base_of(n) == TRIM for n in far.mats])[far.m]
        return far.keep(~trim | ((centre[:, 1] < -0.25) & (centre[:, 2] < 0.15)))

    def near_walls(self, run, tier):
        """The graph's own ashlar at tier 0; at tier 1 the same wall flat, its openings cut and lined."""
        return run.wall if tier == 0 else run.flat

    def standing(self, run):
        return run.flat.coloured(0.72)

    def chimneys(self, width, depth, floors):
        """The stacks a Paris roof is read by: rendered stone, a row of clay pots on each, along the top
        of both long slopes where the party walls and flues come up, one every two bays. The graph has none."""
        _, roof_m, _, _ = self.heights(floors)
        foot = roof_m + MANSARD_FOOT_M + MANSARD_SLOPE_M - 0.3  # in the slope's top, under the cap's edge
        out = []
        for side in (-1, 1):
            y = side * (depth / 2 - MANSARD_IN_M - 0.45)
            for x in np.arange(-width / 2 + 4.0, width / 2 - 3.0, 6.0):
                out.append(damage.box((x - 0.8, y - 0.35, foot), (x + 0.8, y + 0.35, foot + CHIMNEY_M), TRIM, 0.82))
                out += [damage.box((px - 0.11, y - 0.11, foot + CHIMNEY_M), (px + 0.11, y + 0.11, foot + CHIMNEY_M + 0.35), POT, 1.0)
                        for px in (x - 0.45, x, x + 0.45)]
        return Soup.join(out)

    # ------------------------------------------------------------ a block
    def assemble(self, graph_, template):
        name, floors, bays_x, bays_y, detail_seed, pattern, style = template
        width, depth = 3.0 * bays_x + 2.0, 3.0 * bays_y + 2.0
        inputs = dict(self.INPUTS, **{"Bays X": bays_x, "Bays Y": bays_y, "Floors": floors - 2, "Detail Seed": detail_seed,
                                      "Detail Pattern": pattern, "Detail Style": style})
        tap, rows = graph_.tap(inputs)
        shift = np.eye(4)
        shift[:2, 3] = (-width / 2, -depth / 2)
        rows = [(n, shift @ m, None) for n, m, _ in rows]
        rect = (0.0, 0.0, width / 2, depth / 2)
        loops = outline([rect])
        runs, owners = [], np.zeros(len(rows), int)  # how many runs took each of the graph's rows
        for run in loops[0]:
            start, end = np.array(run[0], dtype=np.float64), np.array(run[1], dtype=np.float64)
            length = float(np.linalg.norm(end - start))
            along = (end - start) / length
            out = np.array([along[1], -along[0]])
            mine = []
            for k, (n, m, t) in enumerate(rows):  # a corner piece is its run's that starts there
                facing = m[:2, :2] @ np.array([0.0, -1.0])
                at = m[:2, 3] - start
                corner = n.startswith(("C", "T1_", "T2_"))
                if corner and np.linalg.norm(at) < 1e-3 or not corner and facing @ out > 0.99 and abs(at @ out) < 1e-3:
                    mine.append((n, m, t))
                    owners[k] += 1
            openings = [(n, m) for n, m, _ in mine if n.startswith(WINDOWED)]
            rooms = []
            for n, m in openings:
                kind = next((room for prefix, room in ROOMED.items() if n.startswith(prefix)), None)
                if kind is None:
                    continue
                lo, hi = self.bounds_of(tap.meshes, n)
                floor_z = m[2, 3]
                z0 = max(lo[2] - 0.03, self.ROOM_STEP_M if floor_z < 0.01 else 0.0)
                z1 = (self.GROUND_M if floor_z < 0.01 else self.STOREY_M) - 0.03
                box = np.diag([hi[0] - lo[0] + 0.1, ROOM_DEPTH_M, z1 - z0, 1.0])
                box[:3, 3] = ((lo[0] + hi[0]) / 2, hi[1] + 0.01, z0)
                rooms.append((kind, m @ box, None))
            masonry = []
            for n, m, _ in mine:
                soup = tap.meshes[n]
                wall = soup.keep(np.array(soup.mats)[soup.m] == STONE)
                if len(wall):
                    masonry.append(wall.transformed(m))
            wall = Soup.join(masonry)
            wall = Soup(wall.v, wall.c, wall.t, wall.m, wall.s, [self.WALL + MASK for _ in wall.mats])
            flat = self.holed(start, along, out, length, floors, [(n, m) for n, m in openings if not n.startswith("R")], tap.meshes)
            # a module with nothing but ashlar in it is the shell's alone
            mine = [row for row in mine if not WALLS.issuperset(tap.meshes[row[0]].mats)]
            runs.append(self.fit_rooms(SimpleNamespace(
                start=start, along=along, out=out, length=length, wall=wall, flat=flat, inner=Soup.empty(), cubes=Soup.empty(),
                # a dormer is lined while it stands; it is no opening of the walls, and goes with the mansard in a fire
                rows=mine + rooms, openings=[(n, m) for n, m in openings if not n.startswith("R")],
                bare=[(n, m) for n, m in openings if not n.startswith(tuple(ROOMED))]),
                [rect]))
        if (owners != 1).any():
            k = int(np.argmax(owners != 1))
            raise SystemExit(f"{name}: {rows[k][0]} at {rows[k][1][:3, 3].round(2)} stands on {owners[k]} runs, not one")
        top = Soup.join([tap.own.transformed(shift), self.chimneys(width, depth, floors)])
        return SimpleNamespace(
            name=name, floors=floors, parts={"body": rect}, rects=[rect], loops=loops, runs=runs, seed=detail_seed,
            wall=(255, 255, 255), roof_rows=[], rows=[row for run in runs for row in run.rows], top=top,
            recipe={"inputs": inputs})


if __name__ == "__main__":
    Paris().main()
