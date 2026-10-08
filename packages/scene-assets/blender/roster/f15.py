"""F-15E Strike Eagle and F-15EX Eagle II, from assets/references/f_15_eagle/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/f15.py -- [--variant=<card>] [--wreck]

What the photos settle: the slab-sided body between two big box intakes
raked forward at the top, the shoulder wing cropped square at the tips, twin
upright fins on the tail booms, stabilators below them, two engines' nozzles
between the booms with the stinger between them; the two-seat canopy (the
EX keeps the E's), conformal fuel tanks along the intakes' flanks below the
wing, tall gear legs. The Strike Eagle wears gunship grey, the EX two-tone
compass grey; the E carries bombs on its CFTs, the EX missiles.

Dimensions stated from the references (USAF and Boeing figures agree):
length 19.43 m, span 13.05 m, height 5.63 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

E, EX = "us_f_15_eagle_f_15e_strike_eagle", "us_f_15_eagle_f_15ex_eagle_ii"
CARDS = {E: (19.43, 13.05, 5.63), EX: (19.43, 13.05, 5.63)}
SCHEME = {E: "us_gunship_grey", EX: "us_compass_grey"}

WING_Z = 2.62


def spec(variant):
    s = dict(
        fuselage=[(9.68, 0.0, 2.3, 2.3), (9.0, 0.36, 2.0, 2.6, 2.28), (7.8, 0.6, 1.76, 2.9, 2.25),
                  (6.2, 0.72, 1.64, 3.02, 2.25, 2.4), (4.4, 0.8, 1.6, 3.0, 2.2, 2.6),
                  (2.6, 1.42, 1.46, 2.86, 2.1, 4.0), (-2.0, 1.52, 1.44, 2.8, 2.05, 4.5),
                  (-5.4, 1.42, 1.5, 2.66, 2.05, 4.0), (-8.0, 1.2, 1.6, 2.5, 2.02, 3.0), (-8.62, 1.16, 1.62, 2.42, 2.0)],
        bodies=[
            # The two intake trunks, from their raked mouths back into the body.
            ("fuselage_intake_L", [(5.62, 0.5, 1.42, 2.66, 2.0, 5.0), (4.0, 0.52, 1.42, 2.7, 2.0, 5.0),
                                   (2.4, 0.5, 1.44, 2.72, 2.0, 4.0)]),
            ("fuselage_intake_R", [(5.62, 0.5, 1.42, 2.66, 2.0, 5.0), (4.0, 0.52, 1.42, 2.7, 2.0, 5.0),
                                   (2.4, 0.5, 1.44, 2.72, 2.0, 4.0)]),
            # Conformal fuel tanks along the intakes' flanks below the wing.
            ("fuselage_cft_L", [(4.4, 0.0, 1.9, 1.9), (3.6, 0.32, 1.5, 2.4, 1.95, 3.0), (-2.6, 0.34, 1.5, 2.44, 1.97, 3.0),
                                (-3.6, 0.0, 2.0, 2.0)]),
            ("fuselage_cft_R", [(4.4, 0.0, 1.9, 1.9), (3.6, 0.32, 1.5, 2.4, 1.95, 3.0), (-2.6, 0.34, 1.5, 2.44, 1.97, 3.0),
                                (-3.6, 0.0, 2.0, 2.0)]),
            # The stinger between the nozzles.
            ("tail_stinger", [(-8.2, 0.3, 1.86, 2.3), (-9.4, 0.14, 1.96, 2.18)]),
        ],
        body_offsets={"fuselage_intake_L": 1.16, "fuselage_intake_R": -1.16, "fuselage_cft_L": 1.62,
                      "fuselage_cft_R": -1.62},
        canopy=dict(x_front=7.4, x_back=4.2, sill=2.9, top=3.5, half_width=0.5, bows=(5.75, 4.55), peak=0.42),
        wing=[(2.7, 1.4, WING_Z, 5.6, 0.32), (-2.35, 6.52, WING_Z - 0.08, 1.95, 0.08)],
        stab=[(-6.3, 1.3, 2.18, 2.95, 0.14), (-8.25, 4.3, 2.18, 1.35, 0.05)],
        fins=[dict(root_x=-5.45, root_z=2.62, height=3.01, root_chord=3.45, tip_chord=1.35, sweep_m=2.35, y=1.16),
              dict(root_x=-5.45, root_z=2.62, height=3.01, root_chord=3.45, tip_chord=1.35, sweep_m=2.35, y=-1.16)],
        intakes=[("rect", (5.66, 1.16, 2.04), 0.86, 1.16, 0.2, (0, 0.22, 0)),
                 ("rect", (5.66, -1.16, 2.04), 0.86, 1.16, 0.2, (0, 0.22, 0))],
        nozzles=[dict(loc=(-8.6, 0.64, 2.0), r_front=0.58, r_exit=0.52, length=0.82, petals=16),
                 dict(loc=(-8.6, -0.64, 2.0), r_front=0.58, r_exit=0.52, length=0.82, petals=16)],
        gear=[dict(name="nose", x=6.55, y=0.0, top=1.68, radius=0.33, width=0.19, door=(0.9, 0.5)),
              dict(name="main_L", x=-0.4, y=1.38, top=1.5, radius=0.52, width=0.26, door=(1.3, 0.6), rake=0.08),
              dict(name="main_R", x=-0.4, y=-1.38, top=1.5, radius=0.52, width=0.26, door=(1.3, 0.6), rake=0.08)],
        pylons=[((-0.3, 3.1, WING_Z - 0.1), 1.3, 0.36)],
        stores=[("tank", (-0.4, 3.1, 1.66), 5.0, 0.46), ("missile", (-0.3, 3.55, 1.95), 2.9, 0.08)],
    )
    if variant["id"] == E:
        # Bombs in tandem along the CFTs' undersides.
        s["stores"] += [("bomb", (1.4, 1.62, 1.22), 2.3, 0.2), ("bomb", (-1.4, 1.62, 1.22), 2.3, 0.2)]
    else:
        s["stores"] += [("missile", (1.4, 1.88, 1.42), 3.65, 0.09), ("missile", (-1.6, 1.88, 1.42), 3.65, 0.09)]
    return s


def build(variant, v):
    s = spec(variant)
    offsets = s.pop("body_offsets")
    bodies = s.pop("bodies")
    m = A.jet(v, s)
    hull = v.hull
    for name, stations in bodies:
        A.body(name, stations, m["paint"], hull, seg=18, loc=(0, offsets.get(name, 0.0), 0))
    # Splitter plates and the ramp hinge line inside each intake mouth.
    A.mirrored(lambda side, k: box(f"intake_splitter_{k}", (1.6, 0.04, 1.1), (5.0, side * 0.62, 2.1), m["paint"], hull,
                                   lods=MID))
    box("antiglare", (1.4, 0.6, 0.03), (8.3, 0, 2.66), m["dark"], hull, rot=(0, 0.2, 0), lods=MID)
    # The speed brake on the spine behind the canopy, closed; the gun's port in the right wing root.
    box("speed_brake", (2.4, 1.2, 0.04), (2.6, 0, 2.99), m["paint"], hull, bevel=0.02, lods=MID)
    box("gun_port", (0.3, 0.12, 0.1), (3.0, -1.55, 2.6), m["black"], hull, lods=NEAR)
    A.blade_antenna("antenna_spine", (0.4, 0, 2.86), 0.25, m, hull)
    A.blade_antenna("antenna_belly", (0.8, 0, 1.46), 0.2, m, hull, down=True)
    A.mirrored(lambda side, k: box(f"wing_tip_light_{k}", (0.3, 0.05, 0.08), (-2.5, side * 6.5, WING_Z - 0.06),
                                   m["lamp"], hull, lods=NEAR))


def wreck(variant, v):
    A.crash(v, tail_x=-5.6, wing_y=3.8, wing_side=1, tail_yaw=0.25, seed=15)


run_disabled("f15", CARDS, SCHEME, build, wreck)
