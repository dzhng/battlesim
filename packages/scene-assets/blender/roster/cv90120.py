"""CV90120 light tank, from assets/references/cv90120/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/cv90120.py -- [--wreck]

The CV90 hull is the CV90 family's (`cv90.py`): its seven road wheels with no
return rollers and the front drive, the nose, glacis, bolted side armour,
rear door and hatches; its side and rear are the CV90's own references
(`assets/references/cv90/`). What the CV90120-T photos change: a low, wide
wedge turret set just behind the hull's middle, its faceted cheeks running out from
a narrow gun shield to nearly the hull's width, then flat sides and a deep
bustle; the long 120 mm smoothbore with a thermal sleeve in sections, no
fume extractor, and a perforated muzzle brake; the commander's sight ball on
its post at the left front of the roof, the gunner's sight box on the right,
smoke tubes on the cheeks, a pintle machine gun by the gunner's hatch.
Swedish splinter scheme (the CV90 family's home army; the photos' exhibition
finishes are not followed).

Built to the catalog frame: the CV90 hull's length and width and the turret
roof's height, measured off the photos against the road wheels. The turret,
and the commander in his hatch, are built at the frame's pivot, 0.3 m behind
the hull's middle.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import cv90 as C  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

TRUNNION = 0.95
ROOF = 0.58  # the turret roof above the pivot
WAIST = 0.14  # the turret's upright foot, from the hull roof
# The left cheek's line (front, back) at the waist and at the roof: the
# cheeks lean well back, so the front reads as a low wedge.
CHEEK_WAIST = ((1.85, 0.52), (1.05, 1.50))
CHEEK_ROOF = ((1.20, 0.42), (0.45, 1.24))
SIDE_WAIST, SIDE_ROOF = 1.52, 1.26  # the flanks' half-width, leaning in


def turret_body(v, turret):
    m = v.mats
    foot = C.ROOF_Z - v.frame["mounts"][0]["pivot_m"][2]  # the hull roof, in the turret's frame
    waist = foot + WAIST
    # A wide wedge: the cheeks run from a prow either side of the gun
    # shield out to nearly the hull's width well forward, so the turret's
    # front is broad and low, not a narrow box set back.
    def plan(cheek, side, nose, back):
        left = [(nose, 0.36), *cheek, (-1.25, side), (back, side - 0.20)]
        return left + [(x, -y) for x, y in reversed(left)]

    base = plan(CHEEK_WAIST, SIDE_WAIST, 1.15, -2.20)
    crown = plan(CHEEK_ROOF, SIDE_ROOF, 0.95, -2.15)
    cyl("turret_ring_guard", 1.05, 0.08, (0, 0, foot - 0.03), "Z", m["dark"], turret, seg=36, lods=MID)
    loft("turret_shell", [(foot, base), (waist, base), (ROOF, crown)], mat=m["paint"], parent=turret,
         bevel=0.05)
    VP.roof_fittings("roof", crown, ROOF, m, turret, periscopes=((-0.10, 0.95, 0.5),))
    VP.laser_warners("laser_warner", crown, ROOF, m, turret)
    for side, s in ((1, "L"), (-1, "R")):
        VP.weld_line(f"cheek_weld_{s}", [(x, side * y, waist) for x, y in CHEEK_WAIST], m,
                     turret)
        # On the turret's leaning flank: the shell's side at x -0.55, from
        # the top of its upright foot to its crown.
        loc, rot = VP.on_side(-0.55, foot + 0.32, side, (SIDE_WAIST, waist), (SIDE_ROOF, ROOF))
        VP.bolted_panel(f"turret_side_{s}", loc, (1.30, 0.36, 0.05), m, turret, bolts=(4, 2), bevel=0.02,
                        lods=VP.ALL, rot=rot)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.30, side * 1.20, ROOF - 0.08), m, turret, count=4,
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
    VP.stowage_box("bustle_box", (-1.95, 0, foot + 0.08), (0.45, 2.20, 0.42), m, turret)
    VP.tarp_roll("bustle_tarp", (-1.60, 0.0, ROOF + 0.10), 1.70, 0.14, m, turret, straps=3)
    # The photos' add-on armour and stowage: tile packs on the cheeks' faces
    # (lying on the leaning cheek, facing out along its normal) and the
    # flanks' rear, a basket round the bustle box.
    (fx, fy), (bx, by) = CHEEK_WAIST
    facing = math.atan2(fx - bx, by - fy)  # the cheek's outward normal in plan
    normal = (math.cos(facing), math.sin(facing))
    mid_waist = ((fx + bx) / 2, (fy + by) / 2)
    mid_roof = [sum(c) / 2 for c in zip(*CHEEK_ROOF)]
    setback = (mid_waist[0] - mid_roof[0]) * normal[0] + (mid_waist[1] - mid_roof[1]) * normal[1]
    lean = math.atan2(setback, ROOF - waist)
    t = 0.12 / (ROOF - waist)
    cheek = [w + (c - w) * t for w, c in zip(mid_waist, mid_roof)]
    for side, s in ((1, "L"), (-1, "R")):
        VP.armour_tiles(f"cheek_armour_{s}", (cheek[0], side * cheek[1], waist + 0.12), (0.62, 0.26), (2, 1), 0.06, m,
                        turret, rot=(0, math.pi / 2 - lean, side * facing))
        loc, rot = VP.on_side(-1.65, foot + 0.32, side, (SIDE_WAIST, waist), (SIDE_ROOF, ROOF))
        VP.armour_tiles(f"flank_armour_{s}", loc, (0.80, 0.36), (2, 1), 0.06, m, turret, rot=rot)
    VP.slat_armour("bustle_basket", (-2.28, 0, foot + 0.05), (2.30, 0.50), m, turret, spacing=0.10, bar=0.014,
                   rot=(0, 0, math.pi / 2))


def gun_120(v, gun):
    """The 120 mm: a narrow gun shield, the barrel in thermal sleeve sections
    and the perforated muzzle brake."""
    m = v.mats
    reach = v.frame["mounts"][0]["muzzle_m"][0] - TRUNNION
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
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_120(v, gun)
    cyl("pintle", 0.04, 0.20, (0, 0, -0.08), "Z", v.mats["dark"], hmg, seg=10)
    VP.browning_m2(hmg_gun, v.frame["mounts"][1]["muzzle_m"][0], v.mats, grips=True)
    pivot = v.frame["mounts"][0]["pivot_m"]
    v.head_out("commander", turret, pivot[0] - 0.55, 0.55, pivot[2] + ROOF)


def wreck(variant, v):
    """The CV90's damage (`cv90.wreck`): the right track off along the hull,
    road wheels gone, side panels fallen at its foot, the rear door blown
    off, plates warped; the bustle box burst. `wreckage.burn` heaves the
    turret."""
    from wreckage import remove
    remove("bustle_box_lid")
    C.wreck(variant, v)


run("cv90120", "swedish_splinter", build, wreck, chip=1.0)
