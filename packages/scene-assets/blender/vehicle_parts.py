"""The vehicle part kernel: the contracts every vehicle part keeps, the tiers,
and the placement, spinning-node and tube helpers the part modules share.

The parts live beside it, one module per concept (the folder's README names
them).

Every part is a pure Blender function built on `parts.py`: it reads no catalog
and writes no file. It takes a unique `name`, a placement in its parent's
frame (`loc`, and `rot` as XYZ Euler radians where the part can turn), its
sizes in metres, a material set `mats` (a dict of role -> material) and a
`parent` object.

Contracts every part keeps:

- Tiers: each piece is a `parts.py` primitive made per tier (`<name>_<piece>_LOD<n>`).
  Fine detail drops out at tiers 1-3 and the coarse silhouette stays, so a
  vehicle built from these parts has fewer triangles at each tier.
- Materials: each material is its own object (a tyre and its rim are two),
  because box UVs take the tile of an object's first material only; so each
  surface keeps its own texel scale. A part reads only the roles its
  docstring names. To give one part another colour (a jerrycan's green), pass
  it its own dict.
- Spinning nodes: wheel-like parts make an empty named `name` (which must
  start with `wheel_`) at `loc`, unrotated, carrying `radius_m`; the renderer
  turns it about its local Y. Their meshes are its direct children and they
  return that empty. A track's node is `running_gear`'s (`track_loop`).
- Static parts make no empty (every empty is an articulated node in the
  bundle); their meshes hang straight off `parent`, placed by `loc` and `rot`,
  and they return the list of mesh objects they made.

Roles used across the parts: 'paint' (the vehicle's scheme), 'dark' (dark
painted running gear and fittings), 'steel' (bare or worn metal), 'black'
(bores, recesses and openings), 'rubber', 'glass' (dark optics), 'lamp'
(lamp lenses), 'canvas' and 'track' (the belt's link texture).
"""
import math
import os
import sys

import bmesh
from mathutils import Euler, Matrix, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import empty, mesh_part  # noqa: E402

FINE = (0,)
NEAR = (0, 1)
MID = (0, 1, 2)
ALL = (0, 1, 2, 3)
# Sides of a swept tube (a hook, a guard bar, a weld bead) per tier.
TUBE_SIDES = (8, 6, 4, 4)


def place(objects, loc, rot):
    """Move objects built in a part's own frame into its parent's frame."""
    frame = Matrix.LocRotScale(Vector(loc), Euler(rot), None)
    for o in objects:
        o.matrix_basis = frame @ o.matrix_basis
    return objects


def wheel_node(name, loc, radius, parent):
    """The empty the renderer spins: named `wheel_*`, unrotated, with its radius."""
    if not name.startswith("wheel_"):
        raise ValueError(f"a spinning part's node must be named wheel_*: {name}")
    return empty(name, loc=loc, parent=parent, props={"radius_m": radius})


def _tube(bm, path, radius, sides):
    """Into `bm`: a capped tube of `sides` sides swept along the points `path`.
    Its cross-section is carried point to point by parallel transport, so it
    does not twist round bends."""
    points = [Vector(p) for p in path]
    last = len(points) - 1
    tangents = []
    for i in range(len(points)):
        ahead = points[min(i + 1, last)]
        behind = points[max(i - 1, 0)]
        tangents.append((ahead - behind).normalized())
    normal = tangents[0].orthogonal().normalized()
    rings = []
    for i, point in enumerate(points):
        if i > 0:
            turn = tangents[i - 1].rotation_difference(tangents[i])
            normal = turn @ normal
            normal = (normal - tangents[i] * normal.dot(tangents[i])).normalized()
        binormal = tangents[i].cross(normal)
        ring = []
        for k in range(sides):
            angle = k * math.tau / sides
            offset = normal * math.cos(angle) + binormal * math.sin(angle)
            ring.append(bm.verts.new(point + offset * radius))
        rings.append(ring)
    for a, b in zip(rings, rings[1:]):
        for k in range(sides):
            n = (k + 1) % sides
            bm.faces.new((a[k], a[n], b[n], b[k]))
    bm.faces.new(rings[0][::-1])
    bm.faces.new(rings[-1])
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)


def arc(centre, radius, start, end, steps):
    """Points on an arc in the XZ plane, angles in radians from +X towards +Z."""
    points = []
    for j in range(steps + 1):
        angle = start + (end - start) * j / steps
        points.append((centre[0] + radius * math.cos(angle), 0.0, centre[1] + radius * math.sin(angle)))
    return points


def tube_part(name, path_for, radius, mat, parent, lods):
    """A swept tube as a tiered part; `path_for(lod)` gives its points."""

    def build(bm, lod):
        _tube(bm, path=path_for(lod), radius=radius, sides=TUBE_SIDES[lod])

    return mesh_part(name, build, mat=mat, parent=parent, lods=lods, smooth=True)
