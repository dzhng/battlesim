"""UH-60M Black Hawk and Z-20, from assets/references/uh_60_black_hawk/ and
assets/references/z_20/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/hawk.py -- [--variant=<card>] [--wreck]

What the photos settle: one layout (the Z-20 is the Chinese medium
helicopter on the Black Hawk's pattern). A low, wide cabin with big sliding
doors, the glazed cockpit with its sloping windscreen, two engines side by
side on the roof either side of the transmission, the main rotor on its
mast; the tail cone rising to a swept tail pylon with the tail rotor canted
on its right side, the stabilator low on the tail; fixed main wheels on
struts under the cabin and a tail wheel. The UH-60M: four wide-chord
blades, door guns, Army green. The Z-20: five blades, a slimmer nose and
fairing, PLA grey.

Built to the catalog frame, from published figures (the rule for every
rotorcraft here: fuselage length nose to tail without blades, width over the
widest fixed part, height to the top of the main rotor head): UH-60M 15.26 x
4.38 x 3.76 m; Z-20 15.4 x 4.4 x 3.9 m.

The gun in the left gunner's window is the `door` mount's HMG rig (on the
UH-60M the right one stays stowed, the hull's; the Z-20 carries the one).
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe  # noqa: E402
from parts import box, cyl, flat_paint  # noqa: E402
from vehicle_export import rig, run  # noqa: E402
from vehicle_parts import MID, NEAR  # noqa: E402

UH60, Z20 = "us_uh_60_black_hawk_uh_60m", "eastern_z_20_utility_transport"
# The door gun's elevation on its pintle, degrees.
DOOR_PITCH_DEG = (-60.0, 20.0)
SCHEME = {UH60: "us_army_aviation", Z20: "chinese_air_grey"}


def blade_dampers(head, blades, phase, mats):
    # Lead-lag dampers run tangentially from the hub to each blade grip.
    for i in range(blades):
        angle = phase + i * math.tau / blades
        def point(radial, tangent, z):
            return (radial * math.cos(angle) - tangent * math.sin(angle),
                    radial * math.sin(angle) + tangent * math.cos(angle), z)
        inner, collar, outer = point(.26, .23, .025), point(.52, .20, .065), point(.73, .11, .07)
        A.tube_part(f"damper_body_{i}", lambda lod: [inner, collar], .046, mats["dark"], head, NEAR)
        A.tube_part(f"damper_rod_{i}", lambda lod: [collar, outer], .022, mats["steel"], head, NEAR)
        cyl(f"blade_hinge_{i}", .064, .045, point(.58, 0, .082), "Z", mats["steel"], head,
            seg=10, lods=NEAR)


def build(variant, v):
    hawk = variant["id"] == UH60
    mats = A.fit(v)
    if hawk:
        mats["canopy"] = flat_paint("hawk_glazing", (.006, .012, .016), rough=.10,
                                   grime=0, role="glass")
    else:
        mats["canopy"] = flat_paint("z20_glass", (.006, .011, .017), rough=.09, metal=.09,
                                   grime=0, role="glass")
        mats["radome"] = flat_paint("nose_finish", (.042, .046, .049), rough=.66, grime=.04, role="paint")
    fitted_airframe(v, "uh_60_black_hawk" if hawk else "z_20", mats)
    if not v.wreck:
        # Two inward-facing utility seats interrupt the view through the open doors.
        fabric = flat_paint("hawk_seat_canvas", (.090, .095, .078), rough=.94, grime=.06, role="paint")
        webbing = flat_paint("hawk_seat_webbing", (.032, .036, .028), rough=.88, grime=0, role="paint")
        for i, (x, floors) in enumerate(((.53, (.550, .513)), (1.19, (.488, .473)))):
            box(f"troop_cushion_{i}", (.51, .45, .11), (x, -.39, .90), fabric, v.hull, bevel=.045, lods=MID)
            box(f"troop_back_{i}", (.48, .07, .55), (x, -.59, 1.22), fabric, v.hull,
                bevel=.034, rot=(.08, 0, 0), lods=MID)
            for side in (-1, 1):
                A.line(f"troop_belt_{i}_{side}", (x+side*.16, -.55, 1.46), (x+side*.075, -.50, 1.00),
                       webbing, v.hull, width=.048, height=.013)
                A.line(f"troop_leg_{i}_{side}", (x+side*.18, -.47, floors[side > 0]), (x+side*.18, -.39, .846),
                       mats["steel"], v.hull, width=.028, height=.028, lods=MID)
    if hawk:
        # Hub centres measured on the fitted donor. The authored blades use
        # the full flight diameter and turn on the existing rotor nodes.
        head = A.rotor("main", (3.25, 0, 3.38), 8.18, 4, 0.53, mats, v.hull,
                hub=0.36, mast=0.53, droop=0.025, phase=0.4)
        A.markings(v, [("text", dict(text="318", height=.235, centre=(2.04, 1.25, 2.55),
                                     normal=(0, 1, 0), up=(0, 0, 1), onto=("fuselage",),
                                     colour="lowvis_light"))])
        A.rotor("tail", (-7.14, -0.39, 3.31), 1.68, 4, 0.25, mats, v.hull,
                hub=0.16, mast=0.12, droop=0, rot=(math.pi / 2 - 0.35, 0, 0))
    else:
        head = A.rotor("main", (3.28, 0, 3.48), 8.20, 5, .53, mats, v.hull,
                hub=.39, mast=.34, droop=.024, phase=.2)
        A.rotor("tail", (-7.21, -.39, 3.40), 1.68, 4, .25, mats, v.hull, hub=.16, mast=.12,
                droop=0, rot=(math.pi/2-.35, 0, 0))
        A.markings(v, [("insignia", dict(kind="cn_star", centre=(-2.20, .48, 1.42), normal=(0, 1, 0),
                                        up=(0, 0, 1), size=.165, onto=("fuselage",)))])
    blade_dampers(head, 4 if hawk else 5, .4 if hawk else .2, mats)
    A.door_guns(v, rig(v.frame, v.root), mats, DOOR_PITCH_DEG, inboard=.2)


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-3.0, tail_yaw=0.35, tail_drop=0.1, seed=60)


run("hawk", SCHEME, build, wreck,
    references=["assets/references/uh_60_black_hawk/references.json", "assets/references/z_20/references.json"])
