"""Reading a vendored geometry-nodes building before it is realized.

A graph building is a modifier on one object. Evaluated as saved, it is one
realized mesh; with its Realize Instances nodes muted (in memory, never saved)
it is what the exporter wants: the instances of its kit meshes, each with a
transform and the attributes the graph stored on it, beside the mesh the graph
generated for this recipe alone.

`Soup` is how every mesh travels from there to the GLB: triangles over
vertices that are never shared between two materials (the renderer tags
material per vertex), a colour multiplier per vertex, and the source material's
name per triangle. Positions are metres, Z up.
"""
import hashlib

import bpy
import numpy as np


def open_blend(path):
    bpy.ops.wm.open_mainfile(filepath=path, load_ui=False)


def modifier(object_name, group_name):
    ob = bpy.data.objects[object_name]
    return ob, next(m for m in ob.modifiers if m.type == "NODES" and m.node_group and m.node_group.name == group_name)


def set_inputs(mod, values):
    """Set modifier inputs by their interface names (Blender 5.2: `properties.inputs`).
    Every input the graph has must be named: the files are saved in states that are
    not their defaults, and nothing clamps a value set from Python."""
    sockets = {it.name: it for it in mod.node_group.interface.items_tree
               if it.item_type == "SOCKET" and it.in_out == "INPUT"}
    missing = sorted(set(sockets) - set(values))
    unknown = sorted(set(values) - set(sockets))
    if missing or unknown:
        raise SystemExit(f"{mod.node_group.name}: inputs not set {missing}, inputs it does not have {unknown}")
    for name, value in values.items():
        it = sockets[name]
        kind = {"NodeSocketFloat": float, "NodeSocketInt": int}.get(it.socket_type)
        getattr(mod.properties.inputs, it.identifier).value = kind(value) if kind else value


class Soup:
    """Triangles: `v` (n, 3) positions, `c` (n, 3) colour multipliers, `t` (m, 3)
    vertex indices, `m` (m,) indices into `mats` (source material names), `s` (m,)
    smooth flags."""

    def __init__(self, v, c, t, m, s, mats):
        self.v = np.asarray(v, dtype=np.float64).reshape(-1, 3)
        self.c = np.asarray(c, dtype=np.float64).reshape(-1, 3)
        self.t = np.asarray(t, dtype=np.int64).reshape(-1, 3)
        self.m = np.asarray(m, dtype=np.int64).reshape(-1)
        self.s = np.asarray(s, dtype=bool).reshape(-1)
        self.mats = list(mats)

    @staticmethod
    def empty():
        return Soup(np.zeros((0, 3)), np.zeros((0, 3)), np.zeros((0, 3), int), [], [], [])

    def __len__(self):
        return len(self.t)

    def material_names(self):
        return [self.mats[i] for i in self.m]

    def keep(self, mask):
        """The triangles `mask` selects, with the vertices they use."""
        mask = np.asarray(mask, dtype=bool)
        t = self.t[mask]
        used = np.unique(t)
        remap = np.full(len(self.v), -1, dtype=np.int64)
        remap[used] = np.arange(len(used))
        return Soup(self.v[used], self.c[used], remap[t], self.m[mask], self.s[mask], self.mats)

    def without(self, materials):
        drop = np.array([name in materials for name in self.mats], dtype=bool)
        return self.keep(~drop[self.m]) if len(self) else self

    def transformed(self, matrix):
        """Moved by a 4x4; a mirroring matrix turns the winding back."""
        a = np.asarray(matrix, dtype=np.float64)
        v = self.v @ a[:3, :3].T + a[:3, 3]
        t = self.t[:, ::-1] if np.linalg.det(a[:3, :3]) < 0 else self.t
        return Soup(v, self.c, t, self.m, self.s, self.mats)

    def coloured(self, rgb):
        return Soup(self.v, np.broadcast_to(np.asarray(rgb, dtype=np.float64), self.c.shape).copy(), self.t, self.m, self.s,
                    self.mats)

    def normals(self):
        a, b, c = (self.v[self.t[:, i]] for i in range(3))
        n = np.cross(b - a, c - a)
        length = np.linalg.norm(n, axis=1, keepdims=True)
        return n / np.where(length > 1e-20, length, 1.0), 0.5 * length[:, 0]

    def bounds(self):
        return self.v.min(0), self.v.max(0)

    @staticmethod
    def join(soups):
        soups = [s for s in soups if len(s)]
        if not soups:
            return Soup.empty()
        mats = sorted({name for s in soups for name in s.mats})
        index = {name: i for i, name in enumerate(mats)}
        v, c, t, m, sm, base = [], [], [], [], [], 0
        for s in soups:
            v.append(s.v)
            c.append(s.c)
            t.append(s.t + base)
            m.append(np.array([index[name] for name in s.mats], dtype=np.int64)[s.m])
            sm.append(s.s)
            base += len(s.v)
        return Soup(np.concatenate(v), np.concatenate(c), np.concatenate(t), np.concatenate(m), np.concatenate(sm), mats)


def read_mesh(me, colour="Col", tinted=frozenset()):
    """A Blender mesh as a soup. Its colour attribute is the China kit's: rgb is a
    tint and alpha says how much of it a surface takes, so the multiplier is
    mix(1, rgb, alpha) on the materials in `tinted` and 1 elsewhere. The alpha never
    travels further: in our bundles a vertex colour's alpha means wear."""
    n = len(me.vertices)
    co = np.empty(n * 3, dtype=np.float32)
    me.vertices.foreach_get("co", co)
    co = co.reshape(n, 3).astype(np.float64)
    col = np.ones((n, 3))
    attr = me.attributes.get(colour)
    if attr is not None and attr.domain == "POINT" and n:
        raw = np.empty(n * 4, dtype=np.float32)
        attr.data.foreach_get("color", raw)
        raw = raw.reshape(n, 4).astype(np.float64)
        col = 1.0 + raw[:, 3:4] * (raw[:, :3] - 1.0)
    me.calc_loop_triangles()
    k = len(me.loop_triangles)
    tri = np.empty(k * 3, dtype=np.int32)
    me.loop_triangles.foreach_get("vertices", tri)
    tri = tri.reshape(k, 3).astype(np.int64)
    mat = np.empty(k, dtype=np.int32)
    me.loop_triangles.foreach_get("material_index", mat)
    smooth = np.empty(k, dtype=bool)
    me.loop_triangles.foreach_get("use_smooth", smooth)
    names = [m.name if m else "" for m in me.materials] or [""]
    mat = np.minimum(mat, len(names) - 1)
    # one vertex per (source vertex, material): no vertex is shared across a material seam
    key = tri * len(names) + mat[:, None]
    used, inverse = np.unique(key, return_inverse=True)
    source = used // len(names)
    takes = np.array([name in tinted for name in names], dtype=bool)[used % len(names)]
    c = np.where(takes[:, None], col[source], 1.0)
    return Soup(co[source], c, inverse.reshape(k, 3), mat, smooth, names)


class Tap:
    """One evaluation: `own` is the mesh the graph generated for this recipe, `rows`
    its instances as (reference name, 4x4 matrix, the graph's `_tint` rgba or None)
    and `meshes` the instanced meshes by reference name. A generated primitive (a
    cube the graph stretches into trim) has no kit object behind it; it is named by
    its material, `primitive:<material>`."""

    def __init__(self, own, rows, meshes):
        self.own, self.rows, self.meshes = own, rows, meshes


def tap(ob, tinted=frozenset(), tint_attribute="_tint", shaped=False):
    """`shaped` names a generated primitive by its shape too (`primitive:<material>~<hash>`),
    for a graph that stretches several different primitives in one material."""
    ob.update_tag()
    bpy.context.view_layer.update()
    dg = bpy.context.evaluated_depsgraph_get()
    dg.update()
    gs = ob.evaluated_get(dg).evaluated_geometry()
    own = read_mesh(gs.mesh, tinted=tinted) if gs.mesh is not None else Soup.empty()
    rows, meshes = [], {}
    _instances(gs, np.eye(4), None, tinted, tint_attribute, shaped, rows, meshes)
    return Tap(own, rows, meshes)


def _instances(gs, parent, parent_tint, tinted, tint_attribute, shaped, rows, meshes):
    """Every leaf instance under `gs`, through instances of instances (a collection
    is instanced as one reference holding its objects)."""
    pc = gs.instances_pointcloud()
    n = len(pc.points) if pc is not None else 0
    if not n:
        return
    index = np.empty(n, dtype=np.int32)
    pc.attributes[".reference_index"].data.foreach_get("value", index)
    matrix = np.empty(n * 16, dtype=np.float32)
    pc.attributes["instance_transform"].data.foreach_get("value", matrix)
    # Blender stores a 4x4 attribute column-major
    matrix = matrix.reshape(n, 4, 4).transpose(0, 2, 1).astype(np.float64)
    tint = None
    if tint_attribute in pc.attributes:
        tint = np.empty(n * 4, dtype=np.float32)
        pc.attributes[tint_attribute].data.foreach_get("color", tint)
        tint = tint.reshape(n, 4).astype(np.float64)
    names = []
    refs = gs.instance_references()
    for r in refs:
        if isinstance(r, bpy.types.Object) and r.type == "MESH":
            me = r.data  # a kit object instanced whole (Object Info as an instance): no instances under it
        elif hasattr(r, "instances_pointcloud"):
            me = r.mesh
        else:
            raise SystemExit(f"instance reference {r!r} is neither a geometry set nor a mesh object")
        if me is None or not len(me.vertices):
            names.append(None)  # nested instances only, or the empty geometry of a switch that is off
            continue
        name = r.name or "primitive:" + "+".join(sorted(m.name for m in me.materials if m))
        soup = read_mesh(me, tinted=tinted)
        if shaped and not r.name:
            name += "~" + hashlib.sha1(np.round(soup.v, 4).tobytes() + soup.t.tobytes()).hexdigest()[:8]
        known = meshes.get(name)
        if known is not None and (len(known.v) != len(soup.v) or not np.allclose(known.v, soup.v, atol=1e-5)):
            raise SystemExit(f"two different meshes are instanced under the name {name}")
        meshes[name] = soup
        names.append(name)
    for i in range(n):
        world = parent @ matrix[i]
        t = tint[i] if tint is not None and tint[i, 3] > 0.5 else parent_tint
        if names[index[i]] is not None:
            rows.append((names[index[i]], world, t))
        if hasattr(refs[index[i]], "instances_pointcloud"):
            _instances(refs[index[i]], world, t, tinted, tint_attribute, shaped, rows, meshes)
