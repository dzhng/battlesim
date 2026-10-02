"""The authoring helper for a hand-scripted city set: modules, templates and the two files.

A set's script (`homes.py` is the worked example) does three things with it:

    kit = Kit("homes", "homes.py", fit_side_m=0.5, fit_top_m=0.9)

    m = kit.module("window_a")                       # a module: build its parts under m.root,
    box(m.n("glass"), ..., parent=m.root)            #   named m.n(part), with the usual tier suffixes

    t = kit.template("china-home-10x8-1f", "detached_home", "china", recipe={...})
    t.part("body", -5, 5, -4, 4, height=5.2)         # the physical boxes (axis-aligned)
    t.floors(0.0)
    t.lattice("body-south", bay_at=0.0)              # every exposed edge gets its 3 m bay lattice
    t.entrance("body-south", 0.0)
    t.place("home_10x8_1f_shell")                    # rows: a module in the template's frame ...
    for o in t.bays("body-south"):
        t.mount("window_a", "body-south", o, z=0.95) # ... or on a facade, facing out of it

    kit.write()                                      # bake, check, write kit.glb and templates.json

A template's facade edges and joins are derived from its parts: a face, or the
stretch of one, that another part stands against is internal and joined to that
part's matching stretch; the rest is exposed, and named `<part>-<side>` (with
`-<n>` along the face when it is split). The contract (`BuildingTemplateDescriptor`,
`require_complete`) is the authority on what a complete descriptor is; this
helper only builds one that cannot miss an edge.

A module that mounts on a wall is authored facing -Y, with +X along the wall:
`mount` turns it so +X runs along the edge's own offsets.

Every module is baked on its own (it is drawn on many buildings), so its paint
sees its own frame: `ground=True` for one that stands on the ground at z = 0
(mud rises on it and the ground occludes it).

The same script writes the same bytes: names sort, numbers round, nothing reads the clock.
"""
import json
import math
import os
import re
import sys

import bpy
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from parts import *  # noqa: E402,F401,F403

REPO = os.path.abspath(os.path.join(HERE, "../../../.."))
BAY_PITCH_M = 3.0
# Which detail tiers a row draws at (bit n is tier n).
EVERY_TIER, TIERS_0_TO_2, TIERS_0_TO_1, TIER_0 = 15, 7, 3, 1
WHITE = (255, 255, 255)
# What the hand-scripted sets' houses keep in common (the rows at the foot of this file follow from them).
SILL_M = 0.95  # a window's sill above its floor
OVER_M, VERGE_M = 0.35, 0.25  # how far eaves and gable verges overhang
CHIMNEY_M = 1.8  # a chimney module's height
FITTING = dict(ao_distance=0.4, paint_scale=8.0)  # a small module's bake: no face needs splitting for paint
# side -> (the contract's facade, outward normal, the direction its offsets run)
SIDES = {
    "east": ("positive_x", (1, 0), (0, 1)),
    "north": ("positive_y", (0, 1), (-1, 0)),
    "west": ("negative_x", (-1, 0), (0, -1)),
    "south": ("negative_y", (0, -1), (1, 0)),
}
OPPOSITE = {"east": "west", "west": "east", "north": "south", "south": "north"}


def _fail(message):
    raise SystemExit(f"kit: {message}")


class Module:
    def __init__(self, name, ground, bake):
        self.name, self.ground, self.bake = name, ground, bake
        self.root = empty(name)
        self.triangles = None  # per tier, after the bake
        self.points = None  # every vertex of every tier, in the module's frame

    def n(self, part):
        """A part's object name: Blender's names are one namespace, so a part carries its module's."""
        return f"{self.name}_{part}"

    def meshes(self):
        return sorted((o for o in self.root.children_recursive if o.type == "MESH"), key=lambda o: o.name)


class Template:
    def __init__(self, kit, id_, category, family, recipe, status):
        self.kit, self.id, self.category, self.family = kit, id_, category, family
        self.recipe, self.status = recipe or {}, status
        self.parts, self.floor_heights, self.entrances = [], [], []
        self.lattices, self.rows = {}, {"intact": []}
        self._edges = None

    # -- the physical template
    def part(self, id_, x0, x1, y0, y1, height, base_z=0.0):
        """A box of the building: what stops movers, rounds and sight."""
        if self._edges is not None:
            _fail(f"{self.id}: add every part before asking for edges")
        self.parts.append(dict(id=id_, x0=x0, x1=x1, y0=y0, y1=y1, base=base_z, top=base_z + height))

    def floors(self, *heights):
        """The floor datums soldiers stand on, lowest first."""
        self.floor_heights = [float(h) for h in heights]

    def lattice(self, edge, bay_at):
        """Give an exposed edge its bays: 3 m apart, one of them at offset `bay_at`."""
        if not self.edges()[edge]["exposed"]:
            _fail(f"{self.id}: {edge} is internal and has no bays")
        self.lattices[edge] = bay_at % BAY_PITCH_M

    def entrance(self, edge, offset_m):
        self.entrances.append(dict(id=f"door-{len(self.entrances)}", edge=edge, offset_m=float(offset_m)))

    def edges(self):
        """Every part face as edges, by id: exposed, or internal and joined to a neighbour."""
        if self._edges is None:
            self._edges, self._joins = self._derive()
        return self._edges

    def _derive(self):
        edges, joined = {}, {}
        for p in self.parts:
            cx, cy = (p["x0"] + p["x1"]) / 2, (p["y0"] + p["y1"]) / 2
            for side, (facade, normal, along) in SIDES.items():
                axis = 0 if normal[0] else 1  # the axis the face is across
                lo, hi = ("x0", "x1") if axis == 0 else ("y0", "y1")
                face = p[hi] if sum(normal) > 0 else p[lo]
                t0, t1 = ("y0", "y1") if axis == 0 else ("x0", "x1")  # the face's extent across the other axis
                centre, sign = (cy, along[1]) if axis == 0 else (cx, along[0])
                half = (p[t1] - p[t0]) / 2
                covered = []
                for q in self.parts:
                    if q is p or q["top"] <= p["base"] or q["base"] >= p["top"]:
                        continue
                    a, b = max(p[t0], q[t0]), min(p[t1], q[t1])
                    if b - a <= 1e-9:
                        continue
                    touching = abs((q[lo] if sum(normal) > 0 else q[hi]) - face) < 1e-9
                    if not touching:
                        if q[lo] < face < q[hi]:
                            _fail(f"{self.id}: parts {p['id']} and {q['id']} overlap; parts may only touch")
                        continue
                    if (q["base"], q["top"]) != (p["base"], p["top"]):
                        _fail(f"{self.id}: {p['id']} and {q['id']} touch but differ in base or top: a join needs both equal")
                    covered.append((*sorted(((a - centre) * sign, (b - centre) * sign)), q["id"]))
                covered.sort()
                spans, at = [], -half
                for a, b, other in covered:
                    if a - at > 1e-9:
                        spans.append((at, a, None))
                    spans.append((a, b, other))
                    at = b
                if half - at > 1e-9:
                    spans.append((at, half, None))
                for k, (a, b, other) in enumerate(spans):
                    id_ = f"{p['id']}-{side}" + (f"-{k}" if len(spans) > 1 else "")
                    edges[id_] = dict(id=id_, part=p["id"], side=side, facade=facade, span=(a, b), exposed=other is None,
                                      origin=(cx + normal[0] * (p["x1"] - p["x0"]) / 2, cy + normal[1] * (p["y1"] - p["y0"]) / 2),
                                      base=p["base"])
                    if other is not None:
                        joined[(p["id"], other, side)] = id_
        pairs = sorted({tuple(sorted((id_, joined[(b, a, OPPOSITE[side])]))) for (a, b, side), id_ in joined.items()})
        return edges, [dict(id=f"join-{k}", edges=list(pair)) for k, pair in enumerate(pairs)]

    def bays(self, edge):
        """The offsets of an edge's bays: where its windows, doors and garrison seats are."""
        if edge not in self.lattices:
            _fail(f"{self.id}: {edge} has no lattice yet")
        phase, (a, b) = self.lattices[edge], self.edges()[edge]["span"]
        first = math.floor((a - phase) / BAY_PITCH_M) + 1
        last = math.ceil((b - phase) / BAY_PITCH_M) - 1
        return [phase + k * BAY_PITCH_M for k in range(first, last + 1)]

    def at(self, edge, offset_m, z=0.0, out=0.0):
        """(x, y, z, yaw) of a wall-mounted module at `offset_m` along an edge, `out` metres proud of it."""
        e = self.edges()[edge]
        _, normal, along = SIDES[e["side"]]
        x = e["origin"][0] + along[0] * offset_m + normal[0] * out
        y = e["origin"][1] + along[1] * offset_m + normal[1] * out
        return x, y, e["base"] + z, math.atan2(normal[1], normal[0]) + math.pi / 2

    # -- the rows
    def place(self, module, x=0.0, y=0.0, z=0.0, yaw=0.0, scale=(1.0, 1.0, 1.0), tiers=EVERY_TIER, tint=WHITE,
              state="intact"):
        """One row: `module` scaled, turned about +Z, then moved, in the template's frame."""
        if module not in self.kit.modules:
            _fail(f"{self.id}: no module {module}")
        if min(scale) <= 0 or not 1 <= tiers <= 15 or any(c != int(c) or not 0 <= c <= 255 for c in tint):
            _fail(f"{self.id}: {module}: a row's scales are positive, its tiers 1 to 15 and its tint whole numbers to 255")
        self.rows.setdefault(state, []).append((module, x, y, z, yaw % (2 * math.pi), *scale, tiers, *tint))

    def mount(self, module, edge, offset_m, z=0.0, out=0.0, **row):
        x, y, z, yaw = self.at(edge, offset_m, z, out)
        self.place(module, x, y, z, yaw, **row)

    def descriptor(self):
        """The contract's `BuildingTemplateDescriptor`, in its own field order."""
        edges = self.edges()
        missing = sorted(id_ for id_, e in edges.items() if e["exposed"] and id_ not in self.lattices)
        if missing:
            _fail(f"{self.id}: exposed edges without a lattice: {missing}")
        if not self.floor_heights or not self.entrances:
            _fail(f"{self.id}: a complete template has floors and an entrance")
        for door in self.entrances:
            e = edges.get(door["edge"])
            if e is None or not e["exposed"] or not e["span"][0] < door["offset_m"] < e["span"][1]:
                _fail(f"{self.id}: {door['id']} is not inside an exposed edge")
            if e["side"] != edges[self.entrances[0]["edge"]]["side"]:
                _fail(f"{self.id}: {door['id']} is not on the street side (the first entrance's)")
        return dict(
            id=self.id, category=self.category, regional_family=self.family,
            parts=[dict(id=p["id"], center=[(p["x0"] + p["x1"]) / 2, (p["y0"] + p["y1"]) / 2], yaw=0.0,
                        half_extents=[(p["x1"] - p["x0"]) / 2, (p["y1"] - p["y0"]) / 2, (p["top"] - p["base"]) / 2],
                        base_z=p["base"]) for p in self.parts],
            floor_heights_m=self.floor_heights,
            entrances=self.entrances,
            edges=[dict(id=e["id"], part=e["part"], facade=e["facade"], span_m=list(e["span"]), exposed=e["exposed"],
                        bays=dict(pitch_m=BAY_PITCH_M, phase_m=self.lattices[id_]) if e["exposed"] else None)
                   for id_, e in edges.items()],
            joins=self._joins)


class Kit:
    def __init__(self, set_id, script, fit_side_m, fit_top_m):
        reset()
        self.set, self.script, self.fit = set_id, script, dict(side_m=fit_side_m, top_m=fit_top_m)
        self.modules, self.templates = {}, []

    def module(self, name, ground=False, **bake):
        """A new module. `bake` overrides what `parts.finish` is given (occlusion reach, paint edge)."""
        if not re.fullmatch(r"[a-z0-9_]+", name) or name in self.modules:
            _fail(f"module names are unique and [a-z0-9_]+: {name}")
        self.modules[name] = Module(name, ground, dict(ao_distance=2.5, ao_strength=0.5, ao_rays=10, paint_scale=3.0) | bake)
        return self.modules[name]

    def template(self, id_, category, family, recipe=None, status="release"):
        self.templates.append(Template(self, id_, category, family, recipe, status))
        return self.templates[-1]

    # -- writing
    def _bake(self):
        bpy.context.view_layer.update()
        for name in sorted(self.modules):
            m = self.modules[name]
            meshes = m.meshes()
            finish(objects=meshes, ground_ao=m.ground, **m.bake)
            m.triangles, points = [0, 0, 0, 0], []
            for o in meshes:
                o.data.calc_loop_triangles()
                tier = tier_of(o)
                for k in TIERS:
                    if tier is None or tier == k:
                        m.triangles[k] += len(o.data.loop_triangles)
                co = np.empty(len(o.data.vertices) * 3)
                o.data.vertices.foreach_get("co", co)
                points.append(np.c_[co.reshape(-1, 3), np.ones(len(co) // 3)] @ np.array(o.matrix_world).T[:, :3])
            if not all(m.triangles) or any(a < b for a, b in zip(m.triangles, m.triangles[1:])):
                _fail(f"module {name} needs geometry in every tier, never more in a coarser one: {m.triangles}")
            m.points = np.concatenate(points)

    def _check_fit(self, t):
        """Nothing a template draws reaches past its parts by more than the set's fit, or below one's base."""
        side, top = self.fit["side_m"], self.fit["top_m"]
        for state, rows in t.rows.items():
            for module, x, y, z, yaw, sx, sy, sz, *_ in rows:
                p = self.modules[module].points * (sx, sy, sz)
                c, s = math.cos(yaw), math.sin(yaw)
                wx, wy, wz = p[:, 0] * c - p[:, 1] * s + x, p[:, 0] * s + p[:, 1] * c + y, p[:, 2] + z
                inside = np.zeros(len(p), bool)
                for b in t.parts:
                    inside |= ((wx >= b["x0"] - side) & (wx <= b["x1"] + side) & (wy >= b["y0"] - side) & (wy <= b["y1"] + side)
                               & (wz >= b["base"] - 0.001) & (wz <= b["top"] + top))
                if not inside.all():
                    k = int(np.argmin(inside))
                    _fail(f"{t.id} ({state}): {module} at ({x:.2f}, {y:.2f}, {z:.2f}) reaches ({wx[k]:.3f}, {wy[k]:.3f}, {wz[k]:.3f}), "
                          f"outside every part grown by the fit {self.fit}")

    def write(self, out_dir=None):
        """Bake every module, hold every template to its boxes, and write `kit.glb` and `templates.json`."""
        out_dir = out_dir or os.path.join(REPO, "assets", "source", "city", self.set)
        self._bake()
        names = sorted(self.modules)
        index = {name: i for i, name in enumerate(names)}
        used = {row[0] for t in self.templates for rows in t.rows.values() for row in rows}
        if set(names) - used:
            _fail(f"modules no template places: {sorted(set(names) - used)}")
        templates = sorted(self.templates, key=lambda t: t.id)
        if len({t.id for t in templates}) != len(templates):
            _fail("template ids are unique")
        body = []
        for t in templates:
            descriptor = t.descriptor()
            self._check_fit(t)
            states = ",\n".join(
                f'        "{state}": [\n'
                + ",\n".join("          [" + ", ".join([str(index[r[0]])] + [_number(v) for v in r[1:]]) + "]" for r in sorted(rows))
                + "\n        ]" for state, rows in sorted(t.rows.items()))
            body.append("    {\n"
                        f'      "status": {json.dumps(t.status)},\n'
                        f'      "recipe": {json.dumps(t.recipe)},\n'
                        f'      "descriptor": {json.dumps(_rounded(descriptor))},\n'
                        f'      "states": {{\n{states}\n      }}\n'
                        "    }")
        text = ("{\n"
                f'  "set": {json.dumps(self.set)},\n'
                f'  "kit": {json.dumps("city_kit_" + self.set)},\n'
                f'  "fit": {json.dumps(self.fit)},\n'
                f'  "source": {json.dumps(dict(script=self.script, blender=bpy.app.version_string.split()[0]))},\n'
                f'  "modules": {json.dumps(names)},\n'
                '  "templates": [\n' + ",\n".join(body) + "\n  ]\n}\n")
        export(os.path.join(out_dir, "kit.glb"))
        with open(os.path.join(out_dir, "templates.json"), "w") as f:
            f.write(text)
        self._report(templates)

    def _report(self, templates):
        print(f"\nKIT {self.set}: {len(self.modules)} modules, {len(templates)} templates, fit {self.fit}")
        print(f"{'module':34} {'tier 0':>7} {'tier 1':>7} {'tier 2':>7} {'tier 3':>7} {'placed':>7}")
        for name in sorted(self.modules):
            placed = sum(row[0] == name for t in templates for row in t.rows["intact"])
            print(f"{name:34} " + " ".join(f"{n:7d}" for n in self.modules[name].triangles) + f" {placed:7d}")
        print(f"\n{'template (intact), triangles drawn':34} {'tier 0':>7} {'tier 1':>7} {'tier 2':>7} {'tier 3':>7} {'rows':>7}")
        for t in templates:
            drawn = [sum(self.modules[row[0]].triangles[k] for row in t.rows["intact"] if row[8] >> k & 1) for k in TIERS]
            print(f"{t.id:34} " + " ".join(f"{n:7d}" for n in drawn) + f" {len(t.rows['intact']):7d}")


def _number(v):
    """A row's number: an integer as one, anything else to a hundredth of a millimetre (or of a milliradian)."""
    if isinstance(v, int):
        return str(v)
    v = round(float(v), 5) + 0.0  # + 0.0: never "-0"
    return str(int(v)) if v == int(v) else repr(v)


def _rounded(value):
    """A descriptor's numbers to the micrometre, so a sum's rounding never reaches the file."""
    if isinstance(value, float):
        return round(value, 6) + 0.0
    if isinstance(value, dict):
        return {k: _rounded(v) for k, v in value.items()}
    if isinstance(value, (list, tuple)):
        return [_rounded(v) for v in value]
    return value


# ---------------------------------------------------------------- rows houses share
# A set that calls these has modules named `step`, `gutter` (a metre along +X) and
# `downpipe` (a metre up +Z), and chimney modules `CHIMNEY_M` tall.
def glaze(t, edge, floors, window, ground=None, skip=(), tint=WHITE, sill=SILL_M):
    """A window in every bay of `edge` on every floor. `ground` is the ground floor's
    window if it differs; `skip` are ground-floor offsets something else fills (a door)."""
    for k, floor in enumerate(floors):
        for o in t.bays(edge):
            if k == 0 and any(abs(o - s) < 1.2 for s in skip):
                continue
            t.mount(ground if k == 0 and ground else window, edge, o, z=floor + sill, tiers=TIERS_0_TO_2, tint=tint)


def front_door(t, edge, offset, door, tint):
    t.entrance(edge, offset)
    t.mount(door, edge, offset, z=0.0, tiers=TIERS_0_TO_2, tint=tint)
    t.mount("step", edge, offset, tiers=TIERS_0_TO_1)


def rainwater(t, edge, shape, length, pipes=(-1, 1), over=OVER_M):
    """A gutter under the eave over `edge` and a downpipe at each named end of it."""
    a, b = t.edges()[edge]["span"]
    mid = (a + b) / 2
    t.mount("gutter", edge, mid, z=shape.edge_z - 0.07, out=over + 0.05, scale=(length, 1.0, 1.0), tiers=TIERS_0_TO_1)
    for s in pipes:
        t.mount("downpipe", edge, mid + s * ((b - a) / 2 - 0.3), out=0.07, scale=(1.0, 1.0, shape.eave - 0.3), tiers=TIER_0)


def stack(t, module, x, y, ridge, yaw=0.0):
    """A chimney whose pots clear the ridge."""
    t.place(module, x, y, ridge + 0.5 - CHIMNEY_M, yaw, tiers=TIERS_0_TO_2)
