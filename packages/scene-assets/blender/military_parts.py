"""What the military bodies' exporters share (`military.py`): the registry of kinds, the
model's bodies, and the bars, tubes and sheets they are built from."""
import bmesh, math
from mathutils import Vector

from parts import *
import textures_military  # noqa: F401 (registers the military recipes)

KINDS = {}
ROOT = None  # the model's root node, which the exporter makes before it builds a kind


def kind(fn):
    """Register `fn` as the kind of its name: it builds the model and returns its box's
    half extents."""
    KINDS[fn.__name__] = fn
    return fn


def body(name):
    """A body of the model: a node under the root whose parts the lint keeps clear of
    every other body's."""
    return empty(name, parent=ROOT)


def tube(name, a, b, r, mat, parent, lods=TIERS, seg=8):
    """A round bar from `a` to `b`; four-sided on the far tiers."""
    a, b = Vector(a), Vector(b)
    d = b - a
    rot = Vector((0, 0, 1)).rotation_difference(d.normalized()).to_euler()
    return cyl(name, r, d.length, (a + b) / 2, "Z", mat, parent, seg=seg, rot=rot, lods=lods, min_seg=4)


def bar(name, a, b, w, h, mat, parent, lods=TIERS):
    """A square-section member `w` wide and `h` deep from `a` to `b`, its depth upright
    when the run is level (an angle iron, a beam)."""
    a, b = Vector(a), Vector(b)
    d = b - a
    yaw = math.atan2(d.y, d.x)
    pitch = -math.atan2(d.z, math.hypot(d.x, d.y))
    return box(name, (d.length, w, h), (a + b) / 2, mat, parent, rot=(0, pitch, yaw), lods=lods)


def grid(bm, nu, nv, at, uv=None, flip=False):
    """Into `bm`: a sheet of `nu` by `nv` quads whose vertex (i, j) is `at(i / nu, j / nv)`,
    facing along the cross product of its u and v directions, or the other way when `flip`.
    `uv(u, v)` gives each corner its own texture coordinate. Returns the vertex rows."""
    layer = bm.loops.layers.uv.verify() if uv else None
    rows = [[bm.verts.new(at(i / nu, j / nv)) for j in range(nv + 1)] for i in range(nu + 1)]
    for i in range(nu):
        for j in range(nv):
            corners = ((i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1))
            corners = corners[::-1] if flip else corners
            f = bm.faces.new([rows[a][b] for a, b in corners])
            if layer:
                for loop, (a, b) in zip(f.loops, corners):
                    loop[layer].uv = uv(a / nu, b / nv)
    return rows


def solid_sheet(bm, nu, nv, at, keep=lambda i, j: True, thick=0.03, uv=None):
    """Into `bm`: a sheet of `nu` by `nv` quads (`at(u, v)`, as `grid`), given a back face
    `thick` below its front so a torn edge shows no hole through it. `keep(i, j)` says which
    quads stand (a tear drops the rest)."""
    layer = bm.loops.layers.uv.verify() if uv else None
    for side, off in ((0, 0.0), (1, -thick)):
        rows = [[bm.verts.new(Vector(at(i / nu, j / nv)) + Vector((0, 0, off))) for j in range(nv + 1)]
                for i in range(nu + 1)]
        for i in range(nu):
            for j in range(nv):
                if not keep(i, j):
                    continue
                corners = ((i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1))
                f = bm.faces.new([rows[a][b] for a, b in (corners if side == 0 else corners[::-1])])
                if layer:
                    for loop, (a, b) in zip(f.loops, corners if side == 0 else corners[::-1]):
                        loop[layer].uv = uv(a / nu, b / nv)
