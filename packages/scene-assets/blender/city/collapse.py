"""How a destroyed building ends: the simulation's rule, read from the catalog row that owns it.

A building of `MAX_FLOORS` floors or fewer collapses: each of its parts is replaced
by remains on the same plan, as tall as `ruin_height` says (a share of the
building's height, between a least and a most). A taller one stands, gutted, at
its full height. A template has the one damage state its floors call for
(`damage_state`), and a ruin's art is held to the remains.

The numbers are the building prop type's own (`fixtures/props/generic/structures.json`,
`props.building.destroyed.into`); `templateSource.ts` reads the same row through
the unit catalog. Nothing here needs Blender.
"""
import json
import os

_FIXTURE = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../../../fixtures/props/generic/structures.json")
_INTO = json.load(open(_FIXTURE))["props"]["building"]["destroyed"]["into"]
MIN_HEIGHT_M = float(_INTO["height_m"])
HEIGHT_FRACTION = float(_INTO["building"]["height_fraction"])
MAX_HEIGHT_M = float(_INTO["building"]["max_height_m"])
MAX_FLOORS = int(_INTO["building"]["collapse_max_floors"])


def damage_state(floors):
    """The state a destroyed building of `floors` floors is known in: "ruin" or "gutted"."""
    return "ruin" if floors <= MAX_FLOORS else "gutted"


def ruin_height(building_height_m):
    """How tall the remains of every part are, for a building this tall (its highest part's top)."""
    return min(max(building_height_m * HEIGHT_FRACTION, MIN_HEIGHT_M), MAX_HEIGHT_M)
