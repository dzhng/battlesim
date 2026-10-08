"""The wheeled chassis the disabled support cards stand on: frame rails,
axles and wheels, fenders, a cab in one of the four shapes the references
show, and the fittings every truck carries (fuel tank, battery box, spare
wheel, lights, mirrors, mudflaps). A family script (`rocket_artillery.py`,
`caesar.py`, `air_defence.py`) places its mission load on the deck.

Cab shapes, from the references:
- `bonnet`: a Ural-375D/4320 style cab behind a long bonnet (BM-21);
- `armoured`: a flat-fronted armoured cab-over with small windows (the
  HIMARS's FMTV, the CAESAR's Tatra, the Pantsir's KamAZ);
- `maz_split`: the MAZ-543's two cabs either side of the engine (Tornado-S).

Every part reads the roles `vehicle_parts` documents.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID


def chassis(v, axles, wheel_r, wheel_w, track, rail_z, cab, cab_x, cab_len, cab_h, deck_z=None):
    """The truck under a load: `axles` (x of each), wheels of radius
    `wheel_r` and width `wheel_w` at ±`track`, frame rails at `rail_z`, a
    `cab` of its shape ahead of `cab_x` (its rear face) `cab_len` long and
    `cab_h` tall. Returns the deck's height (the rails' top, or `deck_z`)."""
    m, h = v.mats, v.hull
    half = v.length / 2
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate(axles):
            VP.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * track, wheel_r), wheel_r, wheel_w, side, m, h,
                          rim_radius=wheel_r * 0.55, ctis=True)
        box(f"frame_rail_{s}", (v.length - 0.6, 0.12, 0.30), (-0.2, side * 0.50, rail_z), m["dark"], h, bevel=0.01)
        for k, x in enumerate(axles):
            box(f"axle_{s}_{k}", (0.20, track - 0.45, 0.16), (x, side * (track / 2 + 0.05), wheel_r), m["dark"], h,
                lods=MID)
            box(f"spring_{s}_{k}", (0.9, 0.10, 0.10), (x, side * 0.50, rail_z - 0.22), m["dark"], h, lods=NEAR)
            # A fender over each wheel.
            box(f"fender_{s}_{k}", (wheel_r * 2.3, wheel_w + 0.12, 0.04),
                (x, side * track, wheel_r * 2 + 0.10), m["paint"], h, bevel=0.01, lods=MID)
        VP.mudflap(f"mudflap_{s}", (axles[-1] - wheel_r - 0.15, side * track, wheel_r + 0.55), (wheel_w + 0.05, 0.5),
                   m, h)
        VP.light_with_guard(f"tail_light_{s}", (-half + 0.05, side * (track - 0.05), rail_z + 0.05), 0.05,
                            dict(m, lamp=m["tail"]), h, rot=(0, 0, math.pi))
    box("rear_bumper", (0.12, track * 2 - 0.2, 0.14), (-half + 0.06, 0, rail_z - 0.05), m["dark"], h, bevel=0.01)
    VP.tow_hook("rear_tow", (-half + 0.02, 0, rail_z - 0.05), m, h, size=0.13, rot=(0, 0, math.pi))
    # Fuel tank and battery box between the cab and the first rear axle.
    gap = (cab_x + axles[1]) / 2 if len(axles) > 1 else cab_x - 0.6
    cyl("fuel_tank", 0.24, 0.95, (gap, 0.85, rail_z - 0.05), "X", m["paint"], h, seg=18, bevel=0.02)
    VP.stowage_box("battery_box", (gap, -0.85, rail_z - 0.30), (0.60, 0.30, 0.40), m, h, rot=(0, 0, math.pi))
    cab_shape(v, cab, cab_x, cab_len, cab_h, track, rail_z, wheel_r)
    return deck_z if deck_z is not None else rail_z + 0.15


def cab_shape(v, cab, x0, length, height, track, rail_z, wheel_r):
    m, h = v.mats, v.hull
    half_w = track + 0.05
    front = x0 + length
    floor = rail_z + 0.10
    if cab == "bonnet":
        bonnet = length * 0.42
        loft("cab_body", [(floor, _plan(x0, front - bonnet, half_w - 0.05, 0.08)),
                          (height - 0.55, _plan(x0, front - bonnet, half_w - 0.05, 0.08)),
                          (height, _plan(x0 + 0.05, front - bonnet - 0.20, half_w - 0.20, 0.10))],
             mat=m["paint"], parent=h, bevel=0.03)
        loft("bonnet", [(floor, _plan(front - bonnet, front, 0.55, 0.05)),
                        (floor + 0.85, _plan(front - bonnet, front - 0.05, 0.50, 0.08))],
             mat=m["paint"], parent=h, bevel=0.03)
        VP.grille("radiator_grille", (front + 0.01, 0, floor + 0.45), (0.70, 0.80), m, h, slats=9,
                  rot=(0, -math.pi / 2, 0))
        for side, s in ((1, "L"), (-1, "R")):
            box(f"front_wing_{s}", (bonnet + 0.2, 0.42, 0.05), (front - bonnet / 2, side * (half_w - 0.20),
                                                               wheel_r * 2 + 0.12), m["paint"], h, bevel=0.01)
            VP.light_with_guard(f"headlight_{s}", (front - 0.10, side * 0.75, wheel_r * 2 + 0.25), 0.08, m, h)
        windscreen_x, windscreen_z = front - bonnet - 0.12, height - 0.30
        box("windscreen", (0.02, half_w * 1.6, 0.42), (windscreen_x, 0, windscreen_z), m["glass"], h,
            rot=(0, -0.15, 0), lods=MID)
        box("front_bumper", (0.14, half_w * 2, 0.16), (front + 0.05, 0, floor - 0.05), m["dark"], h, bevel=0.01)
    elif cab == "maz_split":
        for side, s in ((1, "L"), (-1, "R")):
            loft(f"cab_{s}", [(floor, _plan(x0, front, 0.62, 0.10, side * (half_w - 0.62))),
                              (height, _plan(x0 + 0.10, front - 0.30, 0.55, 0.12, side * (half_w - 0.62)))],
                 mat=m["paint"], parent=h, bevel=0.04)
            box(f"windscreen_{s}", (0.02, 0.95, 0.45), (front - 0.18, side * (half_w - 0.62), height - 0.40),
                m["glass"], h, rot=(0, -0.45, 0), lods=MID)
            VP.light_with_guard(f"headlight_{s}", (front + 0.02, side * (half_w - 0.35), floor + 0.30), 0.08, m, h)
        box("engine_hood", (length, 0.80, height - floor - 0.25), (x0 + length / 2, 0, floor + (height - floor - 0.25)
                                                                    / 2), m["paint"], h, bevel=0.04)
        VP.grille("radiator_grille", (front + 0.01, 0, floor + 0.55), (0.75, 0.70), m, h, slats=9,
                  rot=(0, -math.pi / 2, 0))
        box("front_bumper", (0.14, half_w * 2, 0.20), (front + 0.05, 0, floor - 0.05), m["dark"], h, bevel=0.01)
    else:  # armoured cab-over
        loft("cab_body", [(floor, _plan(x0, front, half_w, 0.10)),
                          (height - 0.50, _plan(x0, front, half_w, 0.10)),
                          (height, _plan(x0 + 0.05, front - 0.25, half_w - 0.12, 0.12))],
             mat=m["paint"], parent=h, bevel=0.04)
        for k, y in enumerate((0.42, -0.42)):
            box(f"windscreen_{k}", (0.03, 0.70, 0.40), (front - 0.10, y, height - 0.35), m["glass"], h,
                rot=(0, -0.20, 0), lods=MID)
        VP.grille("radiator_grille", (front + 0.01, 0, floor + 0.40), (0.90, 0.45), m, h, slats=8,
                  rot=(0, -math.pi / 2, 0))
        for side, s in ((1, "L"), (-1, "R")):
            VP.light_with_guard(f"headlight_{s}", (front + 0.02, side * (half_w - 0.30), floor + 0.25), 0.07, m, h)
            box(f"side_window_{s}", (0.45, 0.02, 0.32), (front - 0.55, side * (half_w + 0.005), height - 0.40),
                m["glass"], h, lods=MID)
            VP.bolted_plate(f"cab_door_armour_{s}", (front - 0.75, side * (half_w + 0.02), floor + 0.35),
                            (0.85, 0.04, 0.70), m, h, bolts=(2, 1), bevel=0.01)
        box("front_bumper", (0.16, half_w * 2, 0.20), (front + 0.05, 0, floor - 0.05), m["dark"], h, bevel=0.01)
        VP.hatch("cab_roof_hatch", (front - 0.80, 0.35, height), m, h, radius=0.28)
    # Mirrors at the windscreen's corners: on a bonneted cab that is behind the bonnet.
    mirror_x = front - length * 0.42 - 0.15 if cab == "bonnet" else front - 0.20
    for side, s in ((1, "L"), (-1, "R")):
        mirror = empty(f"dressing_mirror_{s}", parent=h)
        box(f"mirror_arm_{s}", (0.04, 0.30, 0.04), (mirror_x, side * (half_w + 0.10), height - 0.45), m["dark"],
            mirror, lods=MID)
        box(f"mirror_{s}", (0.04, 0.10, 0.22), (mirror_x, side * (half_w + 0.25), height - 0.55), m["dark"],
            mirror, lods=MID)
        VP.tow_hook(f"front_tow_{s}", (front + 0.12, side * 0.60, floor - 0.10), m, h, size=0.12)
    whip = empty("dressing_antenna_cab", parent=h)
    VP.antenna("antenna_cab", (x0 + 0.20, -(half_w - 0.15), height), m, whip, height=2.2)


def _plan(rear, front, half, chamfer, y=0.0):
    return [(rear, y - half), (front - chamfer, y - half), (front, y - half + chamfer), (front, y + half - chamfer),
            (front - chamfer, y + half), (rear, y + half)]


def spare_wheel(v, loc, radius, width, rot=(0, 0, 0)):
    """A spare wheel carried upright behind the cab (static: it does not spin)."""
    m, h = v.mats, v.hull
    cyl("spare_tyre", radius, width, loc, "Y", m["rubber"], h, seg=24, rot=rot)
    cyl("spare_rim", radius * 0.55, width + 0.02, loc, "Y", m["paint"], h, seg=18, rot=rot, lods=MID)
