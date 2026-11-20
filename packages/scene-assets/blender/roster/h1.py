"""Marine Corps AH-1Z and UH-1Y, adapted from licensed H-1 airframes.
The source pins record the original authors and changes to each aircraft.
The game owns rotor motion, crew, mount limits and crash poses.
"""

import math
import os
import sys
import bpy

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A
from aircraft_donor import fitted_airframe
from aircraft_panes import panel_shell
from parts import box, cyl, flat_paint
from vehicle_export import rig, run


def huey_panel_details(h, m):
    shell = bpy.data.objects["fuselage_huey_LOD0"]
    # Project the broad maintenance breaks onto the donor shell. These are
    # deliberately sparse at game distance and stay attached to the airframe.
    for side, suffix in ((1, "L"), (-1, "R")):
        for index, (x, lo, hi) in enumerate(((-3.65, 1.12, 1.84), (-2.65, 1.08, 1.90),
                                              (-1.42, 1.06, 1.86), (.05, 1.08, 1.94),
                                              (3.45, 1.10, 1.92))):
            points = []
            for z in (lo, hi):
                hit, point, normal, _ = shell.ray_cast((x, side * 2.0, z), (0, -side, 0))
                if hit:
                    points.append(tuple(point + normal * .006))
            if len(points) == 2:
                A.line(f"huey_access_seam_{suffix}_{index}", points[0], points[1], m["canopy_frame"], h,
                       width=.024, height=.024, lods=A.NEAR)
        # A lower sill break ties the cabin door, sponson and belly together.
        points = []
        for x in (-3.8, -2.1, -.3, 1.5, 3.1):
            hit, point, normal, _ = shell.ray_cast((x, side * 2.0, 1.02), (0, -side, 0))
            if hit:
                points.append(tuple(point + normal * .007))
        for a, b in zip(points, points[1:]):
            A.line(f"huey_sill_{suffix}_{a[0]:.1f}", a, b, m["dark"], h,
                   width=.026, height=.026, lods=A.MID)



def sliding_doors(m, h):
    glass = flat_paint(
        "smoked_door_lights", (0.010, 0.028, 0.038), rough=0.10, metal=0.12, grime=0, coverage=("blended", 0.44), role="glass"
    )
    for side, label in ((1, "L"), (-1, "R")):
        metal = []
        panes = []

        def at(x, z):
            y = 1.31 + 0.048 * (x - 0.85) - 0.18 * max(0.0, z - 1.55) - 0.10 * max(0.0, 1.03 - z)
            return (x, side * y, z)

        for lo, hi in ((0.05, 0.85), (0.85, 1.65)):
            middle = (lo + hi) * 0.5
            # Eight outer stations leave a rounded corner at each light.
            outline = [
                (lo, 0.76),
                (hi, 0.76),
                (hi, 1.08),
                (hi, 1.91),
                (hi - 0.15, 1.91),
                (lo, 1.91),
                (lo, 1.69),
                (lo, 1.22),
            ]
            cut = [
                (middle - 0.22, 1.29),
                (middle + 0.22, 1.29),
                (middle + 0.28, 1.35),
                (middle + 0.28, 1.68),
                (middle + 0.21, 1.75),
                (middle - 0.20, 1.75),
                (middle - 0.28, 1.67),
                (middle - 0.28, 1.36),
            ]
            outside = [at(x, z) for x, z in outline]
            inside = [at(x, z) for x, z in cut]
            for i in range(8):
                f = [outside[i - 1], outside[i], inside[i], inside[i - 1]]
                metal.append(f[::-1] if side > 0 else f)
            panes.append(inside[::-1] if side > 0 else inside)
        panel_shell("sliding_door_skin_" + label, metal, m["paint"], h)
        panel_shell("sliding_door_lights_" + label, panes, glass, h)
        box(
            "sliding_door_handle_" + label,
            (0.15, 0.037, 0.036),
            at(1.48, 1.21),
            m["dark"],
            h,
            bevel=0.009,
            lods=A.MID,
        )


def viper_skid_braces(m, h):
    """Seat the Cobra's two-point skid struts against the fitted donor shell."""
    shell = bpy.data.objects["fuselage_LOD0"]
    for side, label in ((1, "L"), (-1, "R")):
        for index, x in enumerate((3.55, 1.82)):
            hit, point, normal, _ = shell.ray_cast((x, side * 2.0, 0.82), (0, -side, 0))
            if not hit:
                raise ValueError(f"Viper skid brace misses the fuselage at {x}, side {side}")
            upper = tuple(point + normal * 0.012)
            if index == 1:
                continue
            lower_z = 0.28 if index == 1 else 0.18
            lower = (x - 0.10, side * 0.96, lower_z)
            A.line(
                f"viper_skid_brace_{label}_{index}",
                upper,
                lower,
                m["gear"],
                h,
                width=0.10,
                height=0.10,
                lods=A.ALL,
            )
            A.line(
                f"viper_skid_brace_rear_{label}_{index}",
                upper,
                (x + 0.20, side * 0.96, lower_z),
                m["gear"],
                h,
                width=0.075,
                height=0.075,
                lods=A.ALL,
            )


def venom(v):
    m = A.fit(v)
    h = v.hull
    m["canopy"] = flat_paint(
        "marine_laminated_glass",
        (0.014, 0.032, 0.043),
        rough=0.10,
        metal=0.14,
        grime=0,
        coverage=("blended", 0.42),
        role="glass",
    )
    fitted_airframe(v, "h1", m)
    huey_panel_details(h, m)
    A.body(
        "transmission_cowling",
        [
            (2.93, 0.23, 2.44, 2.77, 2.62, 3.6),
            (2.72, 0.43, 2.48, 3.09, 2.72, 3.6),
            (2.54, 0.46, 2.50, 3.14, 2.75, 3.8),
            (1.32, 0.46, 2.52, 3.12, 2.75, 3.8),
            (0.96, 0.42, 2.55, 3.02, 2.76, 3.6),
            (0.84, 0.30, 2.58, 2.86, 2.70, 3.4),
        ],
        m["paint"],
        h,
    )
    A.rotor("main", (2.12, 0, 3.57), 7.44, 4, 0.56, m, h, hub=0.37, mast=0.48, phase=0.32, droop=0.024)
    A.rotor(
        "tail", (-6.52, -0.30, 3.38), 1.27, 4, 0.24, m, h, hub=0.13, mast=0.25, droop=0, rot=(math.pi / 2, 0, 0)
    )
    for side, label in ((1, "L"), (-1, "R")):
        for i in range(4):
            box(
                "transmission_grille_" + label + str(i),
                (0.035, 0.023, 0.15),
                (1.48 + 0.067 * i, side * 0.452, 2.87),
                m["dark"],
                h,
                bevel=0.008,
                lods=A.MID,
            )
    # The Venom's lower nose has a broad, chin-like step under the windscreen.
    # The donor shell tapers too sharply here, so this loft carries the
    # characteristic bulbous lower profile into the FLIR mount.
    A.body("nose_lower_fairing", [(6.66, .03, .36, .92), (6.46, .30, .24, 1.22),
                                   (6.12, .52, .22, 1.18), (5.76, .58, .32, 1.06)],
            m["paint"], h, seg=20)
    # The Venom's FLIR hangs below a short collar instead of disappearing into
    # the rounded nose.
    A.body("nose_flir_fairing", [(6.18, .10, .34, .62), (6.02, .20, .28, .72),
                                  (5.82, .18, .29, .69), (5.70, .08, .34, .55)], m["paint"], h, seg=16)
    cyl("nose_flir_collar", .17, .13, (5.82, 0, .30), "Z", m["dark"], h, seg=16)
    A.sensor_ball("nose_sensor", (5.82, 0, .24), .20, m, h, window=.72)
    A.blade_antenna("belly_antenna", (3.10, 0, .82), .20, m, h, down=True)
    sliding_doors(m, h)
    # Give the open cabin a readable floor, seat frames and bulkhead depth.
    box("cabin_floor", (2.45, 2.20, .075), (1.10, 0, 1.02), m["dark"], h, bevel=.025, lods=A.MID)
    box("cabin_bulkhead", (.08, 2.12, 1.26), (-.10, 0, 1.63), m["dark"], h, bevel=.02, lods=A.MID)
    for side, label in ((1, "L"), (-1, "R")):
        y = side * .76
        box("cabin_seat_pan_" + label, (1.02, .42, .08), (1.05, y, 1.39), m["canvas"], h, bevel=.025, lods=A.MID)
        box("cabin_seat_back_" + label, (.08, .42, .56), (.58, y, 1.68), m["canvas"], h, bevel=.025, lods=A.MID)
        for x in (.70, 1.40):
            A.line("cabin_seat_leg_" + label + str(x), (x, y, 1.05), (x, y, 1.37), m["steel"], h,
                   width=.035, height=.035, lods=A.MID)
        # Two braces make the skid's attachment readable at gameplay distance.
        A.line("skid_brace_front_" + label, (2.85, side*.72, 1.18), (2.42, side*1.32, .05),
               m["gear"], h, width=.082, height=.082, lods=A.ALL)
    for side, label in ((1, "L"), (-1, "R")):
        shell = bpy.data.objects["fuselage_huey_LOD0"]
        for j, x in enumerate((1.20, 3.72)):
            hit, point, _, _ = shell.ray_cast((x, side * 2, 2.012), (0, -side, 0))
            if not hit:
                raise ValueError(f"Door rail bracket misses the fuselage at {x}, side {side}")
            A.line(
                "track_bracket_" + label + str(j),
                tuple(point),
                (x, side * 1.314, 2.012),
                m["paint"],
                h,
                width=0.060,
                height=0.060,
            )
        box("door_track_" + label, (2.58, 0.050, 0.065), (2.46, side * 1.315, 2.012), m["dark"], h)
        A.body(
            "exhaust_shroud_" + label,
            [
                (1.19, 0.27, 2.37, 2.89, 2.61, 3.1),
                (0.88, 0.34, 2.33, 2.97, 2.67, 3.2),
                (0.08, 0.32, 2.34, 2.93, 2.63, 3),
                (-0.52, 0.26, 2.40, 2.85, 2.63, 2.8),
            ],
            m["paint"],
            h,
            loc=(0, side * 0.61, 0),
        )
        A.nozzle("exhaust_" + label, (-0.49, side * 0.61, 2.61), 0.23, 0.25, 0.52, m, h, petals=8)
        A.round_intake("inlet_" + label, (1.18, side * 0.63, 2.64), 0.205, 0.10, m, h)
    A.door_guns(v, rig(v.frame, v.root), m, (-60, 20), inboard=0.25)
    A.markings(
        v,
        [
            ("insignia", dict(kind="us_lowvis", centre=(-3.25, 1.4, 1.90),
                              normal=(0, 1, 0), up=(0, 0, 1), size=.28, onto=("fuselage",))),
            (
                "text",
                dict(
                    text="MARINES",
                    height=0.19,
                    centre=(-2.65, 1.4, 1.94),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    onto=("fuselage",),
                    colour="lowvis_dark",
                ),
            )
        ],
    )


def viper(v):
    m = A.fit(v)
    hull = v.hull
    m["canopy"] = flat_paint(
        "cobra_glazing", (0.010, 0.024, 0.033), rough=0.09, metal=0.14, grime=0, coverage=("blended", 0.36), role="glass"
    )
    fitted_airframe(v, "h1", m)
    viper_skid_braces(m, hull)
    # The Z's nose camera has a broad fixed visor and a separate faceted housing.
    A.body(
        "nose_camera_chin",
        [
            (6.82, 0.07, 0.98, 1.14),
            (6.71, 0.22, 0.83, 1.28),
            (6.55, 0.30, 0.81, 1.29),
            (6.28, 0.29, 0.92, 1.28),
            (6.12, 0.15, 1.04, 1.23),
        ],
        m["paint"],
        hull,
        seg=12,
    )
    box("nose_camera_brow", (0.36, 0.63, 0.12), (6.54, 0, 1.28), m["paint"], hull, bevel=0.05)
    cyl("nose_camera_optic", 0.086, 0.035, (6.76, 0.084, 1.05), "X", m["glass"], hull, seg=20)
    A.chin_gun(v, rig(v.frame, v.root), m, (-50.0, 18.0), 0.06, drum=0.24)
    if not v.wreck:
        for name, x, z in [("pilot", 3.25, 1.71), ("gunner", 4.56, 1.40)]:
            v.crew.append(
                (
                    name,
                    hull,
                    (x, 0, z),
                    ((x + 0.3, 0.17, z + 0.24), (x + 0.3, -0.17, z + 0.24)),
                    ((x + 0.48, 0.17, z - 0.36), (x + 0.48, -0.17, z - 0.36)),
                )
            )
    A.body(
        "transmission_cap",
        [
            (3.14, 0.09, 2.76, 3.20),
            (3.05, 0.29, 2.71, 3.34, 3.10, 5),
            (2.72, 0.34, 2.73, 3.38, 3.10, 5),
            (2.38, 0.34, 2.72, 3.36, 3.08, 5),
            (2.11, 0.29, 2.76, 3.19),
            (1.96, 0.1, 2.77, 2.94),
        ],
        m["paint"],
        hull,
        loc=(0, 0, -0.065),
    )
    cyl("transmission_shaft", 0.095, 0.48, (2.57, 0, 3.19), "Z", m["steel"], hull, seg=18)
    A.rotor("main", (2.57, 0, 3.51), 7.32, 4, 0.52, m, hull, hub=0.36, mast=0.10, droop=0.025, phase=0.2)
    A.rotor(
        "tail",
        (-6.44, -0.36, 2.415),
        1.17,
        4,
        0.24,
        m,
        hull,
        hub=0.13,
        mast=0.39,
        droop=0,
        rot=(math.pi / 2, 0, 0),
        phase=0.4,
    )


def build(variant, v):
    if variant["id"] == "us_ah_1z_viper_attack_helicopter":
        viper(v)
    else:
        venom(v)


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.2, tail_yaw=-0.45, tail_drop=0.1, seed=11)


run(
    "h1",
    "us_compass_grey",
    build,
    wreck,
    crew_army="usmc_marpat",
    references=["assets/references/ah_1z_viper/references.json", "assets/references/uh_1y_venom/references.json"],
    authoring="Adapted from drcrazzie’s Bell AH-1 SuperCobra and helijah’s Bell UH-1N Twin Huey, both CC BY 4.0.",
)
