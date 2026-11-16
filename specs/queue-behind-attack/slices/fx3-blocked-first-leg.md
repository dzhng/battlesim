# FX3 — a blocked unit's first queued leg is drawn (existing bug)

**Unlocks.** Chains that start without a destination circle (S8, S9 build on it).
`queuedLegs` starts from `dest`, which is null for a blocked unit, so its first leg has
zero length (`routeBetween` with no start circle).

**Seam.** `packages/battle-renderer/src/orderOverlay.ts::queuedLegs`: the first leg
leaves the blocked mark at `u.goal` when there is no destination circle. Jets keep
today's drawing (F3).

**Test first.** `web/tests/queuedLegs.test.ts` (new): a blocked `OrderView` with one
queued point draws dashed geometry spanning goal → queued ring (sample mesh vertices
along the segment; never colours or names). A fixed-wing view's mesh is unchanged.

**Visual.** One variable: a blocked unit's first queued leg; crop that unit and its
queue with Space held. Run `screenshot-critique` unprimed as the last check, then
`compare-screenshots` before against after, then show the user inline
(`preview-shots` is the procedure; the user prefers pictures in the chat). The human
checkpoint never blocks: give ~5 minutes, then decide on the evidence and record it.
`journey-space-orders.png` stays matched.

**Commit.** `fix(overlay): a blocked unit's first queued leg is drawn`.

**State.** Done. `web/tests/queuedLegs.test.ts` failed before the fix and passes after.
The picture ([assets/fx3-blocked-first-leg-before-after.png](../assets/fx3-blocked-first-leg-before-after.png),
before left, after right) differs only by the dashed leg from the blocked ring to the
queued point (pixel diff). `journey-space-orders` differs from its baseline only in the
top score bar, on the base commit too: the baseline predates staging's score meter and
needs a person to re-bless it.
