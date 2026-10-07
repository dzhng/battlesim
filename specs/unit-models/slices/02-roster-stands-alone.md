# 02 Roster stands alone

**Unlocks:** the roster resolves without the test units.

Today roster soldier chains reach generic soldier kinds: `rifleman` (via
`roster_assault_rifle`, `rifle_squad`), `grenadier` (`rifle_squad`, US Force
Recon), `scout` (all four scout and recon patrols), `at_rifleman` (TOW,
Kornet, RPG-7, RPG-29 teams, guard and gunner). 13 roster intermediates are
concrete though no card reaches them (`roster_assault_rifle`,
`roster_{tow,kornet,rpg_light,rpg_heavy}_gunner`,
`roster_marksman_rifle_1..6`, `roster_heavy_sniper_1/2`), and some name generic
appearances (`rifle*`, `at*`) that no card wears.

## Work (decision 8)

1. **Test first:** resolve the catalog from roster, profiles, roles and props
   only (no generic or test documents); it fails today naming the missing parents.
2. Add `abstract` roster base kinds (`roster_rifleman`, `roster_grenadier`,
   `roster_scout`, `roster_at_rifleman`) in the roster's own files, carrying
   exactly what the chains inherit today: hp 100, the name and description
   strings, and the `rifles` mount `{weapons: [rifle], squad: true}` **as the
   first mount** (mount order is the publication's and the operator's mount
   index; `sounds.json` keys by mount name). Repoint the chains. Make the 13
   intermediates `abstract`; drop the generic appearances from them.
3. **Proof of no change:** a script diff of the resolved `fixtures/catalog.json`
   before and after, restricted to cards and every soldier a card's squad
   slots: identical. Recorded in choices.md. (Not the skirmish parity file:
   `fixtures/parity/skirmish-purchases.json` carries its own frozen catalog, so
   it can't see this change.)
4. `Rules` serialises every resolved soldier kind into `config_digest`, so
   roster battle digests move even with identical cards. Expected; regenerate
   and name it.

## Delegated

Base kind names; which roster file holds them.
