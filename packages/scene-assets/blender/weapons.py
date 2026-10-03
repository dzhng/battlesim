"""Infantry weapons, Blender-scripted, and the holds they are carried with.

Each weapon is built in its own frame: local -Y forward, Z up, origin at the
pistol grip, in the units its hold is defined in, then scaled by the hold's
`scale` when placed. The `muzzle` socket sits at the bore's end.

A hold family is one clip set (`clips_infantry.py`): its grip and support
points, its low carry, its sight for shouldered poses and its recoil. The
rifle and the recon DMR share the rifle family: the DMR keeps the carbine's
grip-to-handguard geometry, so the same hands hold it. The AT launcher has its
own family, carried on the shoulder.
"""

import math

from mathutils import Matrix

from common import box, cyl, empty
from parts import stencil
from infantry_rig import weapon_matrix


def _parts(root, name, parts):
    for n, fn, kw in parts:
        fn(f"{name}_{n}", parent=root, **kw)


def rifle(mats, name="rifle"):
    """The rifleman's modern carbine (spike 03, detailed), about 0.86 m."""
    black = mats["gun_black"]
    root = empty(name, (0, 0, 0), size=0.05)
    _parts(root, name, [
        ("upper", box, dict(size=(0.032, 0.25, 0.055), loc=(0, -0.05, 0.045), mat_=black, bevel_=0.003)),
        ("rail", box, dict(size=(0.022, 0.52, 0.012), loc=(0, -0.20, 0.078), mat_=black)),
        ("lower", box, dict(size=(0.030, 0.17, 0.045), loc=(0, 0.0, 0.0), mat_=black, bevel_=0.003)),
        ("guard", box, dict(size=(0.046, 0.30, 0.05), loc=(0, -0.32, 0.045), mat_=black, bevel_=0.006)),
        ("barrel", cyl, dict(r=0.009, depth=0.14, loc=(0, -0.54, 0.045), axis="Y", mat_=black, seg=10)),
        ("muzzle_dev", cyl, dict(r=0.015, depth=0.06, loc=(0, -0.63, 0.045), axis="Y", mat_=black, seg=10)),
        ("mag", box, dict(size=(0.024, 0.07, 0.17), loc=(0, -0.07, -0.08), mat_=black, bevel_=0.004, rot=(math.radians(-12), 0, 0))),
        ("grip", box, dict(size=(0.028, 0.038, 0.10), loc=(0, 0.06, -0.05), mat_=black, bevel_=0.005, rot=(math.radians(22), 0, 0))),
        ("stock", box, dict(size=(0.036, 0.20, 0.07), loc=(0, 0.28, 0.03), mat_=black, bevel_=0.008)),
        ("buffer", cyl, dict(r=0.016, depth=0.12, loc=(0, 0.16, 0.045), axis="Y", mat_=black, seg=10)),
        ("optic", cyl, dict(r=0.018, depth=0.11, loc=(0, -0.05, 0.11), axis="Y", mat_=black, seg=14)),
        ("optic_mount", box, dict(size=(0.02, 0.05, 0.03), loc=(0, -0.05, 0.085), mat_=black)),
        ("fore_grip", box, dict(size=(0.022, 0.03, 0.07), loc=(0, -0.30, -0.01), mat_=black, bevel_=0.004)),
        ("light", cyl, dict(r=0.012, depth=0.07, loc=(0.03, -0.40, 0.05), axis="Y", mat_=black, seg=10)),
    ])
    empty("muzzle", (0, -0.67, 0.045), root, 0.02)
    return root


def dmr(mats, name="dmr"):
    """The recon team's designated marksman rifle: a long barrel and handguard, a
    magnified scope and a folded bipod, on the carbine's grip-to-handguard geometry."""
    black = mats["gun_black"]
    root = empty(name, (0, 0, 0), size=0.05)
    _parts(root, name, [
        ("upper", box, dict(size=(0.034, 0.27, 0.058), loc=(0, -0.05, 0.045), mat_=black, bevel_=0.003)),
        ("lower", box, dict(size=(0.032, 0.18, 0.048), loc=(0, 0.0, 0.0), mat_=black, bevel_=0.003)),
        ("guard", box, dict(size=(0.05, 0.42, 0.054), loc=(0, -0.38, 0.045), mat_=black, bevel_=0.006)),
        ("barrel", cyl, dict(r=0.010, depth=0.24, loc=(0, -0.70, 0.045), axis="Y", mat_=black, seg=10)),
        ("suppressor", cyl, dict(r=0.019, depth=0.17, loc=(0, -0.88, 0.045), axis="Y", mat_=black, seg=12)),
        ("mag", box, dict(size=(0.03, 0.08, 0.12), loc=(0, -0.07, -0.06), mat_=black, bevel_=0.004)),
        ("grip", box, dict(size=(0.028, 0.038, 0.10), loc=(0, 0.06, -0.05), mat_=black, bevel_=0.005, rot=(math.radians(22), 0, 0))),
        ("stock", box, dict(size=(0.04, 0.22, 0.085), loc=(0, 0.29, 0.03), mat_=black, bevel_=0.01)),
        ("cheek", box, dict(size=(0.036, 0.12, 0.03), loc=(0, 0.26, 0.085), mat_=black, bevel_=0.008)),
        ("scope", cyl, dict(r=0.02, depth=0.26, loc=(0, -0.05, 0.115), axis="Y", mat_=black, seg=14)),
        ("objective", cyl, dict(r=0.028, depth=0.07, loc=(0, -0.20, 0.115), axis="Y", mat_=black, seg=14)),
        ("turret", cyl, dict(r=0.014, depth=0.05, loc=(0, -0.04, 0.14), axis="Z", mat_=black, seg=10)),
        ("mount", box, dict(size=(0.022, 0.12, 0.03), loc=(0, -0.05, 0.088), mat_=black)),
        ("bipod", box, dict(size=(0.03, 0.2, 0.02), loc=(0, -0.48, 0.005), mat_=black, bevel_=0.004)),
    ])
    empty("muzzle", (0, -0.965, 0.045), root, 0.02)
    return root


def launcher(mats, name="launcher"):
    """The AT gunner's command launch unit on its missile tube, carried on the right
    shoulder. The grip is the CLU's right handle; the tube's axis is 0.1 m above it."""
    black, tube_m, strap = mats["gun_black"], mats["launcher"], mats["webbing"]
    root = empty(name, (0, 0, 0), size=0.05)
    _parts(root, name, [
        ("tube", cyl, dict(r=0.07, depth=1.18, loc=(0, -0.02, 0.10), axis="Y", mat_=tube_m, seg=20, bevel_=0.004)),
        ("end_front", cyl, dict(r=0.078, depth=0.05, loc=(0, -0.60, 0.10), axis="Y", mat_=black, seg=20, bevel_=0.004)),
        ("end_rear", cyl, dict(r=0.082, depth=0.07, loc=(0, 0.57, 0.10), axis="Y", mat_=black, seg=20, bevel_=0.004)),
        ("band_front", cyl, dict(r=0.074, depth=0.03, loc=(0, -0.30, 0.10), axis="Y", mat_=black, seg=20)),
        ("band_rear", cyl, dict(r=0.074, depth=0.03, loc=(0, 0.28, 0.10), axis="Y", mat_=black, seg=20)),
        ("clu", box, dict(size=(0.12, 0.26, 0.15), loc=(0.12, 0.02, 0.10), mat_=tube_m, bevel_=0.012)),
        ("clu_sight", box, dict(size=(0.07, 0.07, 0.07), loc=(0.14, 0.17, 0.13), mat_=black, bevel_=0.008)),
        ("clu_lens", cyl, dict(r=0.035, depth=0.03, loc=(0.12, -0.12, 0.12), axis="Y", mat_=black, seg=14)),
        ("clu_hood", cyl, dict(r=0.03, depth=0.05, loc=(0.14, 0.23, 0.13), axis="Y", mat_=black, seg=12)),
        ("clu_battery", box, dict(size=(0.06, 0.1, 0.06), loc=(0.19, 0.06, 0.06), mat_=black, bevel_=0.006)),
        ("clu_handle_l", box, dict(size=(0.025, 0.03, 0.09), loc=(0.2, -0.02, 0.05), mat_=black, bevel_=0.005)),
        ("rear_ring", cyl, dict(r=0.086, depth=0.02, loc=(0, 0.53, 0.10), axis="Y", mat_=tube_m, seg=20)),
        ("front_ring", cyl, dict(r=0.082, depth=0.02, loc=(0, -0.56, 0.10), axis="Y", mat_=tube_m, seg=20)),
        ("sling_front", box, dict(size=(0.02, 0.03, 0.04), loc=(-0.07, -0.4, 0.1), mat_=black)),
        ("sling_rear", box, dict(size=(0.02, 0.03, 0.04), loc=(-0.07, 0.4, 0.1), mat_=black)),
        ("grip", box, dict(size=(0.03, 0.04, 0.10), loc=(0, 0.06, -0.02), mat_=black, bevel_=0.006, rot=(math.radians(15), 0, 0))),
        ("fore_grip", box, dict(size=(0.03, 0.04, 0.09), loc=(0, -0.25, 0.0), mat_=black, bevel_=0.006)),
        ("strap", box, dict(size=(0.005, 0.8, 0.03), loc=(-0.075, 0.0, 0.08), mat_=strap)),
    ])
    # the tube's stencilled nomenclature on both sides, reading muzzle-ward, and its
    # coloured ring bands; rubber caps at both ends
    outboard = (Matrix.Rotation(-math.pi / 2, 4, "Y") @ Matrix.Rotation(-math.pi / 2, 4, "Z")).to_euler()
    inboard = (Matrix.Rotation(math.pi / 2, 4, "Y") @ Matrix.Rotation(math.pi / 2, 4, "Z")).to_euler()
    labels = (("LAUNCHER, GUIDED MISSILE", -0.072, -0.12, 0.112, outboard), ("ROUND 2-86  FRONT", -0.072, 0.3, 0.09, outboard),
              ("CAUTION  BACKBLAST AREA", 0.072, 0.36, 0.1, inboard))
    for k, (text, x, y, z, facing) in enumerate(labels):
        stencil(f"{name}_label_{k}", text, 0.018, (x, y, z), facing, mats["marking"], root, lods=(0,), depth=0.002)
    for k, y in enumerate((-0.46, -0.42)):
        cyl(f"{name}_ring_band_{k}", 0.0712, 0.018, loc=(0, y, 0.10), axis="Y", mat_=mats["pads"], parent=root, seg=20)
    for k, (y, r) in enumerate(((-0.63, 0.072), (0.615, 0.076))):
        cyl(f"{name}_cap_{k}", r, 0.03, loc=(0, y, 0.10), axis="Y", mat_=mats["pads"], parent=root, seg=20, bevel_=0.006)
    empty("muzzle", (0, -0.625, 0.10), root, 0.02)
    return root


RIFLE_SCALE = 0.86

# Rifle family (spike 03's holds): grip and support points in the weapon's frame; the
# low-ready carry (stock at the right chest, muzzle forward-down and a little left);
# the optic, 8 cm ahead of the eye when shouldered, halfway across toward the right
# shoulder pocket; the kneel_fire recoil (frame, back m, up deg); prone with the butt
# in the shoulder and the support hand 10 cm back on the handguard.
RIFLE_HOLD = {
    "family": "rifle",
    "grip": (0, 0.06, -0.035),
    "support": (0, -0.30, 0.0),
    "scale": RIFLE_SCALE,
    "low": weapon_matrix((-0.12, -0.22, 1.16), yaw_deg=25, pitch_deg=-38),
    "low_poles": ((-0.45, 0.25, 0.95), (0.35, -0.10, 0.85)),
    "sight": (0, 0.005, 0.11),
    "cheek_blend": 0.5,
    "recoil": ((0, 0, 0), (3, 0, 0), (5, 0.06, 7.0), (8, 0.03, 3.5), (14, 0, 0), (36, 0, 0)),
    "prone_butt": (0, 0.39, 0.03),
    "prone_support_back": 0.1,
    "death_hold": 0.26,
}

# Launcher family: the tube rides the right shoulder in the carry, level and pointing
# forward; shouldered, the CLU's sight sits at the eye; the missile's launch jolt is
# a short push back; he falls with the tube lying across his chest (`death_hold`, its
# height off the chest in metres; None keeps the carry, as the rifle does).
LAUNCHER_HOLD = {
    "family": "launcher",
    "grip": (0, 0.06, -0.035),
    "support": (0, -0.25, 0.0),
    "scale": 1.0,
    "low": weapon_matrix((-0.16, -0.15, 1.42), yaw_deg=4, pitch_deg=-18),
    "low_poles": ((-0.6, 0.1, 0.95), (0.4, -0.2, 1.0)),
    "sight": (0.14, 0.21, 0.13),
    "cheek_blend": 0.0,
    "recoil": ((0, 0, 0), (3, 0, 0), (5, 0.03, 2.0), (9, 0.01, 0.8), (16, 0, 0), (36, 0, 0)),
    "prone_butt": None,
    "prone_support_back": 0.0,
    "death_hold": 0.04,
}

FAMILIES = {"rifle": RIFLE_HOLD, "launcher": LAUNCHER_HOLD}

# Each infantry kind: its weapon and the hold family it is carried with.
KINDS = {
    "rifle": {"build": rifle, "hold": RIFLE_HOLD},
    "recon": {"build": dmr, "hold": RIFLE_HOLD},
    "at": {"build": launcher, "hold": LAUNCHER_HOLD},
    "at_carried": {"build": rifle, "hold": RIFLE_HOLD},
}
