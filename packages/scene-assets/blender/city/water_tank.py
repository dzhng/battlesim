"""New York's rooftop water tank: a barrel of cedar staves bound in steel hoops under a
cone, on a steel stand, as towers and lofts carry them. One owner for its shape and its
materials, so a tower's and a loft's are the same tank.

`water_tank` builds it into a module at a place, at every tier (a set's module, which
rows place near), or `far`: the two coarse tiers only, for a shell that folds its roof's
furniture into itself. `HEIGHT_M` is how high it stands over what it stands on: a set
that places one on a roof sets its top fit from it (`fit_over`).
"""
import bpy

from parts import TIERS, box, cyl, flat_paint

LEGS_M, RADIUS_M, BARREL_M, CONE_M = 1.45, 1.45, 2.5, 0.75
HEIGHT_M = LEGS_M + BARREL_M + CONE_M


def fit_over(roof_below_top_m):
    """The top fit a set needs for tanks standing on a roof `roof_below_top_m` under its part's top."""
    return round(HEIGHT_M - roof_below_top_m, 2)


def _materials():
    """The staves, the cone and the steel, made once a run."""
    if "tank_wood" not in bpy.data.materials:
        flat_paint("tank_wood", (0.13, 0.1, 0.075), rough=0.9, grime=0.4)
        flat_paint("tank_cone", (0.06, 0.055, 0.05), rough=0.8, grime=0.3)
        flat_paint("tank_steel", (0.08, 0.085, 0.09), rough=0.6, metal=0.4, grime=0.0)
    return (bpy.data.materials[n] for n in ("tank_wood", "tank_cone", "tank_steel"))


def water_tank(m, tag, x, y, z, far=False, stand=True):
    """The tank standing at (x, y, z) in module `m`. `stand` false leaves out the coarsest tier's
    block under it (a gutted tower's budget has no room for it)."""
    wood, cone, steel = _materials()
    lods = (2, 3) if far else TIERS
    for k, (sx, sy) in enumerate(((-1, -1), (1, -1), (1, 1), (-1, 1))):
        box(m.n(f"{tag}_leg_{k}"), (0.16, 0.16, LEGS_M), (x + sx * 1.05, y + sy * 1.05, z + LEGS_M / 2), steel, m.root,
            lods=(2,) if far else (0, 1, 2))
    if not far:
        for k in range(2):
            box(m.n(f"{tag}_beam_{k}"), (2.6, 0.18, 0.22) if k == 0 else (0.18, 2.6, 0.22), (x, y, z + LEGS_M - 0.11), steel, m.root,
                lods=(0, 1))
    if far and stand:  # at the coarsest the stand is one dark block
        box(m.n(f"{tag}_stand"), (2.2, 2.2, LEGS_M), (x, y, z + LEGS_M / 2), steel, m.root, lods=(3,))
    cyl(m.n(f"{tag}_barrel"), RADIUS_M, BARREL_M, (x, y, z + LEGS_M + BARREL_M / 2), "Z", wood, m.root, seg=14 if far else 20, lods=lods)
    cyl(m.n(f"{tag}_cone"), RADIUS_M + 0.08, CONE_M, (x, y, z + LEGS_M + BARREL_M + CONE_M / 2), "Z", cone, m.root, seg=14 if far else 20,
        r2=0.1, caps=False, lods=lods)
    for k in range(3) if not far else ():
        cyl(m.n(f"{tag}_hoop_{k}"), RADIUS_M + 0.025, 0.06, (x, y, z + LEGS_M + 0.4 + 0.85 * k), "Z", steel, m.root, seg=20, caps=False,
            lods=(0,))
