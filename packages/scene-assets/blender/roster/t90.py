"""T-90M Proryv, from assets/references/t90/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t90.py -- [--variant=<id>] [--wreck]

What the photos settle (Army-2020 to 2023): the T-72-derived hull and
running gear (six ribbed road wheels, three return rollers, Relikt panels in
two rows over the front of the hull sides above a scalloped rubber skirt,
slat panels over the rear), Relikt in rows on the glacis, and two external
fuel drums across the hull rear over a slat panel. The welded turret is
compact and angular, standing clear of the hull over a shadowed ring: Relikt
wedges pointing forward either side of the gun, its flat sides hung with net
screens, a short bustle in a net cage leaving the engine deck bare behind it,
smoke dischargers on the front roof corners, the gunner's Sosna-U box on the left, the commander's
tall panoramic sight on the right and the remote machine gun beside it.

The hull, running gear, skirts, gun and the commander's gun are the T-72's
(`roster/t72.py`): the T-90 is a T-72 derivative.

Built to the catalog frame (hull 6.86 x 3.78 x 2.22 m, turret pivot 1.332 m,
cannon muzzle 6.2 m ahead): nothing here moves it.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from t72 import TRUNNION, cupola_mg, gun_2a46, soviet_gear, soviet_hull, soviet_skirts, soviet_wreck  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
FOOT = -0.02
ROOF = 0.88  # 2.21 m: the roof reaches the frame's top


# The turret's plan, its left half from the gun slot back to the bustle's
# corner: compact, widest just behind the cheeks, narrowing to a short
# bustle that leaves the engine deck bare behind it.
TURRET_HALF = [(1.40, 0.40), (1.35, 1.26), (0.40, 1.34), (-0.55, 1.20), (-1.15, 1.00), (-1.30, 0.84)]
UNDERCUT = 0.20  # the shadowed band under the turret's walls, over its ring
WALL = FOOT + UNDERCUT  # where the walls start, overhanging the ring
WEDGE_TIP = 0.48  # the cheek wedges' forward tip, a third of the way up the wall


def turret_rings():
    """The welded turret: a narrow neck over its ring, the walls jutting out
    over it and standing near-vertical to a flat roof, narrowing to the
    bustle. The overhang casts the dark band that sets the turret apart from
    the hull."""
    def ring(inset, lean=0.0):
        left = [(x - inset if x > 0 else x + inset, y - inset - lean) for x, y in TURRET_HALF]
        return left + [(x, -y) for x, y in reversed(left)]

    return [(FOOT, ring(0.30)), (WALL - 0.03, ring(0.30)), (WALL, ring(0.0)), (ROOF, ring(0.02, 0.04))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.12, 0.10, (0, 0, FOOT - 0.05), "Z", m["dark"], turret, seg=40, lods=MID)
    side_yaw = math.atan2(TURRET_HALF[2][1] - TURRET_HALF[4][1], TURRET_HALF[2][0] - TURRET_HALF[4][0])
    for side, s in ((1, "L"), (-1, "R")):
        # The Relikt wedge on each cheek, an arrowhead pointing forward
        # beside the gun: its tiled top slopes down from the roof to a tip
        # a third of the way up, its underside tucks back to the wall foot.
        # The slope starts at the roof's front edge, so the wedge reads as
        # the turret's own nose, not a block bolted on.
        head, nose = 1.42, 2.30  # where the slope leaves the roof, and its tip
        foot = [(1.30, 0.44), (nose - 0.35, 0.46), (1.60, 1.40), (1.25, 1.40)]
        tip = [(1.30, 0.44), (nose, 0.46), (1.80, 1.44), (1.25, 1.44)]
        top = [(1.25, 0.44), (head, 0.46), (head - 0.10, 1.32), (1.20, 1.32)]
        rings = []
        for z, pts in ((WALL, foot), (WEDGE_TIP, tip), (ROOF - 0.02, top)):
            p = [(x, side * y) for x, y in pts]
            rings.append((z, p if side > 0 else p[::-1]))
        loft(f"cheek_wedge_{s}", rings, mat=m["paint"], parent=turret, bevel=0.04)
        slope = math.atan2(ROOF - 0.02 - WEDGE_TIP, nose - head)
        face_x = head + 0.32
        VP.armour_tiles(f"cheek_face_{s}", (face_x, side * 0.85, ROOF - 0.02 - (face_x - head) * math.tan(slope)),
                        (0.56, 0.70), (2, 3), 0.06, m, turret, rot=(0, slope, 0))
        VP.armour_tiles(f"cheek_tiles_{s}", (1.10, side * 0.92, ROOF - 0.02), (0.44, 0.88), (1, 3), 0.06, m, turret)
        # Net screens hung along the turret's flank, the cage round the bustle.
        # Both stand clear of the undercut, so the turret sits apart from the hull.
        VP.slat_armour(f"net_screen_{s}", (-0.15, side * 1.38, WALL + 0.02), (1.30, 0.50), m, turret,
                       spacing=0.10, bar=0.012, rot=(0, 0, side * side_yaw))
        VP.slat_armour(f"bustle_cage_{s}", (-1.12, side * 1.06, FOOT + 0.16), (0.50, 0.58), m, turret, spacing=0.10,
                       bar=0.014)
        VP.smoke_discharger_bank(f"smoke_{s}", (0.90, side * 1.16, ROOF - 0.08), m, turret, count=6, tube_radius=0.045,
                                 tube_length=0.22, elevation=0.30, spread=0.6, rot=(0, 0, side * 0.60))
        VP.stowage_box(f"bustle_bin_{s}", (-0.95, side * 0.62, ROOF - 0.06), (0.55, 0.28, 0.12), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.00, side * 0.85, ROOF), m, whip, height=2.2)
    VP.slat_armour("bustle_cage_rear", (-1.45, 0, FOOT + 0.16), (2.00, 0.58), m, turret, spacing=0.10, bar=0.014,
                   rot=(0, 0, math.pi / 2))
    # Roof: the gunner's sight box on the left, the commander's tall
    # panoramic sight drum on the right (both stand over the frame's top, so
    # they are dressing), his hatch behind the remote gun, the loader-less
    # gunner's hatch.
    sights = empty("dressing_sights", parent=turret)
    VP.sight_housing("sosna_sight", (0.62, 0.62, ROOF - 0.02), m, sights, size=(0.44, 0.36, 0.24))
    cyl("commander_sight_drum", 0.24, 0.20, (0.25, -0.88, ROOF + 0.08), "Z", m["paint"], sights, seg=20,
        bevel=0.02, lods=MID)
    VP.sight_housing("commander_sight", (0.25, -0.88, ROOF + 0.18), m, sights, size=(0.40, 0.40, 0.10))
    VP.cupola("commander_cupola", (-0.75, -0.58, ROOF - 0.06), m, turret, radius=0.30, periscopes=4, lid_open=False)
    VP.hatch("gunner_hatch", (-0.45, 0.58, ROOF - 0.04), m, turret, radius=0.30)


def rear_drums(v):
    """The two external fuel drums across the hull rear, their brackets,
    and the slat panel under them."""
    m, hull = v.mats, v.hull
    drums = empty("dressing_fuel_drums", parent=hull)
    for side in (-1, 1):
        cyl(f"fuel_drum_{side}", 0.29, 1.30, (-3.42, side * 0.80, 1.22), "Y", m["paint"], drums, seg=24, bevel=0.03)
        for k in (-0.45, 0.45):
            cyl(f"drum_band_{side}_{k}", 0.30, 0.04, (-3.42, side * 0.80 + k, 1.22), "Y", m["dark"], drums, seg=24,
                lods=NEAR)
    box("drum_bracket", (0.30, 3.00, 0.06), (-3.36, 0, 0.92), m["dark"], hull, lods=MID)
    VP.slat_armour("rear_bars", (-3.48, 0, 0.50), (3.20, 0.40), m, hull, rot=(0, 0, math.pi / 2))


def build(variant, v):
    soviet_hull(v, rear_cage=False)
    soviet_gear(v)
    soviet_skirts(v, label="114", outer=1.85)
    rear_drums(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    hmg, hmg_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    gun_2a46(v, gun)
    cupola_mg(v, hmg, hmg_gun, remote=True)


def wreck(variant, v):
    soviet_wreck(v, ("gunner_hatch", "net_screen_L", "bustle_bin_"), seed=3)


run("t90", "russian_green", build, wreck, chip=1.0)
