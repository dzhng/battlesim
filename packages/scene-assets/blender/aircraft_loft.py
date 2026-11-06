"""A body lofted through drawn cross-sections, for the airframes whose
sections no superellipse (`aircraft_body.body`) makes: a blended wing-body
whose strakes run out to a sharp edge, a forebody flat under its canopy, a
flat-bottomed intake duct, an engine bay between flat tail booms.

A station is `(x, outline[, rake])`. Its outline is the half section on the
left (+Y) side, from the top of the centreline round to its bottom, as
`(y, z)` points: the same number at every station, so that each point runs
the body's length as one line (the strake's edge stays one edge). A point
`(y, z, True)` is a crease: the outline turns there and the shading keeps
the line sharp. The outline is mirrored across the centreline and smoothed
through its points; stations are interpolated ring by ring at the near
tiers as `body`'s are. `rake` leans a station forward by `rake` metres per
metre above its lowest point, for a raked intake's mouth.

A station whose points all lie on the centreline at one height closes the
body to a point; otherwise its ends are capped, unless `caps` says not (a
shell open at its ends: a nozzle's petals round the pipe inside them).
"""
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from aircraft_body import BODY_STEPS  # noqa: E402
from parts import mesh_part  # noqa: E402
from vehicle_parts import ALL  # noqa: E402

# Smoothed points per span between two outline points, per tier.
OUTLINE_STEPS = (6, 3, 1, 1)


def _cr(a, b, c, d, t):
    return 0.5 * ((2 * b) + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t * t + (-a + 3 * b - 3 * c + d) * t ** 3)


def _loop(outline):
    """The whole section as a closed loop: the half outline, then its mirror
    back up the right side; the centreline points are not repeated."""
    pts = [(p[0], p[1], len(p) > 2 and bool(p[2])) for p in outline]
    return pts + [(-y, z, c) for y, z, c in pts[-2:0:-1]]


def _ring(loop, steps):
    """The loop smoothed through its points (Catmull-Rom, `steps` per span),
    turning sharply at a crease: a span beside one runs straight into it.
    Returns the ring's (y, z) points and which of them are creases."""
    n = len(loop)
    out, creases = [], []
    for i in range(n):
        p1, p2 = loop[i], loop[(i + 1) % n]
        p0 = p1 if p1[2] else loop[i - 1]
        p3 = p2 if p2[2] else loop[(i + 2) % n]
        for k in range(steps):
            t = k / steps
            out.append((_cr(p0[0], p1[0], p2[0], p3[0], t), _cr(p0[1], p1[1], p2[1], p3[1], t)))
            creases.append(k == 0 and p1[2])
    return out, creases


def _stations(stations, steps):
    """Stations with `steps` Catmull-Rom rings per interval (x stays linear),
    each ring `(x, rake, [(y, z)...])`, clamped as `body` clamps its rings."""
    if steps <= 1:
        return stations
    out = []
    n = len(stations)
    for i in range(n - 1):
        s0, s1, s2, s3 = stations[max(i - 1, 0)], stations[i], stations[i + 1], stations[min(i + 2, n - 1)]
        for k in range(steps):
            t = k / steps
            pts = []
            for j in range(len(s1[2])):
                row = []
                for a in range(2):
                    b, c = s1[2][j][a], s2[2][j][a]
                    v = _cr(s0[2][j][a], b, c, s3[2][j][a], t)
                    lo, hi = min(b, c), max(b, c)
                    row.append(min(max(v, lo - 0.25 * (hi - lo)), hi + 0.25 * (hi - lo)))
                pts.append(tuple(row))
            out.append((s1[0] + (s2[0] - s1[0]) * t, s1[1] + (s2[1] - s1[1]) * t, pts))
    out.append(stations[-1])
    return out


def loft(name, stations, mat, parent, lods=ALL, loc=(0, 0, 0), rot=(0, 0, 0), steps=BODY_STEPS,
         outline_steps=OUTLINE_STEPS, caps=True):
    """A smooth body through `stations` (see the module's doc), one part per
    tier: fuselages a superellipse can't draw."""
    counts = {len(s[1]) for s in stations}
    if len(counts) != 1:
        raise ValueError(f"{name}: every station needs the same number of outline points, got {sorted(counts)}")

    def build(bm, lod):
        rings, crease = [], None
        for st in stations:
            pts, crease = _ring(_loop(st[1]), outline_steps[lod])
            rings.append((st[0], st[2] if len(st) > 2 else 0.0, pts))
        verts = []
        for x, rake, pts in _stations(rings, steps[lod]):
            lowest = min(z for _, z in pts)
            if max(abs(y) for y, _ in pts) < 1e-3 and max(z for _, z in pts) - lowest < 1e-3:
                verts.append([bm.verts.new((x, 0.0, lowest))])
            else:
                verts.append([bm.verts.new((x + rake * (z - lowest), y, z)) for y, z in pts])
        n = len(crease)
        for a, b in zip(verts, verts[1:]):
            if len(a) == 1 and len(b) == 1:
                continue
            for k in range(n):
                if len(a) == 1:
                    bm.faces.new((a[0], b[(k + 1) % n], b[k]))
                elif len(b) == 1:
                    bm.faces.new((a[k], a[(k + 1) % n], b[0]))
                else:
                    bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))
                    if crease[k]:
                        bm.edges.get((a[k], b[k])).smooth = False
        for f in bm.faces:
            f.smooth = True
        for ring in (verts[0], verts[-1]) if caps else ():
            if len(ring) > 1:
                bm.faces.new(ring).smooth = False
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, parent, lods=lods, loc=loc, rot=rot)
