"""What stands on a hull or turret roof: hatches, cupolas, periscopes,
sight housings, roof rails, laser warners, smoke discharger banks and antennas.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import math
import os
import sys

from mathutils import Euler, Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl, empty  # noqa: E402
from vehicle_parts import FINE, MID, NEAR, place, tube_part  # noqa: E402


def roof_fittings(name, crown, top, mats, parent, periscopes=(), rails=True):
    """What a turret roof carries whatever its shape: a lifting eye at each
    corner of its `crown` ring (plan points at height `top`), a grab rail
    along each side's rear half, and a vision block at each (x, y, yaw) of
    `periscopes`. Returns its meshes. Roles: steel, paint, glass."""
    xs = [x for x, _ in crown]
    half = max(abs(y) for _, y in crown)
    front, rear = max(xs), min(xs)
    made = []
    for k, (x, y) in enumerate(((front - 0.30, half * 0.55), (front - 0.30, -half * 0.55),
                                (rear + 0.18, half - 0.18), (rear + 0.18, -(half - 0.18)))):
        made += box(f"{name}_eye_{k}", (0.12, 0.035, 0.09), (x, y, top + 0.045), mats["steel"], parent, lods=FINE)
    if rails:
        x0, x1 = rear + 0.30, (front + rear) / 2
        for side in (-1, 1):
            y = side * (half - 0.16)
            made += tube_part(f"{name}_rail_{side}", lambda lod, y=y: [(x0, y, top), (x0, y, top + 0.08),
                                                                       (x1, y, top + 0.08), (x1, y, top)],
                              radius=0.014, mat=mats["steel"], parent=parent, lods=NEAR)
    for k, (x, y, yaw) in enumerate(periscopes):
        made += periscope(f"{name}_periscope_{k}", (x, y, top), mats, parent, rot=(0, 0, yaw))
    return made


def laser_warners(name, crown, top, mats, parent, inset=0.35):
    """Laser-warning sensor heads on the four corners of a turret roof's
    `crown` ring at height `top`: an armoured head with a dark-glass band.
    Returns its meshes. Roles: dark, glass."""
    xs = [x for x, _ in crown]
    half = max(abs(y) for _, y in crown)
    made = []
    for k, (x, y) in enumerate(((max(xs) - inset, half - (inset - 0.05)), (max(xs) - inset, -(half - (inset - 0.05))),
                                (min(xs) + inset, half - (inset - 0.05)), (min(xs) + inset, -(half - (inset - 0.05))))):
        made += box(f"{name}_{k}", (0.12, 0.12, 0.10), (x, y, top + 0.05), mats["dark"], parent, bevel=0.015, lods=MID)
        made += box(f"{name}_glass_{k}", (0.13, 0.08, 0.05), (x, y, top + 0.07), mats["glass"], parent, lods=NEAR)
    return made


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

    made += tube_part(name=f"{name}_handle", path_for=handle, radius=0.01, mat=mats["steel"], parent=parent,
                       lods=FINE)
    return place(made, loc, rot)


def cupola(name, loc, mats, parent, radius=0.36, periscopes=6, height=0.09, rot=(0, 0, 0)):
    """A commander's or loader's cupola on the roof at `loc`: a raised ring
    with `periscopes` vision blocks round its front, its lid closed on the
    ring. Returns its meshes. Roles: paint, glass."""
    made = cyl(f"{name}_ring", radius + 0.04, height, (0, 0, height / 2), "Z", mats["paint"], parent, seg=28,
               bevel=0.015, lods=MID)
    for k in range(periscopes):
        a = -1.6 + k * 3.2 / max(1, periscopes - 1)
        made += periscope(f"{name}_periscope_{k}", (radius * math.cos(a), radius * math.sin(a), height - 0.01),
                          mats, parent, size=(0.10, 0.12, 0.07), rot=(0, 0, a))
    made += cyl(f"{name}_lid", radius * 0.92, 0.06, (0, 0, height + 0.03), "Z", mats["paint"], parent, seg=28,
                bevel=0.015, lods=MID)
    return place(made, loc, rot)


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
    return place(made, loc, rot)


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
    return place(made, loc, rot)


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
    return place(made, loc, rot)


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
    return place(made, loc, rot)


def whip_pair(parent, at, mats, height=2.4):
    """A whip antenna either side of the centre line (`antenna`): at `at`
    (x, y, z) on the left, mirrored on the right, each on its own
    `dressing_antenna_L` / `_R` node under `parent`."""
    x, y, z = at
    for side, s in ((1, "L"), (-1, "R")):
        whip = empty(f"dressing_antenna_{s}", parent=parent)
        antenna(f"antenna_{s}", (x, side * y, z), mats, whip, height=height)
