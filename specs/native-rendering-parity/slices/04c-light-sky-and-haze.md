# 04c — Light: sky, clouds and aerial haze

**Contract:** native draws the browser's sky, clouds and distance haze, so the horizon reads as the same weather.

**Seam:**
- The manifest gains `light.sky` and `light.aerial`: the resolved `skyModelParams`, cloud parameters and `aerialParams` (`light/skyParameters.ts`, `aerialParameters.ts`).
- Native ports `world/sky.ts`'s formulas into a sky shader that replaces the current procedural one (`reel.gd:635-684`).
- Haze is depth-based and applied once, in the post pass from 04a or a sibling CompositorEffect, so phase 2's imported models inherit it.
- No fog-term constants are lifted: phase 1 paints no fog overlay, and nothing would read them.

**Runnable artifact:** one broad plate frame including the horizon, and one city-horizon frame from paris-corner.

**Visual variable:** sky gradient, cloud cover and distance haze.

**Focused gates:**
- A TS test checks the manifest's sky and aerial values against the resolved fixture.
- The shader compiles under Forward+.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** terrain and grass colour, and tone (fixed in 04a).

**Delegated:** the cloud noise implementation, provided it is driven by the manifest's values and the frame record's `simTime`.
