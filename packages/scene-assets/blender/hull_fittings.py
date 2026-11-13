"""Fittings hung on a hull: lights and their guards, tow hooks, shackles,
tow cables, grilles, exhausts, mudflaps and external fuel tanks.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import math
import os
import sys

from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl, prism  # noqa: E402
from vehicle_parts import FINE, MID, NEAR, arc, place, tube_part  # noqa: E402


def cable(name, points, mats, parent, radius=0.02, eyes=True, loc=(0, 0, 0), rot=(0, 0, 0)):
    """A steel tow cable along `points` (in the part's frame), clipped down
    where it runs, with a loop eye at each end where `eyes`. Returns its
    meshes. Roles: steel, dark."""
    made = tube_part(name=f"{name}_rope", path_for=lambda lod: points, radius=radius, mat=mats["steel"],
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

            made += tube_part(name=f"{name}_eye_{k}", path_for=eye, radius=radius * 0.8, mat=mats["steel"],
                               parent=parent, lods=NEAR)
    for k, p in enumerate(points[1:-1]):
        made += box(name=f"{name}_clip_{k}", size=(0.05, radius * 4, radius * 2.5), loc=p, mat=mats["dark"],
                    parent=parent, lods=FINE)
    return place(made, loc, rot)


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

        made += tube_part(name=f"{name}_guard_{k}", path_for=hoop, radius=0.009, mat=mats["paint"],
                           parent=parent, lods=NEAR)
    return place(made, loc, rot)


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
        points += arc(centre=(size - curl, curl * 0.1), radius=curl, start=-math.pi / 2, end=math.pi * 0.55,
                       steps=steps)
        return points

    made += tube_part(name=f"{name}_hook", path_for=hook, radius=size * 0.13, mat=mats["steel"], parent=parent,
                       lods=NEAR)
    return place(made, loc, rot)


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
        points += arc(centre=(0, -size + half), radius=half, start=math.pi, end=math.tau, steps=steps)
        points.append((half, 0, 0))
        return points

    made += tube_part(name=f"{name}_bow", path_for=bow, radius=bar, mat=mats["paint"], parent=parent,
                       lods=NEAR)
    made += cyl(name=f"{name}_pin", r=bar * 0.9, depth=half * 2 + bar * 4, axis="X", mat=mats["steel"],
                parent=parent, seg=8, lods=FINE)
    return place(made, loc, rot)


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
    return place(made, loc, rot)


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
    outer = arc(centre=(0, 0), radius=radius * 1.45, start=0, end=math.pi, steps=6)
    inner = arc(centre=(0, 0), radius=radius * 1.3, start=math.pi, end=0, steps=6)
    arch = [(x, z) for x, _, z in outer + inner]
    made += prism(name=f"{name}_guard", profile_xz=arch, width=length * 0.7, loc=(-length * 0.45, 0, 0),
                  rot=(0, 0, math.pi / 2), mat=mats["paint"], parent=parent, lods=NEAR)
    return place(made, loc, rot)


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
    return place(made, loc, rot)


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
    return place(made, loc, rot)
