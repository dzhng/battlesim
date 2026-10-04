# 06 Garden dressing in suburbs and villages

**Unlocks:** lived-in lawns in `garden_suburb` and `village`.

**Seam:** catalog rows `garden_shed`, `hedge`, `garden_fence` (modular, like `heras_fence`), `washing_line`, `garden_table`, decided with tweak-mechanics (a hedge conceals and stops nothing). Presets `props.gardens` rows per district. A consumer that generalises today's `yards()`: for each built lot, pieces in the rear setback (`LotPlan.ring` rear edge, a presets depth), and boundary runs on the lot's rear and side edges through the fence routine, capped per lot. Stream `street-props/<lot>/garden`.

**Verify:** pieces inside their own lot; front gardens and doors clear; reachability as slice 05; prop count on suburb-heavy maps within limits; deterministic. Stand-in shots of a suburb street at 65 m per region; screenshot-critique.

**Delegated:** densities and the boundary mix per region (hedge vs fence).
