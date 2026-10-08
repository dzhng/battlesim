"""AH-6M and MH-6M Little Bird, from assets/references/ah_6_mh_6_little_bird/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/little_bird.py -- [--variant=<card>] [--wreck]

What the photos settle: the small egg-shaped cabin almost all glazing, the
thin tail boom with its T-tail and small end fins, the four-blade tail rotor,
the six-blade main rotor on a short mast, skids; the AH-6M carries weapon
planks with rocket pods and miniguns, the MH-6M the outboard benches its
assaulters ride. Special operations black.

Dimensions stated from published figures (rotorcraft rule: fuselage length
without blades, width over the planks' stores or benches, height to the top
of the rotor head): AH-6M 7.5 x 3.3 x 2.69 m; MH-6M 7.5 x 3.0 x 2.69 m.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

AH6, MH6 = "us_ah_6_mh_6_little_bird_ah_6m", "us_ah_6_mh_6_little_bird_mh_6m"
CARDS = {AH6: (7.5, 3.3, 2.69), MH6: (7.5, 3.0, 2.69)}
SPEC = dict(
    fuselage=[(2.6, 0.0, 1.0, 1.0), (2.4, 0.55, 0.56, 1.6, 1.05, 2.2), (1.6, 0.72, 0.42, 1.86, 1.05, 2.4),
              (0.4, 0.72, 0.44, 1.96, 1.1, 2.4), (-0.8, 0.55, 0.6, 1.86, 1.2, 2.2), (-1.4, 0.2, 1.08, 1.56, 1.3),
              (-4.6, 0.1, 1.22, 1.4, 1.3), (-4.9, 0.08, 1.24, 1.38, 1.3)],
    canopy=dict(x_front=2.56, x_back=0.9, sill=0.86, top=1.8, half_width=0.72, peak=0.65),
    stab=[(-4.2, 0.08, 1.95, 0.45, 0.05), (-4.3, 0.75, 1.95, 0.35, 0.04)],
    fins=[dict(root_x=-4.0, root_z=1.38, height=0.62, root_chord=0.6, tip_chord=0.45, sweep_m=0.3, rudder=False),
          dict(root_x=-4.2, root_z=1.95, height=0.3, root_chord=0.35, tip_chord=0.3, sweep_m=0.1, y=0.75,
               rudder=False),
          dict(root_x=-4.2, root_z=1.95, height=0.3, root_chord=0.35, tip_chord=0.3, sweep_m=0.1, y=-0.75,
               rudder=False)],
)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    cyl("mast", 0.1, 0.55, (0.3, 0, 2.15), "Z", m["dark"], hull, seg=10)
    A.rotor("main", (0.3, 0, 2.5), 4.17, 6, 0.2, m, hull, hub=0.24, mast=0.0, droop=0.02, phase=0.25)
    A.rotor("tail", (-4.7, 0.16, 1.55), 0.7, 4, 0.1, m, hull, hub=0.08, mast=0.0, droop=0.0,
            rot=(-math.pi / 2, 0, 0), thick=0.12)
    A.skids(1.6, -1.3, 0.95, (1.0, -0.6), 0.5, m, hull, radius=0.035)
    cyl("exhaust", 0.12, 0.3, (-1.2, 0, 1.55), "X", m["nozzle"], hull, seg=10, lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        box(f"door_frame_{k}", (0.9, 0.03, 0.9), (1.2, side * 0.71, 1.15), m["dark"], hull, lods=MID)
        if variant["id"] == AH6:
            # The weapon plank with a rocket pod and a minigun.
            box(f"plank_{k}", (0.4, 0.95, 0.08), (0.4, side * 1.12, 0.78), m["dark"], hull, bevel=0.01)
            A.store(f"rocket_pod_{k}", "pod", (0.6, side * 1.45, 0.58), 1.4, 0.19, m, hull)
            cyl(f"minigun_{k}", 0.08, 0.9, (0.6, side * 1.15, 0.52), "X", m["steel"], hull, seg=8, lods=MID)
        else:
            # The assault bench along the cabin's side.
            box(f"bench_{k}", (1.4, 0.55, 0.06), (0.6, side * 1.2, 0.68), m["dark"], hull, bevel=0.01)
            box(f"bench_strut_{k}", (0.06, 0.5, 0.06), (1.1, side * 1.05, 0.6), m["dark"], hull, lods=NEAR)

    # The 160th's near-bare black: U.S. ARMY on the boom.
    A.markings(v, [
        ("text", dict(text="U.S. ARMY", height=0.12, centre=(-2.6, 1.2, 1.32), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="black")),
    ])

def wreck(variant, v):
    A.crash(v, tail_x=-1.6, tail_yaw=0.5, tail_drop=0.12, blades_broken=(("main", 2),), seed=6)


run_disabled("little_bird", CARDS, "us_army_aviation", build, wreck, skip=("dressing_", "blade_"))
