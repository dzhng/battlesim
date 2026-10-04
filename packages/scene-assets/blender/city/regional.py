"""What the regional families' houses and farms are built with beyond the shared house kit
(`kit.py`, `masonry.py`): roofs that break their pitch, flat roofs behind parapets, cornices,
inset porches, stoops, and the windows of an American frame house and a French one.

`homes.py` and `farmsteads.py` build each family's modules from these; the family's own
look (its materials, tints and plans) is theirs. Everything here keeps the rules of the city
readme: walls stand on a part's faces, a fitting overhangs by at most the set's side fit, and
a module's coarse tiers are the same picture with less geometry.
"""
import math

from kit import *  # noqa: F401,F403
from masonry import _ring, bpy_dark, chimney, house_walls, panel_door, pitched_roof, RoofShape, WALL_BANDS_M  # noqa: F401


# ---------------------------------------------------------------- small shared fittings
def build_step(kit, mat):
    m = kit.module("step", ground=True, **FITTING)
    box(m.n("slab"), (1.5, 0.4, 0.16), (0, -0.2, 0.08), mat, m.root)


def build_chimney(kit, name, plan, mat, cap, pot, pots):
    """A chimney module `CHIMNEY_M` tall (`stack` stands it on a ridge)."""
    m = kit.module(name, **FITTING)
    chimney(m.n("c"), plan, CHIMNEY_M, mat, cap, m.root, pots=pots, pot_mat=pot)


def build_rainwater(kit, metal):
    # A metre of gutter along +X and a metre of downpipe up +Z: rows stretch them to length.
    m = kit.module("gutter", **FITTING)
    cyl(m.n("run"), 0.06, 1.0, (0, 0, 0), "X", metal, m.root, seg=6, caps=False)
    m = kit.module("downpipe", **FITTING)
    cyl(m.n("pipe"), 0.045, 1.0, (0, 0, 0.5), "Z", metal, m.root, seg=6, caps=False)


# ---------------------------------------------------------------- roofs
MANSARD_OVER_M = 0.12  # a mansard's steep slope overhangs its cornice barely: further, it hung over the top floor's windows


class BrokenRoof:
    """A roof whose pitch breaks: steep from the eave to a knuckle `run` metres in from the
    wall and `knee` above the eave, then shallow up to the ridge. Its ends are gabled (a
    gambrel, the American barn's and the Dutch Colonial's) or, with `hipped`, it is steep
    all round under a low hip (a mansard). It answers what `RoofShape` answers, so the
    house builders (`house_walls`, `pitched_roof`, a row's end wall) take either."""

    def __init__(self, x0, x1, y0, y1, eave, knee, ridge, run, along="x", hipped=False, over=OVER_M, verge=(VERGE_M, VERGE_M)):
        self.swap = along == "y"
        self.a0, self.a1, self.b0, self.b1 = (y0, y1, x0, x1) if self.swap else (x0, x1, y0, y1)
        self.eave, self.ridge, self.run, self.knee_z = eave, ridge, run, eave + knee
        self.bc, self.half = (self.b0 + self.b1) / 2, (self.b1 - self.b0) / 2
        if not 0 < run < self.half or not eave < self.knee_z < ridge:
            raise SystemExit("a broken roof's knuckle lies between its eave and its ridge, inside its span")
        if hipped and self.a1 - self.a0 < self.b1 - self.b0 - 1e-9:
            raise SystemExit("a mansard's ridge runs along its longer side")
        self.tan = knee / run  # the steep slope's, which a dormer stands on
        self.tan_top = (ridge - self.knee_z) / (self.half - run)
        self.hipped, self.hips = hipped, (1.0, 1.0) if hipped else (0.0, 0.0)
        self.over = over
        self.reach = self.half + over
        self.edge_z = eave - over * self.tan
        self.ends = [self.a0 - (over if hipped else verge[0]), self.a1 + (over if hipped else verge[1])]

    def world(self, a, b, z):
        return (b, a, z) if self.swap else (a, b, z)

    def _profile(self):
        """(inset from the wall, height): the eave's edge, the knuckle and the ridge."""
        return [(-self.over, self.edge_z), (self.run, self.knee_z), (self.half, self.ridge)]

    def _corner(self, d, z, k):
        """Corner `k` (counter-clockwise from the low-a, low-b one) of the ring `d` in from the walls, at `z`."""
        a = (self.a0 + d, self.a1 - d, self.a1 - d, self.a0 + d)[k]
        b = (self.b0 + d, self.b0 + d, self.b1 - d, self.b1 - d)[k]
        return self.world(a, b, z)

    def planes(self):
        (d0, z0), (d1, z1), (d2, z2) = self._profile()
        if not self.hipped:
            out = []
            for s in (-1, 1):
                for (da, za), (db, zb) in (((d0, z0), (d1, z1)), ((d1, z1), (d2, z2))):
                    ba, bb = self.bc + s * (self.half - da), self.bc + s * (self.half - db)
                    out.append([self.world(self.ends[0], ba, za), self.world(self.ends[1], ba, za),
                                self.world(self.ends[1], bb, zb), self.world(self.ends[0], bb, zb)])
            return out
        out = [[self._corner(d0, z0, k), self._corner(d0, z0, (k + 1) % 4), self._corner(d1, z1, (k + 1) % 4), self._corner(d1, z1, k)]
               for k in range(4)]
        r0, r1 = self.world(self.a0 + self.half, self.bc, z2), self.world(self.a1 - self.half, self.bc, z2)
        inner = [self._corner(d1, z1, k) for k in range(4)]
        out.append(_ring([inner[0], inner[1], r1, r0]))
        out.append(_ring([inner[2], inner[3], r0, r1]))
        out.append([inner[1], inner[2], r1])
        out.append([inner[3], inner[0], r0])
        return out

    def crests(self):
        (d0, z0), (d1, z1), (d2, z2) = self._profile()
        if not self.hipped:
            return [(self.world(self.ends[0], self.bc, z2), self.world(self.ends[1], self.bc, z2))]
        r0, r1 = self.world(self.a0 + self.half, self.bc, z2), self.world(self.a1 - self.half, self.bc, z2)
        out = [(r0, r1)] if math.dist(r0, r1) > 1e-6 else []
        for k in range(4):
            out.append((self._corner(d0, z0, k), self._corner(d1, z1, k)))
            out.append((self._corner(d1, z1, k), r0 if k in (0, 3) else r1))
        return out

    def end_wall_top(self, k, drop):
        top = self.eave - drop
        if self.hipped:
            return [(self.b1, top), (self.b0, top)]
        return [(self.b1, top), (self.b1 - self.run, self.knee_z - drop), (self.bc, self.ridge - drop),
                (self.b0 + self.run, self.knee_z - drop), (self.b0, top)]


class FlatShape:
    """A flat roof's box for `house_walls`: walls straight up to the parapet's top `top`."""

    def __init__(self, x0, x1, y0, y1, top):
        self.swap = False
        self.a0, self.a1, self.b0, self.b1 = x0, x1, y0, y1
        self.eave = self.ridge = self.edge_z = top
        self.hips = (1.0, 1.0)

    def world(self, a, b, z):
        return (a, b, z)

    def end_wall_top(self, k, drop):
        return [(self.b1, self.eave - drop), (self.b0, self.eave - drop)]


PARAPET_M, PARAPET_THICK_M = 0.6, 0.3  # a flat roof lies this far under its parapet's top, inside walls this thick


def flat_top(m, tag, rect, top, roof, coping, walled=("south", "north", "west", "east")):
    """A flat roof behind parapets into module `m`: the roof `PARAPET_M` under the walls' top
    (none if `roof` is None: a row's end wall stands at the edge of its houses' roofs), the
    parapets' inner faces on the `walled` sides, and a coping along each. From far off the
    coping's pale line is what draws the roof's edge."""
    x0, x1, y0, y1 = rect
    th = PARAPET_THICK_M
    ix0, ix1 = x0 + (th if "west" in walled else 0.0), x1 - (th if "east" in walled else 0.0)
    iy0, iy1 = y0 + (th if "south" in walled else 0.0), y1 - (th if "north" in walled else 0.0)
    low = top - PARAPET_M
    if roof is not None:
        flat_faces(m.n(f"{tag}_roof"), [([(ix0, iy0, low), (ix1, iy0, low), (ix1, iy1, low), (ix0, iy1, low)], (0, 0, 1))], roof, m.root)
    inner = dict(south=([(ix0, iy0, low), (ix1, iy0, low), (ix1, iy0, top), (ix0, iy0, top)], (0, 1, 0)),
                 north=([(ix0, iy1, low), (ix1, iy1, low), (ix1, iy1, top), (ix0, iy1, top)], (0, -1, 0)),
                 west=([(ix0, iy0, low), (ix0, iy1, low), (ix0, iy1, top), (ix0, iy0, top)], (1, 0, 0)),
                 east=([(ix1, iy0, low), (ix1, iy1, low), (ix1, iy1, top), (ix1, iy0, top)], (-1, 0, 0)))
    flat_faces(m.n(f"{tag}_parapet"), [inner[s] for s in walled], coping, m.root)
    for s in walled:
        if s in ("south", "north"):
            y = y0 + th / 2 if s == "south" else y1 - th / 2
            box(m.n(f"{tag}_coping_{s}"), (x1 - x0, th + 0.06, 0.08), ((x0 + x1) / 2, y, top - 0.02), coping, m.root)
        else:
            x = x0 + th / 2 if s == "west" else x1 - th / 2
            box(m.n(f"{tag}_coping_{s}"), (th + 0.06, y1 - y0 - 2 * th, 0.08), (x, (y0 + y1) / 2, top - 0.02), coping, m.root)
    return low


def cornice(m, tag, rect, side, top, mat, brackets=1.0, reach=0.42):
    """A bracketed cornice along `side` of `rect` at the walls' top: a frieze, a crown
    `reach` proud of the wall (inside the side fit) and its brackets under it, one about
    every `brackets` metres. The crown and frieze hold at every tier: from far off a row of
    flat roofs is told by the dark line of its cornices."""
    x0, x1, y0, y1 = rect
    along_x = side in ("south", "north")
    a0, a1 = (x0, x1) if along_x else (y0, y1)
    face = dict(south=y0, north=y1, west=x0, east=x1)[side]
    out = -1 if side in ("south", "west") else 1
    length = a1 - a0

    def place(name, size, along, proud, z, lods=TIERS):
        w, d, h = size
        if along_x:
            box(m.n(f"{tag}_{name}"), (w, d, h), (along, face + out * proud, z), mat, m.root, lods=lods)
        else:
            box(m.n(f"{tag}_{name}"), (d, w, h), (face + out * proud, along, z), mat, m.root, lods=lods)

    mid = (a0 + a1) / 2
    place("frieze", (length, 0.12, 0.7), mid, 0.06, top - 0.65)
    place("crown", (length, reach, 0.3), mid, reach / 2, top - 0.15)
    n = max(2, round(length / brackets))
    for k in range(n):
        place(f"bracket_{k}", (0.16, reach - 0.04, 0.5), a0 + (k + 0.5) * length / n, (reach - 0.04) / 2, top - 0.55, lods=(0, 1, 2))


# ---------------------------------------------------------------- porches
PORCH_DECK_M = 0.3


def porch_shell(m, tag, shape, depth, ceiling, wall, roof, trim, post, deck, plinth_mat, posts, sides=("a0", "a1", "b0", "b1")):
    """A house's walls and roof with an inset porch across its south front: the ground
    floor's south wall stands `depth` back from the box's face under a ceiling at
    `ceiling`, on posts (`posts`, their x) along the face; above the ceiling the walls
    stand on the box's faces as any house's do. The porch is inside the physical box, as
    a porch under a house's own roof is: what the box hides, the house hides."""
    x0, x1, y0, y1 = (shape.b0, shape.b1, shape.a0, shape.a1) if shape.swap else (shape.a0, shape.a1, shape.b0, shape.b1)
    upper = shape.eave - 0.03 - ceiling
    house_walls(m.n(f"{tag}_upper"), shape, wall, m.root, sides, base=ceiling,
                bands=WALL_BANDS_M if upper >= sum(WALL_BANDS_M) + 0.3 else None)
    along = "y" if shape.swap else "x"
    lower = RoofShape(x0, x1, y0 + depth, y1, ceiling + 0.03, ceiling + 0.031, along, (0.0, 0.0), 0.0, (0.0, 0.0))
    house_walls(m.n(f"{tag}_lower"), lower, wall, m.root, sides, bands=WALL_BANDS_M)
    if roof is not None:
        pitched_roof(m.n(f"{tag}_roof"), shape, roof, trim, m.root)
    box(m.n(f"{tag}_plinth"), (x1 - x0 + 0.08, y1 - y0 - depth + 0.08, 0.4), ((x0 + x1) / 2, (y0 + depth + y1) / 2, 0.2), plinth_mat, m.root,
        lods=(0, 1, 2))
    box(m.n(f"{tag}_deck"), (x1 - x0, depth, PORCH_DECK_M), ((x0 + x1) / 2, y0 + depth / 2, PORCH_DECK_M / 2), deck, m.root)
    flat_faces(m.n(f"{tag}_ceiling"), [([(x0, y0, ceiling), (x1, y0, ceiling), (x1, y0 + depth, ceiling), (x0, y0 + depth, ceiling)],
                                        (0, 0, -1))], trim, m.root, lods=(0, 1, 2))
    box(m.n(f"{tag}_beam"), (x1 - x0, 0.2, 0.3), ((x0 + x1) / 2, y0 + 0.12, ceiling - 0.15), post, m.root)
    for s, x in (("w", x0 + 0.12), ("e", x1 - 0.12)):
        box(m.n(f"{tag}_beam_{s}"), (0.2, depth - 0.2, 0.3), (x, y0 + 0.2 + (depth - 0.2) / 2, ceiling - 0.15), post, m.root, lods=(0, 1, 2))
    tall = ceiling - 0.3 - PORCH_DECK_M
    for k, x in enumerate(sorted({x0 + 0.12, x1 - 0.12, *posts})):
        box(m.n(f"{tag}_post_{k}"), (0.2, 0.2, tall), (x, y0 + 0.12, PORCH_DECK_M + tall / 2), post, m.root)


def porch_rail(kit, name, w, rail, opening_h=1.75):
    """A bay of porch railing, facing -Y on the box's face: a top and a bottom rail and
    balusters between them, the open front above it the bay's opening (where a soldier on
    the porch looks out)."""
    m = kit.module(name, **FITTING)
    m.opening = (w, opening_h, 0.95)
    y = 0.12
    box(m.n("top"), (w, 0.12, 0.08), (0, y, 0.95), rail, m.root)
    box(m.n("bottom"), (w, 0.1, 0.07), (0, y, PORCH_DECK_M + 0.1), rail, m.root, lods=(0, 1))
    n = int(w / 0.16)
    for k in range(n):
        x = -w / 2 + (k + 0.5) * w / n
        box(m.n(f"baluster_{k}"), (0.045, 0.045, 0.5), (x, y, PORCH_DECK_M + 0.38), rail, m.root, lods=(0,) if k % 2 else (0, 1))
    return m


def porch_steps(kit, name, w, tread, opening_h=2.0):
    """The steps up to a porch's deck at its entrance bay, inside the side fit; the open front
    over them is the bay's opening."""
    m = kit.module(name, ground=True, **FITTING)
    m.opening = (w, opening_h, PORCH_DECK_M)
    rise = PORCH_DECK_M / 2
    box(m.n("lower"), (w, 0.3, rise), (0, -0.3, rise / 2), tread, m.root)
    box(m.n("upper"), (w, 0.3, 2 * rise), (0, -0.05, rise), tread, m.root, lods=(0, 1, 2))
    return m


PORCH_RAIL_M = 2.0  # a porch rail module's length: a row's scale fits it to its bay


def porch_front(t, edge, depth, entrance, rails, door, tint, windows=(), window="window_sash_porch", rail_w=2.6):
    """An inset porch's front rows (`t` is a template, or a farm standing in for one): steps up
    at the entrance, a railing in each other bay, and the door and windows on the wall set
    back `depth` behind it."""
    t.entrance(edge, entrance)
    t.mount("porch_steps", edge, entrance, tiers=TIERS_0_TO_2)
    t.mount(door, edge, entrance, out=-depth, tiers=TIERS_0_TO_2, tint=tint)
    for o in rails:
        t.mount("porch_rail", edge, o, scale=(rail_w / PORCH_RAIL_M, 1.0, 1.0), tiers=TIERS_0_TO_2)
    for o in windows:
        t.mount(window, edge, o, z=SILL_M, out=-depth, tiers=TIERS_0_TO_2)


def porch_side(t, edge, offset, windows, floors, rail_w=1.9):
    """A porch's open side: a railing in its bay, and `windows` in the side wall's other bays."""
    t.mount("porch_rail", edge, offset, scale=(rail_w / PORCH_RAIL_M, 1.0, 1.0), tiers=TIERS_0_TO_2)
    glaze(t, edge, floors, windows, skip=(offset,))


# ---------------------------------------------------------------- dormers
# A dormer, one to each roof covering: a small gabled window standing on a slope, its foot's
# middle at the origin and its back run into the roof behind it. It faces -Y; a row tints its cheeks
# the house's colour. One, off to a side, is what tells a house from its mirror image from above.
DORMER_WINDOW = (0.8, 0.7, 0.25)  # its opening: width, height and foot


def build_dormer(kit, name, cheeks, roof, glass, pane, frame, pitched=False):
    """A dormer module. `pitched` gives it a true pitched roof with its own UVs, its gable closed in
    its cheeks' material, and its cheeks and pane at the coarsest tier too, so a row of them holds
    as rows at every tier; without it the roof is one prism, box-projected, and the far tiers keep
    the dormer as its building's folded boxes (`dormer_far`)."""
    m = kit.module(name, **FITTING)
    # near, its face is open round the window, with glass in the opening over the dark of the attic
    x, d, top, (w, h, foot) = 0.65, 2.3, 1.05, DORMER_WINDOW
    face = [(-x, -w / 2, 0, top), (w / 2, x, 0, top), (-w / 2, w / 2, 0, foot), (-w / 2, w / 2, foot + h, top)]
    quads = [([(a, 0, c), (b, 0, c), (b, 0, e), (a, 0, e)], (0, -1, 0)) for a, b, c, e in face]
    quads += [([(s * x, 0, 0), (s * x, d, 0), (s * x, d, top), (s * x, 0, top)], (s, 0, 0)) for s in (-1, 1)]
    flat_faces(m.n("cheeks"), quads, cheeks, m.root, OPEN_TIERS)
    box(m.n("cheeks_far"), (2 * x, d, top), (0, d / 2, top / 2), cheeks, m.root, lods=(2, 3) if pitched else (2,))
    reveal(m, w, h, window_reveal(), foot, depth=0.1)
    sheet(m.n("glass"), w, h, (0, 0.06, foot), glass, m.root, lods=OPEN_TIERS)
    reveal(m, w, h, bpy_dark(), foot, depth=0.9, start=0.1, back=True, tag="recess")  # (too small for a room box: a squeezed room is a pale panel)
    box(m.n("pane"), (0.8, 0.04, 0.7), (0, -0.02, 0.6), pane, m.root, lods=(2, 3) if pitched else (2,))
    box(m.n("frame"), (0.96, 0.05, 0.08), (0, -0.025, 0.2), frame, m.root, lods=(0,))
    if not pitched:
        prism(m.n("roof"), [(-0.85, 0.98), (0.85, 0.98), (0.0, 1.45)], 2.5, (0, 1.1, 0), roof, m.root)
        return m
    # a prism's box-projected courses ran down one slope and across the other: a checkerboard from the camera
    # low-pitched: from the steep camera a tall one's shaded half read as a dark triangle on the roof
    pitched_roof(m.n("roof"), RoofShape(-x, x, 0.0, d, top, 1.3, "y", (0.0, 0.0), 0.2, (0.15, 0.0)), roof, frame, m.root)
    flat_faces(m.n("gable"), [([(-x, 0, top), (x, 0, top), (0, 0, 1.3)], (0, -1, 0))], cheeks, m.root)
    return m


def dormer_far(cheeks, roof):
    """What the coarse tiers keep of a dormer (`fold_far`'s boxes): its cheeks and its roof."""
    return [((1.3, 2.3, 1.05), (0, 1.15, 0.525), cheeks), ((1.7, 2.5, 0.44), (0, 1.1, 1.2), roof)]


def place_dormer(t, shape, module, side, along, inset, tint, tiers=TIERS_0_TO_2):
    """A dormer on the `side` slope of a house's roof, `along` the eave from the house's middle, its
    face `inset` metres up the slope from the wall."""
    nx, ny = SIDES[side][1]
    reach = (shape.half if (side in ("south", "north")) != shape.swap else (shape.a1 - shape.a0) / 2) - inset
    cx, cy = (shape.b0 + shape.b1) / 2 if shape.swap else (shape.a0 + shape.a1) / 2, shape.bc if not shape.swap else (shape.a0 + shape.a1) / 2
    x, y = (cx + along, cy + ny * reach) if ny else (cx + nx * reach, cy + along)
    t.place(module, x, y, shape.eave + inset * shape.tan, math.atan2(ny, nx) + math.pi / 2, tiers=tiers, tint=tint)


# ---------------------------------------------------------------- stoops
STOOP_M = 0.51  # a stoop's top: a raised front door's threshold
STOOPS = {}  # stoop module -> its top, where its door stands


def stoop(kit, name, stone, iron, w=1.6, top=STOOP_M, steps=3):
    """A rowhouse's stoop: stone steps up to a raised front door at `top`, inside the side fit,
    between cheek walls carrying iron railings. A brownstone's is steep and high: its parlour door
    stands well over the street."""
    m = kit.module(name, ground=True, **FITTING)
    STOOPS[name] = top
    rise, going = top / steps, 0.48 / steps
    for k in range(steps):
        y = -(steps - k) * going
        box(m.n(f"step_{k}"), (w, (steps - k) * going, rise), (0, y / 2, rise * (k + 0.5)), stone, m.root, lods=(0, 1, 2) if k else TIERS)
    run = steps * going
    for s in (-1, 1):
        tag = "ab"[s > 0]
        prism(m.n(f"cheek_{tag}"), [(-run, 0), (0, 0), (0, top + 0.25), (-run, 0.3)], 0.16,
              (s * (w / 2 + 0.08), 0, 0), stone, m.root, rot=(0, 0, math.pi / 2), lods=TIERS)
        box(m.n(f"rail_{tag}"), (0.04, run, 0.04), (s * (w / 2 + 0.08), -run / 2, top + 0.85), iron, m.root, lods=(0, 1))
        for k, y in enumerate((-run + 0.03, -0.03)):
            box(m.n(f"newel_{tag}_{k}"), (0.035, 0.035, 0.6), (s * (w / 2 + 0.08), y, top + 0.55 - (0.25 if k == 0 else 0)), iron, m.root,
                lods=(0,))
    return m


# ---------------------------------------------------------------- doors
def door_module(kit, name, w, h, leaf, frame, pane=None, light=0.0):
    """A panelled front door as a module (`masonry.panel_door`), `light` metres of fanlight over it;
    its leaf is the opening the facade checks see."""
    m = kit.module(name, ground=True, **FITTING)
    m.opening = (w, h, 0.0)
    panel_door(m.n("door"), w, h, leaf, frame, m.root, pane, light=light)
    return m


def x_barn_doors(kit, name, w, h, leaf, trim, frame, hoist=0.0, ground=True):
    """An American barn's doors facing -Y, the threshold's centre on the wall plane at the origin: two
    board leaves, each framed and crossed by a white X, under a lintel; `hoist` metres of beam stand
    out over a loft's doors. The X is what tells an American barn across a field."""
    m = kit.module(name, ground=ground, **FITTING)
    m.opening = (w, h, 0.0)
    half, bar = w / 2, 0.14
    for s in (-1, 1):
        tag = "ab"[s > 0]
        cx = s * half / 2
        box(m.n(f"leaf_{tag}"), (half - 0.02, 0.06, h), (cx, -0.03, h / 2), leaf, m.root)
        for k, (dx, dz, bw, bh) in enumerate(((0, bar / 2, half - 0.02, bar), (0, h - bar / 2, half - 0.02, bar),
                                              (-half / 2 + bar / 2 + 0.01, h / 2, bar, h), (half / 2 - bar / 2 - 0.01, h / 2, bar, h))):
            box(m.n(f"rim_{tag}_{k}"), (bw, 0.03, bh), (cx + dx, -0.075, dz), trim, m.root, lods=(0, 1, 2))
        run, rise = half - 2 * bar, h - 2 * bar
        for k, sign in enumerate((-1, 1)):
            box(m.n(f"cross_{tag}_{k}"), (math.hypot(run, rise), 0.03, bar), (cx, -0.075, h / 2), trim, m.root,
                rot=(0, sign * math.atan2(rise, run), 0), lods=(0, 1))
    box(m.n("lintel"), (w + 0.4, 0.14, 0.22), (0, -0.07, h + 0.11), frame, m.root, lods=(0, 1, 2))
    if hoist:
        box(m.n("hoist"), (0.16, hoist, 0.18), (0, -hoist / 2, h + 0.5), frame, m.root, lods=(0, 1, 2))
    return m


# ---------------------------------------------------------------- windows
CASING_M = 0.12  # a frame house's flat trim round its windows


def sash(kit, name, w, h, frame, glass, pane, sill, casing=None, shutter=None, hood=None, panes=(1, 1), behind="rooms", dark=None):
    """An American double-hung window, a module facing -Y with its sill's middle on the wall
    plane at the origin: an upper and a lower sash meeting half way up, `panes` (across,
    up) lights in each; a flat casing round it in `casing` (a frame house's trim), a sill,
    and `hood`, a stone lintel over it (a brick or brownstone front). `shutter` hangs a
    louvred shutter either side. Its opening, glass and room are the shared window's."""
    m = kit.module(name, **FITTING)
    m.opening = (w, h, 0.0)
    trim = casing or frame
    c = CASING_M
    box(m.n("head"), (w + 2 * c, 0.05, c), (0, -0.025, h + c / 2), trim, m.root, lods=OPEN_TIERS)
    box(m.n("sill"), (w + 2 * c + 0.08, 0.14, 0.06), (0, -0.07, -0.03), sill, m.root, lods=OPEN_TIERS)
    for s in (-1, 1):
        tag = "ab"[s > 0]
        box(m.n(f"jamb_{tag}"), (c, 0.05, h), (s * (w / 2 + c / 2), -0.025, h / 2), trim, m.root, lods=OPEN_TIERS)
        if shutter is not None:
            box(m.n(f"shutter_{tag}"), (w / 2 + 0.04, 0.04, h + 0.06), (s * (w * 0.75 + c + 0.06), -0.03, h / 2), shutter, m.root,
                lods=OPEN_TIERS)
    if hood is not None:
        box(m.n("hood"), (w + 2 * c + 0.3, 0.16, 0.24), (0, -0.08, h + c + 0.12), hood, m.root, lods=OPEN_TIERS)
    across, up = panes
    uprights = [-w / 2 + k * w / across for k in range(1, across)]
    rails = [h / 2] + [h / 2 * k / up for k in range(1, up)] + [h / 2 + h / 2 * k / up for k in range(1, up)]
    reveal(m, w, h, window_reveal())
    sheet(m.n("glass"), w, h, (0, GLASS_AT_M, 0), glass, m.root, lods=OPEN_TIERS)
    glazing_bars(m, w, h, frame, uprights=uprights, rails=rails, lods=OPEN_TIERS)
    sheet(m.n("pane"), w, h, (0, -0.02, 0), pane, m.root, lods=(2, 3))
    if behind is None:
        reveal(m, w, h, dark, depth=REVEAL_M + 0.6, start=REVEAL_M, back=True, tag="recess")
    opening(kit, name, w, h, 0.0, behind)
    return m


def french_window(kit, name, w, h, frame, glass, pane, surround, shutter, railing=None, foot=0.0, behind="rooms"):
    """A French window, a module facing -Y with its origin on the wall plane at a window's
    sill height (`SILL_M` over its floor, as every window's is): its opening's foot is
    `foot` from there, so a door-window reaching nearly to the floor has a foot below the
    origin. Two tall leaves meet in the middle, three or four lights up each, in a flat
    stone surround; louvred shutters (persiennes) stand folded back against the wall either
    side; `railing`, a wrought-iron guard, runs across a door-window from its foot up to
    the sill line. Its opening, glass and room are the shared window's."""
    m = kit.module(name, **FITTING)
    m.opening = (w, h, foot)
    c = 0.14
    box(m.n("head"), (w + 2 * c, 0.04, c), (0, -0.02, foot + h + c / 2), surround, m.root, lods=OPEN_TIERS)
    box(m.n("sill"), (w + 2 * c + 0.06, 0.12, 0.06), (0, -0.06, foot - 0.03), surround, m.root, lods=OPEN_TIERS)
    for s in (-1, 1):
        tag = "ab"[s > 0]
        box(m.n(f"jamb_{tag}"), (c, 0.04, h), (s * (w / 2 + c / 2), -0.02, foot + h / 2), surround, m.root, lods=OPEN_TIERS)
        box(m.n(f"shutter_{tag}"), (w / 2 + 0.03, 0.04, h + 0.04), (s * (w * 0.75 + c + 0.05), -0.04, foot + h / 2), shutter, m.root,
            lods=OPEN_TIERS)
    lights = 4 if h > 1.8 else 3
    rails = [h * k / lights for k in range(1, lights)]
    reveal(m, w, h, window_reveal(), foot)
    sheet(m.n("glass"), w, h, (0, GLASS_AT_M, foot), glass, m.root, lods=OPEN_TIERS)
    glazing_bars(m, w, h, frame, uprights=(0.0,), rails=rails, foot=foot, lods=OPEN_TIERS)
    sheet(m.n("pane"), w, h, (0, -0.02, foot), pane, m.root, lods=(2, 3))
    if railing is not None:
        sheet(m.n("guard"), w + 0.1, -foot, (0, -0.06, foot), railing, m.root, lods=OPEN_TIERS)
        box(m.n("guard_rail"), (w + 0.14, 0.05, 0.04), (0, -0.06, 0.02), railing_rail(), m.root, lods=OPEN_TIERS)
    opening(kit, name, w, h, foot, behind)
    return m


_IRON = []


def railing_rail():
    """The dark iron of a guard's handrail and a stoop's railings."""
    if not _IRON:
        _IRON.append(flat_paint("wrought_iron", (0.025, 0.025, 0.024), rough=0.55, metal=0.5, grime=0.0))
    return _IRON[0]
