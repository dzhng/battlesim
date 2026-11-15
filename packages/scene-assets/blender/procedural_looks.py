"""The procedural cloth shells' look tables, until every look wears a generated body:
`roster/infantry_equipment.py` builds a kit's materials from them. Slice 08 of
specs/generated-soldiers deletes this module with the shells."""
from infantry_kit import COYOTE, RANGER
from textures import UNIFORMS
import infantry_roster as roster

# What the procedural shells still read of an army's look, named as its
# reference variant (assets/references/<kit>/): the uniform print
# (textures.UNIFORMS), whether carrier, pouches and helmet cover are printed,
# helmet, boots, webbing, and per look a, b, c what differs. An army's faction,
# gloves and weapons are its data (`armies.json`). This table goes once every
# look wears a generated body (specs/generated-soldiers, slice 08).
PROCEDURAL_LOOKS = {
    "us_army_ocp": dict(
        print=UNIFORMS["us_army_ocp"], printed_gear=True, vest="print", pouch="print",
        straps=COYOTE, accent="print", helmet="ach", boots=(0.075, 0.056, 0.036),
        variants={"a": dict(head="nvg"), "b": dict(head="scrim", scarf=False), "c": dict(head="cover", pack="hydration")},
    ),
    # Marines wear MARPAT woodland under coyote nylon (Plate Carrier Gen III,
    # pouches and packs), a woodland helmet cover and coyote boots.
    "usmc_marpat": dict(
        print=UNIFORMS["usmc_marpat"], printed_gear=False, printed_cover=True, vest=COYOTE,
        pouch=COYOTE, straps=COYOTE, accent=COYOTE, helmet="ach", boots=(0.075, 0.056, 0.036),
        variants={"a": dict(head="nvg"), "b": dict(head="cover", scarf=False, eyewear=True),
                  "c": dict(head="cover", pack="hydration", vest=RANGER)},
    ),
    # Europe's shared kits wear the Bundeswehr's
    # (specs/done/unit-models/choices.md): Flecktarn, a Flecktarn carrier and
    # pouches, a covered helmet, black boots and olive webbing.
    "german_flecktarn": dict(
        print=UNIFORMS["german_flecktarn"], printed_gear=True, vest="print", pouch="print",
        straps=(0.055, 0.062, 0.038), accent="print", helmet="ach", boots=(0.018, 0.017, 0.015),
        variants={"a": dict(head="nvg", eyewear=False), "b": dict(head="cover", scarf=False),
                  "c": dict(head="cover", pack="hydration", accent=COYOTE)},
    ),
    "eastern_emr": dict(
        print=UNIFORMS["eastern_emr"], printed_gear=True, vest="print", pouch="print",
        straps=(0.07, 0.078, 0.048), accent="print", helmet="6b47", rails=False, headset=False, collar=True, groin=True,
        boots=(0.018, 0.017, 0.015),
        variants={"a": dict(head="cover", eyewear=False, balaclava=True, knee_pads=False),
                  "b": dict(head="cover", eyewear=False, balaclava=True, scarf=False, pack="none"),
                  "c": dict(head="cover", eyewear=False, radio=False, pack="hydration")},
    ),
}


def army_look(army):
    """An army's look as `Kit` reads it: its procedural shells' table with the
    army's gloves and weapons (`armies.json`). An army with no shells (generated
    only) borrows its faction's first table for the kit's materials; it always
    wears a pinned body."""
    armies = roster.armies()
    data = armies[army]
    shells = PROCEDURAL_LOOKS.get(army) or next(
        PROCEDURAL_LOOKS[a] for a in PROCEDURAL_LOOKS if armies[a]["faction"] == data["faction"])
    return {**shells, "gloves": tuple(data["gloves"]), "rifle": data["rifle"], "lmg": data.get("lmg")}
