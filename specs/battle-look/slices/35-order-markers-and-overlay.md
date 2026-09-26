# 35 — Order markers and the Space overlay

**Status:** planned. **Depends on:** 33, 34, 39. **Lane:** controls. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Total War-style markers (D2): every ground unit shows its final position and facing on the ground. **Holding Space (D2+)** shows, for every own unit and soldier: a marker under his *current* position, the route lines, and the final marker with its facing. Under each soldier's marker there is a **cover icon**: yellow for light, light green for medium, dark green for heavy, nothing for none. At the current marker it shows the cover he has now; at the final marker, the cover that spot gives. Right-drag sets facing (Q9). A reversing unit's final marker shows its held facing with a reverse indicator (Q31).

## API seam

- Publication (own units only): each soldier's and vehicle's resolved destination, facing, and current route waypoints, sent as deltas when they change, plus **each soldier's current cover tier and his destination's cover tier**.
- `web/src/battle/input`: right-drag facing in `CommandBindings`; Space held in `heldKeys`.
- Overlay drawing composites after post (the overlay rule), and it is readable over fog.

## Verification

- Scene: markers match the sim's resolved spots; Space shows routes equal to the published ones; nothing of the enemy's plan is published (metamorphic).
- Unprimed critique of the overlay.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** marker styling within the overlay palette.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
