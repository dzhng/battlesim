# 34c — Sim: destroyable props

**Status:** done (one integrity store for every prop kind with `hp`; direct rounds by armour and bursts by distance; trees fall to cleared ground, crates and fences are removed, sandbags and walls become rubble, wrecks a lighter wreck, buildings their ruin; sides learn destructions by sight; the renderer reads `destroyablePropKinds`; trunks block infantry; drawn fog follows known cleared ground; `choices.md` and `decisions.md`, slice 34c). **Depends on:** 34b. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html) (Q17).

## Contract

Every prop kind can take damage and be destroyed (user 2026-09-26). This generalises today's building path (`damage::Outcome.structural` → `structures.damage` → `garrison::collapse`, `battle.rs:1025`) from buildings to one table for all kinds. A tree that takes enough fire, from blasts and rounds around it, is knocked down, and its spot stops being forest, exactly like a lane a tank clears (34b). Other kinds break into their destroyed state.

## API seam

- **One owner.** `structures` becomes the single store of prop integrity for every destroyable prop, and buildings are one row of it. The renderer's `FALLIBLE_KINDS` (`worldMesh.ts:65`) is deleted; it reads what can be destroyed from the body table (Q19).
- **Fixture.** `props.<kind>.{hp, destroyed}`, where `destroyed` is `removed`, `cleared` (for trees: the spot becomes open ground via 34b's `cleared` channel), or another prop kind (a building → `ruin`; sandbags → a low `rubble` that is light cover and blocks nothing).
- **Damage sources**, all through the existing damage outcome:
  - blast overpressure within the radius, scaled by distance;
  - a direct round hitting a prop collider, scaled by the prop kind's armour factor.
- **Knowledge.** A destruction a side saw, or learns by contact, updates its known props through the `replaces` mechanism ruins already use. An unseen destruction stays unseen (L1).
- **Consequences.** Navigation, cover and fog revisions bump, and soldiers re-resolve cover (D5) when theirs is destroyed.
- The digest covers every prop's integrity.

## Proposed default table (edit in the fixture)

| Kind | Destroyable | Destroyed state |
|---|---|---|
| Tree (trunk) | yes, by heavy fire nearby | cleared: open ground |
| Crate | yes, easily | removed |
| Fence | yes, easily | removed |
| Sandbags | yes | rubble: light cover, blocks nothing |
| Field wall | yes, by heavy fire | rubble |
| Building | yes (today's rule) | ruin (today's rule) |
| Wreck | yes, by heavy fire | a smaller burnt wreck, one weight class lighter |
| Anti-tank wall | no, by ordinary fire (Q18) | — |
| Ruin, bridge deck, trench | no | — |

## Verification

- Scenario runner, reviewed by the agent:
  - an artillery barrage clears a patch of forest, which then reads as open ground;
  - sandbags are shot to rubble, and the squad behind them re-resolves cover.
- Native tests:
  - damage from each source per kind;
  - destroyed-state swaps;
  - an unseen destruction is not learned (metamorphic);
  - revisions bump and cover re-resolves;
  - digest and replay parity.
- Paired village and endurance reports, logged.

## Decision budget

- **Delegated:** the hp and armour numbers, within the fixture.
- **Not delegated:** the table's shape, and the rules above.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test (garrison collapse included). `bun run check` and `bun run verify` at closeout. Record frame cost (the benchmark's short run) and the endurance per-tick time against Q12's soft target.
