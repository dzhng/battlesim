# C64: one forest rule

**Depends on:** G0. **Kind:** slice.

## Question
Do all forests follow one density and canopy height (Q-G8b)?

## Contract it unlocks
- `forests.densities` collapses to **one row** (today's `medium`: trunk spacing 9 m, its concealment and attenuation, canopy radius 6.5 m). `Forest.density` is removed from the contract.
- The village's `light` forest becomes the one rule.
- **Named village digest change.**

## API seam
`contract::map` (`Forest`), `fixtures/village.json`, `sim::world::forest`.

## What the human can run or see
A GIF of a squad moving through the former light wood.

## Verification
- Run tweak-mechanics first (a short pass: the east wood now hides a squad slightly better).
- `village_report -- --quick --compare main`.
- Native tests updated.

## Delegated to the implementer
None. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Forest concealment semantics.
