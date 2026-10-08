"""The wheeled chassis the disabled support cards stand on: frame rails,
axles and wheels, fenders, a cab in one of the four shapes the references
show, and the fittings every truck carries (fuel tank, battery box, spare
wheel, lights, mirrors, mudflaps). A family script (`rocket_artillery.py`,
`caesar.py`, `air_defence.py`) places its mission load on the deck.

The cab shapes, from the references (the BM-21's Ural cab is the roster
Ural's own, `ural.cab`):
- `cabover`: the same cab unarmoured, big windscreen and door windows (the
  Pantsir-SM's KamAZ);
- `armoured`: a flat-fronted armoured cab-over with small windows (the
  HIMARS's FMTV, the CAESAR's Tatra);
- `maz_split`: the MAZ-543's two cabs either side of the engine (Tornado-S).

Every part reads the roles `vehicle_parts` documents.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID


def chassis(v, axles, wheel_r, wheel_w, track, rail_z, cab, cab_x, cab_len, cab_h, deck_z=None, doors=1):
    """The truck under a load: `axles` (x of each), wheels of radius
    `wheel_r` and width `wheel_w` at ±`track`, frame rails at `rail_z`, a
    `cab` of its shape ahead of `cab_x` (its rear face) `cab_len` long and
    `cab_h` tall, with `doors` doors a side (an armoured crew cab's two).
    Returns the deck's height (the rails' top, or `deck_z`)."""
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
    VP.fuel_tank("fuel_tank", (gap, 0.85, rail_z - 0.05), 0.95, 0.24, m, h)
    VP.stowage_box("battery_box", (gap, -0.85, rail_z - 0.30), (0.60, 0.30, 0.40), m, h, rot=(0, 0, math.pi))
    cab_shape(v, cab, cab_x, cab_len, cab_h, track, rail_z, wheel_r, doors)
    return deck_z if deck_z is not None else rail_z + 0.15


def cab_shape(v, cab, x0, length, height, track, rail_z, wheel_r, doors=1):
    """The cab of shape `cab` (`armoured`, `cabover` or `maz_split`), its rear face at
    `x0`, `length` long and `height` tall, with its mirrors and antenna."""
    m, h = v.mats, v.hull
    # An armoured cab spans the vehicle's width; a soft cab its wheels'.
    half_w = v.width / 2 - 0.08 if cab in ("armoured", "cabover") else track + 0.05
    front = x0 + length
    floor = rail_z + 0.10
    if cab == "maz_split":
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
    else:  # a cab-over, armoured or soft
        armoured_cab(v, x0, length, height, floor, wheel_r, doors, soft=cab == "cabover")
    # Mirrors at the windscreen's corners.
    mirror_x = front - 0.20
    for side, s in ((1, "L"), (-1, "R")):
        mirror = empty(f"dressing_mirror_{s}", parent=h)
        box(f"mirror_arm_{s}", (0.04, 0.30, 0.04), (mirror_x, side * (half_w + 0.10), height - 0.45), m["dark"],
            mirror, lods=MID)
        box(f"mirror_{s}", (0.04, 0.10, 0.22), (mirror_x, side * (half_w + 0.25), height - 0.55), m["dark"],
            mirror, lods=MID)
    if cab == "maz_split":
        for side, s in ((1, "L"), (-1, "R")):
            VP.tow_hook(f"front_tow_{s}", (front + 0.12, side * 0.60, floor - 0.10), m, h, size=0.12)
        cab_fittings(v, cab, x0, front, height, half_w, floor, wheel_r)
    whip = empty("dressing_antenna_cab", parent=h)
    VP.antenna("antenna_cab", (x0 + 0.20, -(half_w - 0.15), height), m, whip, height=2.2)


def armoured_cab(v, x0, length, height, floor, wheel_r, doors=1, soft=False):
    """The flat-fronted armoured cab-over the references show on the FMTV
    (HIMARS) and the Tatra (CAESAR): an upright lower
    box to the waist, upper sides leaning in to the roof, the raked front
    with two thick windscreens in frames and their wipers, the bolted door
    with its small armoured window, handle and steps, the grille and lights
    low on the front, the roof hatch, and the dash and seats behind the
    glass. A `soft` cab (the Pantsir-SM's KamAZ-6560) is the same shape
    unarmoured: a tall two-pane windscreen and wide door windows, and no
    bolted plates."""
    m, h = v.mats, v.hull
    half_w = v.width / 2 - 0.08
    front = x0 + length
    waist = height - 0.78
    roof_half = half_w - 0.20
    rake = 0.28
    loft("cab_body", [(floor, _plan(x0, front, half_w, 0.10)),
                      (waist, _plan(x0, front, half_w, 0.10)),
                      (height, _plan(x0 + 0.06, front - rake, roof_half, 0.14))],
         mat=m["paint"], parent=h, bevel=0.04)
    face = ((half_w, waist), (roof_half, height))
    pitch = math.atan2(rake, height - waist)
    # Windscreens: thick panes in raised frames on the raked front, a wiper on each.
    for k, y in enumerate((0.47, -0.47)):
        x, z = front - rake * 0.45, waist + (height - waist) * 0.48
        pane = (0.03, half_w - 0.12, 0.66) if soft else (0.03, 0.80, 0.50)
        box(f"windscreen_{k}", pane, (x + 0.01, y, z), m["glass"], h, rot=(0, -pitch, 0), lods=MID)
        box(f"windscreen_frame_{k}", (0.05, pane[1] + 0.12, pane[2] + 0.12), (x - 0.01, y, z), m["paint"], h,
            rot=(0, -pitch, 0),
            bevel=0.015, lods=NEAR)
        box(f"wiper_{k}", (0.02, 0.03, 0.42), (x + 0.04, y - 0.18, z - 0.02), m["black"], h,
            rot=(-0.5, -pitch, 0), lods=FINE)
    box("windscreen_mullion", (0.06, 0.10, 0.56), (front - rake * 0.45 + 0.01, 0, waist + (height - waist) * 0.48),
        m["paint"], h, rot=(0, -pitch, 0), bevel=0.01, lods=MID)
    # Under the windscreens: the armoured front's bolted plate, grille, lights and bumper.
    if not soft:
        VP.bolted_panel("front_plate", (front + 0.005, 0, waist - 0.25), (0.45, half_w * 1.7, 0.03), m, h,
                        bolts=(2, 6), rot=(0, math.pi / 2, 0), bevel=0.01)
    VP.grille("radiator_grille", (front + 0.04, 0, floor + 0.32), (0.42, half_w * 1.1), m, h, slats=7,
              rot=(0, -math.pi / 2, 0))
    box("front_bumper", (0.18, half_w * 2 + 0.06, 0.22), (front + 0.07, 0, floor - 0.06), m["dark"], h, bevel=0.015)
    for side, s in ((1, "L"), (-1, "R")):
        VP.light_with_guard(f"headlight_{s}", (front + 0.06, side * (half_w - 0.22), floor + 0.30), 0.07, m, h)
        box(f"marker_{s}", (0.03, 0.08, 0.06), (front + 0.02, side * (half_w - 0.06), waist - 0.05), m["lamp"], h,
            lods=FINE)
        VP.shackle(f"front_shackle_{s}", (front + 0.17, side * 0.55, floor - 0.06), m, h, size=0.13,
                   rot=(0, 0, math.pi / 2))
        # Each door: its armour, a small thick window, handle, hinges, steps under it.
        y = side * (half_w + 0.005)
        for d in range(doors):
            door_x = front - 0.70 - d * 1.05
            n = f"{s}_{d}" if d else s
            if soft:
                VP.weld_line(f"door_seam_{n}", [(door_x + 0.48, y, floor + 0.10), (door_x + 0.48, y, waist),
                                                (door_x - 0.48, y, waist), (door_x - 0.48, y, floor + 0.10)], m, h,
                             radius=0.012)
            else:
                VP.bolted_panel(f"door_{n}", (door_x, y, floor + 0.15), (0.95, waist - floor - 0.25, 0.035), m, h,
                                bolts=(3, 3), rot=(-side * math.pi / 2, 0, 0), bevel=0.012)
            loc, rot = VP.on_side(door_x - 0.05, waist + 0.30, side, *face, proud=0.015)
            window = (0.80, 0.55, 0.03) if soft else (0.48, 0.34, 0.03)
            box(f"door_window_{n}", window, loc, m["glass"], h, rot=rot, lods=MID)
            loc, rot = VP.on_side(door_x - 0.05, waist + 0.30, side, *face, proud=0.005)
            box(f"door_window_frame_{n}", (window[0] + 0.10, window[1] + 0.10, 0.03), loc, m["paint"], h, rot=rot,
                bevel=0.01, lods=NEAR)
            box(f"door_handle_{n}", (0.16, 0.04, 0.04), (door_x - 0.36, side * (half_w + 0.06), waist - 0.20),
                m["steel"], h, lods=FINE)
            for k, z in enumerate((floor + 0.05, waist - 0.35)):
                box(f"door_hinge_{n}_{k}", (0.06, 0.05, 0.12), (door_x + 0.49, side * (half_w + 0.05), z + 0.2),
                    m["steel"], h, lods=FINE)
            for k, z in enumerate((wheel_r * 0.75, wheel_r * 0.75 + 0.32)):
                box(f"step_{n}_{k}", (0.42, 0.24, 0.04), (door_x - 0.1, side * (half_w - 0.10), z), m["steel"], h,
                    lods=MID)
            box(f"grab_rail_{n}", (0.03, 0.03, 0.60), (door_x - 0.55, side * (half_w + 0.05), waist - 0.45),
                m["steel"], h, lods=NEAR)
        # The rear quarter window behind the door.
        loc, rot = VP.on_side(x0 + 0.30, waist + 0.30, side, *face, proud=0.012)
        box(f"rear_window_{s}", (0.30, 0.30, 0.03), loc, m["glass"], h, rot=rot, lods=MID)
    # The roof: hatch, marker lamps along its front edge; inside, the dash and seats.
    VP.hatch("cab_roof_hatch", (x0 + 0.75, 0.35, height), m, h, radius=0.30)
    for k in range(3):
        cyl(f"roof_marker_{k}", 0.03, 0.04, (front - rake - 0.06, -0.35 + k * 0.35, height + 0.02), "Z", m["tail"], h,
            seg=8, lods=FINE)
    box("dash", (0.35, half_w * 1.7, 0.20), (front - 0.55, 0, waist - 0.05), m["black"], h, lods=NEAR)
    for side in (-1, 1):
        box(f"seat_back_{side}", (0.12, 0.50, 0.60), (x0 + 0.45, side * 0.50, waist - 0.05), m["black"], h,
            lods=NEAR)


def cab_fittings(v, cab, x0, front, height, half_w, floor, wheel_r):
    """What the MAZ's split cabs carry beside their shape: door seams,
    windows and handles, the steps under the doors, and each cab's dash and
    seat behind its glass."""
    m, h = v.mats, v.hull
    cab_front = front - 0.15
    for side in (1, -1):
        y = side * (half_w + 0.005)
        d0, d1 = x0 + 0.12, cab_front - 0.25
        VP.weld_line(f"door_seam_{side}", [(d1, y, floor + 0.08), (d1, y, height - 0.12), (d0, y, height - 0.12),
                                           (d0, y, floor + 0.08)], m, h, radius=0.012)
        box(f"door_window_{side}", ((d1 - d0) * 0.70, 0.03, 0.40), ((d0 + d1) / 2, y, height - 0.40), m["glass"], h,
            lods=MID)
        box(f"door_handle_{side}", (0.14, 0.04, 0.04), (d0 + 0.15, y + side * 0.02, height - 0.80), m["steel"], h,
            lods=FINE)
        box(f"step_{side}", (0.32, 0.22, 0.04), ((d0 + d1) / 2, y - side * 0.05, wheel_r * 0.8), m["steel"], h,
            lods=MID)
    for side in (-1, 1):
        # Each half cab's dash and seat, beside the engine between them.
        y = side * (half_w - 0.62)
        box(f"dash_{side}", (0.30, 0.90, 0.18), (cab_front - 0.35, y, height - 0.75), m["black"], h, lods=NEAR)
        box(f"seat_back_{side}", (0.12, 0.42, 0.55), (x0 + 0.30, y, height - 0.80), m["black"], h, lods=NEAR)


def _plan(rear, front, half, chamfer, y=0.0):
    """`vehicle_parts.hull_plan`, centred `y` off the centre line (a split cab's half)."""
    return [(x, py + y) for x, py in VP.hull_plan(rear, front, half, chamfer)]


def spare_wheel(v, loc, radius, width, rot=(0, 0, 0)):
    """A spare wheel carried upright behind the cab (static: it does not spin)."""
    m, h = v.mats, v.hull
    cyl("spare_tyre", radius, width, loc, "Y", m["rubber"], h, seg=24, rot=rot)
    cyl("spare_rim", radius * 0.55, width + 0.02, loc, "Y", m["paint"], h, seg=18, rot=rot, lods=MID)
