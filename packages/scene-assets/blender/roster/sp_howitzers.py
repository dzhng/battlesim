"""Tracked 155/152 mm self-propelled howitzers: M109A7 Paladin,
PzH 2000, 2S19M2 Msta-S and PLZ-05, from assets/references/<family>/.

    bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/sp_howitzers.py -- [--variant=<card id>] [--wreck]

Each wears a scripted body (`scripted_body`): Gemini Argon's build script
after its second round, edited since (each folder's README lists the
edits), under `assets/source/skins/<family>/argon-r2/`. This script adds
the rig, the whip antennas and the wreck. Each gun is a mount fixed in the
hull that the gun rig draws (`assets/catalog.json`): its pivot is the
trunnion, so the turret never turns and the barrel pitches on the frame's
pitch node.

What the photos settle, shared: a tracked hull with the engine and driver at
the front (the 2S19's T-80 hull has its engine at the rear), a big slab-sided
turret over the rear, its long gun travelling level over the glacis on a
folding travel lock, a cupola with a machine gun, smoke banks, stowage boxes
on the turret's flanks and a rear door into the turret.

- M109A7 (desert tan, Bull Battery and PIM photos): six road wheels (the
  Bradley's), the boxy turret almost the hull's width, the M284's bore
  evacuator and double-baffle brake, the M2 on the commander's cupola.
- PzH 2000 (German three-tone, Hungarian, Croatian and Munster photos):
  seven road wheels (the Leopard's), the turret's angular front and flat
  sides, the 52-calibre gun's pepperpot brake, no evacuator.
- 2S19M2 (Russian green, Oboronexpo 2014): six road wheels, the T-80 hull
  with its engine at the rear, the tall turret centred, the 2A64's fume
  extractor and brake, the remote MG over the commander's hatch, the
  dozer blade under the nose.
- PLZ-05 (PLA digital, Beijing museum and parade): seven road wheels, the
  turret long and low at the rear, the 52-calibre gun with fume extractor
  and multi-baffle brake.

Each card's frame is the hull's length, width and height stated from its
published dimensions (the gun excluded), recorded in its receipt as
`references`.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
import roof_parts as RF  # noqa: E402
import scripted_body as SB  # noqa: E402
from parts import empty  # noqa: E402
from vehicle_export import rig, run  # noqa: E402

# Each card: its source family and scheme, its track's link pitch and the
# whips' feet on its turret roof (hull frame).
SPECS = {
    "us_m109_paladin_m109a7": dict(family="m109_paladin", scheme="us_desert_tan", pitch=0.15,
                                   whips=((-2.95, 1.35, 2.80), (-2.95, -1.35, 2.80))),
    "europe_pzh_2000_tracked_155_mm_howitzer": dict(family="pzh_2000", scheme="german_three_tone", pitch=0.16,
                                                    whips=((-3.70, 1.20, 3.06), (-3.70, -1.20, 3.06))),
    "eastern_2s19_msta_s_2s19m2": dict(family="2s19_msta_s", scheme="russian_green", pitch=0.15,
                                       whips=((-2.10, 1.30, 2.82), (-2.10, -1.30, 2.82))),
    "eastern_plz_05_chinese_tracked_155_mm_howitzer": dict(family="plz_05", scheme="chinese_digital", pitch=0.15,
                                                           whips=((-3.10, 1.25, 3.20), (-3.10, -1.25, 3.20))),
}


def build(variant, v):
    s = SPECS[variant["id"]]
    rig(v.frame, v.root)
    SB.wear(v, f"assets/source/skins/{s['family']}/argon-r2/build.py", s["pitch"])
    for k, at in enumerate(s["whips"]):
        whip = empty(f"dressing_antenna_{k}", parent=v.hull)
        RF.antenna(f"antenna_{k}", at, v.mats, whip, height=2.2)


def wreck(variant, v):
    """A howitzer after its ammunition burned: the left track off with two
    road wheels gone, plates warped, the turret heaved (`scripted_body.wreck`)."""
    SB.wreck(v, "L", (2, 3))


if __name__ == "__main__":
    run("m109_paladin", {card: s["scheme"] for card, s in SPECS.items()}, build, wreck,
        appearances=list(SPECS), script="sp_howitzers.py",
        receipt_file="assets/source/roster/m109_paladin/sp_howitzers.source-receipt.json",
        references=[f"assets/references/{s['family']}/references.json" for s in SPECS.values()], chip=1.0)
