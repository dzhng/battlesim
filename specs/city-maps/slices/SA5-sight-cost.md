# SA5: sight and fog cost at full extent

**Depends on:** SA4, the grid-update pass in [SA2](SA2-counted-route-integration.md). **Kind:** slice, performance only.

## Question
Does working out what each side sees stay cheap on a 10 km map with a hundred units a side?

## Contract it unlocks
The cost of a side's sight and fog update is bounded by what its eyes can reach, not by the map or by every occluder on it. No rule changes: what is seen, when, and by whom is exactly as today.

## API seam
`crates/sim/src/visibility.rs`, the height field's page lookup in `world/terrain.rs`, and whatever index sight needs over occluding bodies. Nothing in the contract, the observation or the publication layout.

## What is known
- Each rebuilt sight map tests every occluder on every ray: about 1 ms with nine eyes on Metro Large, unmeasured with a hundred units a side ([S3](../spikes/S3.md)).
- About 40% of a village battle's CPU samples are hashing height-field pages in the fog sweep (grid-update pass, SA2 Outcome).

## Verification
- Digests identical on the village (`village_report -- --quick`), endurance and every `city_report` probe; replay parity.
- Cost in instructions retired, before and after, on `endurance_report` and on `city_report` for the 6 km and 10 km generated towns ([S1](../spikes/S1.md) names the maps and the tool) with at least a hundred units a side: extend `city_report` if it needs a force size.
- No tick over 33 ms from sight on those runs.

## Delegated to the implementer
The index structure and its cell size, recorded in `../choices.md` under this slice.

## Must stay green
Foliage and fog agreement; every existing sensing and fog test.

## Feedback that would change this slice
A rule that has to change to make sight affordable is a mechanic change: stop and run tweak-mechanics, and name the digest change.
