"""Reusable vehicle detail parts: running gear, hull fittings and stowage.

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
  return that empty. `track_run` makes the empty `track_L` or `track_R`
  carrying `track_length_m` and `link_pitch_m`; every vertex under a track
  node scrolls its UVs with the links, so its belt is the only thing under it.
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
from parts import box, cyl, empty, mesh_part, prism  # noqa: E402

FINE = (0,)
NEAR = (0, 1)
MID = (0, 1, 2)
ALL = (0, 1, 2, 3)
# Sides of a swept tube (a hook, a guard bar, a weld bead) per tier.
TUBE_SIDES = (8, 6, 4, 4)


# ---------------------------------------------------------------- helpers
def _place(objects, loc, rot):
    """Move objects built in a part's own frame into its parent's frame."""
    frame = Matrix.LocRotScale(Vector(loc), Euler(rot), None)
    for o in objects:
        o.matrix_basis = frame @ o.matrix_basis
    return objects


def _wheel_node(name, loc, radius, parent):
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


def _arc(centre, radius, start, end, steps):
    """Points on an arc in the XZ plane, angles in radians from +X towards +Z."""
    points = []
    for j in range(steps + 1):
        angle = start + (end - start) * j / steps
        points.append((centre[0] + radius * math.cos(angle), 0.0, centre[1] + radius * math.sin(angle)))
    return points


def _tube_part(name, path_for, radius, mat, parent, lods):
    """A swept tube as a tiered part; `path_for(lod)` gives its points."""

    def build(bm, lod):
        _tube(bm, path=path_for(lod), radius=radius, sides=TUBE_SIDES[lod])

    return mesh_part(name, build, mat=mat, parent=parent, lods=lods, smooth=True)


# ---------------------------------------------------------------- running gear
def tyre_wheel(name, loc, radius, width, side, mats, parent, rim_radius=None, ctis=False, tread="road",
               hub_bolts=0):
    """A wheeled vehicle's wheel: a black treaded tyre, a rim on its outer face
    (`side` +1 for the left, -1 for the right) and a hub, with a CTIS air line
    from hub to rim where fitted. `rim_radius` defaults to the lifted 0.56 of the
    tyre. `tread` is "road" (an armoured carrier's run-flat road tread: blocks
    across the face) or "bar" (a truck's military bar tread: two staggered rows
    of deep chevron bars). `hub_bolts` rings the hub with that many wheel nuts.
    Node: `wheel_*` empty, returned. Roles: rubber, paint, dark, steel."""
    node = _wheel_node(name, loc, radius, parent)
    rim_r = radius * 0.56 if rim_radius is None else rim_radius
    face = side * width / 2
    cyl(name=f"{name}_tyre", r=radius, depth=width, axis="Y", mat=mats["rubber"], parent=node, seg=28)
    cyl(name=f"{name}_rim", r=rim_r, depth=0.06, loc=(0, face + side * 0.005, 0), axis="Y",
        mat=mats["paint"], parent=node, seg=20, bevel=0.01)
    cyl(name=f"{name}_hub", r=radius * 0.21, depth=0.09, loc=(0, face + side * 0.035, 0), axis="Y",
        mat=mats["dark"], parent=node, seg=12, lods=MID)
    for j in range(hub_bolts):
        angle = j * math.tau / hub_bolts
        at = (radius * 0.21 + rim_r) / 2
        cyl(name=f"{name}_nut_{j}", r=0.018, depth=0.03, loc=(at * math.cos(angle), face + side * 0.045,
                                                             at * math.sin(angle)), axis="Y",
            mat=mats["steel"], parent=node, seg=6, lods=FINE)
    if tread == "bar":
        treads = max(12, round(math.tau * radius / 0.17))
        for j in range(treads):
            for k, y in enumerate((-0.25, 0.25)):
                angle = (j + 0.5 * k) * math.tau / treads
                at = radius - 0.02
                box(name=f"{name}_tread_{j}_{k}", size=(0.07, width * 0.46, 0.04),
                    loc=(at * math.sin(angle), y * width, at * math.cos(angle)),
                    rot=(0, angle, (0.45 if k else -0.45) * side), mat=mats["rubber"], parent=node, lods=NEAR)
        return node
    treads = max(12, round(math.tau * radius / 0.21))
    for j in range(treads):
        angle = j * math.tau / treads
        at = radius - 0.012
        box(name=f"{name}_tread_{j}", size=(0.15, width * 0.88, 0.025),
            loc=(at * math.sin(angle), 0, at * math.cos(angle)), rot=(0, angle, 0.18 * side),
            mat=mats["rubber"], parent=node, lods=NEAR)
    if ctis:
        hub_face = face + side * 0.08
        rim_face = face + side * 0.035

        def air_line(lod):
            return [(0, hub_face, radius * 0.12), (0, rim_face, rim_r * 0.85)]

        _tube_part(name=f"{name}_ctis", path_for=air_line, radius=0.008, mat=mats["steel"], parent=node, lods=FINE)
    return node


def road_wheel(name, loc, radius, width, side, mats, parent, bolts=6):
    """A tracked vehicle's road wheel: a rubber tyre on a painted steel disc,
    a hub and hub bolts on the outer face (`side` +1 left, -1 right). Node:
    `wheel_*` empty, returned. Roles: rubber, paint, dark, steel."""
    node = _wheel_node(name, loc, radius, parent)
    disc_width = width + 0.03
    cyl(name=f"{name}_tyre", r=radius, depth=width, axis="Y", mat=mats["rubber"], parent=node, seg=28)
    cyl(name=f"{name}_disc", r=radius * 0.82, depth=disc_width, axis="Y", mat=mats["paint"], parent=node, seg=24)
    cyl(name=f"{name}_hub", r=radius * 0.26, depth=width + 0.09, axis="Y", mat=mats["dark"], parent=node,
        seg=16, lods=MID)
    for j in range(bolts):
        angle = j * math.tau / bolts
        at = radius * 0.47
        cyl(name=f"{name}_bolt_{j}", r=0.022, depth=0.045,
            loc=(at * math.cos(angle), side * (disc_width / 2 + 0.015), at * math.sin(angle)), axis="Y",
            mat=mats["steel"], parent=node, seg=6, lods=FINE)
    return node


def sprocket(name, loc, radius, width, mats, parent, teeth=12):
    """The toothed drive sprocket: a dark wheel, a painted hub and steel teeth
    round its rim. Node: `wheel_*` empty, returned. Roles: dark, paint, steel."""
    node = _wheel_node(name, loc, radius, parent)
    cyl(name=f"{name}_body", r=radius, depth=width, axis="Y", mat=mats["dark"], parent=node, seg=24)
    cyl(name=f"{name}_hub", r=radius * 0.54, depth=width + 0.05, axis="Y", mat=mats["paint"], parent=node,
        seg=16, lods=MID)
    tooth_length = 0.47 * math.tau * radius / teeth
    tooth_at = radius + 0.015
    for j in range(teeth):
        angle = j * math.tau / teeth
        box(name=f"{name}_tooth_{j}", size=(tooth_length, width + 0.02, radius * 0.16),
            loc=(tooth_at * math.sin(angle), 0, tooth_at * math.cos(angle)), rot=(0, angle, 0),
            mat=mats["steel"], parent=node, lods=NEAR)
    return node


def idler(name, loc, radius, width, mats, parent):
    """The idler wheel at the track's far end: a dark wheel and a painted hub.
    Node: `wheel_*` empty, returned. Roles: dark, paint."""
    node = _wheel_node(name, loc, radius, parent)
    cyl(name=f"{name}_body", r=radius, depth=width, axis="Y", mat=mats["dark"], parent=node, seg=24)
    cyl(name=f"{name}_hub", r=radius * 0.54, depth=width + 0.05, axis="Y", mat=mats["paint"], parent=node,
        seg=16, lods=MID)
    return node


def return_roller(name, loc, radius, width, mats, parent):
    """A small roller carrying the track's upper run. Node: `wheel_*` empty,
    returned. Roles: dark."""
    node = _wheel_node(name, loc, radius, parent)
    cyl(name=f"{name}_roller", r=radius, depth=width, axis="Y", mat=mats["dark"], parent=node, seg=16, lods=NEAR)
    return node


def track_run(name, loc, span, end_radius, width, pitch, mats, parent, thickness=0.08):
    """A closed track belt round two end wheels of radius `end_radius`, their
    centres `span` apart along X, centred on `loc` (x, the track's centre line
    y, the end wheels' z). Links, shoes and guide horns are the 'track'
    texture and normal map only: u runs along the belt in links (metres /
    `pitch`), v across it. Node: the empty `name` (`track_L` or `track_R`) at
    the parent's origin, carrying `track_length_m` (the loop's centre line)
    and `link_pitch_m`, returned; its belt is the only mesh under it. Roles:
    track."""
    if name not in ("track_L", "track_R"):
        raise ValueError(f"a track node is named track_L or track_R: {name}")
    half = span / 2
    loop_length = 2 * span + math.tau * end_radius
    node = empty(name, parent=parent, props={"track_length_m": loop_length, "link_pitch_m": pitch})
    cx, cy, cz = loc

    def belt(bm, lod):
        steps = (24, 14, 8, 5)[lod]
        outer = []
        inner = []
        for end_x, start in ((half, -math.pi / 2), (-half, math.pi / 2)):
            for j in range(steps + 1):
                theta = start + j * math.pi / steps
                outer.append((cx + end_x + (end_radius + thickness / 2) * math.cos(theta),
                              cz + (end_radius + thickness / 2) * math.sin(theta)))
                inner.append((cx + end_x + (end_radius - thickness / 2) * math.cos(theta),
                              cz + (end_radius - thickness / 2) * math.sin(theta)))
        # rings[surface][edge][j]: surface 0 outer, 1 inner; edge 0 at -y, 1 at +y
        rings = []
        for profile in (outer, inner):
            edges = []
            for y in (-width / 2, width / 2):
                edges.append([bm.verts.new((x, cy + y, z)) for x, z in profile])
            rings.append(edges)
        uv = bm.loops.layers.uv.new("UVMap")
        count = len(outer)
        along = 0.0
        for j in range(count):
            nxt = (j + 1) % count
            step = math.hypot(outer[nxt][0] - outer[j][0], outer[nxt][1] - outer[j][1])
            for surface, reverse in ((rings[0], False), (rings[1], True)):
                verts = [surface[0][j], surface[0][nxt], surface[1][nxt], surface[1][j]]
                face = bm.faces.new(verts[::-1] if reverse else verts)
                for loop in face.loops:
                    corner = verts.index(loop.vert)
                    u = (along + (step if corner in (1, 2) else 0)) / pitch
                    v = 0 if corner < 2 else 1
                    loop[uv].uv = (u, v)
            for edge in (0, 1):
                verts = [rings[0][edge][j], rings[1][edge][j], rings[1][edge][nxt], rings[0][edge][nxt]]
                face = bm.faces.new(verts)
                for loop in face.loops:
                    corner = verts.index(loop.vert)
                    u = (along + (step if corner in (2, 3) else 0)) / pitch
                    v = 0 if corner in (0, 3) else thickness / pitch
                    loop[uv].uv = (u, v)
            along += step
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    mesh_part(f"{name}_band", belt, mat=mats["track"], parent=node)
    return node


def _convex_hull_2d(points):
    """The convex hull of 2D points, counter-clockwise (monotone chain)."""
    pts = sorted(set(points))

    def cross(o, a, b):
        return (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])

    lower, upper = [], []
    for p in pts:
        while len(lower) >= 2 and cross(lower[-2], lower[-1], p) <= 0:
            lower.pop()
        lower.append(p)
    for p in reversed(pts):
        while len(upper) >= 2 and cross(upper[-2], upper[-1], p) <= 0:
            upper.pop()
        upper.append(p)
    return lower[:-1] + upper[:-1]


def track_loop(name, wheels, y, width, pitch, mats, parent, thickness=0.08):
    """A closed track belt wrapped round every wheel it touches, as a real run
    is: `wheels` are (x, z, radius) circles in the side plane (road wheels,
    sprocket, idler, return rollers), and the belt's centre line is their
    convex hull grown by half its `thickness`, so the lower run lies under the
    road wheels, the ends wrap the sprocket and idler and the upper run rests
    on the rollers. Centred across on `y`, `width` wide. Links are the 'track'
    texture only: u runs along the belt in links, v across it. Node: the empty
    `name` (`track_L` or `track_R`) at the parent's origin, carrying
    `track_length_m` and `link_pitch_m`, returned; its belt is its only mesh.
    Roles: track."""
    if name not in ("track_L", "track_R"):
        raise ValueError(f"a track node is named track_L or track_R: {name}")
    grown = []
    for x, z, r in wheels:
        for k in range(96):
            a = k * math.tau / 96
            grown.append((round(x + (r + thickness / 2) * math.cos(a), 5), round(z + (r + thickness / 2) * math.sin(a), 5)))
    loop = _convex_hull_2d(grown)
    segments = []
    length = 0.0
    for i, a in enumerate(loop):
        b = loop[(i + 1) % len(loop)]
        step = math.hypot(b[0] - a[0], b[1] - a[1])
        segments.append((a, b, length, step))
        length += step
    node = empty(name, parent=parent, props={"track_length_m": length, "link_pitch_m": pitch})
    spacing = (0.06, 0.14, 0.32, 0.7)

    def at(d):
        for a, b, start, step in segments:
            if d <= start + step or (a, b) == segments[-1][:2]:
                t = 0.0 if step == 0 else min(1.0, (d - start) / step)
                return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
        return segments[-1][1]

    def belt(bm, lod):
        n = max(12, int(length / spacing[lod]))
        centre = [at(k * length / n) for k in range(n)]
        uv = bm.loops.layers.uv.new("UVMap")
        half = thickness / 2
        # across the belt (w 0..1) and out from its centre line (r)
        profile = ((0, -half), (0, half), (1, half), (1, -half))
        rings = []
        for i, p in enumerate(centre):
            ahead = centre[(i + 1) % n]
            behind = centre[i - 1]
            tx, tz = ahead[0] - behind[0], ahead[1] - behind[1]
            norm = math.hypot(tx, tz) or 1.0
            nx, nz = tz / norm, -tx / norm  # outward for a counter-clockwise loop
            rings.append([bm.verts.new((p[0] + nx * r, y - width / 2 + w * width, p[1] + nz * r)) for w, r in profile])
        for i in range(n):
            j = (i + 1) % n
            u0 = i * length / n / pitch
            u1 = (i + 1) * length / n / pitch
            for k in range(4):
                k2 = (k + 1) % 4
                face = bm.faces.new((rings[i][k], rings[i][k2], rings[j][k2], rings[j][k]))
                v0, v1 = profile[k][0], profile[k2][0]
                if k in (0, 2):  # the belt's edge faces: v across its thickness
                    v0, v1 = (0.0, thickness / pitch) if k == 0 else (thickness / pitch, 0.0)
                for loop_, (uu, vv) in zip(face.loops, ((u0, v0), (u0, v1), (u1, v1), (u1, v0))):
                    loop_[uv].uv = (uu, vv)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    mesh_part(f"{name}_band", belt, mat=mats["track"], parent=node)
    return node


# ---------------------------------------------------------------- hull and turret fittings
def cable(name, points, mats, parent, radius=0.02, eyes=True, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A steel tow cable along `points` (in the part's frame), clipped down
    where it runs, with a loop eye at each end where `eyes`. Returns its
    meshes. Roles: steel, dark."""
    made = _tube_part(name=f"{name}_rope", path_for=lambda lod: points, radius=radius, mat=mats["steel"],
                      parent=parent, lods=MID)
    if eyes:
        for k, (end, inner) in enumerate(((points[0], points[1]), (points[-1], points[-2]))):
            d = Vector(end) - Vector(inner)
            yaw = math.atan2(d.y, d.x)
            ex, ey, ez = end
            centre = (ex + math.cos(yaw) * radius * 4, ey + math.sin(yaw) * radius * 4, ez)

            def eye(lod, centre=centre, yaw=yaw):
                steps = (10, 6, 4, 4)[lod]
                ring = []
                for j in range(steps + 1):
                    a = math.tau * j / steps
                    ring.append((centre[0] + math.cos(yaw) * math.cos(a) * radius * 4 - math.sin(yaw) * math.sin(a) * radius * 3,
                                 centre[1] + math.sin(yaw) * math.cos(a) * radius * 4 + math.cos(yaw) * math.sin(a) * radius * 3,
                                 centre[2]))
                return ring

            made += _tube_part(name=f"{name}_eye_{k}", path_for=eye, radius=radius * 0.8, mat=mats["steel"],
                               parent=parent, lods=NEAR)
    for k, p in enumerate(points[1:-1]):
        made += box(name=f"{name}_clip_{k}", size=(0.05, radius * 4, radius * 2.5), loc=p, mat=mats["dark"],
                    parent=parent, lods=FINE)
    return _place(made, loc, rot)



def hatch(name, loc, mats, parent, radius=None, size=None, rot=(0, 0, 0)):
    """A closed hinged hatch lying in its local XY plane on the roof at `loc`:
    round (`radius`) or rectangular (`size` = (x, y)), exactly one of them. A
    raised coaming ring, the lid, a hinge along its rear (-X) edge and a grab
    handle. Returns its meshes. Roles: paint, steel."""
    if (radius is None) == (size is None):
        raise ValueError(f"{name}: give a hatch a radius or a size, not both")
    made = []
    lid_depth = 0.05
    if radius is not None:
        rear = -radius
        hinge_length = radius * 0.9
        made += cyl(name=f"{name}_coaming", r=radius + 0.04, depth=0.04, loc=(0, 0, 0.02),
                    mat=mats["paint"], parent=parent, seg=28, bevel=0.01, lods=NEAR)
        made += cyl(name=f"{name}_lid", r=radius, depth=lid_depth, loc=(0, 0, 0.04 + lid_depth / 2),
                    mat=mats["paint"], parent=parent, seg=28, bevel=0.015, lods=MID)
    else:
        rear = -size[0] / 2
        hinge_length = size[1] * 0.8
        made += box(name=f"{name}_coaming", size=(size[0] + 0.08, size[1] + 0.08, 0.04), loc=(0, 0, 0.02),
                    mat=mats["paint"], parent=parent, bevel=0.01, lods=NEAR)
        made += box(name=f"{name}_lid", size=(size[0], size[1], lid_depth), loc=(0, 0, 0.04 + lid_depth / 2),
                    mat=mats["paint"], parent=parent, bevel=0.015, lods=MID)
    top = 0.04 + lid_depth
    made += cyl(name=f"{name}_hinge", r=0.025, depth=hinge_length, loc=(rear + 0.02, 0, top), axis="Y",
                mat=mats["steel"], parent=parent, seg=10, lods=NEAR)
    handle_x = -rear * 0.55

    def handle(lod):
        return [(handle_x, -0.07, top - 0.005), (handle_x, -0.07, top + 0.035),
                (handle_x, 0.07, top + 0.035), (handle_x, 0.07, top - 0.005)]

    made += _tube_part(name=f"{name}_handle", path_for=handle, radius=0.01, mat=mats["steel"], parent=parent,
                       lods=FINE)
    return _place(made, loc, rot)


def periscope(name, loc, mats, parent, size=(0.15, 0.07, 0.08), rot=(0, 0, 0)):
    """A vision block standing on the roof at `loc`, looking along its local
    +X: an armoured head `size` (x, y, z), dark glass in its face and a brow
    plate over it. Returns its meshes. Roles: paint, glass."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_head", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=0.008, lods=MID)
    made += box(name=f"{name}_glass", size=(0.01, sy * 0.8, sz * 0.5), loc=(sx / 2 + 0.003, 0, sz * 0.45),
                mat=mats["glass"], parent=parent, lods=NEAR)
    made += box(name=f"{name}_brow", size=(sx * 0.35, sy * 1.1, 0.015), loc=(sx / 2 + sx * 0.12, 0, sz * 0.82),
                mat=mats["paint"], parent=parent, lods=FINE)
    return _place(made, loc, rot)


def sight_housing(name, loc, mats, parent, size=(0.32, 0.24, 0.22), rot=(0, 0, 0)):
    """A gunner's or commander's sight: an armoured box `size` (x, y, z)
    standing at `loc`, its dark glass window on the local +X face under a
    hood, with an armoured shutter rail on top. Returns its meshes. Roles:
    paint, glass, dark."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_housing", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=0.02)
    made += box(name=f"{name}_window", size=(0.012, sy * 0.72, sz * 0.48), loc=(sx / 2 + 0.004, 0, sz * 0.5),
                mat=mats["glass"], parent=parent, lods=MID)
    made += box(name=f"{name}_hood", size=(sx * 0.22, sy * 0.92, 0.025), loc=(sx / 2 + sx * 0.09, 0, sz * 0.82),
                mat=mats["paint"], parent=parent, rot=(0, -0.15, 0), lods=NEAR)
    made += box(name=f"{name}_shutter_rail", size=(sx * 0.7, 0.03, 0.03), loc=(0, sy * 0.3, sz + 0.012),
                mat=mats["dark"], parent=parent, lods=FINE)
    return _place(made, loc, rot)


def light_with_guard(name, loc, radius, mats, parent, rot=(0, 0, 0)):
    """A lamp of `radius` shining along local +X, its housing's centre at
    `loc` and its base below on the mounting face, inside a bent-bar guard of
    two hoops. Returns its meshes. Roles: paint, lamp."""
    depth = radius * 1.1
    made = []
    made += cyl(name=f"{name}_housing", r=radius, depth=depth, axis="X", mat=mats["paint"], parent=parent,
                seg=16, bevel=0.006, lods=MID)
    made += cyl(name=f"{name}_lens", r=radius * 0.8, depth=0.01, loc=(depth / 2 + 0.003, 0, 0), axis="X",
                mat=mats["lamp"], parent=parent, seg=16, lods=MID)
    made += box(name=f"{name}_base", size=(depth * 0.8, radius * 1.2, radius * 0.6), loc=(0, 0, -radius * 1.2),
                mat=mats["paint"], parent=parent, lods=NEAR)
    reach = radius * 1.35
    for k, x in enumerate((depth / 2 + radius * 0.35, 0.0)):

        def hoop(lod, x=x):
            steps = (8, 6, 4, 4)[lod]
            points = []
            for j in range(steps + 1):
                angle = math.pi * j / steps
                points.append((x, -reach * math.cos(angle), -radius * 1.2 + (reach + radius * 1.2) * math.sin(angle)))
            return points

        made += _tube_part(name=f"{name}_guard_{k}", path_for=hoop, radius=0.009, mat=mats["paint"],
                           parent=parent, lods=NEAR)
    return _place(made, loc, rot)


def tow_hook(name, loc, mats, parent, size=0.12, rot=(0, 0, 0)):
    """A tow hook on a mounting plate, the plate on its local YZ face at
    `loc` and the hook reaching out along +X and curling up; `size` is the
    hook's reach. Returns its meshes. Roles: paint, steel."""
    made = []
    made += box(name=f"{name}_plate", size=(0.03, size * 1.3, size * 1.1), loc=(0.015, 0, 0), mat=mats["paint"],
                parent=parent, bevel=0.006, lods=MID)
    curl = size * 0.32

    def hook(lod):
        steps = (8, 6, 4, 4)[lod]
        points = [(0.03, 0, 0)]
        points += _arc(centre=(size - curl, curl * 0.1), radius=curl, start=-math.pi / 2, end=math.pi * 0.55,
                       steps=steps)
        return points

    made += _tube_part(name=f"{name}_hook", path_for=hook, radius=size * 0.13, mat=mats["steel"], parent=parent,
                       lods=NEAR)
    return _place(made, loc, rot)


def shackle(name, loc, mats, parent, size=0.1, rot=(0, 0, 0)):
    """A bow shackle hanging from a lug at `loc`: a U of `size` height in the
    local XZ plane hanging down, and its pin across Y at the top. Returns its
    meshes. Roles: paint, steel."""
    half = size * 0.35
    bar = size * 0.12
    made = []

    def bow(lod):
        steps = (10, 6, 4, 4)[lod]
        points = [(-half, 0, 0)]
        points += _arc(centre=(0, -size + half), radius=half, start=math.pi, end=math.tau, steps=steps)
        points.append((half, 0, 0))
        return points

    made += _tube_part(name=f"{name}_bow", path_for=bow, radius=bar, mat=mats["paint"], parent=parent,
                       lods=NEAR)
    made += cyl(name=f"{name}_pin", r=bar * 0.9, depth=half * 2 + bar * 4, axis="X", mat=mats["steel"],
                parent=parent, seg=8, lods=FINE)
    return _place(made, loc, rot)


def smoke_discharger_bank(name, loc, mats, parent, count=4, tube_radius=0.045, tube_length=0.3,
                          elevation=0.5, spread=0.5, rot=(0, 0, 0)):
    """A bank of `count` smoke grenade tubes on a bracket at `loc`, firing
    along local +X raised by `elevation` and fanned across `spread` radians,
    side by side along Y. Returns its meshes. Roles: paint, black."""
    spacing = tube_radius * 2.4
    width = spacing * count
    made = []
    made += box(name=f"{name}_bracket", size=(tube_length * 0.6, width, tube_radius * 1.2),
                loc=(0, 0, tube_radius * 0.6), mat=mats["paint"], parent=parent, bevel=0.01)
    for i in range(count):
        share = 0.5 if count == 1 else i / (count - 1)
        yaw = -spread / 2 + spread * share
        y = (i - (count - 1) / 2) * spacing
        aim = Euler((0, -elevation, yaw)).to_matrix()
        axis = aim @ Vector((1, 0, 0))
        foot = Vector((-tube_length * 0.2, y, tube_radius * 1.6))
        centre = foot + axis * (tube_length / 2)
        mouth = foot + axis * (tube_length + 0.003)
        made += cyl(name=f"{name}_tube_{i}", r=tube_radius, depth=tube_length, loc=tuple(centre), axis="X",
                    rot=(0, -elevation, yaw), mat=mats["paint"], parent=parent, seg=12, bevel=0.006, lods=MID)
        made += cyl(name=f"{name}_bore_{i}", r=tube_radius * 0.75, depth=0.006, loc=tuple(mouth), axis="X",
                    rot=(0, -elevation, yaw), mat=mats["black"], parent=parent, seg=12, lods=FINE)
    return _place(made, loc, rot)


def antenna(name, loc, mats, parent, height=2.4, radius=0.008, rot=(0, 0, 0)):
    """A whip antenna: a mount base at `loc`, a spring and a whip `height`
    tall along local +Z (tilt it with `rot`). Returns its meshes. Roles: dark."""
    made = []
    made += cyl(name=f"{name}_base", r=0.05, depth=0.1, loc=(0, 0, 0.05), mat=mats["dark"], parent=parent,
                seg=12, bevel=0.008, lods=MID)
    made += cyl(name=f"{name}_spring", r=0.022, depth=0.12, loc=(0, 0, 0.16), mat=mats["dark"], parent=parent,
                seg=8, lods=FINE)
    made += cyl(name=f"{name}_whip", r=radius, r2=radius * 0.5, depth=height, loc=(0, 0, 0.1 + height / 2),
                mat=mats["dark"], parent=parent, seg=6, min_seg=4, lods=NEAR)
    return _place(made, loc, rot)


# ---------------------------------------------------------------- weapons
def browning_m2(parent, reach, mats, receiver_x=0.06, grips=False):
    """The M2 heavy machine gun on its mount's pitch node `parent`, firing
    along +X to the muzzle `reach` metres out: receiver centred at
    `receiver_x`, the barrel and its jacket, the flash hider, and with `grips`
    the spade grips a standing gunner holds. Every piece is named `m2_*`.
    Returns its meshes. Roles: dark, steel, black."""
    barrel_from = receiver_x + 0.26
    made = []
    made += box(name="m2_receiver", size=(0.52, 0.13, 0.15), loc=(receiver_x, 0, 0), mat=mats["dark"], parent=parent,
                bevel=0.012)
    made += cyl(name="m2_barrel", r=0.024, depth=reach - barrel_from, loc=((reach + barrel_from) / 2, 0, 0), axis="X",
                mat=mats["steel"], parent=parent, seg=10)
    made += cyl(name="m2_jacket", r=0.040, depth=0.22, loc=(barrel_from + 0.11, 0, 0), axis="X", mat=mats["dark"],
                parent=parent, seg=12, lods=MID)
    made += cyl(name="m2_flash_hider", r=0.034, r2=0.026, depth=0.08, loc=(reach - 0.04, 0, 0), axis="X",
                mat=mats["steel"], parent=parent, seg=10, lods=NEAR)
    if grips:
        for side in (-1, 1):
            made += cyl(name=f"m2_grip_{side}", r=0.018, depth=0.12, loc=(receiver_x - 0.30, side * 0.07, -0.02),
                        mat=mats["black"], parent=parent, seg=8, lods=NEAR)
    return made


# ---------------------------------------------------------------- stowage
def jerrycan(name, loc, mats, parent, size=(0.165, 0.345, 0.47), rot=(0, 0, 0)):
    """A standing jerrycan, its foot's centre at `loc`: a can `size` (x
    thickness, y width, z height; the real can by default), three carrying
    handles on top and a spout. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_can", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=0.018, lods=MID)
    made += box(name=f"{name}_spout", size=(sx * 0.4, sy * 0.18, 0.04), loc=(0, sy * 0.36, sz + 0.02),
                mat=mats["paint"], parent=parent, lods=NEAR)
    for k in range(3):
        y = (k - 1) * sy * 0.22 - sy * 0.08

        def grip(lod, y=y):
            return [(0, y - 0.03, sz - 0.005), (0, y - 0.03, sz + 0.03), (0, y + 0.03, sz + 0.03),
                    (0, y + 0.03, sz - 0.005)]

        made += _tube_part(name=f"{name}_handle_{k}", path_for=grip, radius=0.007, mat=mats["steel"],
                           parent=parent, lods=FINE)
    return _place(made, loc, rot)


def stowage_box(name, loc, size, mats, parent, rot=(0, 0, 0)):
    """A stowage bin `size` (x, y, z) standing at `loc` (its foot's centre):
    the box, an overlapping lid, two latches on its +Y face and a hinge along
    the rear of the lid. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_body", size=(sx, sy, sz * 0.9), loc=(0, 0, sz * 0.45), mat=mats["paint"],
                parent=parent, bevel=0.015)
    made += box(name=f"{name}_lid", size=(sx + 0.02, sy + 0.02, sz * 0.1), loc=(0, 0, sz * 0.95),
                mat=mats["paint"], parent=parent, bevel=0.01, lods=NEAR)
    for k, x in enumerate((-sx * 0.3, sx * 0.3)):
        made += box(name=f"{name}_latch_{k}", size=(0.04, 0.015, 0.06), loc=(x, sy / 2 + 0.008, sz * 0.86),
                    mat=mats["steel"], parent=parent, lods=FINE)
    made += cyl(name=f"{name}_hinge", r=0.012, depth=sx * 0.9, loc=(0, -sy / 2 - 0.01, sz * 0.92), axis="X",
                mat=mats["steel"], parent=parent, seg=8, lods=FINE)
    return _place(made, loc, rot)


def tarp_roll(name, loc, length, radius, mats, parent, straps=2, rot=(0, 0, 0)):
    """A rolled tarpaulin lying along local Y, its axis at `loc`, bound by
    `straps` dark straps. Returns its meshes. Roles: canvas, dark."""
    made = []
    made += cyl(name=f"{name}_roll", r=radius, depth=length, axis="Y", mat=mats["canvas"], parent=parent,
                seg=20, bevel=radius * 0.25)
    for k in range(straps):
        share = (k + 1) / (straps + 1)
        y = -length / 2 + length * share
        made += cyl(name=f"{name}_strap_{k}", r=radius * 1.04, depth=0.04, loc=(0, y, 0), axis="Y",
                    mat=mats["dark"], parent=parent, seg=20, lods=NEAR)
    return _place(made, loc, rot)


# ---------------------------------------------------------------- panels and fittings
def grille(name, loc, size, mats, parent, slats=8, rot=(0, 0, 0)):
    """An engine deck grille lying on the deck at `loc`, `size` (x, y): a
    dark louvred opening, a raised frame and `slats` bars across Y. Returns its
    meshes. Roles: black, paint."""
    sx, sy = size
    rim = 0.04
    made = []
    made += box(name=f"{name}_opening", size=(sx, sy, 0.02), loc=(0, 0, 0.01), mat=mats["black"], parent=parent)
    for k, (fx, fy, lx, ly) in enumerate(((0, 1, sx + rim, rim), (0, -1, sx + rim, rim),
                                          (1, 0, rim, sy), (-1, 0, rim, sy))):
        made += box(name=f"{name}_frame_{k}", size=(lx, ly, 0.035), loc=(fx * sx / 2, fy * sy / 2, 0.0175),
                    mat=mats["paint"], parent=parent, lods=MID)
    gap = sx / slats
    for j in range(slats):
        x = -sx / 2 + gap * (j + 0.5)
        made += box(name=f"{name}_slat_{j}", size=(gap * 0.45, sy, 0.012), loc=(x, 0, 0.026),
                    rot=(0, 0.35, 0), mat=mats["paint"], parent=parent, lods=NEAR)
    return _place(made, loc, rot)


def exhaust(name, loc, radius, length, mats, parent, rot=(0, 0, 0)):
    """An exhaust pipe along local +X, its outlet at `loc`, under a slotted
    heat guard, with a black bore. Returns its meshes. Roles: dark, paint,
    black."""
    made = []
    made += cyl(name=f"{name}_pipe", r=radius, depth=length, loc=(-length / 2, 0, 0), axis="X", mat=mats["dark"],
                parent=parent, seg=16, bevel=0.005, lods=MID)
    made += cyl(name=f"{name}_bore", r=radius * 0.78, depth=0.006, loc=(0.002, 0, 0), axis="X", mat=mats["black"],
                parent=parent, seg=16, lods=NEAR)
    # an arched sheet over the pipe's top: outer arc out, inner arc back
    outer = _arc(centre=(0, 0), radius=radius * 1.45, start=0, end=math.pi, steps=6)
    inner = _arc(centre=(0, 0), radius=radius * 1.3, start=math.pi, end=0, steps=6)
    arch = [(x, z) for x, _, z in outer + inner]
    made += prism(name=f"{name}_guard", profile_xz=arch, width=length * 0.7, loc=(-length * 0.45, 0, 0),
                  rot=(0, 0, math.pi / 2), mat=mats["paint"], parent=parent, lods=NEAR)
    return _place(made, loc, rot)


def mudflap(name, loc, size, mats, parent, rot=(0, 0, 0)):
    """A rubber mudflap hanging from a bracket at `loc` (its top edge's
    centre), `size` (y width, z drop), facing along X. Returns its meshes.
    Roles: rubber, steel."""
    sy, sz = size
    made = []
    made += box(name=f"{name}_flap", size=(0.012, sy, sz), loc=(0, 0, -sz / 2), mat=mats["rubber"], parent=parent,
                lods=MID)
    made += box(name=f"{name}_bracket", size=(0.03, sy + 0.04, 0.04), loc=(0.02, 0, -0.02), mat=mats["steel"],
                parent=parent, lods=NEAR)
    return _place(made, loc, rot)


def cargo_bed(name, loc, size, mats, parent, stakes=5, tarp=None, rot=(0, 0, 0)):
    """A truck's drop-side cargo bed, its floor's centre at `loc`, `size`
    (x length, y width, z side height): the floor on its cross members, the
    side and end boards with their stake posts and hinges, and with `tarp`
    (its height over the sides) the bows and the canvas over them, tied down
    along its foot. Returns its meshes. Roles: paint, dark, steel, canvas."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_floor", size=(sx, sy, 0.10), loc=(0, 0, -0.05), mat=mats["paint"], parent=parent,
                bevel=0.02)
    for k in range(max(2, round(sx / 0.9))):
        x = -sx / 2 + 0.2 + (sx - 0.4) * k / max(1, round(sx / 0.9) - 1)
        made += box(name=f"{name}_member_{k}", size=(0.10, sy - 0.10, 0.14), loc=(x, 0, -0.17), mat=mats["dark"],
                    parent=parent, lods=NEAR)
    for side in (-1, 1):
        made += box(name=f"{name}_side_{side}", size=(sx, 0.05, sz), loc=(0, side * (sy / 2 - 0.025), sz / 2),
                    mat=mats["paint"], parent=parent, bevel=0.015)
        made += box(name=f"{name}_rail_{side}", size=(sx + 0.02, 0.08, 0.06), loc=(0, side * (sy / 2 - 0.03), sz),
                    mat=mats["paint"], parent=parent, bevel=0.012, lods=MID)
        for k in range(stakes):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes - 1)
            made += box(name=f"{name}_stake_{side}_{k}", size=(0.08, 0.04, sz), loc=(x, side * (sy / 2 + 0.01), sz / 2),
                        mat=mats["paint"], parent=parent, bevel=0.01, lods=MID)
            made += box(name=f"{name}_hinge_{side}_{k}", size=(0.10, 0.03, 0.05), loc=(x + 0.2, side * (sy / 2 + 0.01),
                                                                                     0.04),
                        mat=mats["steel"], parent=parent, lods=FINE)
    for end in (-1, 1):
        made += box(name=f"{name}_end_{end}", size=(0.05, sy, sz), loc=(end * (sx / 2 - 0.025), 0, sz / 2),
                    mat=mats["paint"], parent=parent, bevel=0.015)
    if tarp:
        for k in range(stakes):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes - 1)
            made += box(name=f"{name}_bow_{k}", size=(0.05, sy - 0.04, 0.05), loc=(x, 0, sz + tarp - 0.05),
                        mat=mats["dark"], parent=parent, lods=FINE)
        made += box(name=f"{name}_canvas", size=(sx + 0.04, sy + 0.04, tarp + 0.12), loc=(0, 0, sz - 0.06 + tarp / 2),
                    mat=mats["canvas"], parent=parent, bevel=0.09)
        for k in range(stakes * 2 - 1):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes * 2 - 2)
            for side in (-1, 1):
                made += box(name=f"{name}_tie_{side}_{k}", size=(0.03, 0.02, 0.22), loc=(x, side * (sy / 2 + 0.035),
                                                                                       sz - 0.02),
                            mat=mats["dark"], parent=parent, lods=FINE)
    return _place(made, loc, rot)


def fuel_tank(name, loc, length, radius, mats, parent, rot=(0, 0, 0)):
    """A truck's side fuel tank lying along local X, its axis at `loc`: the
    tank, two straps over it and its filler cap. Returns its meshes. Roles:
    paint, dark, steel."""
    made = []
    made += cyl(name=f"{name}_tank", r=radius, depth=length, axis="X", mat=mats["paint"], parent=parent, seg=20,
                bevel=0.03)
    for k, x in enumerate((-length * 0.3, length * 0.3)):
        made += cyl(name=f"{name}_strap_{k}", r=radius * 1.04, depth=0.05, loc=(x, 0, 0), axis="X", mat=mats["dark"],
                    parent=parent, seg=20, lods=NEAR)
    made += cyl(name=f"{name}_cap", r=0.05, depth=0.05, loc=(length * 0.1, 0, radius + 0.02), mat=mats["steel"],
                parent=parent, seg=10, lods=FINE)
    return _place(made, loc, rot)


def bolted_panel(name, loc, size, mats, parent, bolts=(3, 2), rot=(0, 0, 0), bevel=0.02, lods=MID):
    """An applique armour plate bolted onto a face: a slab `size` (x, y, z
    thickness) lying in its local XY plane on the face at `loc` (its back on
    the face, facing local +Z), with chunky bolt heads in a ring `bolts`
    (along x, along y) round its border. Turn it onto a side or a glacis with
    `rot`. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_plate", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=bevel, lods=lods)
    nx, ny = bolts
    spots = set()
    for i in range(nx):
        for j in range(ny):
            if i in (0, nx - 1) or j in (0, ny - 1):
                spots.add((i, j))
    for i, j in sorted(spots):
        x = -sx / 2 + 0.07 + (sx - 0.14) * (i / max(1, nx - 1))
        y = -sy / 2 + 0.07 + (sy - 0.14) * (j / max(1, ny - 1))
        made += cyl(name=f"{name}_bolt_{i}_{j}", r=0.026, depth=0.03, loc=(x, y, sz + 0.012), mat=mats["steel"],
                    parent=parent, seg=6, bevel=0.006, lods=FINE)
    return _place(made, loc, rot)


def weld_line(name, points, mats, parent, radius=0.007, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A weld bead along `points` (on a plate seam, in the part's frame), seen
    close up only. Returns its meshes. Roles: paint."""

    def seam(lod):
        return points

    made = _tube_part(name=name, path_for=seam, radius=radius, mat=mats["paint"], parent=parent, lods=FINE)
    return _place(made, loc, rot)
