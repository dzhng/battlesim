"""Probe: build an Outline for every tracked hull (slice 09 step 1).
Captures every prism/loft named hull* while each tracked appearance builds."""
import json, os, sys
sys.path.insert(0, os.path.abspath("../packages/scene-assets/blender"))
import parts
calls = []
_prism, _loft = parts.prism, parts.loft
def prism(name, profile_xz, width, loc=(0, 0, 0), *a, **k):
    if name.startswith("hull"):
        calls.append(("prism", name, [tuple(p) for p in profile_xz], tuple(loc), tuple(k.get("rot", (0, 0, 0)))))
    return _prism(name, profile_xz, width, loc, *a, **k)
def loft(name, rings, *a, **k):
    if name.startswith("hull"):
        calls.append(("loft", name, [(z, [tuple(p) for p in pts]) for z, pts in rings], tuple(k.get("loc", (0, 0, 0))), tuple(k.get("rot", (0, 0, 0)))))
    return _loft(name, rings, *a, **k)
parts.prism, parts.loft = prism, loft
import vehicle_export
import vehicle_lint_roster as W
from hull_shape import Outline

TRACKED = set(sys.argv[sys.argv.index("--") + 1:][-1].split(","))
failures = []
report = {}
for script, variant, scheme, build, looks in W.appearances(failures):
    if variant["id"] not in TRACKED:
        continue
    calls.clear()
    vehicle_export.build_variant(variant, scheme, build, looks)
    profiles, rings, notes = [], [], []
    for kind, name, shape, loc, rot in calls:
        if any(abs(r) > 1e-9 for r in rot):
            notes.append(f"{name} rotated {rot}")
        if kind == "prism":
            profiles.append((name, [(x + loc[0], z + loc[2]) for x, z in shape]))
        else:
            rings.append((name, [(z + loc[2], [(x + loc[0], y) for x, y in pts]) for z, pts in shape]))
    row = {"script": script, "captured": [n for _, n, *_ in calls], "notes": notes, "parts": {}}
    pieces = []
    for name, p in profiles:
        try:
            Outline(p); row["parts"][name] = "convex"
        except ValueError:
            import hull_shape as HS
            h = HS._hull(p)
            edges = [(a, h[(k + 1) % len(h)]) for k, a in enumerate(h)]
            dents = [(round(q[0], 3), round(q[1], 3), round(min(HS._cross(a, b, q) / HS.math.dist(a, b) for a, b in edges), 3)) for q in p]
            row["parts"][name] = "NON-CONVEX " + str([d for d in dents if d[2] > HS.ON_HULL_M])
        pieces.append(p)
    for name, r in rings:
        try:
            o = Outline.from_rings(r); row["parts"][name] = "convex (rings)"
        except ValueError:
            o = Outline.from_rings(r, envelope=True); row["parts"][name] = "NON-CONVEX (rings)"
        pieces.append(o.points)
    if pieces:
        o = Outline(*pieces, envelope=True)
        row["outline"] = [(round(x, 3), round(z, 3)) for x, z in o.points]
    report[variant["id"]] = row
print("PROBE", json.dumps({"failures": [str(f) for f in failures], "missing": sorted(TRACKED - set(report)), "report": report}, indent=1))
