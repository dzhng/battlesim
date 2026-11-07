"""A generated airframe skin: the aircraft's outer shell made by an
image-to-3D model from shaded views of the airframe measured against its
references, worn in place of lofted body sections: the measured proportions,
blended into one smooth surface. The rest of the airframe (gear, stores,
pylons, nozzles, antennas, markings) is still built here and seated on it.

A skin is pinned in `skins/<family>.json` beside this file: its GLB under
`assets/source/skins/<family>/`, its sha256, the endpoint and request that
made it and the images it was made from (committed beside it). `skin`
refuses a file whose hash is not the pinned one, so a model is only ever
built from the skin its receipt names.

The skin arrives in its generator's frame and scale. `skin` turns its nose to
+X (the fin's tip marks the tail), scales it to the airframe's length,
centres it and stands its highest point at `top`, then corrects what the
generator got wrong against the measured outline (`lower`, `span`). A
`canopy` plan is fitted to the skin's own hump (`fit_canopy`); `jet` draws
the clear bubble canopy on those stations, and the hump under the glass is
pressed down so the cockpit shows through it. `press` boxes press other
skin down to their floors, `cut` boxes delete the faces of a generated part
a built one replaces (a nozzle). What is left splits into the parts the
rest of the pipeline finds by name: each face goes to the first of `parts`
whose box holds its centre (`wing_L`, `wing_R` for a wreck to fold one and
insignia to land on, `tail_fin_0` for the fin's lettering), the rest to
`fuselage`. Each tier is decimated once, whole and symmetrically, before the
split, so the parts' seams meet at every distance. Its shading is smooth
with edges sharper than `sharp_deg` kept hard (the wings' and fins' edges).
A `Probe` on the built shell seats the parts that sit on it (antennas,
doors) where it is.
"""
import hashlib
import json
import math
import os
import sys
from pathlib import Path

import bmesh
import bpy
import numpy as np
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import mesh_part  # noqa: E402
from vehicle_parts import MID  # noqa: E402

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
PINS = HERE / "skins"
# Triangles per tier for the shell, about what a lofted airframe spends.
TRIS = (26000, 7000, 1500, 450)
# The longest edge round a canopy's rim per tier, so its glass meets the skin cleanly.
RIM_EDGE = (0.08, 0.16, 0.4, 1.0)
# An intake mouth's grid per tier (across, up); the far tier has none.
MOUTH_GRID = ((7, 6), (3, 3), (2, 2))
# Every skin an export wore, path to sha256, for its receipt.
WORN = {}


def pinned(path):
    """The pin of the skin at `path` (repo-relative), its hash checked."""
    pins = {p["glb"]: p for f in sorted(PINS.glob("*.json")) for p in json.loads(f.read_text())}
    pin = pins.get(path)
    if pin is None:
        raise ValueError(f"{path} is not pinned in skins/")
    data = (REPO / path).read_bytes()
    if data.startswith(b"version https://git-lfs"):
        raise ValueError(f"{path} is an LFS pointer: pull it first (git lfs pull -I {path})")
    got = hashlib.sha256(data).hexdigest()
    if got != pin["sha256"]:
        raise ValueError(f"{path} has sha256 {got}, pinned {pin['sha256']}")
    WORN[path] = got
    return pin


def _bounds(bm):
    vs = [v.co for v in bm.verts]
    return (Vector((min(v.x for v in vs), min(v.y for v in vs), min(v.z for v in vs))),
            Vector((max(v.x for v in vs), max(v.y for v in vs), max(v.z for v in vs))))


NOSES = {"+x": 0.0, "-x": math.pi, "+y": -math.pi / 2, "-y": math.pi / 2}


def load(path, length, top, lower=None, span=None, x=0.0, nose=None):
    """The skin at `path` as one bmesh in the engine basis, `length` nose to
    tail, centred on Y and on `x` along its length (a family whose stations
    are not centred on its length), its highest point at `top`. `lower` is
    `(dz, z_from)` for a skin whose fins came out short, standing its body
    too high: everything up to `z_from` drops by `dz` and what is above it
    stretches, so the fins' tips stay at `top`. `span` is `(y_from,
    half_span)` for wings that came out short or long: outboard of `y_from`
    the skin stretches across until its tips reach `half_span`. `nose`
    ("+x", "-x", "+y" or "-y", in the generator's frame) says which way the
    nose points where the usual reading fails: the length along the longer
    side, the tail where the fin's tip is (a span longer than the length, a
    skin without fins)."""
    before = set(bpy.data.objects)
    bpy.ops.import_scene.gltf(filepath=str(REPO / path))
    new = [o for o in bpy.data.objects if o not in before]
    bpy.context.view_layer.update()
    bm = bmesh.new()
    for o in new:
        if o.type == "MESH":
            me = o.data.copy()
            me.transform(o.matrix_world)
            bm.from_mesh(me)
            bpy.data.meshes.remove(me)
    for o in new:
        bpy.data.objects.remove(o, do_unlink=True)
    # Its generator's UV charts go: export lays the paint's own box mapping
    # on a mesh without any, as on every lofted part.
    for layers in (bm.loops.layers.uv, bm.loops.layers.color):
        while len(layers):
            layers.remove(layers[0])
    lo, hi = _bounds(bm)
    if nose is not None:
        turn = NOSES[nose]
    else:
        # The length lies along the longer horizontal axis, the fin's tip at the tail.
        axis = 0 if hi.x - lo.x >= hi.y - lo.y else 1
        tip = max((v.co for v in bm.verts), key=lambda c: c.z)
        tail_high = tip[axis] > (lo[axis] + hi[axis]) / 2
        turn = (math.pi if tail_high else 0.0) if axis == 0 else (math.pi / 2 if tail_high else -math.pi / 2)
    bm.transform(Matrix.Rotation(turn, 4, "Z"))
    lo, hi = _bounds(bm)
    centre = (lo + hi) / 2
    bm.transform(Matrix.Translation((x, 0, top)) @ Matrix.Scale(length / (hi.x - lo.x), 4) @
                 Matrix.Translation((-centre.x, -centre.y, -hi.z)))
    if span is not None:
        y_from, half = span
        k = (half - y_from) / (max(abs(v.co.y) for v in bm.verts) - y_from) - 1
        for v in bm.verts:
            if abs(v.co.y) > y_from:
                v.co.y += (v.co.y - math.copysign(y_from, v.co.y)) * k
    if lower is not None:
        dz, z_from = lower
        for v in bm.verts:
            v.co.z -= dz if v.co.z <= z_from else dz * (top - v.co.z) / (top - z_from)
    bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=1e-5)
    return bm


def _inside(box, p):
    return all(lo <= c <= hi for (lo, hi), c in zip(box, p))


def _decimated(bm, tris):
    """A copy of `bm` collapsed to about `tris` triangles, symmetric across Y."""
    me = bpy.data.meshes.new("skin_decimate")
    bm.to_mesh(me)
    o = bpy.data.objects.new("skin_decimate", me)
    bpy.context.scene.collection.objects.link(o)
    mod = o.modifiers.new("decimate", "DECIMATE")
    mod.ratio = min(1.0, tris / max(1, sum(len(f.verts) - 2 for f in bm.faces)))
    mod.use_symmetry, mod.symmetry_axis = True, "Y"
    ev = o.evaluated_get(bpy.context.evaluated_depsgraph_get())
    out = bmesh.new()
    out.from_mesh(ev.to_mesh())
    ev.to_mesh_clear()
    bpy.data.objects.remove(o, do_unlink=True)
    bpy.data.meshes.remove(me)
    return out


class Probe:
    """An airframe's built shell (the near tier of the parts `prefixes`
    names), in `parent`'s frame, for seating parts on it."""

    def __init__(self, parent, prefixes=("fuselage", "wing_")):
        from aircraft_markings import near_shell
        self.tree, self.world = near_shell(prefixes), parent.matrix_world
        self.local = self.world.inverted_safe()

    def _z(self, x, y, z0, dz):
        hit = self.tree.ray_cast(self.world @ Vector((x, y, z0)), self.world.to_3x3() @ Vector((0, 0, dz)), 40)[0]
        return None if hit is None else (self.local @ hit).z

    def under(self, x, y):
        """The underside's height under (x, y), stepping inboard if a thin edge misses."""
        for k in range(12):
            z = self._z(x, y - math.copysign(0.05 * k, y), -10, 1)
            if z is not None:
                return z
        raise ValueError(f"no skin under ({x}, {y})")

    def over(self, x, y):
        """The top's height over (x, y)."""
        z = self._z(x, y, 20, -1)
        if z is None:
            raise ValueError(f"no skin over ({x}, {y})")
        return z

    def chord(self, y, front, back, step=0.05):
        """The (leading, trailing) edges' x at span `y`, looking down on the
        skin between `front` and `back` (to keep another surface out)."""
        xs = [front - k * step for k in range(int((front - back) / step) + 1)
              if self._z(front - k * step, y, 20, -1) is not None]
        if not xs:
            raise ValueError(f"no skin at span {y} between x {front} and {back}")
        return max(xs), min(xs)

    def along(self, y, z, ahead=True):
        """Where the skin is first met along X at (y, z): from ahead (its
        front there, a nose or a lip) or from behind (its back, a tail)."""
        d = -1 if ahead else 1
        hit = self.tree.ray_cast(self.world @ Vector((-d * 30, y, z)), self.world.to_3x3() @ Vector((d, 0, 0)), 60)[0]
        if hit is None:
            raise ValueError(f"no skin along x at ({y}, {z})")
        return (self.local @ hit).x

    def flank(self, x, y, z):
        """The side's reach at (x, z), on the side `y` is on, seen from outboard."""
        out = math.copysign(1.0, y)
        hit = self.tree.ray_cast(self.world @ Vector((x, out * 8.0, z)), self.world.to_3x3() @ Vector((0, -out, 0)),
                                 16.0)[0]
        if hit is None:
            raise ValueError(f"no skin beside ({x}, {y}, {z})")
        return (self.local @ hit).y


def intake_mouth(name, shell, mat, parent, side, corners, lift=0.012):
    """The dark mouth of an intake on the skin's own intake face: the quad
    `corners` (`(y, z)` inboard low, outboard low, outboard high, inboard
    high, on the left; `side` -1 mirrors it) as a grid on the face's plane:
    fitted to where rays from ahead meet the skin (`shell`, a `Probe`), then
    stood `lift` ahead of the most forward of them, so it follows a raked or
    swept face and covers whatever the generator drew inside the lips (a
    grille's slats)."""
    c = [Vector((0, y, z)) for y, z in corners]

    def build(bm, lod):
        nu, nv = MOUTH_GRID[lod]
        grid = []
        for j in range(nv + 1):
            for i in range(nu + 1):
                p = c[0].lerp(c[1], i / nu).lerp(c[3].lerp(c[2], i / nu), j / nv)
                hit = shell.tree.ray_cast(shell.world @ Vector((20, side * p.y, p.z)),
                                          shell.world.to_3x3() @ Vector((-1, 0, 0)), 40)[0]
                if hit is None:
                    raise ValueError(f"{name}: no intake face ahead of ({side * p.y:.2f}, {p.z:.2f})")
                grid.append((side * p.y, p.z, (shell.local @ hit).x))
        pts = np.array(grid)
        basis = np.column_stack([np.ones(len(pts)), pts[:, 0], pts[:, 1]])
        plane = np.linalg.lstsq(basis, pts[:, 2], rcond=None)[0]
        fitted = basis @ plane
        ahead = float((pts[:, 2] - fitted).max()) + lift
        verts = [bm.verts.new((x + ahead, y, z)) for (y, z, _), x in zip(grid, fitted)]
        rows = [verts[j * (nu + 1):(j + 1) * (nu + 1)] for j in range(nv + 1)]
        for j in range(nv):
            for i in range(nu):
                quad = [rows[j][i], rows[j][i + 1], rows[j + 1][i + 1], rows[j + 1][i]]
                bm.faces.new(quad if side > 0 else quad[::-1])

    mesh_part(name, build, mat, parent, lods=MID)


def fit_canopy(bm, plan, lift=0.015, seat=0.02):
    """A canopy's stations `(x, half width, sill, top)` from its `plan`
    (`(x, half width)` front to back), measured on the skin's own hump: its
    top over the centreline stood `lift` proud, its sill `seat` below where
    the hump's surface is at the glass's half width, so the glass's edge
    sits into the skin rather than on it."""
    tree = BVHTree.FromBMesh(bm)
    out = []
    for x, hw in plan:
        top = tree.ray_cast(Vector((x, 0, 20)), Vector((0, 0, -1)), 40)[0]
        sill = tree.ray_cast(Vector((x, hw, 20)), Vector((0, 0, -1)), 40)[0]
        if top is None or sill is None:
            raise ValueError(f"the canopy's station at x {x} is off the skin")
        out.append((x, hw, min(sill.z, top.z) - seat, top.z + lift))
    return out


def _under_canopy(bm, stations, edge, under=0.03, grow=1.12):
    """Press the hump down under a fitted canopy (`fit_canopy`'s stations):
    every vertex above the sill within the glass's half width (a little
    grown) goes to `under` below its sill, in it or poking through it, so
    the cockpit shows through the glass and nothing of the hump stands
    proud of it. The skin round the glass is first split to edges no longer
    than `edge`, so the line where they meet is not ragged."""
    front, back = stations[0][0], stations[-1][0]
    reach = max(st[1] for st in stations) * 1.6
    floor = min(st[2] for st in stations) - 0.1

    def near(c):
        return back - 0.1 <= c.x <= front + 0.1 and abs(c.y) < reach and c.z > floor
    for _ in range(8):
        long_ = [e for e in bm.edges if e.calc_length() > edge and all(near(v.co) for v in e.verts)]
        if not long_:
            break
        bmesh.ops.subdivide_edges(bm, edges=long_, cuts=1, use_grid_fill=True)
        bmesh.ops.triangulate(bm, faces=[f for f in bm.faces if len(f.verts) > 3])
    for v in bm.verts:
        x, y, z = v.co
        if not back <= x <= front:
            continue
        for a, b in zip(stations, stations[1:]):
            if b[0] <= x <= a[0]:
                t = (a[0] - x) / max(a[0] - b[0], 1e-9)
                hw, sill = (p + (q - p) * t for p, q in zip(a[1:3], b[1:3]))
                break
        if z > sill and abs(y) < hw * grow:
            v.co.z = sill - under


def skin(path, length, top, mats, parent, parts=(), press=(), cut=(), tris=TRIS, sharp_deg=40, lower=None, span=None,
         x=0.0, nose=None, canopy=None):
    """Wear the skin pinned at `path` (see the module doc), in the airframe's
    `mats["paint"]`: `parts` are `(name, ((x0, x1), (y0, y1), (z0, z1)))`;
    `press` are boxes whose skin is pressed to their floor (a
    generated lump flattened), or `(box, z)` pressed to `z` (a plate raised),
    `cut` those whose faces go; `lower`, `span`, `x` and `nose` are
    `load`'s. `canopy` is `dict(glass=plan, tail=plan)`, the clear canopy's
    plan and its opaque tail's, front to back (`fit_canopy`): their stations
    are measured on the hump (after `press`), which is then pressed under
    them, and returned as `(glass, tail)` for `aircraft_canopy.bubble_canopy`."""
    pinned(path)
    bm = load(path, length, top, lower, span, x, nose)
    for entry in press:
        box, z = (entry, entry[2][0]) if len(entry) == 3 else entry
        for v in bm.verts:
            if _inside(box, v.co):
                v.co.z = z
    fitted = stations = None
    if canopy is not None:
        stations = fit_canopy(bm, list(canopy["glass"]) + list(canopy["tail"])[1:])
        fitted = (stations[:len(canopy["glass"])], stations[len(canopy["glass"]) - 1:])
    gone = [f for f in bm.faces if any(_inside(b, f.calc_center_median()) for b in cut)]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    names = [n for n, _ in parts] + ["fuselage"]
    pieces = {}
    for lod, n in enumerate(tris):
        whole = _decimated(bm, n)
        if stations is not None:
            _under_canopy(whole, stations, RIM_EDGE[lod])
        whole.faces.ensure_lookup_table()
        owner = [next((i for i, (_, box) in enumerate(parts) if _inside(box, f.calc_center_median())), len(parts))
                 for f in whole.faces]
        for i, name in enumerate(names):
            piece = whole.copy()
            piece.faces.ensure_lookup_table()
            bmesh.ops.delete(piece, geom=[f for f, o in zip(piece.faces, owner) if o != i], context="FACES")
            me = bpy.data.meshes.new(f"skin_{name}_{lod}")
            piece.to_mesh(me)
            piece.free()
            me.validate()
            pieces[name, lod] = me
        whole.free()
    bm.free()
    for name in names:
        def build(b, lod, name=name):
            me = pieces[name, lod]
            if not me.polygons:
                return False
            b.from_mesh(me)
        for o in mesh_part(name, build, mats["paint"], parent, smooth=True):
            o.data.set_sharp_from_angle(angle=math.radians(sharp_deg))
    for me in pieces.values():
        bpy.data.meshes.remove(me)
    return fitted
