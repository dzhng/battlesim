"""CV90120 light tank (disabled card), from assets/references/cv90120/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/cv90120.py -- [--wreck]

The CV90 hull is the CV90 family's (`cv90.py`): its seven road wheels with no
return rollers and the front drive, the nose, glacis, bolted side armour,
rear door and hatches; its side and rear references are the CV9040C's. What
the CV90120-T photo changes: a low, wide wedge turret set over the hull's
middle, its faceted cheeks running back from a narrow gun shield to flat
sides and a deep bustle; the long 120 mm smoothbore with a thermal sleeve
in sections, no fume extractor, and a perforated muzzle brake; the
commander's sight ball on its post at the left front of the roof, the
gunner's sight box on the right, smoke tubes on the cheeks. Swedish splinter
scheme (the CV90 family's home army; the photo's exhibition finish is not
followed).

No archived frame exists: the frame is the CV90 hull's (its catalog box's
length and width) and the turret roof's height, measured off the photo
against the road wheels. With no mount in a disabled frame, the turret and
gun articulate on nodes stated here from the photo, so its wreck throws the
turret; nothing in the simulation reads them.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cv90 as C  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

CARD = "europe_cv90120_light_tank"
DIMENSIONS = (6.95, 3.30, 2.75)
MOUNTS = [dict(name="cannon", role="gun", on=None, pivot_m=[0.10, 0.0, 1.82], muzzle_m=[5.70, 0.0, 0.36])]
TRUNNION = 0.95
ROOF = 0.70  # the turret roof above the pivot
FOOT = C.ROOF_Z - MOUNTS[0]["pivot_m"][2]


def turret_body(v, turret):
    m = v.mats
    foot = [(1.10, 0.36), (1.55, 0.46), (0.75, 1.42), (-1.25, 1.48), (-2.20, 1.30), (-2.20, -1.30), (-1.25, -1.48),
            (0.75, -1.42), (1.55, -0.46), (1.10, -0.36)]
    crown = [(1.00, 0.36), (1.10, 0.44), (0.45, 1.25), (-1.25, 1.32), (-2.15, 1.18), (-2.15, -1.18), (-1.25, -1.32),
             (0.45, -1.25), (1.10, -0.44), (1.00, -0.36)]
    cyl("turret_ring_guard", 1.05, 0.08, (0, 0, FOOT - 0.03), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(FOOT, foot), (FOOT + 0.14, foot), (ROOF, crown)], mat=m["paint"], parent=turret,
         bevel=0.05)
    for side, s in ((1, "L"), (-1, "R")):
        VP.weld_line(f"cheek_weld_{s}", [(1.45, side * 0.46, FOOT + 0.14), (0.70, side * 1.40, FOOT + 0.14)], m,
                     turret)
        VP.bolted_panel(f"turret_side_{s}", (-0.55, side * 1.46, FOOT + 0.32), (1.30, 0.36, 0.05), m, turret,
                        bolts=(4, 2), bevel=0.02, lods=VP.ALL, rot=(-side * math.pi / 2, 0, 0))
        VP.smoke_discharger_bank(f"smoke_{s}", (0.55, side * 1.30, ROOF - 0.10), m, turret, count=4,
                                 tube_radius=0.045, tube_length=0.20, elevation=0.3, spread=0.3,
                                 rot=(0, 0, side * 0.6))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.95, side * 0.95, ROOF), m, whip, height=2.0)
    VP.sight_housing("gunner_sight", (0.55, -0.55, ROOF - 0.04), m, turret, size=(0.40, 0.32, 0.24))
    ball = empty("dressing_commander_sight", parent=turret)
    cyl("sight_post", 0.08, 0.22, (0.20, 0.62, ROOF + 0.11), "Z", m["dark"], ball, seg=14, lods=MID)
    cyl("sight_ball", 0.20, 0.34, (0.20, 0.62, ROOF + 0.36), "Z", m["paint"], ball, seg=22, bevel=0.09)
    box("sight_ball_window", (0.03, 0.18, 0.12), (0.39, 0.62, ROOF + 0.36), m["glass"], ball, lods=MID)
    VP.hatch("commander_hatch", (-0.55, 0.55, ROOF), m, turret, radius=0.28)
    VP.hatch("gunner_hatch", (-0.55, -0.55, ROOF), m, turret, radius=0.26)
    VP.stowage_box("bustle_box", (-1.95, 0, FOOT + 0.08), (0.45, 2.20, 0.42), m, turret)
    VP.tarp_roll("bustle_tarp", (-1.60, 0.0, ROOF + 0.10), 1.70, 0.14, m, turret, straps=3)


def gun_120(v, gun):
    """The 120 mm: a narrow gun shield, the barrel in thermal sleeve sections
    and the perforated muzzle brake."""
    m = v.mats
    reach = MOUNTS[0]["muzzle_m"][0] - TRUNNION
    box("gun_shield", (0.46, 0.60, 0.44), (0.05, 0, 0.0), m["paint"], gun, bevel=0.05)
    cyl("barrel_root", 0.12, 0.40, (0.45, 0, 0), "X", m["paint"], gun, seg=24)
    joints = [0.65, 2.0, 3.3, reach - 0.62]
    for k, (a, b) in enumerate(zip(joints, joints[1:])):
        cyl(f"thermal_sleeve_{k}", 0.085, b - a - 0.04, ((a + b) / 2, 0, 0), "X", m["paint"], gun, seg=22)
        cyl(f"sleeve_band_{k}", 0.092, 0.04, (b, 0, 0), "X", m["dark"], gun, seg=20, lods=NEAR)
    cyl("muzzle_brake", 0.11, 0.60, (reach - 0.30, 0, 0), "X", m["paint"], gun, seg=22, bevel=0.02)
    for k in range(8):
        for side in (-1, 1):
            cyl(f"brake_port_{k}_{side}", 0.028, 0.02, (reach - 0.55 + k * 0.07, side * 0.105, 0), "Y", m["black"],
                gun, seg=8, lods=FINE)
    cyl("muzzle_bore", 0.06, 0.012, (reach - 0.002, 0, 0), "X", m["black"], gun, seg=18, lods=MID)


def build(variant, v):
    C.hull_body(v)
    VP.tracked_running_gear(v.mats, v.hull, C.TRACK_Y, C.TRACK_W, C.ROAD_X, C.ROAD_Z, C.ROAD_R, 0.20, C.SPROCKET,
                            C.IDLER, (), bolts=6, teeth=11, arm=(0.40, 0.40), pitch=0.15)
    C.hull_sides(v, mk4=False)
    mounts = rig({"mounts": MOUNTS}, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    turret_body(v, turret)
    gun_120(v, gun)
    v.head_out("commander", turret, MOUNTS[0]["pivot_m"][0] - 0.55, 0.55, MOUNTS[0]["pivot_m"][2] + ROOF)


def wreck(variant, v):
    """The CV90's damage (`cv90.wreck`): the right track off along the hull,
    road wheels gone, side panels fallen at its foot, the rear door blown
    off, plates warped; the bustle box burst. `wreckage.burn` heaves the
    turret."""
    from wreckage import remove
    remove("bustle_box_lid")
    C.wreck(variant, v)


if __name__ == "__main__":
    run_disabled("cv90120", {CARD: DIMENSIONS}, "swedish_splinter", build, wreck,
                 skip=("dressing_", "gun", "muzzle"), chip=1.0)
