"""Mi-28NM, from assets/references/mi_28_havoc/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/havoc.py -- [--wreck]

What the photos settle: the stepped tandem cockpits under flat armoured
glass, the sensor turret in the nose and the 30 mm cannon on its chin
mount, engines in nacelles either side of the gearbox with down-turned
exhaust suppressors, stub wings with anhedral and two stations each plus
wing-tip launchers, the tail with its X-shaped tail rotor on the right and
the stabilator, the NM's radar dome over the rotor head; fixed main wheels
and tail wheel. Dark grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the wing tips, height to the top
of the mast radar): 17.01 x 4.88 x 4.7 m.

The 30 mm cannon is the `chin` mount's rig; the missiles are fixed in the
hull (the `missiles` mount's muzzle is the right rack's upper tube).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_parts import MID  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# The 2A42's elevation in its chin mount, degrees, and its stroke.
GUN_PITCH_DEG = (-40.0, 13.0)
GUN_STROKE_M = 0.08

def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    # Keep the donor's framed panes while using the same controlled glass
    # finish as the other attack helicopters, so cockpit depth survives close
    # views without turning the windows into black holes.
    m["canopy"] = flat_paint("havoc_laminated_glass", (0.018, 0.028, 0.034), rough=0.13,
                             grime=0, coverage=("blended", 0.38), role="glass")
    fitted_airframe(v, "mi_28_havoc", m, skip_parts=("cockpit",))
    if not v.wreck:
        # Keep the donor glazing, but give each tandem station a visible seat,
        # headrest, coaming and instrument panel behind it.
        for label, x, z in (("front", 5.35, 1.55), ("rear", 4.25, 1.48)):
            box(f"cockpit_seat_{label}", (0.18, 0.46, 0.62), (x, 0, z), m["cockpit"], hull,
                bevel=0.025, rot=(0, -0.12, 0), lods=MID)
            box(f"cockpit_headrest_{label}", (0.16, 0.34, 0.20), (x - 0.05, 0, z + 0.38), m["black"], hull,
                bevel=0.018, lods=MID)
            box(f"cockpit_coaming_{label}", (0.42, 0.62, 0.12), (x + 0.28, 0, z + 0.31), m["cockpit"], hull,
                bevel=0.025, rot=(0, -0.08, 0), lods=MID)
            box(f"cockpit_panel_{label}", (0.05, 0.48, 0.16), (x + 0.48, 0, z + 0.26), m["dark"], hull,
                bevel=0.012, rot=(0, -0.18, 0), lods=MID)
        for name, x, z in (("pilot", 5.35, 1.55), ("gunner", 4.25, 1.48)):
            v.crew.append((name, hull, (x, 0, z),
                           ((x + .28, .14, z + .22), (x + .28, -.14, z + .22)),
                           ((x + .48, .14, z - .30), (x + .48, -.14, z - .30))))
    A.body("chin_fairing", [(7.06, 0, 1.10, 1.10), (6.90, .25, .94, 1.20, 1.05),
                            (6.54, .31, .93, 1.20, 1.05), (6.23, .23, 1.01, 1.21, 1.12),
                            (6.09, 0, 1.18, 1.18)], m["paint"], hull, seg=16)
    A.chin_gun(v, rig(v.frame, v.root), m, GUN_PITCH_DEG, GUN_STROKE_M, drum=0.28)
    # The donor rotor bearings set the pivots. The radar above the head is
    # fixed to the mast and remains facing forward as the blades turn.
    A.rotor("main", (3.315, 0, 3.74), 8.6, 5, 0.58, m, hull, hub=.44, mast=.33,
            droop=.03, phase=.3)
    cyl("main_rotor_gearbox", .29, .22, (3.315, 0, 3.98), "Z", m["dark"], hull, seg=16, bevel=.025, lods=MID)
    cyl("mast_radar_post", .18, .65, (3.315, 0, 4.20), "Z", m["dark"], hull, seg=12)
    radome = m["paint"]
    sections = []
    for i in range(13):
        angle = math.pi * i / 12
        width = .35 * math.sin(angle)
        sections.append((3.315 + .37 * math.cos(angle), width, 4.50 - width*.58, 4.50 + width*.58))
    A.body("mast_radar", sections, radome, hull, seg=20)
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-3.6, 1, 2.01), normal=(0, 1, 0), up=(0, 0, 1),
                          size=.37, onto=("fuselage",))),
        ("text", dict(text="731", height=.32, centre=(4.93, 1.15, 1.98), normal=(0, 1, 0), up=(0, 0, 1),
                      colour="yellow", onto=("fuselage",))),
    ])
    A.rotor("tail", (-7.51, -.60, 3.09), 1.9, 4, .28, m, hull, hub=.16, mast=.16,
            droop=0, rot=(math.pi / 2, 0, 0), thick=.12, phase=.4)


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-2.8, tail_yaw=0.42, tail_drop=0.1, seed=28)


run("havoc", "russian_air_grey", build, wreck, crew_army="eastern_emr", references="assets/references/mi_28_havoc/references.json")
