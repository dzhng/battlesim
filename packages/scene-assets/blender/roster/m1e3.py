"""M1E3 Abrams prototype, from assets/references/m1e3/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/m1e3.py -- [--wreck]

Built on the Abrams (`abrams.py`): the M1 hull, its running gear,
fittings, gun and wreck damage are that script's. What the one photo of the
early prototype (Detroit, January 2026) changes: the skirts are a long wedge
over the idler and four long flat panels, painted with the star and lettering; the
turret is low, faceted and short, its cheeks a wedge either side of the gun
shield and its sides flat, with no bustle rack and no loader's hatch (the
crew rides in the hull); a remote weapon station stands high on a pedestal
in the middle of the roof (a CROWS-type: its M2 over a three-window
sensor block, the ammunition can and feed chute on its left, drives and
cable harness), and an independent
sight box stands behind it on the left. The prototype is green; the model
wears US desert tan like every US roster vehicle.

The photographed prototype shows no protection system; the card fields
Trophy, so the Abrams' Trophy stations (`abrams.trophy_station`) stand on
the flat turret sides, back from the smoke banks, as on the SEPv3.

Built to the catalog frame, the SEPv3 Trophy's box (hull 7.93 x 3.66 x
2.44 m, gun pivot 1.464 m), with the remote station's mount on its pedestal
mid-roof.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import abrams as A  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft, prism, stencil  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

ROOF = 0.80
# The turret's foot, front to rear, left half: the gun shield's recess, the
# faceted cheek, the flat side and the short rear.
TURRET_PLAN = [(1.40, 0.0), (1.40, 0.42), (2.05, 0.50), (1.05, 1.58), (-1.60, 1.60), (-1.95, 1.30), (-1.95, 0.0)]


def turret_shell():
    left = TURRET_PLAN
    outline = left + [(x, -y) for x, y in reversed(left) if y > 0]

    def ring(z):
        out = []
        for x, y in outline:
            a = abs(y)
            if z == ROOF:  # the roof: cheeks swept back, sides leaning in
                xx = x - (0.38 if x > 1.0 and a > 0.45 else 0.06 if x > -1.9 else -0.04)
                yy = y if a <= 0.50 else math.copysign(a - 0.12, y)
            elif z == 0.0:  # the undercut foot
                xx, yy = (x - 0.04 if x > 1.0 and a > 0.45 else x), (y * 0.94 if a > 0.6 else y)
            else:
                xx, yy = x, y
            out.append((xx, yy))
        return out

    return [(0.0, ring(0.0)), (0.18, ring(0.18)), (ROOF, ring(ROOF))]


def turret_body(v, turret):
    m = v.mats
    loft("turret_shell", turret_shell(), mat=m["paint"], parent=turret, bevel=0.05)
    cyl("turret_ring_guard", 1.05, 0.10, (0, 0, -0.02), "Z", m["dark"], turret, seg=40, lods=MID)
    for side, s in ((1, "L"), (-1, "R")):
        # The cheek's facet lines and the flat side's bolted plates.
        VP.weld_line(f"cheek_weld_{s}", [(1.70, side * 0.50, ROOF + 0.002), (0.95, side * 1.44, ROOF + 0.002)], m,
                     turret)
        loc, rot = VP.on_side(-0.35, 0.45, side, (1.588, 0.18), (1.468, ROOF))
        VP.bolted_panel(f"side_plate_{s}", loc, (1.90, 0.46, 0.05), m, turret, bolts=(4, 2), bevel=0.02, rot=rot,
                        lods=VP.ALL)
        # Four-tube smoke banks at the cheek's rear corner, fanned forward.
        VP.smoke_discharger_bank(f"smoke_{s}", (0.70, side * 1.50, ROOF - 0.10), m, turret, count=4,
                                 tube_radius=0.05, tube_length=0.26, elevation=0.35, spread=0.3,
                                 rot=(0, 0, side * 0.6))
        # Trophy: the radar housing against the flat side, its top as far
        # above the roof as the SEPv3's, the launcher on it.
        A.trophy_station(v, turret, side, s, at=(-1.10, 1.70, ROOF - 0.12))
        whip = empty(f"dressing_antenna_{s}", parent=turret)
        VP.antenna(f"antenna_{s}", (-1.75, side * 1.05, ROOF), m, whip, height=2.2)
    # Roof: the gunner's sight ahead on the right, the independent sight box
    # behind the station on the left, access plates and lifting eyes.
    VP.sight_housing("gunner_sight", (1.05, -0.62, ROOF - 0.05), m, turret, size=(0.52, 0.42, 0.26))
    mast = empty("dressing_sight_box", parent=turret)
    box("sight_box_post", (0.20, 0.20, 0.12), (-1.05, 0.62, ROOF + 0.06), m["dark"], mast, lods=MID)
    VP.sight_housing("sight_box", (-1.05, 0.62, ROOF + 0.12), m, mast, size=(0.40, 0.38, 0.30))
    # The cheeks' sensor windows (the protection system's and the
    # situational-awareness cameras), and the gun shield's bolt rows.
    for side, s in ((1, "L"), (-1, "R")):
        box(f"cheek_sensor_{s}", (0.03, 0.22, 0.12), (1.86, side * 0.66, ROOF - 0.32), m["glass"], turret,
            rot=(0, 0, side * 0.70), lods=MID)
        box(f"cheek_sensor_hood_{s}", (0.10, 0.28, 0.03), (1.84, side * 0.66, ROOF - 0.24), m["paint"], turret,
            rot=(0, 0, side * 0.70), lods=NEAR)
        for k in range(4):
            cyl(f"shield_bolt_{s}_{k}", 0.022, 0.03, (1.415, side * 0.42, 0.12 + k * 0.17), "X", m["steel"], turret,
                seg=6, lods=FINE)
        cyl(f"camera_{s}", 0.03, 0.06, (-1.90, side * 0.9, ROOF - 0.12), "X", m["dark"], turret, seg=10, lods=NEAR)
    box("sight_box_window_2", (0.01, 0.16, 0.10), (-0.84, 0.70, ROOF + 0.30), m["glass"], mast, lods=NEAR)
    for k, (x, y) in enumerate(((-0.30, -0.70), (-1.20, -0.55))):
        box(f"roof_plate_{k}", (0.70, 0.55, 0.025), (x, y, ROOF + 0.01), m["paint"], turret, bevel=0.008, lods=MID)
    for k, (x, y) in enumerate(((1.20, 0.95), (1.20, -0.95), (-1.80, 1.1), (-1.80, -1.1))):
        box(f"lift_eye_{k}", (0.12, 0.035, 0.09), (x, y, ROOF + 0.04), m["steel"], turret, lods=FINE)


def remote_station(v, rws, rws_gun, mount):
    """The remote weapon station high on its pedestal: a column off the roof
    on its bolted flange with the cable harness running down it, the bearing
    and the traverse drive's housing, the cradle with its elevation drive,
    the sensor block under the M2 (its day camera, thermal and rangefinder
    windows under an armoured hood), the ammunition can on the left with its
    latches and the feed chute arcing over to the gun's tray."""
    m = v.mats
    lift = mount["muzzle_m"][2]
    drop = mount["pivot_m"][2] - (v.frame["mounts"][0]["pivot_m"][2] + ROOF)
    cyl("rws_pedestal", 0.16, drop, (0, 0, -drop / 2), "Z", m["dark"], rws, seg=20, lods=MID)
    cyl("rws_flange", 0.26, 0.04, (0, 0, -drop + 0.02), "Z", m["dark"], rws, seg=20, bevel=0.01, lods=MID)
    for k in range(6):
        a = k * math.tau / 6
        cyl(f"rws_flange_bolt_{k}", 0.022, 0.03, (0.21 * math.cos(a), 0.21 * math.sin(a), -drop + 0.05), "Z",
            m["steel"], rws, seg=6, lods=FINE)
    VP.cable("rws_harness", [(-0.17, 0.0, -0.10), (-0.19, 0.02, -drop / 2), (-0.24, 0.05, -drop + 0.06)], m, rws,
             radius=0.022, eyes=False)
    cyl("rws_bearing", 0.26, 0.08, (0, 0, 0.02), "Z", m["dark"], rws, seg=24, bevel=0.01)
    box("rws_base", (0.46, 0.40, 0.10), (-0.02, 0, 0.10), m["paint"], rws, bevel=0.02)
    box("rws_traverse_drive", (0.20, 0.16, 0.16), (-0.20, -0.20, 0.12), m["dark"], rws, bevel=0.015, lods=MID)
    for side in (-1, 1):
        box(f"rws_cradle_{side}", (0.30, 0.05, lift + 0.04), (0.02, side * 0.17, (lift + 0.04) / 2 + 0.10),
            m["paint"], rws, bevel=0.012)
    box("rws_elevation_drive", (0.16, 0.08, 0.16), (-0.04, 0.22, lift - 0.02), m["dark"], rws, bevel=0.012, lods=MID)
    # The sensor block: three windows, the thermal's the largest, under a hood.
    # The photo's station is a big block beside the gun, so the sensor block
    # and the can are drawn at a CROWS's size, not the M2's.
    VP.sight_housing("rws_sight", (0.12, -0.33, -0.16), m, rws_gun, size=(0.40, 0.24, 0.30))
    for k, (y, z, r) in enumerate(((-0.37, 0.07, 0.045), (-0.28, 0.07, 0.032), (-0.33, -0.01, 0.022))):
        cyl(f"rws_window_{k}", r, 0.006, (0.325, y, z - 0.16 + 0.12), "X", m["glass"], rws_gun, seg=12, lods=NEAR)
    box("rws_ammo", (0.42, 0.20, 0.34), (-0.08, 0.33, -0.10), m["paint"], rws_gun, bevel=0.02, lods=MID)
    for k, x in enumerate((-0.22, 0.06)):
        box(f"rws_ammo_latch_{k}", (0.04, 0.012, 0.05), (x, 0.435, -0.02), m["steel"], rws_gun, lods=FINE)
    box("rws_ammo_lid", (0.44, 0.22, 0.03), (-0.08, 0.33, 0.085), m["paint"], rws_gun, bevel=0.008, lods=NEAR)
    VP.cable("rws_feed", [(-0.08, 0.30, 0.08), (-0.05, 0.22, 0.20), (0.0, 0.08, 0.10)], m, rws_gun, radius=0.03,
             eyes=False)
    box("rws_feed_tray", (0.14, 0.10, 0.03), (0.06, 0.07, 0.085), m["dark"], rws_gun, lods=NEAR)
    box("rws_charger", (0.12, 0.05, 0.05), (-0.18, -0.08, 0.02), m["dark"], rws_gun, bevel=0.01, lods=NEAR)
    VP.browning_m2(rws_gun, mount["muzzle_m"][0], m)


def skirts(v):
    """A long wedge over the idler and four long flat panels a side, the flag
    and star stencilled on them; bolts along their tops, which run along the
    hull's side (`abrams.skirt_top`)."""
    m, hull = v.mats, v.hull
    top, foot, sy = A.skirt_top, A.SKIRT_FOOT, A.SKIRT_Y
    panels = [A.front_skirt(2.30)]
    for front, rear in ((2.28, 0.92), (0.90, -0.46), (-0.48, -1.84)):
        panels.append([(rear, foot), (front, foot), (front, top(front)), (rear, top(rear))])
    panels.append([(-3.62, 1.06), (-3.30, 0.94), (-3.02, 0.72), (-2.68, foot), (-1.86, foot), (-1.86, top(-1.86)),
                   (-3.62, top(-3.62))])
    for side, s in ((1, "L"), (-1, "R")):
        face = side * (sy + 0.045)
        for k, outline in enumerate(panels):
            prism(f"skirt_{s}_{k}", outline, 0.11, loc=(0, side * (sy - 0.01), 0), mat=m["paint"], parent=hull,
                  bevel=0.03, lods=VP.ALL)
            xs = [p[0] for p in outline]
            for j in range(4):
                bx = min(xs) + (max(xs) - min(xs)) * (j + 0.5) / 4
                cyl(f"skirt_bolt_{s}_{k}_{j}", 0.022, 0.02, (bx, face + side * 0.006, top(bx) - 0.05), "Y", m["steel"],
                    hull, seg=6, lods=FINE)
        turn = (math.pi / 2, 0, math.pi if side > 0 else 0)
        star = [(1.60 + 0.30 * math.sin(k * math.pi / 5) * (1 if k % 2 == 0 else 0.4),
                 1.00 + 0.30 * math.cos(k * math.pi / 5) * (1 if k % 2 == 0 else 0.4)) for k in range(10)]
        prism(f"skirt_star_{s}", star, 0.006, loc=(0, face, 0), mat=m["marking"], parent=hull, lods=NEAR)
        stencil(f"skirt_army_{s}", "U.S. ARMY", 0.22, (0.15, face + side * 0.003, 1.02), turn, m["marking"], hull)


def build(variant, v):
    A.hull_body(v, sep_v3=False)
    A.running_gear(v)
    skirts(v)
    mounts = rig(v.frame, v.root, trunnion={"cannon": A.TRUNNION})
    turret, gun, _, _ = mounts["cannon"]
    rws, rws_gun, _, _ = mounts["HMG"]
    turret_body(v, turret)
    A.main_gun(v, gun, sep_v3=True)
    remote_station(v, rws, rws_gun, next(mt for mt in v.frame["mounts"] if mt["name"] == "HMG"))


def wreck(variant, v):
    """The Abrams' damage on this hull (`abrams.wreck`): the right track
    thrown with road wheels gone, two long skirt panels torn away, warped
    plates, a dented glacis and debris; Trophy burns off with the dressing,
    and `wreckage.burn` heaves the turret."""
    A.wreck(variant, v)


if __name__ == "__main__":
    run("m1e3", "us_desert_tan", build, wreck, chip=1.0)
