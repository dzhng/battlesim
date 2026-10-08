"""F/A-18E Super Hornet and EA-18G Growler, from assets/references/f_a_18_super_hornet/
and assets/references/ea_18g_growler/. Disabled cards.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/super_hornet.py -- [--variant=<card>] [--wreck]

What the photos settle: one airframe. Long leading-edge extensions from the
cockpit to the wing root, the caret intakes tucked under them, a shoulder
wing with dogtooth, twin fins canted out, stabilators below them, two
nozzles close together; tall twin nose wheels and main gear on the body's
flanks. The E is a single seat with wing-tip AIM-9s, tanks and missiles;
the Growler has the two-seat canopy, ALQ-218 receiver pods on the wing tips
and ALQ-99 jamming pods (with their ram-air turbines) under the wings and
the body. Both in the Navy's tactical greys.

Dimensions stated from the references (Navy fact file figures agree): length
18.31 m, span 13.62 m over the tip stores, height 4.88 m.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

E, G = "us_f_a_18_super_hornet_f_a_18e", "us_ea_18g_growler_electronic_warfare_aircraft"
CARDS = {E: (18.31, 13.62, 4.88), G: (18.31, 13.62, 4.88)}

WING_Z = 2.45
TIP_Y = 6.72


def spec(variant):
    growler = variant["id"] == G
    s = dict(
        fuselage=[(9.28, 0.0, 2.2, 2.2), (8.5, 0.38, 1.92, 2.5, 2.18), (7.2, 0.56, 1.7, 2.76, 2.15),
                  (5.8, 0.64, 1.6, 2.86, 2.1, 2.4), (4.0, 0.74, 1.52, 2.78, 2.05, 2.6),
                  (2.0, 1.08, 1.36, 2.66, 2.0, 3.0), (-1.0, 1.16, 1.36, 2.62, 2.0, 3.5),
                  (-4.0, 1.06, 1.44, 2.52, 1.98, 3.2), (-6.6, 0.96, 1.54, 2.42, 1.96, 2.8),
                  (-7.7, 0.92, 1.58, 2.36, 1.96)],
        canopy=dict(x_front=7.55, x_back=4.0 if growler else 5.0, sill=2.78, top=3.32, half_width=0.46,
                    bows=(5.35,) if growler else (), peak=0.4),
        strake=[(6.4, 0.55, 2.52, 5.0, 0.1), (1.7, 1.45, WING_Z + 0.02, 0.9, 0.08)],
        wing=[(1.75, 1.1, WING_Z, 4.7, 0.28), (-1.45, TIP_Y - 0.08, WING_Z - 0.02, 1.6, 0.06)],
        stab=[(-5.65, 0.9, 2.02, 2.7, 0.12), (-7.65, 3.45, 1.96, 1.25, 0.05)],
        fins=[dict(root_x=-3.55, root_z=2.5, height=2.5, root_chord=3.2, tip_chord=1.3, sweep_m=2.15, y=0.78,
                   cant=0.35),
              dict(root_x=-3.55, root_z=2.5, height=2.5, root_chord=3.2, tip_chord=1.3, sweep_m=2.15, y=-0.78,
                   cant=0.35)],
        intakes=[("rect", (4.05, 0.98, 1.66), 0.52, 0.74, 0.2, (0, 0.15, 0)),
                 ("rect", (4.05, -0.98, 1.66), 0.52, 0.74, 0.2, (0, 0.15, 0))],
        nozzles=[dict(loc=(-7.65, 0.5, 1.96), r_front=0.5, r_exit=0.46, length=0.98, petals=14),
                 dict(loc=(-7.65, -0.5, 1.96), r_front=0.5, r_exit=0.46, length=0.98, petals=14)],
        gear=[dict(name="nose", x=6.4, y=0.0, top=1.6, radius=0.33, width=0.17, wheels=2, spread=0.26,
                   door=(0.9, 0.5)),
              dict(name="main_L", x=-0.6, y=1.56, top=1.4, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1),
              dict(name="main_R", x=-0.6, y=-1.56, top=1.4, radius=0.5, width=0.25, door=(1.2, 0.55), rake=0.1)],
        pylons=[((-0.4, 2.6, WING_Z - 0.1), 1.2, 0.35), ((-1.0, 4.0, WING_Z - 0.08), 1.1, 0.3)],
        stores=[("tank", (-0.5, 2.6, 1.6), 4.7, 0.42)],
    )
    if growler:
        s["stores"] += [("pod", (-0.9, 4.0, 1.66), 4.6, 0.36), ("pod", (-1.0, 0.0, 0.98), 4.6, 0.36),
                        ("pod", (-1.6, TIP_Y, WING_Z), 2.6, 0.17)]
    else:
        s["stores"] += [("missile", (-1.0, 4.0, 1.9), 3.65, 0.09), ("missile", (-1.6, TIP_Y, WING_Z), 3.0, 0.065)]
    return s


def build(variant, v):
    m = A.jet(v, spec(variant))
    hull = v.hull
    if variant["id"] == G:
        # The jamming pods' ram-air turbines on their noses.
        for k, (x, y, z) in enumerate(((1.42, 4.0, 1.66), (1.42, -4.0, 1.66), (1.32, 0.0, 0.98))):
            cyl(f"rat_{k}", 0.16, 0.12, (x, y, z), "X", m["dark"], hull, seg=12, lods=MID)
            for b in range(2):
                box(f"rat_{k}_blade_{b}", (0.03, 0.05, 0.5), (x + 0.05, y, z), m["dark"], hull,
                    rot=(b * 1.5708, 0, 0), lods=NEAR)
    else:
        box("gun_port", (0.25, 0.1, 0.12), (8.4, 0, 2.48), m["black"], hull, lods=NEAR)
        A.mirrored(lambda side, k: box(f"wing_tip_rail_{k}", (2.2, 0.08, 0.1), (-1.5, side * (TIP_Y - 0.02),
                                                                                 WING_Z - 0.02), m["dark"], hull))
    # The dogtooth in each wing's leading edge, the refuelling probe's door, antiglare.
    A.mirrored(lambda side, k: box(f"wing_dogtooth_{k}", (0.3, 0.08, 0.08), (0.4, side * 3.6, WING_Z), m["paint"], hull,
                                   lods=MID))
    box("probe_door", (0.8, 0.12, 0.03), (6.6, -0.42, 2.7), m["dark"], hull, rot=(0.5, 0, 0), lods=NEAR)
    box("antiglare", (1.0, 0.5, 0.03), (8.1, 0, 2.5), m["dark"], hull, rot=(0, 0.22, 0), lods=MID)
    A.blade_antenna("antenna_spine", (1.0, 0, 2.66), 0.24, m, hull)
    A.blade_antenna("antenna_belly", (0.4, 0, 1.38), 0.2, m, hull, down=True)
    # The arrestor hook under the tail.
    box("arrestor_hook", (1.6, 0.08, 0.08), (-6.8, 0, 1.5), m["steel"], hull, rot=(0, 0.12, 0), lods=MID)


def wreck(variant, v):
    A.crash(v, tail_x=-5.0, wing_y=3.6, wing_side=-1, tail_yaw=-0.3, seed=18)


run_disabled("super_hornet", CARDS, "us_compass_grey", build, wreck)
