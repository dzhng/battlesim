"""T-90M Proryv, from assets/references/t90/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t90.py -- [--variant=<id>] [--wreck]

What the photos settle (Army-2020 to 2023): the T-72-derived hull and
running gear (six ribbed road wheels, three return rollers, Relikt panels in
two rows over the front of the hull sides above a scalloped rubber skirt,
slat panels over the rear), Relikt in rows on the glacis, and two external
fuel drums across the hull rear over a slat panel. The welded turret is
angular: Relikt wedges either side of the gun, tiled on top, its flat sides
hung with net screens, a deep box bustle in a net cage, smoke dischargers on
the front roof corners, the gunner's Sosna-U box on the left, the commander's
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
ROOF = 0.62


def turret_rings():
    """The welded turret: a broad front, flat sides, a deep box bustle."""
    half = [(1.35, 0.42), (1.28, 1.42), (-0.55, 1.50), (-0.95, 1.30), (-2.20, 1.26), (-2.32, 1.10)]

    def ring(pts, lean):
        left = [(x, y - lean) for x, y in pts]
        return left + [(x, -y) for x, y in reversed(left)]

    return [(FOOT, ring(half, 0.0)), (ROOF, ring([(x - (0.08 if x > 1.0 else 0.0), y) for x, y in half], 0.08))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_rings(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.12, 0.10, (0, 0, FOOT - 0.05), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # The Relikt wedge on each cheek, tiled on top.
        lo = [(1.30, 0.44), (2.02, 0.52), (1.62, 1.55), (1.10, 1.55)]
        hi = [(1.30, 0.44), (1.78, 0.50), (1.45, 1.42), (1.10, 1.42)]
        rings = []
        for z, pts in ((FOOT, lo), (ROOF - 0.06, hi)):
            p = [(x, side * y) for x, y in pts]
            rings.append((z, p if side > 0 else p[::-1]))
        loft(f"cheek_wedge_{s}", rings, mat=m["paint"], parent=turret, bevel=0.04)
        VP.armour_tiles(f"cheek_tiles_{s}", (1.48, side * 0.98, ROOF - 0.05), (0.62, 0.95), (2, 3), 0.07, m, turret,
                        rot=(0, 0.22, 0))
        VP.armour_tiles(f"cheek_face_{s}", (1.86, side * 1.00, 0.26), (0.50, 0.95), (1, 3), 0.08, m, turret,
                        rot=(0, math.pi / 2 - 0.35, side * -0.30))
        # Net screens hung off the turret side, the cage round the bustle.
        VP.slat_armour(f"net_screen_{s}", (0.20, side * 1.60, FOOT - 0.10), (2.20, 0.52), m, turret, spacing=0.10,
                       bar=0.012)
        VP.slat_armour(f"bustle_cage_{s}", (-1.65, side * 1.42, FOOT - 0.06), (1.20, 0.60), m, turret, spacing=0.10,
                       bar=0.014)
        VP.smoke_discharger_bank(f"smoke_{s}", (1.05, side * 1.22, ROOF), m, turret, count=6, tube_radius=0.045,
                                 tube_length=0.22, elevation=0.30, spread=0.6, rot=(0, 0, side * 0.60))
        VP.stowage_box(f"bustle_bin_{s}", (-1.55, side * 1.30, ROOF - 0.02), (0.90, 0.30, 0.20), m, turret,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.95, side * 0.85, ROOF), m, whip, height=2.2)
    VP.slat_armour("bustle_cage_rear", (-2.46, 0, FOOT - 0.06), (2.40, 0.60), m, turret, spacing=0.10, bar=0.014,
                   rot=(0, 0, math.pi / 2))
    # Roof: the gunner's sight box on the left, the commander's panoramic
    # sight on the right, his hatch behind the remote gun, the loader-less
    # gunner's hatch.
    VP.sight_housing("sosna_sight", (0.62, 0.62, ROOF - 0.02), m, turret, size=(0.44, 0.36, 0.28))
    sight = empty("dressing_commander_sight", parent=turret)
    cyl("commander_sight_base", 0.20, 0.20, (0.30, -0.92, ROOF + 0.10), "Z", m["paint"], sight, seg=20,
        bevel=0.02, lods=MID)
    VP.sight_housing("commander_sight", (0.30, -0.92, ROOF + 0.18), m, sight, size=(0.40, 0.42, 0.30))
    VP.cupola("commander_cupola", (-0.85, -0.62, ROOF), m, turret, radius=0.32, periscopes=4, lid_open=False)
    VP.hatch("gunner_hatch", (-0.50, 0.62, ROOF), m, turret, radius=0.30)
    VP.tarp_roll("bustle_tarp", (-1.75, 0.0, ROOF + 0.12), 1.40, 0.13, m, turret, straps=3)


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
    soviet_wreck(v, ("gunner_hatch", "bustle_tarp", "net_screen_L", "bustle_bin_"), seed=3)


run("t90", "russian_green", build, wreck, chip=1.0)
