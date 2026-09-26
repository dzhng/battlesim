# 33 — Sim: cover tiers and seeking cover

**Status:** planned. **Depends on:** 32. **Lane:** simulation. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html).

## Contract

Soldiers take cover the moment an order is given (D4) and keep it current (D5, Q11). Cover comes in three tiers (D6), applied **only through incoming scatter** (Q5), and **for infantry only**, a hard-coded game rule (Q20); crouching and prone are animation only. When a round is aimed at a soldier, the strongest cover body within about 1.5 m of him that lies between him and the shooter widens that round's scatter by its tier. Trunks count as bodies like any prop. **Vehicles are cover, live or wrecked, consistently (Q24):** a vehicle's cover tier follows its weight class (jeep light, truck medium, tank heavy), a wreck keeps it, and a covering vehicle moving more than ~2 m re-resolves its users' cover. The forest-rect formula (`FOREST_EDGE_COVER + depth/50`) and the crater cap (`ground.rs:89`) are deleted; craters and trenches are ground cover with a tier. Garrisons keep their slot shelter as the one named exception (Q22). A soldier whose line is blocked steps up to 4 m to a clear spot, or sits out (D3, Q8). An attack-move that halts on contact resolves cover facing the enemy (Q9).

## API seam

- `cover::Tier {Light, Medium, Heavy}` and `cover::spots(world, knowledge, threat)`. A spot sits behind a cover face, along its normal, away from the threat (mock lesson), and counts only if the cover lies between the soldier and the threat (Q7).
- Resolving at the order: soldiers claim the best-tier spots near the click; whoever finds none scatters randomly (D4). The resolved spots are stored on the order, which is what slice 35 publishes.
- Re-resolving: on arrival, on a revision change within the cover radius, or when the threat swings more than 45°; at most once a second per squad (Q11).
- The fixture gains `cover.tiers {light, medium, heavy}` spread multipliers, and the Q4 prop and ground tier table (crater light, trench heavy, and so on).

## Verification

- Runner scenarios t1 reviewed: into cover, not enough cover, attack-move halt, cover destroyed mid-fight.
- Native tests: spots are on the far side; the tier multiplier applies; step-out finds the corner exit; re-resolve is throttled; replay parity.
- Paired reports; the balance shift is expected (L9) and goes to 27.

- Verify with the slice-30 scenario runner: add this slice's named scenarios, run them, and **review the GIFs yourself** before accepting. Look for jams, lanes through obstacles, cover on the wrong side and twitching. The user looks last, through a non-blocking preview-shots checkpoint.

## Decision budget

- **Delegated:** spot scoring beyond the tier and the threat side.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (with the new state in the digest). Every existing scene and test, except checks this slice deliberately changes, each recorded in `decisions.md`. `bun run check` and `bun run verify` at closeout. Record frame cost in [`frame-cost.md`](../frame-cost.md) (the benchmark's short run) and the endurance report's per-tick time; the soft target is Q12's.
