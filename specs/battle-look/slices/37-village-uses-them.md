# 37 — The village uses the new rules

**Status:** done (2026-09-27). Its two trenches were removed with the trench on 2026-09-27 ([`decisions.md`](../decisions.md)). **Depends on:** 34b, 34c, 35, 36. **Lane:** battle. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Give each village forest a density (34b), and place sandbags, fences, trenches, anti-tank walls and jeeps in `fixtures/village.json`, so the encounter exercises cover and pushing. Record the balance shift for slice 27's rebalance.

## API seam

- Fixture map edits only; no rule changes.

## Verification

- Paired village reports, logged; `/battle/village` screenshots reviewed; the runner's village scenario GIF reviewed.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** placement within the encounter's intent.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
