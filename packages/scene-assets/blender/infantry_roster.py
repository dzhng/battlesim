"""The infantry roster as data: which kits exist, which armies each is built
in, the armies themselves (`roster/armies.json`), and where each kit look's source
lives. No Blender: plain `python3` reads it, and so do the exporters.
"""
import functools
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "../../.."))

# Each roster kit (a directory of assets/source/roster/infantry/): its
# weapon (the army's squad rifle, or a `LENGTHS` model of the exporter), whether
# its men are scouts (the recon kind), and the armies it is built in, its
# default first: the faction its roster card names first, and a look for every
# other army that fields it.
KITS = {
    "rifle_squad": dict(weapon="rifle", armies=("us_army_ocp", "eastern_emr", "german_flecktarn")),
    "assault_squad": dict(weapon="rifle", armies=("german_flecktarn", "eastern_emr")),
    "us_marine_squad_close_quarters_infantry": dict(weapon="rifle", armies=("usmc_marpat",)),
    "us_army_scouts_light_patrol": dict(weapon="rifle", scout=True, armies=("us_army_ocp",)),
    "us_force_recon_recon_patrol": dict(weapon="rifle", scout=True, armies=("usmc_marpat",)),
    "eastern_scouts_light_patrol": dict(weapon="rifle", scout=True, armies=("eastern_emr",)),
    "europe_recon_patrol_light_patrol": dict(weapon="rifle", scout=True, armies=("german_flecktarn",)),
    "us_marksman_squad_m110_equipped_longer_range_infantry": dict(weapon="m110", armies=("us_army_ocp",)),
    "eastern_marksman_squad_svd_equipped_longer_range_infantry": dict(weapon="svd", armies=("eastern_emr",)),
    "europe_marksman_squad_longer_range_infantry": dict(weapon="g28", armies=("german_flecktarn",)),
    "us_scout_snipers_m107_heavy_sniper_team": dict(weapon="m107", scout=True, armies=("usmc_marpat",)),
    "eastern_sniper_team_asvk_heavy_sniper_team": dict(weapon="asvk", scout=True, armies=("eastern_emr",)),
    "us_atgm_team_bgm_71_tow_2a": dict(weapon="tow", armies=("us_army_ocp",)),
    "eastern_atgm_team_kornet": dict(weapon="kornet", armies=("eastern_emr",)),
    "eastern_rpg_team_rpg_7": dict(weapon="rpg7", armies=("eastern_emr",)),
    "eastern_rpg_team_rpg_29": dict(weapon="rpg29", armies=("eastern_emr",)),
    # The FN-6 team wears the Eastern army's EMR, as every Eastern kit does;
    # there is no Chinese uniform print yet. The Akeron is French, but Europe's
    # kits all wear the Bundeswehr's look; there is no French uniform print.
    "fgm_148_javelin_team": dict(weapon="javelin", armies=("us_army_ocp", "german_flecktarn")),
    "fim_92_stinger_team": dict(weapon="stinger", armies=("us_army_ocp", "german_flecktarn")),
    "eastern_air_defense_team_igla_s": dict(weapon="igla", armies=("eastern_emr",)),
    "eastern_air_defense_team_fn_6": dict(weapon="fn6", armies=("eastern_emr",)),
    "europe_atgm_team_akeron_mp": dict(weapon="akeron", armies=("german_flecktarn",)),
    # A squad's machine gunner and, in the Marines' MARPAT, its machine gunner
    # and marksman: each army's light machine gun (`armies.json` lmg), the M110.
    "machine_gunner": dict(weapon="lmg", armies=("us_army_ocp", "eastern_emr", "german_flecktarn")),
    "us_marine_machine_gunner": dict(weapon="lmg", armies=("usmc_marpat",)),
    "us_marine_marksman": dict(weapon="m110", armies=("usmc_marpat",)),
}

@functools.cache
def data():
    """`roster/armies.json`: the armies, the body kinds' cues and the studio frame."""
    with open(os.path.join(HERE, "roster", "armies.json")) as f:
        return json.load(f)


def armies():
    """Every army, by id."""
    return data()["armies"]


def kind_of(kit):
    """The body kind a kit's men wear: `recon` for scouts, else `rifle`."""
    return "recon" if KITS[kit].get("scout") else "rifle"


def source_path(kit, look, mode, army=None):
    """A kit look's source: `<mode>_<look>.glb` for its first army,
    `<army>_<mode>_<look>.glb` for any other."""
    first = KITS[kit]["armies"][0]
    prefix = "" if army is None or army == first else army + "_"
    return os.path.join(REPO, "assets/source/roster/infantry", kit, f"{prefix}{mode}_{look}.glb")


def crew_source(army):
    """The source a vehicle's crew are cut from: the army's crew kit
    (`armies.json` `crew_kit`), look a, active."""
    return source_path(armies()[army]["crew_kit"], "a", "active", army)
