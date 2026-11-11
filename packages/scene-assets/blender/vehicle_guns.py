"""Guns on vehicle and aircraft mounts, each built along its mount's pitch
node, firing along +X: a tank gun's barrel (`gun_tube`) and the machine guns
(`browning_m2`, `kord`). Infantry weapons are `weapons.py`'s, in their own
frame convention.

Every part keeps the contracts the kernel documents (`vehicle_parts`).
"""
import math
import os
import sys

import bmesh

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from parts import SEG_SCALE, box, cyl, mesh_part  # noqa: E402
from vehicle_parts import ALL, MID, NEAR  # noqa: E402


def gun_tube(name, gun, mats, start, reach, radius, muzzle_radius=None, sleeve=None, evacuator=None, bands=(),
             band=(0.008, 0.05), muzzle=None, bore=None, lods=ALL):
    """A tank gun's barrel on its pitch node `gun`: one continuous core from
    `start` to the muzzle `reach` metres out along +X, tapering from `radius`
    to `muzzle_radius`, with every other piece seated on it, so no section can
    float or leave a see-through gap.

    - `sleeve` is `(radius, [(a, b), ...])`: the thermal sleeve's sections,
      proud of the core between each `a` and `b`.
    - `evacuator` is `(x, radius, length, cap)`: the collar centred at `x`,
      its ends closing onto the tube beneath through cones `cap` long (0: a
      plain collar with chamfered shoulders).
    - `bands` are the clamp bands' x positions, each `band` = (how far proud
      of the tube beneath, length).
    - `muzzle` is `(radius, length)`: the muzzle piece ending at `reach`.
    - `bore` is the bore's radius, drawn as a dark disc in the muzzle face.

    Pieces are named `{name}_core`, `_sleeve_k`, `_evacuator`, `_band_k`,
    `_muzzle` and `_bore`, each in the tiers of `lods` its size earns. Returns
    the meshes. Roles: paint, dark, steel, black."""
    tip = radius if muzzle_radius is None else muzzle_radius
    sleeve_r, sections = sleeve if sleeve else (None, [])

    def core_at(x):
        return radius + (tip - radius) * (x - start) / (reach - start)

    def under(x):
        """The radius of the tube beneath a fitting at `x`."""
        on_sleeve = any(a <= x <= b for a, b in sections)
        return sleeve_r if on_sleeve else core_at(x)

    def tiers(within=ALL):
        return tuple(t for t in lods if t in within)

    made = cyl(f"{name}_core", radius, reach - start, ((start + reach) / 2, 0, 0), "X", mats["paint"], gun,
               seg=24, r2=tip, lods=tiers())
    for k, (a, b) in enumerate(sections):
        made += cyl(f"{name}_sleeve_{k}", sleeve_r, b - a, ((a + b) / 2, 0, 0), "X", mats["paint"], gun, seg=24,
                    lods=tiers())
    if evacuator:
        x, r, length, cap = evacuator
        rear, front = x - length / 2, x + length / 2
        shoulder = 0.0 if cap else 0.02
        # A ring a shoulder's width in from each end keeps the shoulder's
        # smoothing off the collar's face.
        profile = [(rear - cap, under(rear - cap)), (rear, r - shoulder), (rear + shoulder, r),
                   (rear + 2 * shoulder, r), (front - 2 * shoulder, r), (front - shoulder, r),
                   (front, r - shoulder), (front + cap, under(front + cap))]
        made += _lathe(f"{name}_evacuator", profile, 28, mats["paint"], gun, tiers())
    proud, band_length = band
    for k, x in enumerate(bands):
        made += cyl(f"{name}_band_{k}", under(x) + proud, band_length, (x, 0, 0), "X", mats["dark"], gun, seg=20,
                    lods=tiers(NEAR))
    if muzzle:
        r, length = muzzle
        made += cyl(f"{name}_muzzle", r, length, (reach - length / 2, 0, 0), "X", mats["steel"], gun, seg=24,
                    lods=tiers())
    if bore:
        made += cyl(f"{name}_bore", bore, 0.012, (reach - 0.002, 0, 0), "X", mats["black"], gun, seg=20,
                    lods=tiers(MID))
    return made


def _lathe(name, profile, seg, mat, parent, lods):
    """A solid turned about the X axis through `profile`, its (x, radius)
    points from rear to front; each end is closed flat."""

    def build(bm, lod):
        sides = max(6, round(seg * SEG_SCALE[lod]))
        points = [p for k, p in enumerate(profile) if k == 0 or p != profile[k - 1]]
        rings = [[bm.verts.new((x, r * math.cos(2 * math.pi * i / sides), r * math.sin(2 * math.pi * i / sides)))
                  for i in range(sides)] for x, r in points]
        bm.faces.new(rings[0][::-1])
        bm.faces.new(rings[-1])
        for a, b in zip(rings, rings[1:]):
            for i in range(sides):
                j = (i + 1) % sides
                bm.faces.new((a[i], a[j], b[j], b[i]))
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        for f in bm.faces:
            f.smooth = abs(f.normal.x) < 0.9

    return mesh_part(name, build, mat, parent, lods)


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
