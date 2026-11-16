# S5 — a jet's attack ends when its side loses the target (decision f; named digest decision)

**Seam.** `air/orders.rs::task`: an attack on a target the side no longer knows
(`knows_target` false) returns `Hold`, which pops it; the jet circles where it was.
Today the jet aims at its own position forever. Separate from S4 because it changes
behaviour with nothing queued.

**Test.** `order_queue.rs`: the jet's target is lost (out of sight), the jet circles, and
a queued move then runs.

**Digest.** Run the sim digest and replay tests; a move is this commit's named decision.

**Commit.** `feat(sim): a jet's attack ends when its side loses the target`.
