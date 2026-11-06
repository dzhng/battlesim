"""What an airframe carries under its wings and body: the pylons it hangs
stores from, and the stores themselves (missiles, bombs, drop tanks, pods,
rocket launchers), each in its own material role.

`aircraft_parts` imports `pylon` and `store`, so roster scripts reach them
through it as they reach every other airframe part.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from aircraft_body import body  # noqa: E402
from aircraft_markings import mark  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_parts import ALL, FINE, MID, NEAR  # noqa: E402


def pylon(name, loc, length, depth, mats, parent):
    """A store pylon hanging `depth` below `loc`, `length` long."""
    x, y, z = loc
    box(name, (length, 0.1, depth), (x, y, z - depth / 2), mats["paint"], parent, bevel=0.02, taper=(1.15, 1.0),
        lods=MID)
    # The sway braces either end of the ejector rack, under the pylon.
    for k, dx in enumerate((length * 0.3, -length * 0.3)):
        box(f"{name}_sway_{k}", (0.05, 0.16, 0.08), (x + dx, y, z - depth - 0.02), mats["dark"], parent, lods=FINE)


def store(name, kind, loc, length, radius, mats, parent, fins=True, rail=False):
    """A store hung with its nose at +X, centred on `loc`: `missile` (a slim
    body, a pointed seeker, canards and tail fins, its live bands), `bomb`
    (a fat body, its fuze and tail fins), `tank` (a drop tank, in the
    airframe's paint as an air arm paints its tanks), `pod` (a sensor pod
    with its window) or `rockets` (a blunt rocket launcher, its tubes' mouths
    in its dark front face); all but the tank in the stores' grey. `rail`
    hangs it from a launch rail."""
    x, y, z = loc
    h = length / 2
    nose = {"missile": 0.12, "bomb": 0.25, "tank": 0.3, "pod": 0.15, "rockets": 0.04}[kind]
    tail = {"missile": 0.98, "bomb": 0.8, "tank": 0.75, "pod": 0.9, "rockets": 0.94}[kind]
    st = [(x + h, 0.0, z, z), (x + h - length * nose, radius, z - radius, z + radius),
          (x - h + length * (1 - tail), radius, z - radius, z + radius),
          (x - h, radius * (0.4 if kind != "missile" else 0.95), z - radius * 0.5, z + radius * 0.5)]
    body(f"{name}_body", st, mats["paint" if kind == "tank" else "store"], parent, seg=12,
         lods=MID if kind == "missile" else ALL,
         loc=(0, y, 0), steps=(2, 1, 1, 1))
    if kind == "missile":
        cyl(f"{name}_seeker", radius * 0.7, 0.02, (x + h - 0.03, y, z), "X", mats["glass"], parent, seg=8, lods=FINE)
        # The live bands: yellow behind the warhead, brown on the motor.
        for k, (at, colour) in enumerate(((0.3, "yellow"), (0.55, "brown"))):
            cyl(f"{name}_band_{k}", radius * 1.02, 0.06, (x + h - length * at, y, z), "X", mark(mats, colour), parent,
                seg=12, lods=FINE)
    if kind == "bomb":
        cyl(f"{name}_fuze", radius * 0.18, 0.1, (x + h + 0.04, y, z), "X", mats["steel"], parent, seg=8, lods=NEAR)
        cyl(f"{name}_band", radius * 1.02, 0.05, (x + h - length * 0.3, y, z), "X", mark(mats, "yellow"), parent,
            seg=12, lods=FINE)
    if kind == "pod":
        box(f"{name}_window", (0.03, radius * 1.1, radius * 1.1), (x + h - length * nose * 0.4, y, z),
            mats["glass"], parent, lods=MID)
    if kind == "rockets":
        # The front face, and a ring of tube mouths round a centre one.
        cyl(f"{name}_face", radius * 0.94, 0.02, (x + h + 0.01, y, z), "X", mats["black"], parent, seg=14, lods=MID)
        for j in range(7):
            a = math.tau * j / 6
            dy, dz = (0.0, 0.0) if j == 6 else (math.cos(a) * radius * 0.58, math.sin(a) * radius * 0.58)
            cyl(f"{name}_tube_{j}", radius * 0.2, 0.03, (x + h + 0.02, y + dy, z + dz), "X", mats["steel"], parent,
                seg=8, lods=FINE)
            cyl(f"{name}_mouth_{j}", radius * 0.13, 0.03, (x + h + 0.03, y + dy, z + dz), "X", mats["black"], parent,
                seg=8, lods=FINE)
    if rail:
        box(f"{name}_rail", (length * 0.6, 0.07, radius + 0.06), (x - length * 0.05, y, z + radius + 0.01),
            mats["dark"], parent, bevel=0.01, lods=MID)
    if fins and kind in ("missile", "bomb"):
        span = radius * (2.4 if kind == "missile" else 1.8)
        rows = [(length * 0.08, length * 0.12, span, "fin")]
        if kind == "missile":
            rows.append((length * 0.78, length * 0.07, span * 0.75, "canard"))
        for at, chord, sp, what in rows:
            for k in range(4):
                a = math.pi / 4 + k * math.pi / 2
                box(f"{name}_{what}_{k}", (chord, 0.015, sp),
                    (x - h + at, y + math.cos(a) * sp / 2, z + math.sin(a) * sp / 2), mats["store"], parent,
                    rot=(a - math.pi / 2, 0, 0), lods=NEAR)
