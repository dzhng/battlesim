"""Running gear: tyred wheels, and the track belt wrapped round a tracked
vehicle's wheels (a scripted body draws the wheels themselves).

Every part keeps the contracts the kernel documents (`vehicle_parts`). The
nodes this module makes are the renderer's: each wheel is a spinning `wheel_*`
node (`vehicle_parts.wheel_node`), and `track_loop` makes the empty `track_L`
or `track_R` carrying `track_length_m` and `link_pitch_m`; every vertex under a
track node scrolls its UVs with the links, so its belt is the only thing under
it.
"""
import math
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl, empty, mesh_part  # noqa: E402
from vehicle_parts import FINE, MID, NEAR, tube_part, wheel_node  # noqa: E402


def tyre_wheel(name, loc, radius, width, side, mats, parent, rim_radius=None, ctis=False, tread="road",
               hub_bolts=0):
    """A wheeled vehicle's wheel: a black treaded tyre, a rim on its outer face
    (`side` +1 for the left, -1 for the right) and a hub, with a CTIS air line
    from hub to rim where fitted. `rim_radius` defaults to the lifted 0.56 of the
    tyre. `tread` is "road" (an armoured carrier's run-flat road tread: blocks
    across the face) or "bar" (a truck's military bar tread: two staggered rows
    of deep chevron bars). `hub_bolts` rings the hub with that many wheel nuts.
    Node: `wheel_*` empty, returned. Roles: rubber, paint, dark, steel."""
    node = wheel_node(name, loc, radius, parent)
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

        tube_part(name=f"{name}_ctis", path_for=air_line, radius=0.008, mat=mats["steel"], parent=node, lods=FINE)
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


# The belt's droop between two supports of its top run, per metre of span: gentle.
TRACK_SAG = 0.03
# Spacing of the points along a sagging span: finer than the belt's finest
# rings (`track_loop`), so the curve and its wrap round a support survive them.
SAG_STEP = 0.05
# How far a sagging belt's inner face stays above a wheel under its span. Clear
# of the lint's 5 mm contact tolerance, small beside a wheel or the belt.
SAG_CLEARANCE = 0.01
# A wheel this close under a straight span carries it: the hull's points are
# rounded to 0.01 mm, so wheels in line touch the span only to within that.
SAG_TOUCH = 1e-4


def _clearance(a, b, wheels, supports):
    """The wheel under the straight span `a`-`b` nearest to it, leaving out
    its two `supports` (wheel indices), as (vertical gap down to its circle,
    its index); (infinity, None) when no other wheel lies under the span. The
    gap to one circle is convex in x, so its least value is where the circle's
    slope matches the span's, held within the overlap of span and circle."""
    (ax, az), (bx, bz) = a, b
    slope = (bz - az) / (bx - ax)
    least = (math.inf, None)
    for i, (x, z, r) in enumerate(wheels):
        lo, hi = max(min(ax, bx), x - r), min(max(ax, bx), x + r)
        if i in supports or lo > hi:
            continue
        at = min(hi, max(lo, x - slope * r / math.hypot(1.0, slope)))
        least = min(least, (az + slope * (at - ax) - z - math.sqrt(max(0.0, r * r - (at - x) ** 2)), i))
    return least


def _outside(wheels, half, point):
    """`point`, pushed straight out of any wheel circle grown by `half` that
    it lies inside onto that circle's edge."""
    px, pz = point
    for x, z, r in wheels:
        gap = math.hypot(px - x, pz - z)
        if 0 < gap < r + half:
            px, pz = x + (px - x) * (r + half) / gap, z + (pz - z) * (r + half) / gap
    return px, pz


def _hang(a, b, supports, wheels, half, sag):
    """The points strictly between `a` and `b` of the belt hanging from the
    wheels `supports` there. A wheel the straight span touches carries it too,
    so the span hangs from it in two. Otherwise it is a parabola dropping
    `sag` times its length at its middle, held `SAG_CLEARANCE` above the
    nearest wheel under it, and resting on a support's rim where it would dip
    into it."""
    dx, dz = b[0] - a[0], b[1] - a[1]
    span = math.hypot(dx, dz)
    gap, under = _clearance(a, b, wheels, supports)
    if gap - half < SAG_TOUCH:
        x, z, r = wheels[under]
        rest = (x + (r + half) * dz / span, z - (r + half) * dx / span)
        return (_hang(a, rest, (supports[0], under), wheels, half, sag) + [rest]
                + _hang(rest, b, (under, supports[1]), wheels, half, sag))
    drop = min(sag * span, gap - half - SAG_CLEARANCE)
    if drop <= 0:
        return []
    n = max(2, round(span / SAG_STEP))
    return [_outside(wheels, half, (a[0] + dx * k / n, a[1] + dz * k / n - 4 * drop * k / n * (1 - k / n)))
            for k in range(1, n)]


def track_path(wheels, thickness, sag=TRACK_SAG):
    """The closed counter-clockwise centre line of a belt `thickness` thick
    wrapped round `wheels`, (x, z, radius) circles in the side plane: the
    convex hull of the circles grown by half the thickness, so the lower run
    lies under the road wheels and the ends wrap the sprocket and idler. Each
    span of its top run from one wheel to another (outward normal more up than
    sideways) sags between them (`_hang`). The lint can't see a belt cut into
    its own wheels, so the belt keeps clear of them here: the sag is held
    above the wheels under a span (a span with no room stays straight), and
    where the curve leaves a support it wraps further round its rim, as a
    sagging belt does."""
    half = thickness / 2
    owner = {}
    for i, (x, z, r) in enumerate(wheels):
        for k in range(96):
            a = k * math.tau / 96
            owner.setdefault((round(x + (r + half) * math.cos(a), 5), round(z + (r + half) * math.sin(a), 5)), i)
    hull = _convex_hull_2d(owner)
    path = []
    for i, a in enumerate(hull):
        b = hull[(i + 1) % len(hull)]
        path.append(a)
        supports = (owner[a], owner[b])
        if supports[0] != supports[1] and a[0] - b[0] > 0.5 * math.dist(a, b):
            path += _hang(a, b, supports, wheels, half, sag)
    return path


def track_loop(name, wheels, y, width, pitch, mats, parent, thickness=0.08, sag=TRACK_SAG):
    """A closed track belt wrapped round every wheel it touches, as a real run
    is: `wheels` are (x, z, radius) circles in the side plane (road wheels,
    sprocket, idler, return rollers), and the belt's centre line is
    `track_path`: the lower run lies under the road wheels, the ends wrap the
    sprocket and idler and the upper run rests on the rollers, sagging
    between them by `sag`. Centred across on `y`, `width` wide. Links are the
    'track' texture only: u runs along the belt in links, v across it. Node:
    the empty `name` (`track_L` or `track_R`) at the parent's origin, carrying
    `track_length_m` (the length of that centre line) and `link_pitch_m`,
    returned; its belt is its only mesh. Roles: track."""
    if name not in ("track_L", "track_R"):
        raise ValueError(f"a track node is named track_L or track_R: {name}")
    loop = track_path(wheels, thickness, sag)
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
