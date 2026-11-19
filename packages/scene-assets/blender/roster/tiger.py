"""Tiger HAD and UHT, from assets/references/ec665_tiger/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/tiger.py -- [--variant=<card>] [--wreck]

What the photos settle: a slim tandem two-seater, pilot forward and gunner
behind and above under separate canopies, the engines either side of the
rotor mast with their exhausts turned up and out, stub wings with two
stations a side, a horizontal tailplane with end-plate fins, the fin with
the three-blade tail rotor on its right, four main blades, fixed main wheels
and a tail wheel. The HAD (Spain, France) has the roof sight over the
cockpit and the 30 mm chin turret; the UHT (Germany) has the mast-mounted
sight on the rotor head and no turret. Army green-grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the stub wings' stores, height to
the top of the rotor head or what stands on it): HAD 14.08 x 4.32 x 3.83 m;
UHT 14.08 x 4.32 x 4.32 m.

The HAD's 30 mm gun is the `chin` mount's rig, within the THL-30 turret's
elevation. Both carry their missiles fixed in the hull (the `missiles`
mount's muzzle is the right outer rail's missile front); the UHT's inboard
stations carry rocket launchers, fixed too (the `rockets` mount's muzzle is
the left launcher's mouth).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

HAD, UHT = "europe_ec665_tiger_had", "europe_ec665_tiger_uht"
# The THL-30 turret's elevation, degrees, and the gun's stroke.
GUN_PITCH_DEG = (-28.0, 28.0)
GUN_STROKE_M = 0.06


def build(variant, v):
    had = variant["id"] == HAD
    hull = v.hull
    m = A.fit(v)
    m["canopy"] = flat_paint("tiger_glass", (.006, .012, .016), rough=.10, metal=.12,
                             grime=0, role="glass")
    m["store"] = flat_paint("tiger_store_enamel", (.083, .091, .072), rough=.75, grime=.10, role="paint")
    lens = flat_paint("coated_optics", (.009, .024, .034), metal=.32, rough=.10, grime=0, role="glass")
    fitted_airframe(v, "ec665_tiger", m)
    # The crew sit at the front and rear seat pans, beneath the two
    # different windscreen heights. Hands follow the existing controls.
    if not v.wreck:
        for name, x, z in (("front_crew", 4.58, 1.24), ("rear_crew", 3.02, 1.63)):
            v.crew.append((name, hull, (x, 0, z),
                           ((x+.31, .18, z+.25), (x+.34, -.12, z+.23)),
                           ((x+.55, .16, z-.36), (x+.55, -.16, z-.36))))
    for side, suffix in ((1, "L"), (-1, "R")):
        for slot, lateral in (("inner", 1.05), ("outer", 2.0)):
            A.pylon(f"pylon_{slot}_{suffix}", (2.12, side*lateral, 1.66), .74, .27, m, hull)
        A.store("inner_store_" + suffix, "pod" if had else "rockets", (2.12, side*1.05, 1.15),
                1.6, .22, m, hull)
        A.store("outer_store_" + suffix, "missile", (2.12, side*2, 1.24),
                1.63 if had else 1.5, .09 if had else .13, m, hull, rail=True)
    A.rotor("main", (1.90, 0, 3.66), 6.5, 4, .5, m, hull, hub=.36, mast=.30, phase=.5)
    A.rotor("tail", (-6.11, -.49, 2.93), 1.35, 3, .24, m, hull, hub=.13, mast=.19,
            droop=0, rot=(math.pi / 2, 0, 0), thick=.12)
    for side, suffix in ((1, "L"), (-1, "R")):
        # Round black ports sit flush with the sloping nose cheeks.
        yaw = -side * .28
        cyl("nose_port_rim_" + suffix, .073, .020, (6.644, side*.245, 1.267), "Y",
            m["dark"], hull, seg=20, rot=(0, 0, yaw), lods=A.NEAR)
        cyl("nose_port_lens_" + suffix, .055, .012, (6.648, side*.259, 1.267), "Y",
            m["glass"], hull, seg=20, rot=(0, 0, yaw), lods=A.NEAR)
    A.line("rear_windscreen_wiper", (4.09, -.20, 2.52), (3.79, -.03, 2.70),
           m["dark"], hull, width=.014, height=.014)
    A.line("rear_windscreen_blade", (3.87, -.075, 2.647), (3.55, .105, 2.831),
           m["dark"], hull, width=.020, height=.012)
    if had:
        # The HAD roof sight has a transverse barrel and an optical face,
        # cradled by the two cheeks immediately over the rear canopy.
        # The armoured hood wraps around the optical drum. A narrow reveal
        # between the hood and its cheeks keeps the gimbal visible in profile.
        A.body("roof_sight_hood", [(3.33, .18, 2.996, 3.18, 3.08, 3.0),
                                   (3.26, .225, 2.954, 3.263, 3.09, 3.0),
                                   (3.06, .229, 2.949, 3.288, 3.10, 3.1),
                                   (2.92, .205, 2.961, 3.246, 3.09, 3.0),
                                   (2.86, .16, 3.01, 3.16)], m["paint"], hull, seg=20)
        for side, suffix in ((1, "L"), (-1, "R")):
            cyl("roof_sight_cheek_" + suffix, .17, .06, (3.07, side*.246, 3.077),
                "Y", m["paint"], hull, seg=20, bevel=.012)
        box("roof_sight_mask", (.032, .33, .20), (3.335, 0, 3.095), m["dark"], hull, bevel=.018, lods=MID)
        for y, radius in ((-.079, .072), (.084, .055)):
            cyl("roof_sight_aperture_" + str(y), radius, .016, (3.354, y, 3.095), "X", lens, hull, seg=20)
        mounts = rig(v.frame, v.root)
        A.chin_gun(v, mounts, m, GUN_PITCH_DEG, GUN_STROKE_M, drum=.24)
        # Faceted outer fairing seen under the HAD nose.
        pitch = mounts["chin"][1]
        A.body("chin_shroud", [(.38, .10, -.18, -.04), (.25, .18, -.22, .03),
                               (-.20, .20, -.22, .10), (-.29, .15, -.14, .09)],
               m["dark"], pitch, seg=8, steps=(1, 1, 1, 1))
    else:
        cyl("mast_sight_post", .10, .26, (1.90, 0, 3.88), "Z", m["dark"], hull, seg=12)
        # The OSIRIS head rounds over the transverse trunnions. The large
        # window occupies the upper face above two smaller optical apertures.
        A.body("mast_sight_housing", [
            (2.17, .195, 3.998, 4.267, 4.14, 3.5),
            (2.10, .26, 3.949, 4.313, 4.14, 3.0),
            (1.92, .305, 3.937, 4.318),
            (1.76, .282, 3.956, 4.30),
            (1.64, .18, 4.015, 4.256),
            (1.61, .11, 4.056, 4.219),
        ], m["paint"], hull, seg=24)
        box("mast_sight_mask", (.024, .37, .247), (2.175, 0, 4.134),
            m["dark"], hull, bevel=.018, lods=MID)
        window = flat_paint("mast_optical_window", (.010, .022, .037),
                            metal=.5, rough=.12, grime=0, role="glass")
        box("mast_upper_window", (.016, .255, .112), (2.19, 0, 4.181),
            window, hull, bevel=.016, lods=MID)
        for y, radius in ((-.077, .047), (.074, .039)):
            cyl("mast_sight_lens_" + str(y), radius, .014, (2.191, y, 4.074),
                "X", lens, hull, seg=20)
        A.sensor_ball("nose_flir", (6.35, 0, .92), .23, m, hull)
        for side, suffix in ((1, "L"), (-1, "R")):
            cyl("mast_sight_side_" + suffix, .13, .045, (1.90, side*.30, 4.12),
                "Y", m["paint"], hull, seg=16)
    # The ALAT's roundel on the HAD, the Heer's cross and serial on the UHT.
    if variant["id"] == HAD:
        A.markings(v, [
            ("insignia", dict(kind="fr", centre=(-2.5, 1.2, 1.68), normal=(0, 1, 0), up=(0, 0, 1), size=0.22, onto=("fuselage",))),
            ("text", dict(text="ALAT", height=0.16, centre=(-4.2, 1.2, 1.62), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="lowvis_dark")),
        ])
    else:
        A.markings(v, [
            ("insignia", dict(kind="de", centre=(-2.5, 1.2, 1.68), normal=(0, 1, 0), up=(0, 0, 1), size=0.22, onto=("fuselage",))),
            ("text", dict(text="74+27", height=0.2, centre=(-4.3, 1.2, 1.62), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
        ])

def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.4, tail_yaw=0.42, tail_drop=0.1, seed=66)


run("tiger", "nato_helicopter_green", build, wreck, crew_army={HAD: "french_cce", UHT: "german_flecktarn"},
    references="assets/references/ec665_tiger/references.json")
