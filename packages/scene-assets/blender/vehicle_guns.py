"""Machine guns on vehicle and aircraft mounts, each built along its mount's
pitch node, firing along +X (`browning_m2`, `kord`). Infantry weapons are `weapons.py`'s, in their own
frame convention.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import box, cyl  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402


def browning_m2(parent, reach, mats, receiver_x=0.06, grips=False, prefix="m2"):
    """The M2 heavy machine gun on its mount's pitch node `parent`, firing
    along +X to the muzzle `reach` metres out: receiver centred at
    `receiver_x`, the barrel and its jacket, the flash hider, and with `grips`
    the spade grips a standing gunner holds. Every piece is named `<prefix>_*` (`m2_*`).
    Returns its meshes. Roles: dark, steel, black."""
    barrel_from = receiver_x + 0.26
    made = []
    made += box(name=f"{prefix}_receiver", size=(0.52, 0.13, 0.15), loc=(receiver_x, 0, 0), mat=mats["dark"], parent=parent,
                bevel=0.012)
    made += cyl(name=f"{prefix}_barrel", r=0.024, depth=reach - barrel_from, loc=((reach + barrel_from) / 2, 0, 0), axis="X",
                mat=mats["steel"], parent=parent, seg=10)
    made += cyl(name=f"{prefix}_jacket", r=0.040, depth=0.22, loc=(barrel_from + 0.11, 0, 0), axis="X", mat=mats["dark"],
                parent=parent, seg=12, lods=MID)
    made += cyl(name=f"{prefix}_flash_hider", r=0.034, r2=0.026, depth=0.08, loc=(reach - 0.04, 0, 0), axis="X",
                mat=mats["steel"], parent=parent, seg=10, lods=NEAR)
    if grips:
        made += box(name=f"{prefix}_grip_yoke", size=(0.10, 0.16, 0.04),
                    loc=(receiver_x - 0.26, 0, -0.04), mat=mats["dark"], parent=parent, lods=NEAR)
        for side in (-1, 1):
            made += cyl(name=f"{prefix}_grip_{side}", r=0.018, depth=0.12, loc=(receiver_x - 0.30, side * 0.07, -0.02),
                        mat=mats["black"], parent=parent, seg=8, lods=NEAR)
    return made


def kord(parent, reach, mats, prefix="kord"):
    """The 12.7 mm Kord on its mount's pitch node `parent`, firing along +X to
    the muzzle `reach` metres out: a long receiver, the barrel, its muzzle
    brake and the ammunition box. Pieces are named `<prefix>_*` (`kord_*`). Returns its
    meshes. Roles: dark, steel, paint."""
    made = box(f"{prefix}_receiver", (0.62, 0.10, 0.14), (0.02, 0, 0), mats["dark"], parent, bevel=0.012)
    made += cyl(f"{prefix}_barrel", 0.028, reach - 0.40, ((reach + 0.30) / 2, 0, 0), "X", mats["dark"], parent, seg=10)
    made += cyl(f"{prefix}_brake", 0.040, 0.10, (reach - 0.05, 0, 0), "X", mats["steel"], parent, seg=10, lods=NEAR)
    made += box(f"{prefix}_ammo", (0.26, 0.12, 0.20), (0.0, 0.13, -0.06), mats["paint"], parent, bevel=0.015, lods=MID)
    return made
