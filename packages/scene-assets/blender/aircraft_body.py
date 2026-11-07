"""The lofted body every airframe is built from: a fuselage, a nacelle, a
boom, a store's case. Stations along X, each a superellipse section,
interpolated ring by ring at the near tiers (`BODY_STEPS`) and closed to a
point where a station has no width.

`aircraft_parts` imports `body`, so roster scripts reach it through it as
they reach every other airframe part.
"""
import math
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import SEG_SCALE, mesh_part  # noqa: E402
from vehicle_parts import ALL  # noqa: E402

# Interpolated rings per station interval, per tier: a fuselage keeps its
# curves at the near tiers and its stations alone at the far ones.
BODY_STEPS = (3, 2, 1, 1)


def _station(s):
    x, w, lo, hi = s[:4]
    mid = s[4] if len(s) > 4 and s[4] is not None else (lo + hi) / 2
    p = s[5] if len(s) > 5 else 2.0
    return [x, w, lo, hi, mid, p]


def _spline(stations, steps):
    """Stations with `steps` Catmull-Rom rings per interval (x stays linear)."""
    if steps <= 1:
        return stations
    out = []
    n = len(stations)
    for i in range(n - 1):
        p0, p1, p2, p3 = (stations[max(i - 1, 0)], stations[i], stations[i + 1], stations[min(i + 2, n - 1)])
        for k in range(steps):
            t = k / steps
            row = [p1[0] + (p2[0] - p1[0]) * t]
            for j in range(1, 6):
                a, b, c, d = p0[j], p1[j], p2[j], p3[j]
                val = 0.5 * ((2 * b) + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t * t + (-a + 3 * b - 3 * c + d) * t ** 3)
                lo_, hi_ = min(b, c), max(b, c)
                row.append(min(max(val, lo_ - 0.25 * (hi_ - lo_)), hi_ + 0.25 * (hi_ - lo_)))
            row[1] = max(row[1], 0.0)
            out.append(row)
    out.append(stations[-1])
    return out


def _ring(st, n):
    x, w, lo, hi, mid, p = st
    pts = []
    for k in range(n):
        a = math.tau * k / n
        c, s = math.cos(a), math.sin(a)
        cy = math.copysign(abs(c) ** (2 / p), c)
        sz = math.copysign(abs(s) ** (2 / p), s)
        z = mid + (hi - mid) * sz if sz >= 0 else mid + (mid - lo) * sz
        pts.append((x, w * cy, z))
    return pts


def body(name, stations, mat, parent, seg=24, lods=ALL, loc=(0, 0, 0), rot=(0, 0, 0), steps=BODY_STEPS):
    """A smooth body lofted along X through `stations`, each
    `(x, half_width, bottom_z, top_z[, widest_z[, power]])`: a superellipse
    section (power 2 an ellipse, higher squarer, below 2 a chined diamond)
    widest at `widest_z`. A station of no width closes the body to a point;
    otherwise its ends are capped. Fuselages, nacelles, booms, stores."""
    st = [_station(s) for s in stations]

    def build(bm, lod):
        n = max(6, int(round(seg * SEG_SCALE[lod] / 2)) * 2)
        rows = _spline(st, steps[lod])
        rings = []
        for row in rows:
            if row[1] < 1e-3:
                rings.append([bm.verts.new((row[0], 0.0, row[4]))])
            else:
                rings.append([bm.verts.new(p) for p in _ring(row, n)])
        for a, b in zip(rings, rings[1:]):
            if len(a) == 1 and len(b) == 1:
                continue
            if len(a) == 1:
                for k in range(n):
                    bm.faces.new((a[0], b[(k + 1) % n], b[k]))
            elif len(b) == 1:
                for k in range(n):
                    bm.faces.new((a[k], a[(k + 1) % n], b[0]))
            else:
                for k in range(n):
                    bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))
        for f in bm.faces:
            f.smooth = True
        for ring in (rings[0], rings[-1]):
            if len(ring) > 1:
                cap = bm.faces.new(ring)
                cap.smooth = False
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)

    return mesh_part(name, build, mat, parent, lods=lods, loc=loc, rot=rot)
