# 05: Enemies see it land

**Contract.** (Q7, P12, P20)
- An enemy knowledge track refreshes its layer and class from the unit at every
  sighting, and the knowledge digest folds both in. A side that first saw the
  heli flying sees a ground vehicle after touchdown, and an aircraft again after
  it packs.
- An identified enemy that deploys publishes `deployment: Option<DeploymentState>`
  (progress and target, the own unit's type). Own and identified units publish
  `grounded: bool`, true for an aircraft standing on the ground.
- The observation's "never health, ammunition, orders or timers" note is
  rewritten to name the one visible exception.
- The type-keyed client and effect sites read `grounded`.

**Seam.**
- `crates/sim/src/knowledge.rs`: on a continuing sighting (~470-479), set
  `track.layer` and `track.class` from `Unit::layer()` and
  `Unit::target_class(rules)`. Fold both into `SideKnowledge::digest` (~640).
  The LastSeen contact's layer (~221-230) comes from the track, not the live
  unit. Rewrite the doc comment at ~29-31 ("from its type") to "as last seen".
- `crates/contract/src/observation.rs`: `IdentifiedUnit.deployment`,
  `IdentifiedUnit.grounded`, `OwnUnit.grounded`, and the D5 doc note.
- `crates/sim/src/publication.rs`: `IDENTIFIED_FIELDS` (~296) and the own-unit
  rows gain the columns; also the reader (`publication/reader.rs`,
  `reader/view.rs`), the game-wasm reader, and the TS decoders
  (`web/src/battle/sim/observationView.ts` and the decoders it uses).
- Presentation: `units/interpolate.rs` (~180) blends identified deployment the
  way it blends own deployment. `units/feed.rs` carries `grounded` and deployment
  for both.
- Switch `effects/cook_offs.rs` (~107) and `effects/publication.rs` (~206) from
  the type's airborne flag to `grounded`.

**Commits.** Two, in this order:
1. **The track refresh and digest fold alone.** It moves every digest of a
   battle holding a track, so it is a named digest change. Before re-blessing,
   rebuild with the fold temporarily removed and confirm every digest matches
   the previous commit. That proves only the fold moved them.
2. **The publication contract.**

**Run / see.** In `heli-fire`, switch the lab to the red side's view: the enemy
heli's deployment shows in the readout probe, and `grounded` flips at
touchdown. Rotor looks follow in slice 06.

**Verify (test first).**
- `supply_heli.rs`:
  - `an_enemy_track_refiles_it_as_a_ground_vehicle_at_touchdown_and_as_an_aircraft_once_packed`;
  - `a_jet_ordered_onto_a_landed_heli_picks_its_weapons_by_what_it_is_now` (`air/mod.rs` ~363 reads the refreshed class).
- Publication round-trip: `crates/sim/tests/publication*.rs`,
  `publication_reader.rs`, the native/wasm parity reader, and
  `web/tests/observation.test.ts`. An identified truck's deployment is published
  (P20). A unit that doesn't deploy publishes none.
- Presentation: an identified unit's deployment blends between samples.

**Digest.** Commit 1: knowledge digests (named). Commit 2: publication layout.
Re-record `fixtures/parity/publication`, `fixtures/parity/capture/**` and
`codec-vectors.json` whole, never by hand (P21). Battle digests are unchanged by
commit 2.

**Delegated.** Field order in the publication; whether the progress column is
quantised like the own unit's.

**Stays green.** Everything above, plus `crates/presentation/tests` and the web
publication tests.

**Feedback that would change it.** None expected.
