"""Drones (disabled cards): the Orlan-10, the five scout drones and the six
kamikaze drones, from assets/references/orlan_10/, scout_drone/ and
kamikaze_drone/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/drones.py -- [--variant=<card id>] [--wreck]

The scout and kamikaze cards name a class, not a type; each is drawn as
the type its army fields, from the references:
- scout drones: a folding-arm quadcopter with a gimbal under its nose, the
  Skydio X2D (US), Parrot ANAFI USA (Europe) and DJI Mavic 3T class
  (Eastern); a thermal card's gimbal is the larger dual-sensor head;
- anti-armour kamikaze drones: a tube-bodied loitering munition with
  folding wings and a pusher propeller, the Switchblade 600 (US), the
  Hero-120 (Europe) and the Lancet with its two X wings (Eastern);
- anti-personnel kamikaze drones: the Switchblade 300 (US) and, for Europe
  and the East, an FPV quadcopter with an RPG warhead slung under it;
- Orlan-10: the high straight wing, a tractor propeller on the nose, the
  inverted-V tail, the gimbal under the belly.

Each rests on the ground (on its legs or rails, or its belly and wing tips),
so the tier-0 contact sits at z = 0 as every appearance's does; nothing it
rests on rolls, so none of it needs a node. Each frame is the type's published length, span and height,
recorded as `references`. Rotors and propellers are drawn still: nothing
here spins until a flight mechanic says how.
"""
import math
import os
import sys

import bpy
from mathutils import Vector

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, loft, textured  # noqa: E402
from vehicle_export import run_disabled  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID

# card: (family folder, scheme, kind, frame dims (length, span, height))
CARDS = {
    "eastern_orlan_10_reconnaissance_drone": ("orlan_10", "russian_green", "orlan", (1.80, 3.10, 0.55)),
    "us_scout_drone_standard": ("scout_drone", "us_desert_tan", "skydio", (0.66, 0.57, 0.14)),
    "us_scout_drone_thermal": ("scout_drone", "us_desert_tan", "skydio_thermal", (0.66, 0.57, 0.15)),
    "europe_scout_drone_standard": ("scout_drone", "german_three_tone", "anafi", (0.28, 0.38, 0.10)),
    "europe_scout_drone_thermal": ("scout_drone", "german_three_tone", "anafi_thermal", (0.28, 0.38, 0.12)),
    "eastern_scout_drone_thermal": ("scout_drone", "russian_green", "mavic_thermal", (0.35, 0.29, 0.12)),
    "us_kamikaze_drone_anti_armor": ("kamikaze_drone", "us_desert_tan", "switchblade_600", (1.30, 1.40, 0.22)),
    "us_kamikaze_drone_anti_personnel": ("kamikaze_drone", "us_desert_tan", "switchblade_300", (0.50, 0.70, 0.10)),
    "europe_kamikaze_drone_anti_armor": ("kamikaze_drone", "german_three_tone", "hero_120", (1.30, 1.10, 0.30)),
    "europe_kamikaze_drone_anti_personnel": ("kamikaze_drone", "german_three_tone", "fpv", (0.45, 0.30, 0.17)),
    "eastern_kamikaze_drone_anti_armor": ("kamikaze_drone", "russian_green", "lancet", (1.65, 0.72, 0.72)),
    "eastern_kamikaze_drone_anti_personnel": ("kamikaze_drone", "russian_green", "fpv", (0.45, 0.30, 0.17)),
}


def shell_mats(v):
    """A commercial or composite airframe's grey and black: role paint, so
    only paint carries the side's tint (here none: these are bought, not
    painted)."""
    return dict(v.mats,
                grey=textured("airframe_grey", "olive_paint", colour=(0.055, 0.057, 0.058), chip=0.1, dirt=0.2,
                              streak=0.0, role="paint"),
                carbon=textured("carbon", "olive_paint", colour=(0.012, 0.012, 0.013), chip=0.05, dirt=0.15,
                                streak=0.0, role="paint"))


# ---------------------------------------------------------------- parts
def propeller(name, loc, radius, axis, mat, parent, blades=2, chord=None):
    """A still propeller about `axis` ('X' for a tractor or pusher, 'Z' for a
    rotor): its hub, a tractor's or pusher's spinner cap and `blades` blades, each wide at the root
    and tapering to its tip (two pieces, the tip's pitch flatter)."""
    chord = chord or radius * 0.18
    cyl(f"{name}_hub", radius * 0.14, radius * 0.25, loc, axis, mat, parent, seg=10, lods=MID)
    if axis == "X":
        cyl(f"{name}_spinner", radius * 0.12, radius * 0.10, (loc[0] + radius * 0.14, loc[1], loc[2]), axis, mat,
            parent, seg=10, r2=radius * 0.03, lods=NEAR)
    for k in range(blades):
        a = k * math.tau / blades + (math.pi / 4 if axis == "Z" else 0.4)
        for j, (start, end, width, pitch) in enumerate(((0.10, 0.62, 1.0, 0.22), (0.62, 1.0, 0.62, 0.10))):
            at, span = radius * (start + end) / 2, radius * (end - start)
            if axis == "Z":
                box(f"{name}_blade_{k}_{j}", (span, chord * width, 0.006),
                    (loc[0] + math.cos(a) * at, loc[1] + math.sin(a) * at, loc[2]), mat, parent,
                    rot=(pitch, 0, a), lods=NEAR if j == 0 else FINE)
            else:
                box(f"{name}_blade_{k}_{j}", (0.006, span, chord * width),
                    (loc[0], loc[1] + math.cos(a) * at, loc[2] + math.sin(a) * at), mat, parent,
                    rot=(a, pitch, 0), lods=NEAR if j == 0 else FINE)
        if axis != "Z":
            # A tractor or pusher's blades read whole at mid range too.
            box(f"{name}_disc_{k}", (0.006, radius * 0.9, chord * 0.8),
                (loc[0], loc[1] + math.cos(a) * radius * 0.5, loc[2] + math.sin(a) * radius * 0.5), mat, parent,
                rot=(a, 0, 0), lods=(2,))


def gimbal(name, loc, size, m, parent, thermal=False):
    """A camera gimbal: its yoke and ball, the lens dark glass; a thermal
    head is wider and carries a second, larger window."""
    s = size * (1.35 if thermal else 1.0)
    box(f"{name}_yoke", (s * 0.6, s * 1.1, s * 0.3), (loc[0], loc[1], loc[2] + s * 0.45), m["carbon"], parent,
        lods=MID)
    cyl(f"{name}_ball", s * 0.5, s * 0.9, loc, "Y", m["grey"], parent, seg=16, bevel=s * 0.1)
    cyl(f"{name}_lens", s * 0.22, 0.004, (loc[0] + s * 0.5, loc[1] - (s * 0.18 if thermal else 0), loc[2]), "X",
        m["glass"], parent, seg=12, lods=MID)
    if thermal:
        box(f"{name}_thermal_window", (0.004, s * 0.32, s * 0.32), (loc[0] + s * 0.5, loc[1] + s * 0.20, loc[2]),
            m["glass"], parent, lods=MID)


def quad(v, kind):
    """A folding-arm quadcopter on its landing legs: body, four arms and
    motors with their two-blade rotors, the gimbal under the nose; an FPV
    has an open carbon frame and the RPG warhead slung under it."""
    m, h = v.mats, v.hull
    L, W, H = v.length, v.width, v.height
    fpv = kind == "fpv"
    body_mat = m["carbon"] if fpv or kind.startswith("skydio") else m["grey"]
    leg = H * 0.42
    z = leg + H * 0.18
    if fpv:
        box("frame_plate", (L * 0.40, W * 0.30, 0.012), (0, 0, z), m["carbon"], h, bevel=0.003)
        box("stack", (L * 0.22, W * 0.20, 0.035), (0, 0, z + 0.025), m["dark"], h, bevel=0.004)
        box("battery", (L * 0.26, W * 0.16, 0.035), (-0.01, 0, z + 0.06), m["grey"], h, bevel=0.006)
        box("camera", (0.03, 0.03, 0.03), (L * 0.20, 0, z + 0.025), m["carbon"], h, lods=MID)
        cyl("camera_lens", 0.009, 0.004, (L * 0.22, 0, z + 0.025), "X", m["glass"], h, seg=10, lods=NEAR)
        # The PG-7 warhead, nose forward, strapped under the frame.
        cyl("warhead_body", 0.042, L * 0.55, (0.02, 0, z - 0.05), "X", m["paint"], h, seg=14)
        cyl("warhead_cone", 0.042, 0.10, (0.02 + L * 0.275 + 0.05, 0, z - 0.05), "X", m["paint"], h, seg=14, r2=0.012)
        cyl("warhead_tail", 0.020, 0.07, (0.02 - L * 0.275 - 0.035, 0, z - 0.05), "X", m["dark"], h, seg=10, lods=MID)
        cyl("warhead_band", 0.044, 0.012, (0.02 + L * 0.20, 0, z - 0.05), "X", m["dark"], h, seg=14, lods=NEAR)
        for k, x in enumerate((-0.05, 0.08)):
            box(f"warhead_strap_{k}", (0.012, 0.09, 0.10), (x, 0, z - 0.03), m["carbon"], h, lods=NEAR)
        leg = z - 0.05 - 0.042
    else:
        loft("body", [(z - H * 0.12, _oval(L * 0.30, W * 0.16)), (z + H * 0.14, _oval(L * 0.26, W * 0.13))],
             mat=body_mat, parent=h, bevel=0.006)
        gimbal("gimbal", (L * 0.28, 0, z - H * 0.12), min(L, W) * 0.12, m, h, thermal=kind.endswith("thermal"))
        # The battery's seam on the back, its latch, the obstacle cameras
        # round the body, the status light and the GPS puck on top.
        box("battery_seam", (L * 0.30, W * 0.22, 0.003), (-L * 0.10, 0, z + H * 0.14 + 0.001), m["dark"], h,
            lods=NEAR)
        box("battery_latch", (0.012, W * 0.10, 0.006), (-L * 0.25, 0, z + H * 0.12), m["dark"], h, lods=FINE)
        cyl("gps_puck", W * 0.05, 0.006, (-L * 0.02, 0, z + H * 0.14 + 0.003), "Z", m["dark"], h, seg=12, lods=NEAR)
        for k, (x, y, axis) in enumerate(((L * 0.29, W * 0.06, "X"), (L * 0.29, -W * 0.06, "X"),
                                          (-L * 0.29, 0, "X"), (0, W * 0.155, "Y"), (0, -W * 0.155, "Y"))):
            cyl(f"nav_camera_{k}", 0.006, 0.003, (x, y, z + H * 0.05), axis, m["glass"], h, seg=8, lods=FINE)
        cyl("status_light", 0.005, 0.004, (-L * 0.30, 0, z), "X", m["tail"], h, seg=8, lods=FINE)
        for side, s in ((1, "L"), (-1, "R")):
            for k, x in enumerate((-L * 0.16, L * 0.12)):
                box(f"leg_{s}_{k}", (0.008, 0.008, leg), (x, side * W * 0.08, leg / 2), m["carbon"], h, lods=MID)
            box(f"skid_{s}", (L * 0.34, 0.010, 0.006), (-L * 0.02, side * W * 0.08, 0.003), m["carbon"], h, lods=MID)
    # Rotors at the corners, their tips reaching the stated length and span.
    rotor = min(L, W) * 0.27
    # An FPV's props are three-bladed, so one blade always reaches further
    # along the frame than a two-blade prop set at 45 degrees: shorter blades
    # keep the tips where the two-blade ones were.
    reach = rotor * (0.81 if fpv else 1.0)
    for k, (sx, sy) in enumerate(((1, 1), (-1, 1), (-1, -1), (1, -1))):
        ex, ey = sx * (L / 2 - rotor * 0.71), sy * (W / 2 - rotor * 0.71)
        mid = (ex / 2, ey / 2)
        box(f"arm_{k}", (math.hypot(ex, ey), 0.018 if fpv else 0.024, 0.014), (mid[0], mid[1], z), body_mat, h,
            rot=(0, 0, math.atan2(ey, ex)), bevel=0.003)
        cyl(f"motor_{k}", 0.016 if fpv else 0.018, 0.024, (ex, ey, z + 0.016), "Z", m["dark"], h, seg=12)
        cyl(f"motor_bell_{k}", 0.012 if fpv else 0.014, 0.006, (ex, ey, z + 0.030), "Z", m["steel"], h, seg=12,
            lods=NEAR)
        if not fpv:
            # The fold hinge at the arm's root and the arm-tip light (green
            # starboard, red port, as the photos' lit arms show).
            box(f"arm_hinge_{k}", (0.020, 0.020, 0.018), (ex * 0.30, ey * 0.30, z), m["dark"], h, lods=NEAR)
            cyl(f"arm_light_{k}", 0.005, 0.004, (ex, ey, z - 0.010), "Z", m["tail"] if sy > 0 else m["lamp"], h,
                seg=8, lods=FINE)
        propeller(f"rotor_{k}", (ex, ey, z + 0.032), reach, "Z", m["carbon"], h, blades=3 if fpv else 2)
    if fpv:
        # The video antenna rising off the frame's rear, the receiver's two
        # whiskers, and the zip ties holding the warhead.
        cyl("vtx_antenna", 0.004, 0.05, (-L * 0.16, 0, z + 0.035), "Z", m["dark"], h, seg=6, lods=NEAR)
        cyl("vtx_cap", 0.010, 0.012, (-L * 0.16, 0, z + 0.065), "Z", m["dark"], h, seg=10, lods=NEAR)
        for side in (-1, 1):
            box(f"rx_whisker_{side}", (0.05, 0.002, 0.002), (-L * 0.20, side * 0.02, z + 0.01), m["dark"], h,
                rot=(0, 0, side * 0.6), lods=FINE)


def _oval(rx, ry, n=12):
    return [(rx * math.cos(k * math.tau / n), ry * math.sin(k * math.tau / n)) for k in range(n)]


def loitering(v, kind):
    """A tube-bodied loitering munition resting on its belly and wing tips:
    nose seeker under dark glass, the warhead section, folding wings (one
    pair, two in tandem, or the Lancet's two X sets), a pusher propeller."""
    m, h = v.mats, v.hull
    L, W, H = v.length, v.width, v.height
    lancet = kind == "lancet"
    r = H * (0.22 if lancet else 0.40)
    z = r + (H * 0.30 if lancet else 0.0)
    body = m["paint"] if kind in ("switchblade_600", "lancet") else m["grey"]
    cyl("body", r, L * 0.80, (0, 0, z), "X", body, h, seg=16, bevel=r * 0.15)
    cyl("nose", r, L * 0.10, (L * 0.45, 0, z), "X", body, h, seg=16, r2=r * 0.45)
    cyl("seeker", r * 0.42, 0.006, (L * 0.50, 0, z), "X", m["glass"], h, seg=12, lods=MID)
    cyl("tail_cone", r, L * 0.06, (-L * 0.43, 0, z), "X", body, h, seg=16, r2=r * 0.55)
    propeller("pusher", (-L * 0.465, 0, z), r * 2.4, "X", m["carbon"], h)
    for k in range(3):
        cyl(f"body_band_{k}", r * 1.03, 0.012, (L * (0.25 - k * 0.25), 0, z), "X", m["dark"], h, seg=16, lods=NEAR)
    if lancet:
        for j, x in enumerate((L * 0.12, -L * 0.30)):
            for k in range(4):
                a = math.pi / 4 + k * math.pi / 2
                # The X's 1.0 m span runs tip to tip across the diagonal.
                span = (W / 1.414 - r) * (0.9 if j == 0 else 1.0)
                box(f"x_wing_{j}_{k}", (L * 0.12, span, 0.008),
                    (x, math.cos(a) * (r + span / 2), z + math.sin(a) * (r + span / 2)), m["grey"], h,
                    rot=(a, 0, 0), bevel=0.002)
    else:
        tandem = kind in ("switchblade_600", "switchblade_300")
        for j, x in enumerate((L * 0.05, -L * 0.28) if tandem else (0.0,)):
            for side in (-1, 1):
                span = W / 2 * (0.85 if j == 0 and tandem else 1.0)
                box(f"wing_{j}_{side}", (L * 0.10, span - r, 0.008), (x, side * (r + (span - r) / 2), z + r * 0.5),
                    m["grey"], h, rot=(side * -0.06, 0, 0), bevel=0.002)
                # The aileron along the outer trailing edge.
                box(f"aileron_{j}_{side}", (L * 0.025, (span - r) * 0.45, 0.006),
                    (x - L * 0.06, side * (r + (span - r) * 0.70), z + r * 0.5), m["dark"], h,
                    rot=(side * -0.06, 0, 0), lods=NEAR)
        if kind == "hero_120":
            for k in range(4):
                a = math.pi / 4 + k * math.pi / 2
                box(f"tail_fin_{k}", (L * 0.08, r * 1.0, 0.006), (-L * 0.38, math.cos(a) * r * 1.3,
                                                                 z + math.sin(a) * r * 1.3), m["grey"], h,
                    rot=(a, 0, 0), lods=MID)
        else:
            box("tail_fin", (L * 0.08, 0.006, r * 0.9), (-L * 0.38, 0, z + r * 0.9), m["grey"], h, lods=MID)
    if lancet:
        for side, s in ((1, "L"), (-1, "R")):
            box(f"skid_rail_{s}", (L * 0.30, 0.01, z - r), (0, side * r * 0.6, (z - r) / 2), m["carbon"], h, lods=MID)
    # The seeker's bezel, the warhead section's seam and arming plug, the
    # wings' hinge blocks, the datalink antennas on the spine.
    cyl("seeker_bezel", r * 0.52, 0.010, (L * 0.497, 0, z), "X", m["dark"], h, seg=14, lods=NEAR)
    cyl("warhead_seam", r * 1.02, 0.006, (L * 0.36, 0, z), "X", m["black"], h, seg=16, lods=FINE)
    cyl("arming_plug", r * 0.12, 0.010, (L * 0.30, 0, z + r), "Z", m["steel"], h, seg=8, lods=FINE)
    for k, x in enumerate((-L * 0.05, -L * 0.18)):
        box(f"datalink_antenna_{k}", (0.012, 0.004, r * 0.30), (x, 0, z + r * 1.10), m["dark"], h, lods=NEAR)
    for k, x in enumerate((L * 0.05, -L * 0.28) if not lancet else (L * 0.12, -L * 0.30)):
        box(f"wing_hinge_{k}", (L * 0.06, r * 0.9, r * 0.20), (x, 0, z + r * 0.88), m["dark"], h, lods=NEAR)


def orlan(v):
    m, h = v.mats, v.hull
    L, W, H = v.length, v.width, v.height
    z = 0.11
    loft("fuselage", [(z - 0.10, [(0.62, 0.0), (0.47, -0.08), (-0.80, -0.06), (-0.80, 0.06), (0.47, 0.08)]),
                      (z + 0.14, [(0.54, 0.0), (0.42, -0.07), (-0.80, -0.05), (-0.80, 0.05), (0.42, 0.07)])],
         mat=m["paint"], parent=h, bevel=0.02)
    cyl("nose_cowl", 0.08, 0.14, (0.67, 0, z + 0.02), "X", m["paint"], h, seg=16, r2=0.05)
    propeller("propeller", (0.76, 0, z + 0.02), 0.38, "X", m["carbon"], h)
    box("wing", (0.28, W, 0.022), (0.15, 0, z + 0.17), m["paint"], h, bevel=0.006)
    for side in (-1, 1):
        box(f"aileron_{side}", (0.06, W * 0.30, 0.012), (0.0, side * W * 0.32, z + 0.17), m["dark"], h, lods=NEAR)
        box(f"tail_{side}", (0.22, 0.45, 0.012), (-0.80, side * 0.16, z + 0.24), m["paint"], h,
            rot=(side * 0.70, 0, 0), bevel=0.004)
    cyl("tail_boom", 0.03, 0.40, (-0.80, 0, z + 0.06), "X", m["paint"], h, seg=10, lods=MID)
    gimbal("gimbal", (0.40, 0, z - 0.10), 0.09, m, h)
    box("antenna_fin", (0.06, 0.006, 0.08), (-0.20, 0, z + 0.18), m["dark"], h, lods=NEAR)
    # The wing's pylon on the spine, its flaps and wing-tip lights, the
    # engine's cylinder head and exhaust under the cowl, the parachute bay's
    # lid, the pitot and the catapult's launch lug.
    box("wing_pylon", (0.22, 0.10, 0.05), (0.15, 0, z + 0.14), m["paint"], h, bevel=0.01, lods=MID)
    for side in (-1, 1):
        box(f"flap_{side}", (0.06, W * 0.18, 0.012), (0.0, side * W * 0.12, z + 0.17), m["dark"], h, lods=NEAR)
        cyl(f"tip_light_{side}", 0.012, 0.02, (0.20, side * (W / 2 - 0.01), z + 0.17), "Y",
            m["tail"] if side > 0 else m["lamp"], h, seg=8, lods=FINE)
    box("cylinder_head", (0.08, 0.16, 0.06), (0.58, 0, z - 0.08), m["dark"], h, bevel=0.01, lods=MID)
    cyl("exhaust", 0.015, 0.14, (0.50, -0.07, z - 0.09), "X", m["steel"], h, seg=8, lods=NEAR)
    box("parachute_lid", (0.30, 0.10, 0.004), (-0.30, 0, z + 0.12), m["dark"], h, lods=NEAR)
    cyl("pitot", 0.004, 0.10, (0.27, W * 0.30, z + 0.15), "X", m["steel"], h, seg=6, lods=FINE)
    box("launch_lug", (0.06, 0.04, 0.03), (0.10, 0, z - 0.11), m["steel"], h, lods=NEAR)


def build(variant, v):
    kind = CARDS[variant["id"]][2]
    v.mats = shell_mats(v)
    if kind == "orlan":
        orlan(v)
    elif kind in ("switchblade_600", "switchblade_300", "hero_120", "lancet"):
        loitering(v, kind)
    else:
        quad(v, kind)
    # Resting on whatever reaches lowest: legs, belly, or a Lancet's wing tips.
    bpy.context.view_layer.update()
    low = min((o.matrix_world @ Vector(c)).z for o in bpy.data.objects
              if o.type == "MESH" and o.name.endswith("_LOD0") for c in o.bound_box)
    v.root.location.z -= low


def wreck(variant, v):
    """A downed drone: wings or arms broken off, the body cracked and
    scorched where its battery or warhead burned; what it throws is
    `wreckage.scatter`'s."""
    from parts import rest_on_ground
    from wreckage import densify, heat, parts, remove, warp
    remove("arm_1", "motor_1", "rotor_1_", "arm_hinge_1", "arm_light_1", "wing_0_1", "x_wing_0_1", "aileron_1",
           "aileron_0_1", "propeller_blade_1", "propeller_disc_1", "pusher_blade_1", "pusher_disc_1", "gimbal_yoke",
           "warhead_strap_0", "flap_1", "tip_light_1")
    shell = parts("body", "fuselage", "wing", "frame_plate", "x_wing_")
    densify(shell, scale=0.4)
    warp(shell, heat(0.004 * max(1.0, v.length), 0.3 * v.length, seed=99.0))
    v.root.rotation_euler = (0.18, 0.06, 0.4)
    rest_on_ground(0.002)


if __name__ == "__main__":
    run_disabled("drones", {c: dims for c, (_, _, _, dims) in CARDS.items()},
                 {c: scheme for c, (_, scheme, _, _) in CARDS.items()}, build, wreck, chip=0.3)
