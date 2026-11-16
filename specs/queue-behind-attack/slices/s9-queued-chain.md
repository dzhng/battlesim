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
