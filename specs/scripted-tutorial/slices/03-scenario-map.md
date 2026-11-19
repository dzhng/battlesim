# 03 Scenario map category and the Ferryman valley

**Status: shipped 2026-11-19.** `MapCategory::Scenario`; every listing carries
`scenarios` (only a scenario map lists any, and it lists no encounters);
`fixtures/maps/ferryman-valley` as recorded in
[decisions.md](../decisions.md#probes-p1-p2-p4-agent-2026-11-19). Lab and test
loaders already refuse anything but `test` maps. Evidence: the sim map tests
and `web/tests/mapCatalogue.test.ts` pass; the valley picture was reviewed
unprimed and its two naming problems were fixed.

**Unlocks:** the first saved map players see, in a category built for authored
scenarios in general.

**Contract (C2):** `MapCategory::Scenario` and `"scenario"` in `MAP_CATEGORIES`.

- Only scenario documents (04) may be laid on a `scenario` map. Its folder holds
  `scenarios/` and no `encounters/`.
- Labs and tests never borrow it: `fixtureMap` and `sim::fixtures::with_map`
  refuse it as they refuse `capture`.
- `meta.json` lists `scenarios`, checked against the folder by the same
  validator that checks `encounters`.
- Docs: `development.md` ("Content has three owners" becomes four) and the
  fixtures README's saved-maps section ("no saved map is the game's" becomes
  "player battles are generated, except authored scenarios, which stand on
  `scenario` maps").

**Content:** `fixtures/maps/ferryman-valley/` from P1's seed, saved with the
`mapgen` CLI. `meta.json` sets `category: scenario` and `status: released`.

**Playable:** the developer page's geometry inspector, which lists every active
catalogue map with no new route.

**Gates:**

- `crates/sim/tests/maps.rs` and `web/tests/mapCatalogue.test.ts`: the category
  is admitted, lab and test loaders refuse it, and the listing matches the
  folder.
- A top-down picture of the valley, with every [story](../story.md) zone named
  in the caption. Screenshot-critique asks "can a viewer find the road, ridge,
  farm, village, bridge and hill?" Show it inline (non-blocking): this is the
  user's chance to reject the map before anything stands on it.

**Delegated:** the exact map id spelling (keep it general, not `tutorial`).

**Must stay green:** map catalogue tests, capture maps, lab fixture loading.

**Feedback that changes it:** the user dislikes the valley, which sends the work
back to P1 for another seed.
