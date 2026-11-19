"""Ka-52M Alligator, from assets/references/ka_52_alligator/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/ka52.py -- [--wreck]

What the photos settle: two three-blade rotors stacked coaxially on one
mast (no tail rotor), the wide side-by-side cockpit under a big glazed
canopy, the sensor ball under the nose and the radar in its nose dome (the
M), the 30 mm cannon on the right flank, engines either side of the
gearbox, broad stub wings with three stations each and wing-tip pods, the
tail with a horizontal stabiliser carrying two end-plate fins and a
central fin; retractable tricycle gear. Dark grey.

Built to the catalog frame, from published figures (rotorcraft rule:
fuselage length without blades, width over the wing-tip pods, height to the
top of the upper rotor head): 13.53 x 7.3 x 4.93 m.

The cannon on the right flank barely traverses, so it is fixed in the hull
with the missiles: the `cannon` mount's muzzle is the barrel's tip, the
`missiles` mount's the right middle station's missile front. The airframe
turns to bring them to bear.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_export import run  # noqa: E402
from vehicle_parts import MID  # noqa: E402

SPEC = dict(
    fuselage=[(6.98, 0.0, 1.08, 1.08), (6.55, 0.55, 0.88, 1.72, 1.24, 2.0), (5.3, 0.9, 0.82, 2.1, 1.5, 3.0),
              (3.6, 1.0, 0.8, 2.55, 1.6, 3.2), (1.2, 1.0, 0.85, 2.6, 1.65, 3.4), (-1.0, 0.82, 1.0, 2.5, 1.75, 3.2),
              (-3.0, 0.4, 1.45, 2.3, 1.85, 2.6), (-5.6, 0.32, 1.6, 2.25, 1.9, 2.4), (-6.76, 0.3, 1.65, 2.35, 1.95)],
    # The Alligator's two-seat canopy is broad and shallow, with a distinct
    # windscreen bow and a second bow over the rear seat.  A high, continuous
    # bubble loses the side-by-side cockpit's characteristic stepped profile.
    canopy=dict(x_front=6.12, x_back=3.58, sill=1.58, top=2.54, half_width=1.01, peak=0.49,
                bows=(5.72, 5.16, 4.52), tail=0.42, nose=0.12),
    canopy_finish=dict(name="ka52_canopy_glass", tint=(0.012, 0.024, 0.030), rough=0.16, coverage=0.46),
    wing=[(1.4, 0.95, 1.9, 1.7, 0.22), (1.2, 3.5, 1.86, 1.4, 0.14)],
    stab=[(-5.6, 0.3, 2.05, 1.1, 0.12), (-5.7, 1.9, 2.05, 0.9, 0.08)],
    fins=[dict(root_x=-5.2, root_z=2.25, height=1.2, root_chord=1.5, tip_chord=1.0, sweep_m=0.6, thick=0.16),
          dict(root_x=-5.4, root_z=1.6, height=1.0, root_chord=1.0, tip_chord=0.8, sweep_m=0.3, y=1.9, rudder=False),
          dict(root_x=-5.4, root_z=1.6, height=1.0, root_chord=1.0, tip_chord=0.8, sweep_m=0.3, y=-1.9,
               rudder=False)],
    gear=[dict(name="nose", x=5.0, y=0.0, top=0.9, radius=0.28, width=0.16, wheels=2, spread=0.3),
          dict(name="main_L", x=-0.2, y=1.2, top=1.0, radius=0.38, width=0.22),
          dict(name="main_R", x=-0.2, y=-1.2, top=1.0, radius=0.38, width=0.22)],
    pylons=[((1.1, 1.6, 1.84), 0.8, 0.22), ((1.05, 2.4, 1.84), 0.8, 0.22), ((1.0, 3.1, 1.84), 0.8, 0.22)],
    stores=[("pod", (1.0, 1.6, 1.38), 1.7, 0.25), ("missile", (0.95, 2.4, 1.45), 2.0, 0.12),
            ("pod", (0.9, 3.1, 1.38), 1.7, 0.25), ("pod", (0.6, 3.56, 1.86), 1.8, 0.09)],
)
# How far forward the airframe's parts (placed in their drawing's frame)
# move to centre on the hull box.
AFT_M = -0.025


def service_panel_details(m, h):
    """Add shallow, side-seated maintenance breaks to the broad Ka-52 cheeks."""
    for side, label in ((1, "L"), (-1, "R")):
        y = side * 1.06
        for index, x in enumerate((2.02, 2.88)):
            A.line(f"cheek_panel_seam_{label}_v{index}", (x, y, 1.64), (x, y, 2.14), m["dark"], h,
                   width=0.028, height=0.018, lods=MID)
        for index, z in enumerate((1.64, 2.14)):
            A.line(f"cheek_panel_seam_{label}_h{index}", (2.02, y, z), (2.88, y, z), m["dark"], h,
                   width=0.028, height=0.018, lods=MID)
        for index, (x, z) in enumerate(((2.14, 1.76), (2.76, 1.76), (2.14, 2.02), (2.76, 2.02))):
            cyl(f"cheek_panel_fastener_{label}_{index}", 0.025, 0.008, (x, side * 1.045, z), "Y",
                m["panel"], h, seg=8, lods=MID)


def build(variant, v):
    m = A.jet(v, SPEC)
    hull = v.hull
    # Russian grey is a low-contrast finish, but maintenance panels and
    # sensor housings still have distinct satin and rubberized coatings.
    m["panel"] = flat_paint("ka52_panel_grey", (0.072, 0.081, 0.091), rough=0.58, grime=0.03, role="paint")
    m["olive_panel"] = flat_paint("ka52_olive_panel", (0.050, 0.061, 0.056), rough=0.70, grime=0.04, role="paint")
    m["sensor_dark"] = flat_paint("ka52_sensor_dark", (0.018, 0.024, 0.030), rough=0.34, grime=0, role="paint")
    A.body("nose_radar", [(7.02, 0.0, 1.02, 1.02), (6.78, 0.34, 0.86, 1.30, 1.08), (6.35, 0.46, 0.84, 1.62, 1.18)],
           m["dark"], hull, seg=18)
    A.sensor_ball("sensor", (5.48, 0, 0.66), 0.31, {**m, "dark": m["sensor_dark"]}, hull, window=0.62)
    # The M's nose radar is a separate dark dome, with a narrow fairing seam
    # around its base so it reads as a sensor rather than a blunt cap.
    A.body("radar_dome", [(7.04, 0.0, 1.00, 1.00), (6.82, 0.34, 0.84, 1.24, 1.06),
                           (6.56, 0.44, 0.80, 1.48, 1.12)], m["sensor_dark"], hull, seg=18)
    # The 30 mm cannon on the right flank.
    box("gun_mount", (1.6, 0.3, 0.35), (2.0, -1.05, 1.15), m["paint"], hull, bevel=0.06)
    # A short receiver fairing seats the fixed cannon in the cheek before the
    # barrel emerges.  This keeps the weapon from reading as a floating tube.
    box("gun_receiver", (0.55, 0.22, 0.24), (2.82, -1.05, 1.15), m["dark"], hull, bevel=0.04, lods=MID)
    cyl("gun_barrel", 0.06, 1.35, (3.62, -1.05, 1.15), "X", m["steel"], hull, seg=10, lods=MID)
    for side, k in ((1, "L"), (-1, "R")):
        A.body(f"nacelle_{k}", [(1.9, 0.0, 2.55, 2.55), (1.6, 0.36, 2.2, 2.9, 2.55), (-0.8, 0.36, 2.2, 2.9, 2.55),
                                (-1.3, 0.26, 2.3, 2.8, 2.55)], m["paint"], hull, seg=14, loc=(0, side * 0.82, 0))
        # Raised intake lips and dark ducts separate the two engines from the
        # fuselage in the front three-quarter view.
        A.round_intake(f"intake_{k}", (1.78, side * 1.10, 2.67), 0.34, 0.16, m, hull, lods=MID)
        A.cyl(f"exhaust_{k}_lip", 0.29, 0.14, (-1.54, side * 1.0, 2.65), "X", m["nozzle"], hull,
               seg=18, rot=(0, -0.3, side * 0.4), lods=MID)
        A.cyl(f"exhaust_{k}_duct", 0.22, 0.04, (-1.62, side * 1.0, 2.65), "X", m["black"], hull,
               seg=16, rot=(0, -0.3, side * 0.4), lods=MID)
    # Shallow side panels and cooling vents break the uninterrupted grey shell
    # while staying seated against the fuselage's broad cheeks.
    for side in (-1, 1):
        box(f"access_panel_{side}", (0.86, 0.035, 0.54), (2.45, side * 1.025, 1.88), m["panel"], hull,
            bevel=0.025, lods=MID)
        for k in range(3):
            box(f"cooling_vent_{side}_{k}", (0.34, 0.025, 0.045), (-0.52 - k * 0.16, side * 1.205, 2.50),
                m["black"], hull, bevel=0.01, lods=MID)
    service_panel_details(m, hull)
    cyl("mast", 0.2, 2.2, (0.3, 0, 3.75), "Z", m["dark"], hull, seg=12)
    A.rotor("lower", (0.3, 0, 3.5), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=0.0)
    A.rotor("upper", (0.3, 0, 4.7), 7.25, 3, 0.5, m, hull, hub=0.42, mast=0.0, droop=0.02, phase=1.047)

    # The VVS's red star on the boom, a red bort number on the nose.
    A.markings(v, [
        ("insignia", dict(kind="ru_star", centre=(-4.5, 1.2, 1.92), normal=(0, 1, 0), up=(0, 0, 1), size=0.28, onto=("fuselage",))),
        ("text", dict(text="12", height=0.42, centre=(4.0, 1.2, 1.55), normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",), colour="red")),
    ])
    # The airframe as drawn runs off the frame's middle; the frame (and the
    # mounts' pivots in it) is centred on the hull box.
    v.hull.location.x += AFT_M


def wreck(variant, v):
    A.rotorcraft_crash(v, lifting=("upper", "lower"), tail_x=-2.8, tail_yaw=0.35, tail_drop=0.1, seed=52)


run("ka52", "russian_air_grey", build, wreck, references="assets/references/ka_52_alligator/references.json")
