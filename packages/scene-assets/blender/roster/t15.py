"""T-15 Armata heavy IFV, from assets/references/t15/, on the Armata platform (`roster/armata.py`).

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t15.py -- [--wreck]

Engine at the front, under a long smooth glacis that runs from the low,
sharp nose, well ahead of the tracks, in one line up to the troop
compartment's roof, the crew's hatches at the roof's front. The compartment
is the full hull height back to the tail, its sides tall upright modules
level with its roof. The Bumerang-BM module sits on that roof: a broad box
turret on a dark neck, its 30 mm 2A42 on the centre line and a pair of
Kornet tubes on the left; big slab applique modules lie on the deep leaning
band beside the glacis; a tall rear door with stowage hung high either side.

Built to the catalog frame (hull 9.5 x 3.5 x 3.2 m, module pivot 2.22 m on
the troop compartment's roof, so the 30 mm's axis is at about 2.72 m, as on
the real module). The module's ring is 1.2 m behind amidships, over the
fifth road wheel, where the photos put it; the troop hatches are behind it.
"""
from functools import partial
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import vehicle_parts as VP  # noqa: E402
from armata import (DECK, TROOP_FRONT, TROOP_ROOF, Shape, armour_kit, engine_deck, hull,  # noqa: E402
                    running_gear, side_face, skirts, turret_kit, wreck)
from parts import box, cyl, empty, loft, prism  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# The T-15 on the Armata platform (photos: side, three-quarter front, front):
# its nose under a longer glacis raked at about 16 degrees, running up to the
# troop compartment's roof; the nose 0.45 m further past the sprocket than
# the base, a sharper beak; the skirts covering the road wheels; a deep band
# leaning in from a low shoulder, carrying the slab modules, the skirts'
# tops under the glacis by the band's full depth so the band is even from
# the nose back to the glacis' head; the engine at the front; the glacis one
# smooth plate.
SHOULDER = 1.40
SHAPE = Shape(nose_z=1.05, overhang=0.45, skirt_foot=0.41, shoulder=SHOULDER, band_lean=math.radians(40),
              under_glacis=DECK - SHOULDER, front_engine=True, troop_compartment=True, glacis_tiles=False, slab=True,
              number="203")


def build(variant, v):
    m, h = v.mats, v.hull
    half = v.length / 2
    top = hull(v, SHAPE)
    running_gear(v, SHAPE)
    skirts(v, SHAPE)
    armour_kit(v, SHAPE)
    # The engine is under the long glacis: its grilles at the glacis' head.
    engine_deck(v, SHAPE, top + 0.05, top + 0.85)
    # The troop compartment over the deck to the full hull height, its front
    # carrying the glacis on up to its roof; the crew's two hatches at the
    # roof's front, ahead of the module; troop hatches behind the module, the
    # tall rear door between stowage boxes hung high on the rear plate.
    edge = side_face(SHAPE)[1][0]
    prism("troop_compartment", [(-half, DECK - 0.02), (top, DECK - 0.02), (TROOP_FRONT, TROOP_ROOF),
                                (-half, TROOP_ROOF)], 2 * edge, mat=m["paint"], parent=h, bevel=0.04)
    for name, y in (("driver", 0.65), ("commander", -0.65)):
        VP.hatch(f"{name}_hatch", (TROOP_FRONT - 0.32, y, TROOP_ROOF), m, h, radius=0.28)
        VP.periscope(f"{name}_periscope", (TROOP_FRONT - 0.10, y * 0.3, TROOP_ROOF), m, h, size=(0.12, 0.30, 0.10))
    for k, y in enumerate((0.62, -0.62)):
        VP.hatch(f"troop_hatch_{k}", (-3.70, y, TROOP_ROOF), m, h, size=(0.90, 0.62))
    box("rear_door", (0.08, 1.00, 1.30), (-half - 0.03, 0, 1.00), m["paint"], h, bevel=0.02)
    stowage = empty("dressing_rear_stowage", parent=h)
    for side in (-1, 1):
        VP.stowage_box(f"rear_box_{side}", (-half - 0.14, side * 0.95, 1.45), (0.26, 0.62, 0.70), m, stowage)
    mounts = rig(v.frame, v.root)
    turret, gun, _, _ = mounts["autocannon"]
    launcher, launcher_pitch, _, _ = mounts["launcher"]
    base = TROOP_ROOF - v.frame["mounts"][0]["pivot_m"][2]
    bumerang_module(v, turret, gun, base)
    kornet_pair(v, launcher, launcher_pitch)


def bumerang_module(v, turret, gun, base):
    """The Bumerang-BM unmanned module: a broad faceted box on a dark neck
    over its ring, sights on its roof, smoke banks, the 30 mm 2A42 on the
    centre line."""
    m = v.mats
    z0, z1 = base + 0.12, base + 0.86
    foot = [(1.30, 0.50), (1.05, 1.18), (-1.15, 1.22), (-1.40, 0.85), (-1.40, -0.85), (-1.15, -1.22), (1.05, -1.18),
            (1.30, -0.50)]
    crown = [(0.85, 0.45), (0.65, 1.02), (-1.05, 1.08), (-1.28, 0.75), (-1.28, -0.75), (-1.05, -1.08), (0.65, -1.02),
             (0.85, -0.45)]
    cyl("module_ring", 0.95, 0.14, (0, 0, base + 0.06), "Z", m["dark"], turret, seg=40, lods=MID)
    loft("module_shell", [(z0, foot), (z1, crown)], mat=m["paint"], parent=turret, bevel=0.035)
    VP.roof_fittings("roof", crown, z1, m, turret, periscopes=((-0.60, 0.55, 0.4),))
    turret_kit(v, turret, crown, (1.20, z0), (1.05, z1), -1.05, 0.90, z0 + 0.05, z1 - 0.05)
    # The gunner's sight box and the commander's panoramic sight on its
    # pedestal stand over the frame's top (the module's roof): dressing.
    sights = empty("dressing_sights", parent=turret)
    VP.sight_housing("gunner_sight", (0.20, -0.55, z1 - 0.02), m, sights, size=(0.42, 0.32, 0.30))
    cyl("panorama_post", 0.08, 0.10, (-0.55, -0.30, z1 + 0.05), "Z", m["dark"], sights, seg=12)
    VP.sight_housing("panorama_head", (-0.55, -0.30, z1 + 0.08), m, sights, size=(0.34, 0.32, 0.30))
    for side, s in ((1, "L"), (-1, "R")):
        VP.smoke_discharger_bank(f"smoke_{s}", (-0.60, side * 1.15, z0 + 0.32), m, turret, count=3,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.4, spread=0.3,
                                 rot=(0, 0, side * 1.4))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.10, side * 0.60, z1), m, whip, height=2.0)
    VP.stowage_box("module_box", (-1.48, 0, z0), (0.30, 1.30, 0.40), m, turret, rot=(0, 0, math.pi / 2))
    reach = v.frame["mounts"][0]["muzzle_m"][0]
    box("cannon_cradle", (0.80, 0.42, 0.36), (0.80, 0, 0), m["paint"], gun, bevel=0.04)
    cyl("cannon_sleeve", 0.085, 0.65, (1.40, 0, 0), "X", m["dark"], gun, seg=16)
    cyl("cannon_barrel", 0.050, reach - 1.40, ((reach + 1.40) / 2, 0, 0), "X", m["steel"], gun, seg=14)
    cyl("cannon_muzzle", 0.070, 0.22, (reach - 0.11, 0, 0), "X", m["dark"], gun, seg=14)
    cyl("cannon_bore", 0.025, 0.01, (reach, 0, 0), "X", m["black"], gun, seg=10, lods=NEAR)
    cyl("coax_barrel", 0.018, 0.40, (1.35, -0.26, 0.04), "X", m["steel"], gun, seg=8, lods=NEAR)


def kornet_pair(v, launcher, pitch):
    """Two Kornet tubes in their cradle on the module's left."""
    m = v.mats
    reach = v.frame["mounts"][1]["muzzle_m"][0]
    box("kornet_arm", (0.30, 0.30, 0.30), (-0.20, -0.08, -0.12), m["dark"], launcher, bevel=0.02)
    for k, z in enumerate((0.0, 0.24)):
        cyl(f"kornet_tube_{k}", 0.11, reach + 0.55, ((reach - 0.55) / 2, 0, z), "X", m["paint"], pitch, seg=16,
            bevel=0.01)
        cyl(f"kornet_cap_{k}", 0.092, 0.01, (reach, 0, z), "X", m["black"], pitch, seg=14, lods=MID)
        for j, x in enumerate((-0.30, reach - 0.20)):
            cyl(f"kornet_band_{k}_{j}", 0.118, 0.05, (x, 0, z), "X", m["dark"], pitch, seg=16, lods=NEAR)


run("t15", "russian_green", build, partial(wreck, SHAPE), chip=1.0)
