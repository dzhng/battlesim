# S9 — the queued chain, each step drawn as when current (decisions 5, 13, 18)

**Seam.** `orderSteps.ts`: walk `steps[1..]` from where the current step ends (the
reticle for an attack, the destination circle for a move) and call the same per-kind
drawer as the current order, dashed at `queued_alpha`: move = leg + ring; attack =
dashed red line + reticle; garrison = leg + the building's ring; board = leg to the
carrier; unload = the unload mark; exit = leg to the building; return to base = leg to
the map-edge exit. No new marks beyond the reticle. Jets keep today's chain (F3).

**Tests.** `web/tests/orderChain.test.ts`: a step list of every kind draws one continuous
chain (each leg starts where the previous step's mark ends); current and queued drawers
of one kind give the same geometry apart from dashing and alpha.

**Visual.** One variable: the queued chain. Capture the decision-17 ground unit's
six-step chain in the Space view in a real battle — part 1's visual proof (decision 12).
`screenshot-critique` unprimed as the last check, `compare-screenshots` before/after,
show the user inline; non-blocking.

**Commit.** `feat(overlay): queued orders chain through their targets`.

**State.** Done for the chains this branch admits. `web/tests/orderChain.test.ts` failed
before (the chain skipped every step but moves) and passes. Evidence:
[a garrison chain before/after](../assets/s9-garrison-chain-before-after.png) (move,
garrison, exit, move, attack; before, the chain skipped the garrison and exit),
[the same at full size](../assets/s9-garrison-chain.png), and
[a chain of queued attacks](../assets/s9-attack-chain.png) (move, attack a tank, attack
the ground, attack a squad). Queued legs keep today's queued strength, faint at normal
size; named for the human checkpoint.

TODO for the parent, after S2 admits a move behind an attack: capture decision 17's
ground-unit chain (attack → move → attack the same enemy → move → attack another enemy →
return to base) in the Space view of a real skirmish, part 1's visual proof (decision 12).
