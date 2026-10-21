# Launcher follow-ups (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-10-21.

## Next Agent Prompt

You are picking up what was left when the five long and medium-range air-defence launchers got their own models and bodies: the Patriot, the US NASAMS, the IRIS-T SLM, the S-300PMU2 Favorit and the S-350. Each was drawn from the Stryker Dragoon before that work, and it kept the Dragoon's stats. When the user starts this, walk the list with them: which items matter, and in what order. Item 1 changes battle outcomes and needs `tweak-mechanics` first. Then run `write-spec` and turn this page into a sliced plan. Update this section before you end each pass.

The items are independent:

- [ ] Give the five launchers a soft truck's armour.
- [ ] Refresh the stale model receipts.
- [ ] Decide whether the S-300 keeps the wheeled hull length limit.
- [ ] Decide whether the shared truck parts the launchers inherit are close enough.
- [ ] Parse the battle scenario once in the battle view.

## The items

1. **The launchers keep the Stryker's armour.** Each of the five has the Dragoon's hull armour and ricochet chances (`body.hull.armor` in `fixtures/units/roster/{us,europe,eastern}.json`). The other soft air-defence trucks on the same base profile (Europe's NASAMS, the Buk-M3, the Pantsir-SM) have much thinner armour and no ricochet. The clearest mismatch is the US NASAMS, which is the same launcher as Europe's but much tougher. Armour is data that drives outcomes, so changing it moves battle digests and is a named decision. Run `tweak-mechanics` first.
2. **Most model receipts no longer match their scripts.** A receipt (`assets/source/roster/<family>/source-receipt.json` or `<script>.source-receipt.json`) records the hash of every script that built a family's models. Shared helpers (`textures.py`, `catalog_frames.py`, `vehicle_export.py`, `parts.py`, `vehicle_parts.py`) have changed since most families were exported, so most receipts are stale. No check reads them. Re-exporting a family refreshes its receipt, but the model may change with it: helper changes that landed without a re-export show up as new geometry or materials. So each re-export needs the model diffed. Where it changed, it needs the validator, a contact sheet and the bake. Decide first whether a check should hold receipts fresh, or whether a receipt only records what built the committed file.
3. **The S-300 is shorter than the real launcher.** The real 5P85SE2 travels at about 13.1 m. Wheeled hulls are capped by `hull_limits.wheeled` in `fixtures/game.json`, so the card's body is the cap's length, and the containers overhang the tail less than on the real truck. The cap was not loosened to fit it. Raising it is a game-rule decision that affects every wheeled unit's allowed size.
4. **Shared truck parts stand in on the launchers.** The S-300's MAZ-543M chassis is the Tornado-S's (`rocket_artillery.maz_543m`). The S-350's cab and tyres come from `truck_chassis`. The IRIS-T's cab and wheels are the MAN HX script's. Each is close, not exact, to its references. Correcting one changes every truck that shares it, so judge the sharing trucks together against their references (`compare-screenshots`).
5. **The battle view parses the scenario twice.** `apps/battle-lab/src/BattleView.tsx` parses the scenario JSON for the map size, then `battleOverlayScenario` parses it again. It happens once per scenario change, so it costs nothing measurable; fold it into one parse when that code is next touched.

## Open questions for the walk

- Should the launchers' armour follow the existing soft-truck values, or does each get its own from its real vehicle?
- Should a check keep model receipts fresh, or do they only record what built the committed file?
- Is a 13 m wheeled hull worth raising the limit for?
