"""Air defence vehicles: Gepard 1A2, Buk-M3, Pantsir-SM,
Boxer Skyranger 30 and NASAMS 3, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/air_defence.py -- [--variant=<card id>] [--wreck]

What the photos settle:
- Gepard 1A2 (German three-tone) wears a scripted body (`scripted_body`):
  Gemini Argon's build script after its second round, edited since
  (`assets/source/skins/gepard/argon-r2/`, whose README lists the edits).
  Its turret is the gun rig's, the two 35 mm guns one pitching `gun`.
- Buk-M3 TELAR (9A317M, Russian green) wears a scripted body as well
  (`assets/source/skins/buk/argon-r2/`): the GM-569 chassis with seven road
  wheels and the sprocket at the rear, no cab; the turret forward with the
  9S36M radar's "pillow" housing at its front and six canisters lying
  behind it, the raised engine compartment at the rear. Its launcher is a
  turning mount its model draws fixed on the hull.
- Pantsir-SM (Russian green): the KamAZ-6560 8x8's unarmoured cab-over with
  its tall windscreen; the combat module on the rear: the search radar's
  array on top, the tracking radar's flat array on its front, a twin 30 mm
  gun each side and six missile tubes a side, two columns of three, outside
  them.
- Skyranger 30 on Boxer (German three-tone): the Boxer's own hull
  (`boxer.drive`), the turret with its 30 mm KCE gun, the four radar
  panels round its top and a missile launcher box on its side.
- NASAMS 3 (Norway's, NATO green): the canister launcher on its two-axle
  trailer, six AMRAAM canisters in two rows on the turntable, stowed flat,
  levelling legs at the corners. The US Army's is the same launcher in its
  own tan.

Turrets, guns and launchers are drawn stowed and do not articulate yet.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import boxer  # noqa: E402
import armour_parts as AR  # noqa: E402
import hull_fittings as HF  # noqa: E402
import roof_parts as RF  # noqa: E402
import running_gear as RG  # noqa: E402
import scripted_body as SB  # noqa: E402
import stowage as SW  # noqa: E402
import vehicle_parts as VP  # noqa: E402
from parts import box, cyl, empty, loft  # noqa: E402
from truck_chassis import chassis  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

FINE, NEAR, MID = VP.FINE, VP.NEAR, VP.MID
TOW_SIZE = 0.13  # a tow hook's size (`hull_fittings.tow_hook`)
TOW_REACH = TOW_SIZE * 1.15  # how far the hook, curl and all, stands out from its plate

# Each appearance's family (its source folder and reference library) and scheme.
CARDS = {
    "europe_gepard_1a2": ("gepard", "german_three_tone"),
    "eastern_buk_buk_m3_medium_range_air_defense": ("buk", "russian_green"),
    "eastern_pantsir_pantsir_sm": ("pantsir", "russian_green"),
    "europe_boxer_skyranger_30_gun_and_missile_air_defense": ("boxer_skyranger_30", "german_three_tone"),
    "europe_nasams_nasams_3": ("nasams", "german_three_tone"),
    "us_nasams_medium": ("nasams", "us_desert_tan"),
}


def _round(x, y, rx, ry, n=16):
    return [(x + rx * math.cos(k * math.tau / n), y + ry * math.sin(k * math.tau / n)) for k in range(n)]


def pantsir(v):
    m, h = v.mats, v.hull
    rail_z = 1.10
    deck = chassis(v, [3.55, 2.05, -1.55, -3.00], 0.62, 0.42, 1.00, rail_z, "cabover", 3.10, 1.95, 3.00, tow=False)
    # The tow hook hangs under the rear bumper, its reach inside the frame.
    HF.tow_hook("rear_tow", (-v.length / 2 + TOW_REACH, 0, rail_z - 0.12 - TOW_SIZE * 0.55), m, h,
                size=TOW_SIZE, rot=(0, 0, math.pi))
    box("deck_plate", (6.5, 2.4, 0.10), (-1.75, 0, deck + 0.05), m["paint"], h, bevel=0.01)
    # The generator and crew compartment behind the cab: louvred, a door aft.
    SW.stowage_box("generator_box", (1.85, 0, deck), (1.40, 2.30, 1.20), m, h, rot=(0, 0, math.pi / 2))
    for side in (-1, 1):
        HF.grille(f"generator_louvre_{side}", (1.85, side * 1.16, deck + 0.65), (0.90, 0.60), m, h, slats=8,
                  rot=(-side * math.pi / 2, 0, 0))
    for side, s in ((1, "L"), (-1, "R")):
        SW.stowage_box(f"deck_box_{s}", (0.0, side * 1.0, deck + 0.10), (1.40, 0.36, 0.45), m, h,
                       rot=(0, 0, 0 if side > 0 else math.pi))
        box(f"jack_leg_{s}", (0.16, 0.16, 0.85), (-v.length / 2 + 0.55, side * 1.10, deck - 0.30), m["dark"], h,
            bevel=0.01)
        box(f"jack_pad_{s}", (0.36, 0.36, 0.06), (-v.length / 2 + 0.55, side * 1.10, deck - 0.75), m["dark"], h,
            lods=MID)
    mx = -1.6
    box("module_base", (2.0, 2.0, 0.40), (mx, 0, deck + 0.25), m["paint"], h, bevel=0.03)
    cyl("module_ring", 1.0, 0.15, (mx, 0, deck + 0.52), "Z", m["dark"], h, seg=32)
    loft("module_body", [(deck + 0.60, [(mx + 0.90, -0.80), (mx - 1.00, -0.80), (mx - 1.00, 0.80), (mx + 0.90, 0.80)]),
                         (deck + 1.80, [(mx + 0.70, -0.70), (mx - 0.95, -0.70), (mx - 0.95, 0.70),
                                        (mx + 0.70, 0.70)])], mat=m["paint"], parent=h, bevel=0.04)
    RF.hatch("module_hatch", (mx - 0.50, 0.30, deck + 1.80), m, h, radius=0.25)
    # The search radar's phased array on its pedestal over the module, face
    # rearward as it travels; the tracking radar's flat array on the front,
    # the electro-optical station beside it.
    box("search_radar_pedestal", (0.40, 0.40, 0.20), (mx - 0.40, 0, deck + 1.90), m["dark"], h, lods=MID)
    box("search_radar", (0.50, 1.50, 1.00), (mx - 0.40, 0, deck + 2.35), m["paint"], h, bevel=0.04)
    box("search_radar_face", (0.02, 1.40, 0.90), (mx - 0.66, 0, deck + 2.35), m["dark"], h, lods=MID)
    for k in range(5):
        box(f"search_radar_rib_{k}", (0.03, 0.03, 0.92), (mx - 0.68, -0.56 + k * 0.28, deck + 2.35), m["paint"], h,
            lods=FINE)
    box("tracking_radar", (0.20, 0.90, 0.70), (mx + 0.85, 0, deck + 1.30), m["paint"], h, bevel=0.03)
    box("tracking_radar_face", (0.02, 0.80, 0.60), (mx + 0.96, 0, deck + 1.30), m["dark"], h, lods=MID)
    RF.sight_housing("eo_station", (mx + 0.50, 0, deck + 1.80), m, h, size=(0.42, 0.40, 0.32))
    for side, s in ((1, "L"), (-1, "R")):
        # A twin 30 mm gun each side in its shroud, the ammunition feed box.
        box(f"gun_mount_{s}", (0.80, 0.30, 0.40), (mx + 0.20, side * 0.90, deck + 1.20), m["paint"], h, bevel=0.03)
        box(f"ammo_feed_{s}", (0.45, 0.28, 0.30), (mx - 0.40, side * 0.90, deck + 1.10), m["dark"], h, bevel=0.02,
            lods=MID)
        for k, dz in enumerate((-0.08, 0.08)):
            cyl(f"gun_barrel_{s}_{k}", 0.030, 2.10, (mx + 1.65, side * 0.90, deck + 1.20 + dz), "X", m["steel"], h,
                seg=10)
            cyl(f"gun_shroud_{s}_{k}", 0.055, 0.60, (mx + 0.85, side * 0.90, deck + 1.20 + dz), "X", m["dark"], h,
                seg=12, lods=MID)
            cyl(f"muzzle_{s}_{k}", 0.045, 0.14, (mx + 2.68, side * 0.90, deck + 1.20 + dz), "X", m["dark"], h,
                seg=10, lods=NEAR)
        # Six missile tubes a side outside the guns, two columns of three,
        # on their frame.
        for c in range(2):
            for i in range(3):
                y = side * (1.02 + c * 0.20)
                cyl(f"missile_tube_{s}_{c}_{i}", 0.09, 3.20, (mx + 0.10, y, deck + 0.95 + i * 0.24), "X",
                    m["paint"], h, seg=14, bevel=0.01)
                cyl(f"missile_cap_{s}_{c}_{i}", 0.075, 0.01, (mx + 1.71, y, deck + 0.95 + i * 0.24), "X",
                    m["dark"], h, seg=12, lods=MID)
        for k, x in enumerate((mx - 1.0, mx + 1.2)):
            box(f"missile_frame_{s}_{k}", (0.06, 0.40, 0.72), (x, side * 1.12, deck + 1.19), m["dark"], h, lods=MID)
    AR.slat_armour("module_ladder", (mx - 1.05, 0.0, deck + 0.10), (0.6, 1.4), m, h, spacing=0.25,
                   rot=(0, 0, math.pi / 2))


def skyranger(v):
    m, h = v.mats, v.hull
    roof = boxer.ROOF
    boxer.drive(v)
    x = -0.80
    cyl("turret_ring", 0.95, 0.10, (x, 0, roof + 0.05), "Z", m["dark"], h, seg=32, lods=MID)
    loft("turret_shell", [(roof + 0.08, _round(x, 0, 1.00, 0.95, 8)), (roof + 0.75, _round(x - 0.10, 0, 0.80, 0.78, 8))],
         mat=m["paint"], parent=h, bevel=0.04)
    # The four radar panels round its top, each in a frame on the slope.
    for k in range(4):
        a = k * math.pi / 2 + math.pi / 4
        box(f"radar_panel_{k}", (0.06, 0.55, 0.40), (x + 0.82 * math.cos(a), 0.82 * math.sin(a), roof + 0.55),
            m["dark"], h, bevel=0.01, rot=(0, 0.25, a))
        box(f"radar_frame_{k}", (0.05, 0.62, 0.47), (x + 0.79 * math.cos(a), 0.79 * math.sin(a), roof + 0.55),
            m["paint"], h, bevel=0.01, rot=(0, 0.25, a), lods=NEAR)
    # The 30 mm gun in its mantlet, the barrel's sleeve and muzzle sensor.
    box("gun_cradle", (0.80, 0.34, 0.34), (x + 0.75, 0.25, roof + 0.55), m["paint"], h, bevel=0.03)
    box("gun_mantlet", (0.12, 0.46, 0.42), (x + 1.10, 0.25, roof + 0.56), m["paint"], h, bevel=0.03)
    cyl("kce_sleeve", 0.06, 0.55, (x + 1.42, 0.25, roof + 0.58), "X", m["dark"], h, seg=14, lods=MID)
    cyl("kce_barrel", 0.035, 2.10, (x + 2.20, 0.25, roof + 0.58), "X", m["steel"], h, seg=12)
    cyl("kce_muzzle", 0.050, 0.18, (x + 3.25, 0.25, roof + 0.58), "X", m["dark"], h, seg=12, lods=NEAR)
    # The missile launcher on its arm off the turret's right flank.
    box("missile_arm", (0.30, 0.30, 0.15), (x + 0.10, -0.85, roof + 0.45), m["dark"], h, bevel=0.02, lods=MID)
    box("missile_launcher", (1.10, 0.40, 0.45), (x + 0.10, -1.05, roof + 0.55), m["paint"], h, bevel=0.03)
    for i in range(2):
        for j in range(2):
            cyl(f"missile_cap_{i}_{j}", 0.075, 0.01, (x + 0.66, -1.05 + (i - 0.5) * 0.19, roof + 0.45 + j * 0.20), "X",
                m["dark"], h, seg=12, lods=MID)
    RF.sight_housing("eo_sensor", (x - 0.20, 0.45, roof + 0.75), m, h, size=(0.34, 0.30, 0.30))
    RF.hatch("turret_hatch", (x - 0.45, -0.25, roof + 0.75), m, h, radius=0.24)
    for side in (-1, 1):
        RF.smoke_discharger_bank(f"smoke_{side}", (x - 0.55, side * 0.70, roof + 0.62), m, h, count=4,
                                 tube_radius=0.04, tube_length=0.18, elevation=0.5, spread=0.3,
                                 rot=(0, 0, math.pi + side * 0.8))
    whip = empty("dressing_antenna_turret", parent=h)
    RF.antenna("antenna_turret", (x - 0.70, 0.40, roof + 0.70), m, whip, height=1.8)


def nasams(v):
    """The launcher on its two-axle trailer."""
    m, h = v.mats, v.hull
    half = v.length / 2
    rail_z = 0.95
    for side, s in ((1, "L"), (-1, "R")):
        for k, x in enumerate((-0.55, -1.55)):
            RG.tyre_wheel(f"wheel_{s}_{k + 1}", (x, side * 0.95, 0.48), 0.48, 0.35, side, m, h, rim_radius=0.27,
                          tread="bar", hub_bolts=8)
            box(f"axle_{s}_{k}", (0.16, 0.50, 0.14), (x, side * 0.55, 0.48), m["dark"], h, lods=NEAR)
        # One mudguard over the pair, a mudflap behind it.
        box(f"fender_{s}", (2.20, 0.48, 0.04), (-1.05, side * 0.95, 1.02), m["paint"], h, bevel=0.01, lods=MID)
        box(f"fender_lip_{s}", (2.20, 0.03, 0.16), (-1.05, side * 1.19, 0.95), m["paint"], h, lods=NEAR)
        HF.mudflap(f"mudflap_{s}", (-2.20, side * 0.95, 0.95), (0.40, 0.45), m, h)
        box(f"frame_rail_{s}", (v.length - 0.9, 0.14, 0.24), (-0.35, side * 0.55, rail_z), m["dark"], h, bevel=0.01)
        # Levelling legs at the corners, folded up, their winding handles.
        for k, x in enumerate((half - 0.6, -half + 0.3)):
            box(f"leg_{s}_{k}", (0.14, 0.14, 0.80), (x, side * 1.05, rail_z + 0.05), m["dark"], h, lods=MID)
            box(f"leg_pad_{s}_{k}", (0.30, 0.30, 0.05), (x, side * 1.05, rail_z - 0.38), m["dark"], h, lods=MID)
            box(f"leg_crank_{s}_{k}", (0.03, 0.14, 0.03), (x, side * 1.05, rail_z + 0.47), m["steel"], h, lods=FINE)
    # The A-frame drawbar and its eye, the jockey wheel.
    for side in (-1, 1):
        box(f"drawbar_{side}", (1.5, 0.10, 0.10), (half - 0.85, side * 0.28, 0.80), m["dark"], h,
            rot=(0, 0.10, -side * 0.35))
    cyl("drawbar_eye", 0.10, 0.04, (half - 0.02, 0, 0.62), "Z", m["steel"], h, seg=12, lods=MID)
    jockey = VP.wheel_node("wheel_jockey", (half - 0.55, 0.15, 0.12), 0.12, h)
    cyl("jockey_tyre", 0.12, 0.08, (0, 0, 0), "Y", m["rubber"], jockey, seg=16, lods=MID)
    box("jockey_leg", (0.06, 0.06, 0.60), (half - 0.55, 0.15, 0.50), m["steel"], h, lods=MID)
    box("deck_plate", (v.length - 1.2, 2.0, 0.10), (-0.40, 0, rail_z + 0.17), m["paint"], h, bevel=0.01)
    cyl("turntable", 0.65, 0.22, (-0.30, 0, rail_z + 0.33), "Z", m["dark"], h, seg=28)
    box("launcher_cradle", (1.0, 1.4, 0.30), (-0.30, 0, rail_z + 0.58), m["paint"], h, bevel=0.03)
    for side in (-1, 1):
        box(f"launcher_arm_{side}", (2.2, 0.08, 0.45), (-0.70, side * 1.10, rail_z + 1.20), m["paint"], h,
            bevel=0.015)
    # Six canisters, two rows of three: ribbed, caps forward, lugs on top.
    for j in range(2):
        for i in range(3):
            y = (i - 1) * 0.68
            z = rail_z + 0.98 + j * 0.62
            box(f"canister_{j}_{i}", (4.3, 0.62, 0.58), (-0.70, y, z), m["paint"], h, bevel=0.03)
            box(f"canister_cap_{j}_{i}", (0.03, 0.52, 0.50), (1.46, y, z), m["dark"], h, lods=MID)
            box(f"canister_end_{j}_{i}", (0.03, 0.52, 0.50), (-2.86, y, z), m["dark"], h, lods=MID)
            for k in range(3):
                box(f"canister_rib_{j}_{i}_{k}", (0.06, 0.64, 0.60), (-2.40 + k * 1.70, y, z), m["paint"], h,
                    lods=NEAR)
    SW.stowage_box("electronics_box", (half - 1.2, 0, rail_z + 0.22), (0.70, 1.40, 0.55), m, h)
    cyl("cable_reel", 0.22, 0.30, (half - 1.2, -0.95, rail_z + 0.30), "Y", m["dark"], h, seg=16, lods=MID)
    whip = empty("dressing_antenna", parent=h)
    RF.antenna("antenna", (half - 1.40, 0.60, rail_z + 0.77), m, whip, height=1.6)
    for side in (-1, 1):
        HF.light_with_guard(f"tail_light_{side}", (-half + 0.02, side * 0.95, rail_z), 0.05, dict(m, lamp=m["tail"]),
                            h, rot=(0, 0, math.pi))


# The tracked cards' bodies (`scripted_body`): each its own Argon build,
# its track's link pitch.
BODIES = {
    "europe_gepard_1a2": ("assets/source/skins/gepard/argon-r2/build.py", 0.16),
    "eastern_buk_buk_m3_medium_range_air_defense": ("assets/source/skins/buk/argon-r2/build.py", 0.15),
}


def build(variant, v):
    if variant["id"] == "europe_gepard_1a2":
        turret = rig(v.frame, v.root)["gun"][0]
        SB.wear(v, BODIES["europe_gepard_1a2"][0], BODIES["europe_gepard_1a2"][1])
        # The whips on the turret roof's rear corners (turret frame).
        for side, s in ((1, "L"), (-1, "R")):
            whip = empty(f"dressing_antenna_{s}", parent=turret)
            RF.antenna(f"antenna_{s}", (-1.40, side * 0.75, 0.89), v.mats, whip, height=2.0)
        return
    if variant["id"] in BODIES:
        path, pitch = BODIES[variant["id"]]
        return SB.wear(v, path, pitch)
    {"eastern_pantsir_pantsir_sm": pantsir, "europe_boxer_skyranger_30_gun_and_missile_air_defense": skyranger,
     "europe_nasams_nasams_3": nasams, "us_nasams_medium": nasams}[variant["id"]](v)


def wreck(variant, v):
    """An air defence vehicle after its fire: the right side's running gear
    torn (a track off, or wheels gone and the hull down on that side), the
    radar and canisters burst, plates and the module warped; what it
    throws is `wreckage.scatter`'s."""
    from parts import rest_on_ground
    from wreckage import densify, dent, heat, parts, remove, warp
    if variant["id"] in BODIES:
        return SB.wreck(v, "R", (2, 3))
    remove("track_R_band", "wheel_R_1_", "wheel_R_2_", "skirt_R", "search_radar", "canister_cap_", "missile_cap_",
           "missile_tube_R_1_2", "missile_tube_R_0_2", "canister_1_2", "canister_rib_1_2", "radar_dome", "side_bin_R_",
           "mudflap_R", "windscreen", "door_window_", "rear_window_", "dressing_mirror_R", "mirror_R", "mirror_arm_R",
           "basket_tarp", "radar_panel_1", "radar_frame_1")
    shell = parts("hull_upper", "hull_lower", "turret_shell", "boxer_upper", "boxer_lower", "module_body", "cab_body",
                  "radar_housing", "canister_")
    densify(shell, scale=2.0)
    warp(shell, heat(0.025, 0.8, seed=81.0), dent((0.0, -1.2, 1.6), 0.6, 0.12, (0, 1, -0.2)))
    v.root.rotation_euler = (0.03, 0.0, 0)
    v.root.location.z -= 0.05
    rest_on_ground(0.004)


if __name__ == "__main__":
    run("gepard", {c: scheme for c, (_, scheme) in CARDS.items()}, build, wreck,
        appearances=list(CARDS), script="air_defence.py",
        receipt_file="assets/source/roster/gepard/air_defence.source-receipt.json",
        references=[f"assets/references/{family}/references.json" for family, _ in CARDS.values()], chip=1.0)
