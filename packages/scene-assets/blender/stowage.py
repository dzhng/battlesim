"""What a vehicle carries: jerrycans, stowage boxes, tarp rolls and cargo beds.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl  # noqa: E402
from vehicle_parts import FINE, MID, NEAR, place, tube_part  # noqa: E402


def jerrycan(name, loc, mats, parent, size=(0.165, 0.345, 0.47), rot=(0, 0, 0)):
    """A standing jerrycan, its foot's centre at `loc`: a can `size` (x
    thickness, y width, z height; the real can by default), three carrying
    handles on top and a spout. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_can", size=(sx, sy, sz), loc=(0, 0, sz / 2), mat=mats["paint"], parent=parent,
                bevel=0.018, lods=MID)
    made += box(name=f"{name}_spout", size=(sx * 0.4, sy * 0.18, 0.04), loc=(0, sy * 0.36, sz + 0.02),
                mat=mats["paint"], parent=parent, lods=NEAR)
    for k in range(3):
        y = (k - 1) * sy * 0.22 - sy * 0.08

        def grip(lod, y=y):
            return [(0, y - 0.03, sz - 0.005), (0, y - 0.03, sz + 0.03), (0, y + 0.03, sz + 0.03),
                    (0, y + 0.03, sz - 0.005)]

        made += tube_part(name=f"{name}_handle_{k}", path_for=grip, radius=0.007, mat=mats["steel"],
                           parent=parent, lods=FINE)
    return place(made, loc, rot)


def stowage_box(name, loc, size, mats, parent, rot=(0, 0, 0)):
    """A stowage bin `size` (x, y, z) standing at `loc` (its foot's centre):
    the box, an overlapping lid, two latches on its +Y face and a hinge along
    the rear of the lid. Returns its meshes. Roles: paint, steel."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_body", size=(sx, sy, sz * 0.9), loc=(0, 0, sz * 0.45), mat=mats["paint"],
                parent=parent, bevel=0.015)
    made += box(name=f"{name}_lid", size=(sx + 0.02, sy + 0.02, sz * 0.1), loc=(0, 0, sz * 0.95),
                mat=mats["paint"], parent=parent, bevel=0.01, lods=NEAR)
    for k, x in enumerate((-sx * 0.3, sx * 0.3)):
        made += box(name=f"{name}_latch_{k}", size=(0.04, 0.015, 0.06), loc=(x, sy / 2 + 0.008, sz * 0.86),
                    mat=mats["steel"], parent=parent, lods=FINE)
    made += cyl(name=f"{name}_hinge", r=0.012, depth=sx * 0.9, loc=(0, -sy / 2 - 0.01, sz * 0.92), axis="X",
                mat=mats["steel"], parent=parent, seg=8, lods=FINE)
    return place(made, loc, rot)


def tarp_roll(name, loc, length, radius, mats, parent, straps=2, rot=(0, 0, 0)):
    """A rolled tarpaulin lying along local Y, its axis at `loc`, bound by
    `straps` dark straps. Returns its meshes. Roles: canvas, dark."""
    made = []
    made += cyl(name=f"{name}_roll", r=radius, depth=length, axis="Y", mat=mats["canvas"], parent=parent,
                seg=20, bevel=radius * 0.25)
    for k in range(straps):
        share = (k + 1) / (straps + 1)
        y = -length / 2 + length * share
        made += cyl(name=f"{name}_strap_{k}", r=radius * 1.04, depth=0.04, loc=(0, y, 0), axis="Y",
                    mat=mats["dark"], parent=parent, seg=20, lods=NEAR)
    return place(made, loc, rot)


def cargo_bed(name, loc, size, mats, parent, stakes=5, tarp=None, rot=(0, 0, 0)):
    """A truck's drop-side cargo bed, its floor's centre at `loc`, `size`
    (x length, y width, z side height): the floor on its cross members, the
    side and end boards with their stake posts and hinges, and with `tarp`
    (its height over the sides) the bows and the canvas over them, tied down
    along its foot. Returns its meshes. Roles: paint, dark, steel, canvas."""
    sx, sy, sz = size
    made = []
    made += box(name=f"{name}_floor", size=(sx, sy, 0.10), loc=(0, 0, -0.05), mat=mats["paint"], parent=parent,
                bevel=0.02)
    for k in range(max(2, round(sx / 0.9))):
        x = -sx / 2 + 0.2 + (sx - 0.4) * k / max(1, round(sx / 0.9) - 1)
        made += box(name=f"{name}_member_{k}", size=(0.10, sy - 0.10, 0.14), loc=(x, 0, -0.17), mat=mats["dark"],
                    parent=parent, lods=NEAR)
    for side in (-1, 1):
        made += box(name=f"{name}_side_{side}", size=(sx, 0.05, sz), loc=(0, side * (sy / 2 - 0.025), sz / 2),
                    mat=mats["paint"], parent=parent, bevel=0.015)
        made += box(name=f"{name}_rail_{side}", size=(sx + 0.02, 0.08, 0.06), loc=(0, side * (sy / 2 - 0.03), sz),
                    mat=mats["paint"], parent=parent, bevel=0.012, lods=MID)
        for k in range(stakes):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes - 1)
            made += box(name=f"{name}_stake_{side}_{k}", size=(0.08, 0.04, sz), loc=(x, side * (sy / 2 + 0.01), sz / 2),
                        mat=mats["paint"], parent=parent, bevel=0.01, lods=MID)
            hinge_x = min(x + 0.2, sx / 2 - 0.10)
            made += box(name=f"{name}_hinge_{side}_{k}", size=(0.10, 0.03, 0.05), loc=(hinge_x, side * (sy / 2 + 0.01),
                                                                                     0.04),
                        mat=mats["steel"], parent=parent, lods=FINE)
    for end in (-1, 1):
        made += box(name=f"{name}_end_{end}", size=(0.05, sy, sz), loc=(end * (sx / 2 - 0.025), 0, sz / 2),
                    mat=mats["paint"], parent=parent, bevel=0.015)
    if tarp:
        for k in range(stakes):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes - 1)
            made += box(name=f"{name}_bow_{k}", size=(0.05, sy - 0.04, 0.05), loc=(x, 0, sz + tarp - 0.05),
                        mat=mats["dark"], parent=parent, lods=FINE)
        made += box(name=f"{name}_canvas", size=(sx + 0.04, sy + 0.04, tarp + 0.12), loc=(0, 0, sz - 0.06 + tarp / 2),
                    mat=mats["canvas"], parent=parent, bevel=0.09)
        for k in range(stakes * 2 - 1):
            x = -sx / 2 + 0.10 + (sx - 0.20) * k / max(1, stakes * 2 - 2)
            for side in (-1, 1):
                made += box(name=f"{name}_tie_{side}_{k}", size=(0.03, 0.02, 0.22), loc=(x, side * (sy / 2 + 0.035),
                                                                                       sz - 0.02),
                            mat=mats["dark"], parent=parent, lods=FINE)
    return place(made, loc, rot)
