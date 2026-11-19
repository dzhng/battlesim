"""Transport airframe adapted for the Mi-8AMTSh with service camouflage, a formed nose radome, an open cabin, external stations and animated rotors."""

import bpy
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import aircraft_parts as A  # noqa: E402
from aircraft_donor import fitted_airframe
from parts import box, flat_paint, rest_on_ground
from vehicle_export import run, rig


def cabin_interior(m, h):
    """Give the open troop door a readable floor and fold-down bench."""
    rest = flat_paint("hip_cabin_canvas", (0.28, 0.26, 0.20), rough=.78, grime=.04, role="fabric")
    box("hip_cabin_floor", (2.35, 2.05, .08), (4.05, 0, 1.00), m["dark"], h, bevel=.025, lods=A.MID)
    box("hip_cabin_bench", (1.08, .56, .22), (4.80, .92, 1.38), rest, h, bevel=.035, lods=A.MID)
    box("hip_cabin_bench_back", (.12, .56, .56), (5.25, .92, 1.68), rest, h, bevel=.035, lods=A.MID)
    for index, x in enumerate((4.18, 4.78)):
        A.line(f"hip_bench_leg_{index}", (x + .32, .92, 1.04), (x + .32, .92, 1.32), m["gear"], h,
               width=.065, height=.065, lods=A.MID)
        A.line(f"hip_bench_wall_brace_{index}", (x, .92, 1.38), (x, 1.04, 1.38), m["steel"], h,
               width=.055, height=.055, lods=A.MID)
        box(f"hip_bench_wall_mount_{index}", (.16, .11, .22), (x, 1.08, 1.38), rest, h,
            bevel=.02, lods=A.MID)


def build(variant, v):
    m = A.fit(v)
    hull = v.hull
    m["canopy"] = flat_paint(
        "hip_laminated_glass",
        (0.011, 0.025, 0.030),
        rough=0.14,
        grime=0,
        coverage=("blended", 0.36),
        role="glass",
    )
    m["radome"] = flat_paint("hip_radome", (0.064, 0.076, 0.052), rough=0.68, grime=0.04, role="paint")
    fitted_airframe(v, "mi_8_hip", m)
    if not v.wreck:
        cabin_interior(m, hull)
    A.rotor("main", (3.51, 0.012, 4.51), 10.65, 5, 0.52, m, hull, hub=0.48, mast=0.72, droop=0.03, phase=0.2)
    A.rotor(
        "tail",
        (-8.88, 0.64, 3.99),
        1.95,
        3,
        0.30,
        m,
        hull,
        hub=0.18,
        mast=0.36,
        droop=0,
        rot=(-math.pi / 2, 0, 0),
        thick=0.12,
    )
    for side in (-1, 1):
        A.line(
            "main_gear_brace_" + str(side),
            (2.56, side * 2.31, 0.48),
            (3.07, side * 1.31, 1.72),
            m["gear"],
            hull,
            width=0.072,
            height=0.072,
            lods=A.MID,
        )
        A.line(
            "store_rack_" + str(side),
            (0.60, side * 1.22, 1.62),
            (0.60, side * 2.49, 1.41),
            m["dark"],
            hull,
            width=0.10,
            height=0.10,
            lods=A.MID,
        )
        for i, y in enumerate((1.82, 2.45)):
            A.pylon("station_" + str(side) + "_" + str(i), (0.60, side * y, 1.36), 0.69, 0.20, m, hull)
            A.store("pod_" + str(side) + "_" + str(i), "pod", (0.60, side * y, 1.09), 1.54, 0.24, m, hull)
    A.door_guns(v, rig(v.frame, v.root), m, (-60, 20), inboard=0.15, gun="kord")
    A.markings(
        v,
        [
            (
                "insignia",
                dict(
                    kind="ru_star",
                    centre=(-3.42, 0.7, 2.54),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    size=0.32,
                    onto=("fuselage_Material_2460",),
                ),
            ),
            (
                "text",
                dict(
                    text="52",
                    height=0.26,
                    centre=(7.26, 1.45, 1.54),
                    normal=(0, 1, 0),
                    up=(0, 0, 1),
                    onto=("fuselage",),
                    colour="red",
                ),
            ),
        ],
    )


def wreck(variant, v):
    A.rotorcraft_crash(v, tail_x=-3.4, tail_yaw=0.35, tail_drop=0.08, seed=8)
    # The pinned transport includes the wheels in its shell. Settle the
    # collapsed undercarriage onto its nearest contact after the tail break.
    bpy.context.view_layer.update()
    lowest = min((part.matrix_world @ vertex.co).z for part in v.hull.children_recursive
                 if part.type == "MESH" and part.name.endswith("LOD0") for vertex in part.data.vertices)
    v.root.location.z -= lowest - .004
    rest_on_ground(.004)


run("hip", "russian_helicopter_camo", build, wreck, references="assets/references/mi_8_hip/references.json")
