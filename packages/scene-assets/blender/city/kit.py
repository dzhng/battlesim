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

    t.place("home_10x8_1f_ruin", state="ruin")       # ... and the rows of its damage state
    kit.write()                                      # bake, check, write kit.glb and templates.json

A template has `intact` rows and the rows of the one state it is destroyed into
(`collapse.py`): `ruin`, held to the remains its parts leave (the same plans at
`Template.ruin_height()`, with the fit's `ruin_top_m` above them for the jagged tops
of broken walls), or `gutted`, held to the standing parts. Neither draws more
triangles than `intact` at any tier.

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
import random
import re
import sys

import bpy
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
from parts import *  # noqa: E402,F401,F403
from masonry import fill_height, ragged_wall, rubble, rubble_fill, scorched, wall_panels  # noqa: E402
import collapse  # noqa: E402

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

    # -- destroyed
    def damage_state(self):
        """The state this template is destroyed into: "ruin" or "gutted"."""
        return collapse.damage_state(len(self.floor_heights))

    def ruin_height(self):
        """How tall the remains of each of its parts are, once it has collapsed."""
        return collapse.ruin_height(max(p["top"] for p in self.parts))

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
    def __init__(self, set_id, script, fit_side_m, fit_top_m, fit_ruin_top_m=None):
        """`fit_ruin_top_m` is how far a ruin's broken walls may stand above its remains; a set with no ruin has none."""
        reset()
        self.set, self.script, self.fit = set_id, script, dict(side_m=fit_side_m, top_m=fit_top_m)
        if fit_ruin_top_m is not None:
            self.fit["ruin_top_m"] = fit_ruin_top_m
        self.modules, self.templates, self.heaps = {}, [], {}

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
        """Nothing a template draws reaches past its parts by more than the set's fit, or below one's base.
        A ruin is held to the remains: each part's plan at the ruin height."""
        side = self.fit["side_m"]
        for state, rows in t.rows.items():
            top = self.fit.get("ruin_top_m", 0.0) if state == "ruin" else self.fit["top_m"]
            parts = [p | dict(top=p["base"] + t.ruin_height()) for p in t.parts] if state == "ruin" else t.parts
            for module, x, y, z, yaw, sx, sy, sz, *_ in rows:
                p = self.modules[module].points * (sx, sy, sz)
                c, s = math.cos(yaw), math.sin(yaw)
                wx, wy, wz = p[:, 0] * c - p[:, 1] * s + x, p[:, 0] * s + p[:, 1] * c + y, p[:, 2] + z
                inside = np.zeros(len(p), bool)
                for b in parts:
                    inside |= ((wx >= b["x0"] - side) & (wx <= b["x1"] + side) & (wy >= b["y0"] - side) & (wy <= b["y1"] + side)
                               & (wz >= b["base"] - 0.001) & (wz <= b["top"] + top))
                if not inside.all():
                    k = int(np.argmin(inside))
                    _fail(f"{t.id} ({state}): {module} at ({x:.2f}, {y:.2f}, {z:.2f}) reaches ({wx[k]:.3f}, {wy[k]:.3f}, {wz[k]:.3f}), "
                          f"outside every part {'at the ruin height %.3f m ' % t.ruin_height() if state == 'ruin' else ''}"
                          f"grown by the fit {self.fit}")

    def drawn(self, t, state):
        """Triangles `t` draws in `state` at each tier."""
        return [sum(self.modules[row[0]].triangles[k] for row in t.rows[state] if row[8] >> k & 1) for k in TIERS]

    def _check_states(self, t):
        """A template has its damage state and no other, every state draws at every tier, and a damage
        state draws no more than the building did."""
        ends = t.damage_state()
        if sorted(t.rows) != sorted(("intact", ends)) or not t.rows[ends]:
            _fail(f"{t.id}: a template of {len(t.floor_heights)} floors has intact and {ends} rows, not {sorted(t.rows)}")
        intact, damaged = self.drawn(t, "intact"), self.drawn(t, ends)
        if not all(intact) or not all(damaged):
            _fail(f"{t.id}: every state draws at every tier: intact {intact}, {ends} {damaged}")
        if any(d > i for d, i in zip(damaged, intact)):
            _fail(f"{t.id}: its {ends} state draws {damaged} triangles, more than intact's {intact}")

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
            self._check_states(t)
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
            placed = sum(row[0] == name for t in templates for rows in t.rows.values() for row in rows)
            print(f"{name:34} " + " ".join(f"{n:7d}" for n in self.modules[name].triangles) + f" {placed:7d}")
        for title, states in (("intact", ("intact",)), ("destroyed", ("ruin", "gutted"))):
            print(f"\n{f'template ({title}), triangles drawn':34} {'tier 0':>7} {'tier 1':>7} {'tier 2':>7} {'tier 3':>7} {'rows':>7}")
            for t in templates:
                for state in states:
                    if state in t.rows:
                        name = t.id if state == "intact" else f"{t.id} {state}"
                        print(f"{name:34} " + " ".join(f"{n:7d}" for n in self.drawn(t, state)) + f" {len(t.rows[state]):7d}")


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


def fold_far(m, rows, panels, boxes, paint, lods, origin=(0.0, 0.0)):
    """What a shell keeps of its building's fittings where no row draws them: into module
    `m`, at tiers `lods`, each of `rows` (a template's rows, in its frame; `origin` is the
    module's place in it) as flat panels on its wall and as boxes. `panels` is module ->
    [(width, height, foot, paint)], in the fitting's own frame, where the paint is a
    material or "tint" for the row's own colour (`paint(tint)` makes that a material);
    `boxes` is module -> [(size, centre, material)]. A row's scale stretches its panels.
    A panel may add `proud`, how far off the wall it lies (a window's surround lies under
    its pane). So a far wall keeps its openings where they are, in their own colours and weight."""
    faces, count = {}, 0
    for module, x, y, z, yaw, sx, sy, sz, tiers, *tint in rows:
        x, y = x - origin[0], y - origin[1]
        for w, h, foot, mat, *proud in panels.get(module, ()):
            mat = paint(tuple(tint)) if mat == "tint" else mat
            faces.setdefault((mat.name, *proud), (mat, []))[1].append((x, y, z + foot * sz, yaw, w * sx, h * sz))
        c, s = math.cos(yaw), math.sin(yaw)
        for size, (cx, cy, cz), mat in boxes.get(module, ()):
            box(m.n(f"far_box_{lods[0]}_{count}"), size, (x + cx * c - cy * s, y + cx * s + cy * c, z + cz), mat, m.root, rot=(0, 0, yaw), lods=lods)
            count += 1
    for k, key in enumerate(sorted(faces)):
        mat, quads = faces[key]
        wall_panels(m.n(f"far_{lods[0]}_{k}"), quads, mat, m.root, proud=key[1] if len(key) > 1 else 0.03, lods=lods)


def window_far(w, h, glass, surround, shutters=None):
    """`fold_far`'s panels for a window `w` by `h` on its sill: its pane, the pale surround of its
    frame and sill under the pane, and its shutters ("tint", or None) either side."""
    out = [(w + 0.2, h + 0.19, -0.07, surround, 0.012), (w, h, 0.0, glass)]
    return ([(2 * w + 0.32, h + 0.04, -0.02, shutters, 0.02)] if shutters else []) + out


# ---------------------------------------------------------------- ruins masonry buildings share
# A collapsed house, barn or works is its own plan broken down to the remains' height:
# ragged stumps of its walls in its own material, sooted, their windows broken to the
# sills; the mass of the building lying inside them as one heap; pieces of its roof
# slipped over the heap; its chimneys' stumps. That is a module of the template's own
# (`ruin_block`). What lies about on it is rows of the set's shared wreckage (`litter`):
# heaps of rubble, charred beams. From far off the block is a low box in the wall's
# colour under a rubble top with a piece of roof on it.
RUIN_WALL_M = 0.3  # a broken wall's thickness
RUIN = dict(ao_distance=1.5, ao_strength=0.5, ao_rays=10, paint_scale=6.5)  # a ruin module's bake: no stump is split for paint
SIDE_AXIS = {"south": 0, "north": 0, "east": 1, "west": 1}  # the axis a side's wall runs along


def wreckage(kit, rubble_mat, beam_mat, beam="charred_beam", section=(0.18, 0.2)):
    """The wreckage a set's ruins share, as modules: `rubble_heap_s`, `_m` and `_l` (a row
    tints them the building's own masonry), and a beam of `section` (a house's charred
    timber, a shed's steel), a metre of it along +X that a row stretches (`<beam>`) or
    3 m of it fallen with one end up (`<beam>_fallen`)."""
    kit.beam = beam
    for name, radius, height, count, seed in (("rubble_heap_s", 0.7, 0.4, 7, 3), ("rubble_heap_m", 1.15, 0.6, 12, 5),
                                              ("rubble_heap_l", 1.8, 0.8, 18, 7)):
        m = kit.module(name, ground=True, **FITTING)
        rubble(m.n("heap"), (0, 0), radius, height, count, rubble_mat, seed, m.root, seg=(12, 8, 5, 4), rings=(3, 2, 1, 1))
        reach = tall = 0.0
        for o in m.meshes():  # a tumbled block rests on the ground, not in it
            for v in o.data.vertices:
                v.co.z = max(v.co.z, 0.0)
                reach, tall = max(reach, math.hypot(v.co.x, v.co.y)), max(tall, v.co.z)
        kit.heaps[name] = (reach, tall)  # how far its furthest block lies from its middle, and how high
    m = kit.module(beam, **FITTING)
    box(m.n("beam"), (1.0, *section), (0, 0, section[1] / 2), beam_mat, m.root)
    m = kit.module(beam + "_fallen", **FITTING)
    box(m.n("beam"), (3.0, *section), (0, 0, 0.42 + section[1] / 2), beam_mat, m.root, rot=(0, -0.3, 0))


def ruin_sides(t, part, openings, party=False):
    """What stands on each side of a part once it has fallen, for `ruin_block`: side ->
    dict(openings, gaps), in the template's frame. An exposed stretch keeps its wall, with
    an opening wherever an intact row hangs a ground-floor fitting named in `openings`
    (module -> its width); a stretch against a neighbour is open, or with `party` a wall."""
    p = next(p for p in t.parts if p["id"] == part)
    out = {}
    for e in t.edges().values():
        if e["part"] != part:
            continue
        axis = SIDE_AXIS[e["side"]]
        along = SIDES[e["side"]][2][axis]
        centre = ((p["x0"] + p["x1"]) / 2, (p["y0"] + p["y1"]) / 2)[axis]
        a, b = sorted((centre + along * e["span"][0], centre + along * e["span"][1]))
        side = out.setdefault(e["side"], dict(openings=[], gaps=[], walled=False))
        if e["exposed"] or party:
            side["walled"] = True
        else:
            side["gaps"].append(((a + b) / 2, b - a))
    face = dict(south=("y0", 1), north=("y1", 1), west=("x0", 0), east=("x1", 0))
    for module, x, y, z, *_ in t.rows["intact"]:
        if module not in openings or z - p["base"] > 1.6:
            continue
        for side, (key, coordinate) in face.items():
            lo, hi = (p["x0"], p["x1"]) if SIDE_AXIS[side] == 0 else (p["y0"], p["y1"])
            at = (x, y)[SIDE_AXIS[side]]
            if side in out and abs((x, y)[coordinate] - p[key]) < 1e-6 and lo < at < hi:
                out[side]["openings"].append((at, openings[module], max(0.12, z - p["base"])))
    return {side: dict(openings=sorted(v["openings"]), gaps=sorted(v["gaps"])) for side, v in sorted(out.items()) if v["walled"]}


def wall_line(rect, side, inset):
    """Where a wall `inset` inside a rectangle's `side` runs: (from, to, the line it is on, whether it runs along x)."""
    x0, x1, y0, y1 = rect
    along_x = SIDE_AXIS[side] == 0
    return (*((x0, x1) if along_x else (y0, y1)), dict(south=y0 + inset, north=y1 - inset, west=x0 + inset, east=x1 - inset)[side], along_x)


class Ruin:
    """One fallen box, as `ruin_block` built it: where its heap lies and how high."""

    def __init__(self, rect, inner, height, top, seed, sides):
        self.rect, self.inner, self.height, self.top, self.seed, self.sides = rect, inner, height, top, seed, sides
        self.fill = (inner[0] - 0.02, inner[1] + 0.02, inner[2] - 0.02, inner[3] + 0.02)  # the heap's own rectangle

    def heap(self, x, y):
        """The heap's height at (x, y), in the block's frame."""
        return fill_height(x, y, self.fill, self.top, self.seed)


def ruin_block(m, tag, rect, height, over, sides, wall, roof, rubble_mat, seed, stacks=(), level=1.0, charred=0.0, thick=RUIN_WALL_M,
               run=(0.3, 0.9), jagged=1.5, stump=None, cells=(1.3, 2.6, 8.0), coarse=6, roofing=True, far=None, far_wall=None):
    """One box of a collapsed building into module `m`, in the module's frame: `rect` is
    its plan (x0, x1, y0, y1), `height` the remains' height and `over` how far a broken
    wall may stand above it. `sides` is `ruin_sides`' (a side it lacks is open to a
    neighbour); `wall` and `roof` are the building's own materials, burnt here;
    `stacks` are its chimneys as (x, y, (w, d), material). Returns its `Ruin`.

    How it fell: `level` under 1 leaves this box lower than its neighbours, as a
    terrace's houses fall; `charred` is the least soot on its stumps (a boarded barn's
    burn black to the foot). Its walls are `thick`, in runs `run` wide whose tops walk
    `jagged` far, and stand no higher than `stump` (a shed's dado).
    How far off it holds: `cells` are the heap's at each tier, and `coarse` how many
    runs of a stump the third tier draws as one (both larger on a big building); `far`
    names the sides the coarsest tier draws (all that stand, unless a neighbour hides
    one) and `far_wall` its wall material when that is not the stumps'. `roofing` false
    leaves the roof to the caller, who then draws the coarsest tier's top too."""
    x0, x1, y0, y1 = rect
    high = height * level
    crest = high + over - 0.06  # the highest anything stands
    burnt = scorched(wall, high + over, seed, floor=charred)
    inner = [x0 + (thick if "west" in sides else 0.0), x1 - (thick if "east" in sides else 0.0),
             y0 + (thick if "south" in sides else 0.0), y1 - (thick if "north" in sides else 0.0)]
    for k, side in enumerate(("south", "north", "west", "east")):
        if side not in sides:
            continue
        _, _, fixed, along_x = wall_line(rect, side, thick / 2)
        a0, a1 = (x0, x1) if along_x else (inner[2], inner[3])  # a side wall runs between the front's and the back's
        ragged_wall(m.n(f"{tag}_{side}"), a0, a1, fixed, along_x, 0.3 * min(high, stump or high), min(crest, stump or crest),
                    seed * 31 + k, burnt, m.root, thick, gaps=sides[side]["gaps"], openings=sides[side]["openings"], lods=(0, 1, 2),
                    groups=(1, 3, coarse, 8), jagged=jagged, run=run)
    top = high
    ruin = Ruin(rect, inner, high, top, seed, sides)
    rubble_fill(m.n(f"{tag}_fill"), *ruin.fill, top, rubble_mat, seed, m.root, cells=cells)
    for k, (x, y, (w, d), mat) in enumerate(stacks):
        box(m.n(f"{tag}_stack_{k}"), (w, d, crest - 0.04), (x, y, (crest - 0.04) / 2), scorched(mat, high + over, seed + 1.0), m.root,
            lods=(0, 1, 2))
    # the roof, fallen in: pieces of it slipped over the heap
    rng = random.Random(seed * 131 + 7)
    span_x, span_y = inner[1] - inner[0], inner[3] - inner[2]
    fallen = scorched(roof, 3.0 * (high + over), seed + 2.0, floor=0.3) if roofing else None
    pieces = []
    for k in range(min(14, max(2, round(span_x * span_y / 20))) if roofing else 0):
        w, d = rng.uniform(2.2, 3.6), rng.uniform(1.6, 2.6)
        reach = 0.5 * math.hypot(w, d) + 0.2
        x = rng.uniform(inner[0] + min(reach, span_x / 2), inner[1] - min(reach, span_x / 2))
        y = rng.uniform(inner[2] + min(reach, span_y / 2), inner[3] - min(reach, span_y / 2))
        rot = (rng.uniform(-0.22, 0.22), rng.uniform(-0.22, 0.22), rng.uniform(0, math.pi))
        rise = abs(w / 2 * math.sin(rot[1])) + abs(d / 2 * math.sin(rot[0])) + 0.12
        z = min(ruin.heap(x, y) + 0.5 * rise, crest - rise)
        pieces.append(((w, d), (x, y, z), rot))
        box(m.n(f"{tag}_roof_{k}"), (w, d, 0.1), (x, y, z), fallen, m.root, rot=rot, lods=(0, 1))

    def far_roof(bm, lod):
        for (w, d), (x, y, z), rot in pieces[:max(2, len(pieces) // 2)] if lod == 2 else pieces[:1]:
            at = Matrix.Translation((x, y, z if lod == 2 else 0.8 * high + 0.04)) @ Matrix.Rotation(rot[2], 4, "Z")
            grow = 1.0 if lod == 2 else min(1.6, 0.45 * span_x / w, 0.45 * span_y / d) if min(span_x, span_y) > 4 else 1.0
            bm.faces.new([bm.verts.new(at @ Vector((sx * w / 2 * grow, sy * d / 2 * grow, 0))) for sx, sy in ((-1, -1), (1, -1), (1, 1), (-1, 1))])

    if roofing:
        mesh_part(m.n(f"{tag}_roof_far"), far_roof, fallen, m.root, lods=(2, 3))

    # from far off: a low box, its standing sides in the wall's colour, under a rubble top
    def far_sides(bm, lod):
        z = 0.8 * high
        for side in sorted(sides if far is None else far):
            a, b = dict(south=((x0, y0), (x1, y0)), north=((x1, y1), (x0, y1)), west=((x0, y1), (x0, y0)), east=((x1, y0), (x1, y1)))[side]
            bm.faces.new([bm.verts.new(v) for v in ((*a, 0), (*b, 0), (*b, z), (*a, z))])
        return bool(sides)

    def far_top(bm, lod):
        z = 0.8 * high
        bm.faces.new([bm.verts.new(v) for v in ((x0, y0, z), (x1, y0, z), (x1, y1, z), (x0, y1, z))])

    mesh_part(m.n(f"{tag}_far_sides"), far_sides, far_wall or burnt, m.root, lods=(3,))
    if roofing:
        mesh_part(m.n(f"{tag}_far_top"), far_top, rubble_mat, m.root, lods=(3,))
    return ruin


def litter(t, ruin, tint, at=(0.0, 0.0), beams=4, heaps=5, seed=0, level=1.0, long=5.0):
    """Rows of the set's wreckage over one fallen box, at the two fine tiers: charred beams
    lying on its heap, heaps of rubble on it, and more spilt out of its doorways and
    breaches a little way outside (inside the set's side fit). `at` is the block's frame
    in the template's, `level` the height its row scales it to; `tint` the masonry's colour."""
    rng = random.Random(ruin.seed * 977 + seed)
    ix0, ix1, iy0, iy1 = ruin.inner
    dx, dy = at
    crest = level * (ruin.height + t.kit.fit.get("ruin_top_m", 0.0))

    def lie(x, y):
        return level * ruin.heap(x, y)

    for k in range(beams):
        length = rng.uniform(2.4, min(long, max(2.6, 0.7 * max(ix1 - ix0, iy1 - iy0))))
        yaw = rng.uniform(0, math.pi)
        reach = length / 2 + 0.2
        if min(ix1 - ix0, iy1 - iy0) < 2 * reach:  # a narrow box: the beam lies along it
            yaw = (0.0 if ix1 - ix0 > iy1 - iy0 else math.pi / 2) + rng.uniform(-0.25, 0.25)
        rx = min(reach * abs(math.cos(yaw)) + 0.3, (ix1 - ix0) / 2)
        ry = min(reach * abs(math.sin(yaw)) + 0.3, (iy1 - iy0) / 2)
        x, y = rng.uniform(ix0 + rx, ix1 - rx), rng.uniform(iy0 + ry, iy1 - ry)
        if k % 3 == 2:
            z = min(lie(x, y) - 0.25, crest - 1.45)
            t.place(t.kit.beam + "_fallen", x + dx, y + dy, max(0.0, z), yaw, tiers=TIERS_0_TO_1, state="ruin")
        else:
            ends = [lie(x + s * length / 2 * math.cos(yaw), y + s * length / 2 * math.sin(yaw)) for s in (-1, 0, 1)]
            t.place(t.kit.beam, x + dx, y + dy, min(max(ends) - 0.06, crest - 0.4), yaw, scale=(length, 1.0, 1.0),
                    tiers=TIERS_0_TO_1, state="ruin")
    x0, x1, y0, y1 = ruin.rect
    side_m = t.kit.fit["side_m"]
    heaps_ = t.kit.heaps

    def heap(module, x, y, z, scale):
        t.place(module, x + dx, y + dy, min(z, crest - heaps_[module][1] * scale - 0.02), rng.uniform(0, 2 * math.pi),
                scale=(scale, scale, scale), tiers=TIERS_0_TO_1, tint=tint, state="ruin")

    # what poured out of each doorway, and out of a few breaches: as far outside as the fit allows
    breaches = [(side, centre) for side, wall in sorted(ruin.sides.items()) for centre, width, sill in wall["openings"] if sill < 0.3]
    for k in range(heaps // 2 if ruin.sides else 0):
        side = rng.choice(sorted(ruin.sides))
        lo, hi = (x0, x1) if SIDE_AXIS[side] == 0 else (y0, y1)
        breaches.append((side, rng.uniform(lo + 1.2, hi - 1.2)))
    for side, along in breaches:
        scale = rng.uniform(0.85, 1.1)
        out = side_m - 0.04 - heaps_["rubble_heap_s"][0] * scale
        nx, ny = SIDES[side][1]
        face = dict(south=y0, north=y1, west=x0, east=x1)[side]
        x, y = (along, face + ny * out) if SIDE_AXIS[side] == 0 else (face + nx * out, along)
        heap("rubble_heap_s", x, y, 0.0, scale)
    # and what lies on top of the heap
    for k in range(heaps):
        module = rng.choice(("rubble_heap_s", "rubble_heap_m", "rubble_heap_m", "rubble_heap_l"))
        scale = rng.uniform(0.8, 1.1)
        margin = heaps_[module][0] * scale + 0.1
        if ix1 - ix0 < 2 * margin or iy1 - iy0 < 2 * margin:
            continue
        x, y = rng.uniform(ix0 + margin, ix1 - margin), rng.uniform(iy0 + margin, iy1 - margin)
        heap(module, x, y, lie(x, y) - 0.2 * scale, scale)
