"""Slice 03's smoothed candidate: a patch to `t72.dome_rings` (and the loft
that draws it), kept out of the shipped script.

The current dome is five rings of 20 points, flat shaded, with the plan's
front and rear radii switching at the flanks. The patch keeps the same five
rings as control stations and only adds surface between them:

- the profile runs through the five stations on a Hermite curve (Catmull-Rom
  tangents, the end tangents along the end chords); the fit probe measured
  it at most 0.038 m from the current dome;
- the plan blends its front and rear radii smoothly across the flanks
  (rx = rear + (front - rear) * (1 + cos a) / 2), so there is no kink at
  cos a = 0;
- 48 points a ring at tier 0, four sub-rings between stations, smooth
  shaded, the roof poked to a centre vertex so the roof shades as one gentle
  dome.

Tiers scale the ring points with `SEG_SCALE` and the sub-rings 4, 2, 1, 1.

This is the exact code that built `candidates/smoothed.glb`, run from the
slice's runner, which swapped it in for the `turret_shell` loft. It
reaches into `parts`' private `_each` and `_obj` only because `loft` can't
shade smooth. When slice 17 adopts it, `parts.mesh_part(..., smooth=True)`
does the same without them; the bottom cap it also smooths sits below the
deck.
"""
import math

import bmesh

from parts import SEG_SCALE, TIERS, _each, _obj

# (z, front radius, rear radius, half-width): t72.dome_rings' five stations.
STATIONS = [(-0.04, 1.55, 1.40, 1.32), (0.30, 1.52, 1.40, 1.30), (0.56, 1.30, 1.22, 1.12),
            (0.76, 0.95, 0.95, 0.88), (0.82, 0.60, 0.60, 0.56)]
SHIFT = 0.05
POINTS = 48
SUB = (4, 2, 1, 1)


def _profile(sub):
    """The stations with `sub` Hermite steps between each pair."""
    p = STATIONS
    n = len(p)

    def tangent(i):
        if i == 0:
            return [b - a for a, b in zip(p[0], p[1])]
        if i == n - 1:
            return [b - a for a, b in zip(p[-2], p[-1])]
        return [(b - a) / 2 for a, b in zip(p[i - 1], p[i + 1])]

    out = []
    for i in range(n - 1):
        m0, m1 = tangent(i), tangent(i + 1)
        for k in range(sub):
            t = k / sub
            h00, h10 = 2 * t ** 3 - 3 * t ** 2 + 1, t ** 3 - 2 * t ** 2 + t
            h01, h11 = -2 * t ** 3 + 3 * t ** 2, t ** 3 - t ** 2
            out.append(tuple(h00 * a + h10 * ma + h01 * b + h11 * mb
                             for a, b, ma, mb in zip(p[i], p[i + 1], m0, m1)))
    out.append(p[-1])
    return out


def _ring(rf, rr, ry, n):
    pts = []
    for k in range(n):
        a = k * math.tau / n
        c = math.cos(a)
        rx = rr + (rf - rr) * (1 + c) / 2
        pts.append((SHIFT + rx * c, ry * math.sin(a)))
    return pts


def dome_rings(lod=0):
    """The patched `t72.dome_rings`: the same stations, densely and smoothly lofted."""
    n = max(12, 2 * round(POINTS * SEG_SCALE[lod] / 2))
    return [(z, _ring(rf, rr, ry, n)) for z, rf, rr, ry in _profile(SUB[lod])]


def smooth_loft(name, mat=None, parent=None, lods=TIERS):
    """The dome as a smooth-shaded closed loft, its roof poked to a centre vertex."""

    def make(lod, nm):
        rings = dome_rings(lod)
        bm = bmesh.new()
        vs = [[bm.verts.new((x, y, z)) for x, y in pts] for z, pts in rings]
        bottom = bm.faces.new(vs[0][::-1])
        top = bm.faces.new(vs[-1])
        for r in range(len(vs) - 1):
            a, b = vs[r], vs[r + 1]
            for i in range(len(a)):
                j = (i + 1) % len(a)
                bm.faces.new((a[i], a[j], b[j], b[i])).smooth = True
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        poked = bmesh.ops.poke(bm, faces=[top])
        for f in poked["faces"]:
            f.smooth = True
        bottom.smooth = False
        return _obj(nm, bm, mat, parent, (0, 0, 0), (0, 0, 0))

    return _each(name, lods, make)
