"""A generated airframe skin: the aircraft's outer shell made by an
image-to-3D model from a cleaned photo and its measured views, worn in place
of lofted body sections. The rest of the airframe (canopy, gear, stores,
pylons, antennas, markings) is still built here and placed on the skin.

A skin is pinned in `skins/<family>.json` beside this file: its GLB under
`assets/source/skins/<family>/`, its sha256, the endpoint and request that
made it and the images it was made from (committed beside it). `skin`
refuses a file whose hash is not the pinned one, so a model is only ever
built from the skin its receipt names.

The skin arrives in its generator's frame and scale. `skin` turns its nose to
+X (the fin's tip marks the tail), scales it to the airframe's length,
centres it and stands its highest point at `top`; it deletes the faces
inside the `cut` boxes (the canopy's hump, where the clear canopy goes), and
splits what is left into the parts the rest of the pipeline finds by name:
each face goes to the first of `parts` whose box holds its centre
(`wing_L`, `wing_R` for a wreck to fold one and insignia to land on,
`tail_fin_0` for the fin's lettering), the rest to `fuselage`. Each tier is
decimated once, whole and symmetrically, before the split, so the parts'
seams meet at every distance. Its shading is smooth with edges sharper than
`sharp_deg` kept hard (the wings' and fins' edges). A `Probe` on the built
shell seats the parts that sit on it (antennas, doors) where it is.
"""
import hashlib
import json
import math
import os
import sys
from pathlib import Path

import bmesh
import bpy
from mathutils import Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import mesh_part  # noqa: E402

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
PINS = HERE / "skins"
# Triangles per tier for the shell, about what a lofted airframe spends.
TRIS = (18000, 6000, 1300, 420)
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


def load(path, length, top):
    """The skin at `path` as one bmesh in the engine basis, `length` nose to
    tail, centred on X and Y, its highest point at `top`."""
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
    # The length lies along the longer horizontal axis, the fin's tip at the tail.
    axis = 0 if hi.x - lo.x >= hi.y - lo.y else 1
    tip = max((v.co for v in bm.verts), key=lambda c: c.z)
    tail_high = tip[axis] > (lo[axis] + hi[axis]) / 2
    turn = (math.pi if tail_high else 0.0) if axis == 0 else (math.pi / 2 if tail_high else -math.pi / 2)
    bm.transform(Matrix.Rotation(turn, 4, "Z"))
    lo, hi = _bounds(bm)
    centre = (lo + hi) / 2
    bm.transform(Matrix.Translation((0, 0, top)) @ Matrix.Scale(length / (hi.x - lo.x), 4) @
                 Matrix.Translation((-centre.x, -centre.y, -hi.z)))
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
        from aircraft_markings import _targets
        self.tree, self.world = _targets(prefixes), parent.matrix_world
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


def skin(path, length, top, mat, parent, parts=(), cut=(), tris=TRIS, sharp_deg=40):
    """Wear the skin pinned at `path` (see the module doc): `parts` are
    `(name, ((x0, x1), (y0, y1), (z0, z1)))`, `cut` the boxes whose faces
    go."""
    pinned(path)
    bm = load(path, length, top)
    gone = [f for f in bm.faces if any(_inside(b, f.calc_center_median()) for b in cut)]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    names = [n for n, _ in parts] + ["fuselage"]
    pieces = {}
    for lod, n in enumerate(tris):
        whole = _decimated(bm, n)
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
        for o in mesh_part(name, build, mat, parent, smooth=True):
            o.data.set_sharp_from_angle(angle=math.radians(sharp_deg))
    for me in pieces.values():
        bpy.data.meshes.remove(me)
