# S8 — the current attack line, reticle and chase-end reticle (ground units)

**Unlocks.** Decisions 5, 8, 9, 10, 13 for the current order. Load `game-ui` and
`renderer` before writing.

**Seams.**
- `packages/battle-renderer/src/reticleMark.ts` (new): four corner brackets, a centre
  cross for a ground point; one mark for both cases.
- `OrderStyle.attack`, set in `apps/battle-lab/src/gameOverlay.ts` to
  `[...gameHud.bad, 1]` — the attack-ground footprint's red, never `hud.enemy`.
- The attack drawer in `orderSteps.ts`: in range (no `goal`), a solid line from the
  unit's circle to the reticle at the step's point; chasing, the yellow route ends in the
  reticle instead of the destination circle. The overlay's "no goal, skip" early exit
  gives way to an attack front step. Skipped for fixed-wing (F3). Allies draw nothing.
  Identified and contact targets resolve to the observed pose when visible, else the
  step's point.

**Tests.** `web/tests/attackLine.test.ts`: an in-range attack yields a solid segment from
the circle edge to the reticle; a chasing view has no destination circle at its goal; a
fixed-wing view's mesh equals its no-attack mesh. Geometry only.

**Visual.** One variable: the current attack marks. Crops: a unit and its target at
default and far zoom; one blocked unit beside one attacking unit (solid+reticle vs
dashed+cross must read differently). `screenshot-critique` unprimed ("what is each unit
doing?") as the last check; `compare-screenshots` before/after and against
`../visualizations/attack-line-directions.html` panel B for intent and
`journey-space-orders.png` for line weight. Show the user inline; non-blocking (~5 min,
then decide and record).

**Feedback that would change it.** The reticle's shape or size at far zoom.

**Commit.** `feat(overlay): an attack draws a red line to a reticle`.
