# 02 Roster stands alone

**Unlocks:** the roster resolves without `fixtures/units/generic/`.

Today roster soldier chains reach generic soldier kinds: `rifleman` (via
`roster_assault_rifle`, `rifle_squad`), `grenadier` (`rifle_squad`, US Force
Recon), `scout` (all four scout/recon patrols), `at_rifleman` (TOW, Kornet,
RPG-7, RPG-29 teams, guard and gunner). Abstract roster bodies
(`roster_*_body`, `roster_{tow,kornet,rpg_light,rpg_heavy}_gunner`) still
name generic appearances (`rifle*`, `at*`), though no card wears them.

## Work

1. **Test first:** resolve the catalog from roster, profiles, roles and props
   only (no `generic`); it fails today naming the missing parents.
2. Add roster base soldier kinds (`roster_rifleman`, `roster_grenadier`,
   `roster_scout`, `roster_at_rifleman`) in the roster's own files carrying
   exactly the inherited values; repoint the roster chains at them. Drop the
   generic appearances from abstract roster bodies (cards already override).
3. **Proof of no change:** every card entry, and every soldier a card's squad
   slots, in the resolved `fixtures/catalog.json` is identical before and
   after (a script diff of the card-reachable subset, recorded in choices.md).
   Digests unchanged: run one skirmish parity fixture
   (`fixtures/parity/skirmish-purchases.json`) to confirm.

## Delegated

Base kind names; which file holds them.
