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


def _rifle_parts(mats, name, parts):
    """A squad rifle on the carbine's frame: the same grip, support and muzzle
    points (so the same clips hold it and its muzzle socket fits), its own
    silhouette. Parts are (name, builder, kwargs, material key)."""
    root = empty(name, (0, 0, 0), size=0.05)
    _parts(root, name, [(n, fn, {**kw, "mat_": mats[key]}) for n, fn, kw, key in parts])
    empty("muzzle", (0, -0.67, 0.045), root, 0.02)
    return root


def m4a1(mats, name="m4a1"):
    """The US Army's M4A1 (PEO Soldier reference): flat-top upper with an M68
    close-combat optic, quad-rail handguard with a vertical grip, light and
    PEQ box, A-frame front sight, curved 30-round magazine, buffer tube and
    collapsible stock. About 0.84 m; small parts chunky, after the look."""
    g, p, glass = "gun_black", "gun_fde", "lens"
    deg = math.radians
    return _rifle_parts(mats, name, [
        ("lower", box, dict(size=(0.030, 0.20, 0.050), loc=(0, -0.005, 0.0), bevel_=0.004), g),
        ("magwell", box, dict(size=(0.034, 0.075, 0.055), loc=(0, -0.085, -0.03), bevel_=0.004), g),
        ("mag_upper", box, dict(size=(0.026, 0.074, 0.11), loc=(0, -0.09, -0.095), bevel_=0.004, rot=(deg(-8), 0, 0)), g),
        ("mag_lower", box, dict(size=(0.026, 0.07, 0.085), loc=(0, -0.073, -0.185), bevel_=0.004, rot=(deg(-20), 0, 0)), g),
        ("trigger_guard", box, dict(size=(0.026, 0.065, 0.012), loc=(0, 0.015, -0.042)), g),
        ("grip", box, dict(size=(0.030, 0.040, 0.105), loc=(0, 0.06, -0.05), bevel_=0.006, rot=(deg(22), 0, 0)), g),
        ("upper", box, dict(size=(0.034, 0.23, 0.052), loc=(0, -0.04, 0.047), bevel_=0.004), g),
        ("top_rail", box, dict(size=(0.026, 0.23, 0.014), loc=(0, -0.04, 0.079)), g),
        ("charging_handle", box, dict(size=(0.05, 0.03, 0.014), loc=(0, 0.078, 0.07), bevel_=0.003), g),
        ("forward_assist", cyl, dict(r=0.011, depth=0.035, loc=(-0.022, 0.035, 0.055), axis="Y", seg=8), g),
        ("handguard", box, dict(size=(0.056, 0.25, 0.058), loc=(0, -0.28, 0.045), bevel_=0.008), g),
        ("handguard_rail_top", box, dict(size=(0.024, 0.25, 0.012), loc=(0, -0.28, 0.079)), g),
        ("handguard_rail_l", box, dict(size=(0.012, 0.22, 0.024), loc=(0.033, -0.28, 0.045)), g),
        ("handguard_rail_r", box, dict(size=(0.012, 0.22, 0.024), loc=(-0.033, -0.28, 0.045)), g),
        ("front_sight_base", box, dict(size=(0.026, 0.035, 0.03), loc=(0, -0.43, 0.06), bevel_=0.004), g),
        ("front_sight_post", box, dict(size=(0.012, 0.022, 0.055), loc=(0, -0.43, 0.1), bevel_=0.003), g),
        ("barrel", cyl, dict(r=0.0105, depth=0.24, loc=(0, -0.52, 0.045), axis="Y", seg=10), g),
        ("flash_hider", cyl, dict(r=0.015, depth=0.055, loc=(0, -0.6425, 0.045), axis="Y", seg=10), g),
        ("buffer_tube", cyl, dict(r=0.016, depth=0.22, loc=(0, 0.19, 0.045), axis="Y", seg=10), g),
        ("stock", box, dict(size=(0.042, 0.17, 0.07), loc=(0, 0.29, 0.032), bevel_=0.01), g),
        ("stock_heel", box, dict(size=(0.036, 0.09, 0.05), loc=(0, 0.32, -0.02), bevel_=0.008, rot=(deg(-25), 0, 0)), g),
        ("butt_pad", box, dict(size=(0.046, 0.022, 0.115), loc=(0, 0.369, 0.02), bevel_=0.006), g),
        ("optic_mount", box, dict(size=(0.03, 0.06, 0.025), loc=(0, -0.04, 0.094), bevel_=0.003), g),
        ("optic", cyl, dict(r=0.024, depth=0.12, loc=(0, -0.04, 0.125), axis="Y", seg=14, bevel_=0.003), g),
        ("optic_turret", cyl, dict(r=0.011, depth=0.022, loc=(-0.028, -0.04, 0.125), axis="X", seg=8), g),
        ("optic_lens", cyl, dict(r=0.02, depth=0.004, loc=(0, -0.1015, 0.125), axis="Y", seg=14), glass),
        ("peq", box, dict(size=(0.04, 0.075, 0.038), loc=(0, -0.22, 0.104), bevel_=0.005), p),
        ("vertical_grip", cyl, dict(r=0.015, depth=0.08, loc=(0, -0.3, -0.017), axis="Z", seg=10, bevel_=0.003), g),
        ("light", cyl, dict(r=0.014, depth=0.08, loc=(-0.05, -0.36, 0.045), axis="Y", seg=10), g),
        ("light_lens", cyl, dict(r=0.012, depth=0.004, loc=(-0.05, -0.402, 0.045), axis="Y", seg=10), glass),
    ])


def m27(mats, name="m27"):
    """The Marines' M27 (HK416, PEO/USMC references): the M4's flat-top
    receiver on a piston upper with a long free-float rail and no front-sight
    base, a squad day optic (a squat box with a big objective), a bipod grip,
    the enhanced stock and a polymer magazine."""
    g, p, glass = "gun_black", "gun_polymer", "lens"
    deg = math.radians
    return _rifle_parts(mats, name, [
        ("lower", box, dict(size=(0.032, 0.2, 0.052), loc=(0, -0.005, 0.0), bevel_=0.004), g),
        ("magwell", box, dict(size=(0.036, 0.075, 0.056), loc=(0, -0.085, -0.03), bevel_=0.004), g),
        ("mag_upper", box, dict(size=(0.028, 0.074, 0.11), loc=(0, -0.09, -0.095), bevel_=0.004, rot=(deg(-8), 0, 0)), p),
        ("mag_lower", box, dict(size=(0.028, 0.07, 0.085), loc=(0, -0.073, -0.185), bevel_=0.004, rot=(deg(-20), 0, 0)), p),
        ("trigger_guard", box, dict(size=(0.026, 0.065, 0.012), loc=(0, 0.015, -0.042)), g),
        ("grip", box, dict(size=(0.032, 0.044, 0.105), loc=(0, 0.06, -0.05), bevel_=0.007, rot=(deg(20), 0, 0)), p),
        ("upper", box, dict(size=(0.036, 0.2, 0.054), loc=(0, -0.03, 0.047), bevel_=0.004), g),
        ("top_rail", box, dict(size=(0.026, 0.62, 0.014), loc=(0, -0.24, 0.08)), g),
        ("charging_handle", box, dict(size=(0.05, 0.03, 0.014), loc=(0, 0.078, 0.07), bevel_=0.003), g),
        ("rail_handguard", box, dict(size=(0.06, 0.36, 0.062), loc=(0, -0.31, 0.045), bevel_=0.008), g),
        ("handguard_rail_l", box, dict(size=(0.012, 0.32, 0.024), loc=(0.035, -0.31, 0.045)), g),
        ("handguard_rail_r", box, dict(size=(0.012, 0.32, 0.024), loc=(-0.035, -0.31, 0.045)), g),
        ("handguard_rail_b", box, dict(size=(0.024, 0.32, 0.012), loc=(0, -0.31, 0.011)), g),
        ("barrel", cyl, dict(r=0.011, depth=0.15, loc=(0, -0.565, 0.045), axis="Y", seg=10), g),
        ("flash_hider", cyl, dict(r=0.016, depth=0.055, loc=(0, -0.6425, 0.045), axis="Y", seg=10), g),
        ("buffer_tube", cyl, dict(r=0.017, depth=0.2, loc=(0, 0.18, 0.045), axis="Y", seg=10), g),
        ("stock", box, dict(size=(0.05, 0.18, 0.078), loc=(0, 0.285, 0.035), bevel_=0.012), p),
        ("stock_heel", box, dict(size=(0.04, 0.09, 0.05), loc=(0, 0.32, -0.02), bevel_=0.008, rot=(deg(-25), 0, 0)), p),
        ("butt_pad", box, dict(size=(0.05, 0.022, 0.12), loc=(0, 0.37, 0.02), bevel_=0.006), g),
        ("optic_mount", box, dict(size=(0.032, 0.07, 0.022), loc=(0, -0.04, 0.097), bevel_=0.003), g),
        ("optic_body", box, dict(size=(0.05, 0.12, 0.05), loc=(0, -0.04, 0.128), bevel_=0.01), g),
        ("optic_objective", cyl, dict(r=0.026, depth=0.03, loc=(0, -0.11, 0.128), axis="Y", seg=14, bevel_=0.003), g),
        ("optic_lens", cyl, dict(r=0.022, depth=0.004, loc=(0, -0.126, 0.128), axis="Y", seg=14), glass),
        ("bipod_grip", box, dict(size=(0.03, 0.035, 0.1), loc=(0, -0.3, -0.035), bevel_=0.006), p),
        ("light", cyl, dict(r=0.014, depth=0.08, loc=(-0.05, -0.42, 0.045), axis="Y", seg=10), g),
        ("light_lens", cyl, dict(r=0.012, depth=0.004, loc=(-0.05, -0.462, 0.045), axis="Y", seg=10), glass),
    ])


def g36(mats, name="g36"):
    """The Bundeswehr's G36 (references: G36 and G36A3 in German service):
    its silhouette is the tall carry handle with the optic built in, the
    skeleton folding stock, a slim tapered handguard and the curved
    translucent magazine, all in dark polymer."""
    g, p, glass = "gun_black", "gun_polymer", "lens"
    deg = math.radians
    return _rifle_parts(mats, name, [
        ("receiver", box, dict(size=(0.04, 0.3, 0.07), loc=(0, -0.04, 0.03), bevel_=0.008), p),
        ("trigger_group", box, dict(size=(0.034, 0.1, 0.03), loc=(0, 0.03, -0.014), bevel_=0.006), p),
        ("trigger_guard", box, dict(size=(0.028, 0.07, 0.012), loc=(0, 0.015, -0.042)), p),
        ("grip", box, dict(size=(0.032, 0.044, 0.105), loc=(0, 0.06, -0.05), bevel_=0.008, rot=(deg(20), 0, 0)), p),
        ("handle_post_rear", box, dict(size=(0.03, 0.05, 0.05), loc=(0, 0.07, 0.085), bevel_=0.008), p),
        ("handle_post_front", box, dict(size=(0.03, 0.04, 0.05), loc=(0, -0.13, 0.085), bevel_=0.008), p),
        ("carry_handle", box, dict(size=(0.036, 0.26, 0.036), loc=(0, -0.03, 0.127), bevel_=0.01), p),
        ("optic_hood", cyl, dict(r=0.02, depth=0.05, loc=(0, 0.09, 0.124), axis="Y", seg=12), p),
        ("optic_lens", cyl, dict(r=0.016, depth=0.004, loc=(0, -0.162, 0.127), axis="Y", seg=12), glass),
        ("cocking_handle", box, dict(size=(0.05, 0.02, 0.012), loc=(0, -0.12, 0.105)), g),
        ("handguard", box, dict(size=(0.054, 0.26, 0.06), loc=(0, -0.3, 0.035), bevel_=0.014), p),
        ("handguard_tip", box, dict(size=(0.044, 0.04, 0.05), loc=(0, -0.44, 0.04), bevel_=0.012), p),
        ("barrel", cyl, dict(r=0.0105, depth=0.18, loc=(0, -0.55, 0.045), axis="Y", seg=10), g),
        ("flash_hider", cyl, dict(r=0.015, depth=0.06, loc=(0, -0.64, 0.045), axis="Y", seg=10), g),
        ("mag_upper", box, dict(size=(0.028, 0.075, 0.1), loc=(0, -0.1, -0.06), bevel_=0.005, rot=(deg(-10), 0, 0)), p),
        ("mag_lower", box, dict(size=(0.028, 0.07, 0.1), loc=(0, -0.08, -0.15), bevel_=0.005, rot=(deg(-24), 0, 0)), p),
        ("stock_hinge", box, dict(size=(0.036, 0.04, 0.07), loc=(0, 0.115, 0.03), bevel_=0.008), p),
        ("stock_top", box, dict(size=(0.03, 0.24, 0.026), loc=(0, 0.24, 0.062), bevel_=0.008), p),
        ("stock_bottom", box, dict(size=(0.026, 0.23, 0.022), loc=(0, 0.245, -0.012), bevel_=0.008, rot=(deg(-6), 0, 0)), p),
        ("butt_plate", box, dict(size=(0.042, 0.03, 0.12), loc=(0, 0.365, 0.02), bevel_=0.01), p),
    ])


def ak12(mats, name="ak12"):
    """The Russian squad's AK-12 (2023 reference): the AK's slab receiver and
    deep curved magazine under a ribbed dust cover that carries a rail, a
    railed handguard over the gas tube, front-sight block and long brake, a
    skeleton folding and telescoping stock, and a collimator on the rail."""
    g, poly, glass = "gun_black", "gun_polymer", "lens"
    deg = math.radians
    return _rifle_parts(mats, name, [
        ("receiver", box, dict(size=(0.036, 0.29, 0.058), loc=(0, -0.03, 0.018), bevel_=0.004), g),
        ("dust_cover", box, dict(size=(0.034, 0.25, 0.028), loc=(0, -0.01, 0.06), bevel_=0.009), g),
        ("dust_cover_rail", box, dict(size=(0.024, 0.24, 0.012), loc=(0, -0.01, 0.079)), g),
        ("rear_sight_block", box, dict(size=(0.03, 0.05, 0.03), loc=(0, -0.175, 0.062), bevel_=0.004), g),
        ("charging_handle", cyl, dict(r=0.009, depth=0.035, loc=(-0.032, -0.07, 0.04), axis="X", seg=8), g),
        ("safety_lever", box, dict(size=(0.004, 0.11, 0.018), loc=(-0.02, 0.02, 0.025)), g),
        ("trigger_guard", box, dict(size=(0.026, 0.07, 0.01), loc=(0, 0.015, -0.035)), g),
        ("grip", box, dict(size=(0.032, 0.044, 0.1), loc=(0, 0.06, -0.05), bevel_=0.007, rot=(deg(16), 0, 0)), poly),
        ("mag_1", box, dict(size=(0.028, 0.072, 0.075), loc=(0, -0.115, -0.045), bevel_=0.004, rot=(deg(-10), 0, 0)), poly),
        ("mag_2", box, dict(size=(0.028, 0.07, 0.07), loc=(0, -0.1, -0.11), bevel_=0.004, rot=(deg(-25), 0, 0)), poly),
        ("mag_3", box, dict(size=(0.028, 0.068, 0.07), loc=(0, -0.068, -0.17), bevel_=0.004, rot=(deg(-42), 0, 0)), poly),
        ("handguard", box, dict(size=(0.052, 0.24, 0.06), loc=(0, -0.3, 0.045), bevel_=0.01), poly),
        ("handguard_rail", box, dict(size=(0.024, 0.24, 0.012), loc=(0, -0.3, 0.081)), g),
        ("gas_block", box, dict(size=(0.026, 0.04, 0.06), loc=(0, -0.45, 0.065), bevel_=0.005), g),
        ("front_sight", box, dict(size=(0.024, 0.03, 0.05), loc=(0, -0.5, 0.08), bevel_=0.004), g),
        ("barrel", cyl, dict(r=0.0105, depth=0.2, loc=(0, -0.5, 0.045), axis="Y", seg=10), g),
        ("muzzle_brake", cyl, dict(r=0.018, depth=0.1, loc=(0, -0.62, 0.045), axis="Y", seg=12), g),
        ("brake_port", box, dict(size=(0.038, 0.024, 0.014), loc=(0, -0.63, 0.045)), g),
        ("stock_tube", cyl, dict(r=0.014, depth=0.14, loc=(0, 0.17, 0.04), axis="Y", seg=10), g),
        ("stock_top", box, dict(size=(0.03, 0.17, 0.026), loc=(0, 0.27, 0.05), bevel_=0.008), poly),
        ("stock_strut", box, dict(size=(0.026, 0.16, 0.022), loc=(0, 0.27, -0.01), bevel_=0.008, rot=(deg(-14), 0, 0)), poly),
        ("cheek_rest", box, dict(size=(0.036, 0.1, 0.022), loc=(0, 0.27, 0.07), bevel_=0.008), poly),
        ("butt_plate", box, dict(size=(0.044, 0.026, 0.12), loc=(0, 0.365, 0.02), bevel_=0.008), poly),
        ("collimator", box, dict(size=(0.034, 0.06, 0.036), loc=(0, -0.04, 0.103), bevel_=0.008), g),
        ("collimator_lens", box, dict(size=(0.026, 0.004, 0.026), loc=(0, -0.0715, 0.103)), glass),
    ])


# A squad rifle per army (`roster/infantry_equipment.py` ARMIES).
RIFLES = {"m4a1": m4a1, "m27": m27, "g36": g36, "ak12": ak12}


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
