# Pre-ship review: `t3/battle-fps-freeze-diagnostics` against `origin/staging`

Read-only review of `git diff origin/staging...HEAD` (merge base `00d9f04f9`),
`specs/` and `assets/` skipped. Nothing was built or run; every finding is from
reading the code. CONFIRMED means the mechanism was traced in the code;
PLAUSIBLE means the mechanism is there but the trigger was not exercised.

No finding blocks shipping.

## Findings

1. **FIX SOON. An arrival with a larger texture retires every card layer, and
   two of them can force a full rebuild mid-battle.**
   `crates/renderer-core/src/models/plan.rs:22` and `:90`,
   `crates/renderer-core/src/models/store.rs:142`,
   `crates/renderer-core/src/models/textures.rs:41`.
   - What is wrong: a card's key (`CardKey`) includes `texture_shapes`, the
     largest texture edge among the wanted textures, per colour class. The
     model texture arrays take that same edge as their shape. When a newly
     fielded type brings a texture larger than any the battle had, every
     existing card key changes, so every existing card layer becomes a dead
     address, every card is baked again, and that channel's texture array is
     rebuilt with the per-pixel CPU upscale (the 0.26–0.38 s cost in the
     spec's baseline row 1b). The mechanism is CONFIRMED.
   - Failure scenario: the ledger measured 85–90 MB of card layers in the
     reference battle, about 170 layers at 0.5 MB each. One edge change leaves
     about 170 dead addresses. A second (the other colour class, or the next
     size up) passes the bound of 256 (`dead_texture_layers`), `ArtStore::admit`
     starts a fresh store, and all geometry is packed and uploaded again: the
     multi-second freeze this branch removes, with old and new art both
     resident until the swap. PLAUSIBLE: it needs two edge changes in one
     battle, which depends on the roster's texture sizes.
   - The ledger's statement that "monotonic battle growth creates no dead
     addresses" (decisions.md:650) holds for textures and material slots but
     not for cards.
   - Smallest fix: make the array edge a scene constant, not the wanted set's
     maximum (carry the runtime catalog's largest edge in `ArtStorePolicy`),
     so the shape cannot change inside a battle and `texture_shapes` leaves
     the card key. The spec already notes that 57 of 59 colour layers are
     upscaled to the largest edge, so the memory cost is small. If that is not
     wanted now, add the case to `asset_generations` (growth that raises the
     edge) so the cost is counted, and correct the ledger sentence.

2. **FIX SOON. `AtlasSource::PreferResident` should not stay as it is: no page
   code can reach it.**
   `crates/renderer-browser/src/queries.rs:45` and `:60`,
   `crates/renderer-core/src/models/impostors.rs:118`,
   `crates/renderer-core/src/impostor/mod.rs:86`,
   `crates/renderer-core/src/impostor/bake.rs:18`, `:234`, `:283`,
   `packages/battle-renderer/src/models/impostor.ts:16`.
   - What is wrong: the resident branch runs only when the requested spec
     equals `Spec::runtime()`. The browser's `bake_impostor` falls back to
     `Spec::default()` (cell 128, runtime is 64) and both callers
     (`apps/battle-lab/src/routes/workbench.tsx:393`,
     `web/scenes/workbench.mjs:185`) pass no spec. So in the shipped page the
     branch never runs, `ImpostorAtlas.layer` is always null and nothing reads
     it. The only users are the Rust tests in
     `tests/asset_generations/card_reuse.rs`. CONFIRMED.
   - Cost of keeping it: an enum, `ResidentInput`, `CardRequest::resident`,
     the lossless channel reconstruction in `CardRequest::read`, the `layer`
     field on `Atlas` and on the JS type, and a README sentence
     (`crates/renderer-browser/README.md`, "Runtime-spec atlas reviews inspect
     resident card layers") describing behaviour no page triggers.
   - Smallest fix: pass `AtlasSource::Fresh` from `bake_impostor` and drop the
     `layer` result field and the README sentence. Then either delete the
     resident path and prove reuse with `AdmissionWork.cards_baked == 0` plus
     the pixel comparison the reuse tests already make, or keep it only if the
     workbench gets a control that asks for the runtime spec.

3. **FIX SOON. The scenery "needs placement" guard is dead, and its accessor
   is production surface only a test calls.**
   `crates/presentation/src/presentation/catalog/static_art.rs:13`, `:89`,
   `:99`, `:119`; `crates/presentation/src/presentation/catalog.rs:166`.
   - What is wrong: `StaticDraft.carried` is `p.scenery.as_ref().map(..)` and
     `scenery_required` is `p.scenery.is_some()`, so
     `scenery_required && scenery.is_none() && carried.is_none()` is always
     false. `needs_scenery()` therefore always returns false, the
     `adopt_knowledge` error "prepare scenery placement facts before catalog
     commit" cannot fire, and `carried` clones an `Arc` that nothing reads.
     On staging `needs_scenery()` had two production callers; here the only
     caller is `crates/presentation/tests/catalog_statics.rs:297`. The
     decision now lives in `StaticIdentity`
     (`crates/renderer-core/src/runtime/statics.rs`). CONFIRMED.
   - Failure scenario: none at run time. A reader takes the guard as proof
     that a draft with changed scenery art cannot commit without placement;
     it proves nothing.
   - Smallest fix: delete `carried`, `needs_scenery()` (both levels), the
     `adopt_knowledge` check and the test assertion.

4. **FIX SOON. `FieldingOwner` cannot see a catalog reload.**
   `web/src/battle/sim/fielding.ts:68` (`sameSource`), `:45`.
   - What is wrong: `sameSource` compares `a.runtime === b.runtime` and then
     `a.runtime.catalog === b.runtime.catalog`. `runtime` is the one
     `AppearanceLibrary` (`gameRuntime`), and `catalog` is a getter for its
     current catalog, so the second comparison reads the same getter twice and
     is always true. The reuse decision is keyed by the library's address.
     CONFIRMED.
   - Failure scenario: `gameAppearances` loads the catalog again when the
     faction or scenery selection changes
     (`apps/battle-lab/src/gameAppearances.ts:153`). If that happens inside
     one battle scope, Rust's `Fielding` keeps the catalog JSON it was built
     with and resolves names to the old bundle hashes; `contents()` then asks
     the library for hashes the new catalog may not have, and the battle
     fails with "content … was not read" or draws from stale routes.
     PLAUSIBLE: a battle's factions are normally fixed, so this needs a
     reload in tooling or development.
   - Smallest fix: keep the catalog object in `made`
     (`{ fielding, input, catalog: input.runtime.catalog }`) and compare that.

5. **NOTE. A shell that passes no stand-in now gets a hull-sized pick box for
   a unit it does not draw.**
   `crates/presentation/src/presentation/picks.rs:43`,
   `crates/presentation/src/effects/cook_offs.rs:108`,
   `crates/presentation/src/effects/publication.rs:205`.
   - What is wrong: unit types now cover the whole session catalog, and picks
     and cook-offs read `body_half` for all of them. With `stand_in: None`
     (`native/client/src/capture.rs:413`, `presentation/showcase.rs:154`) a
     hull whose type was not fielded is skipped by the drawing code but still
     gets its full pick box and brews up when it dies. On staging it got a
     zero-size pick and no cook-off. CONFIRMED.
   - Failure scenario: none today. The native client fields every kind its
     replay names before the first frame, so its output is unchanged. It
     becomes visible the day a native session gains a type mid-battle.
   - Smallest fix: none needed now; record it beside the native "no grow
     path" firewall.

6. **NOTE. "art committed", which carries the admission work report, is
   emitted on a change of source object, not on a commit.**
   `apps/battle-lab/src/LabViewport.tsx:798`,
   `web/src/battle/render/scene.ts:366`.
   - What is wrong: the trace fires when `scene.appearances()` returns a
     different `InstalledAppearances` object after `flush()`. An admission
     that commits with the same source object (a proxy set change, a grass
     rule change, a staged battle whose art was already installed) reports no
     work, and the page counters taken by `takeAdmissionWork()` roll into the
     next event. CONFIRMED by reading; the growth path normally arrives with
     a new source object, so `battle-growth` is not affected.
   - Smallest fix: drive the trace from `flush()` reporting a commit.

7. **NOTE. One batched widen fails for every caller in it.**
   `packages/scene-assets/src/loader.ts:321` (`queueWiden`).
   - What is wrong: requests made in the same microtask share one `widen` and
     one outcome. A name one caller asked for that is not in the catalog, or
     that passes `MAP_DOWNLOAD_MAX_BYTES`, rejects the others too. On staging
     `withUnits` and `withAppearances` failed independently. PLAUSIBLE: in the
     game there is one caller (`gameAppearances`), so two components asking in
     the same tick is the only way to hit it.
   - Smallest fix: on a batch failure, run each waiter's own request again
     alone, so only the caller with the bad name sees the error.

8. **NOTE. The fielding owner is created in `useMemo` and freed in an effect
   cleanup.** `apps/battle-lab/src/useBattleFielding.ts:23`, `:32`.
   - What is wrong: when React runs the cleanup and the effect again without
     recomputing the memo (Fast Refresh, or Strict Mode if it is ever turned
     on), the same closed owner is reused and `field()` rejects with
     "fielding owner is closed", which the hook throws during render.
     PLAUSIBLE, development only: the repo does not use Strict Mode today.
   - Smallest fix: create and free the owner in the same effect, held in a
     ref keyed by `scope`.

9. **NOTE. Docs.**
   - `crates/renderer-core/README.md`: the added sections restate
     `store.rs`, `texture_array.rs` and `card_store.rs` step by step (about
     eight paragraphs). The contracts are there (one store per scene and
     device, append-only addresses by content, last holder destroys, a fresh
     store is the same admission), but they are buried in mechanism. "Unused
     layers from dropped work can be reused while their allocation survives"
     reads as address recycling, which the code never does; it means a layer
     a dropped preparation wrote stays valid for the same content.
   - `crates/renderer-browser/README.md`: inlines method signatures
     (`missing_content_ids(presentation, json)`,
     `appearance_draft(presentation, detail)`); "Names route to the bundle
     hash …, never to a reuse decision" does not parse; the resident-atlas
     sentence belongs to finding 2.
   - `crates/presentation/README.md` and `web/README.md` state the contracts
     as the code has them.

10. **NOTE. Two new `#[allow(clippy::too_many_arguments)]`**
    (`crates/renderer-core/src/texture_array.rs:133`,
    `crates/renderer-core/src/runtime/assets.rs:171`) carry no wording of what
    blocks the split, which the repo's rule for exemptions asks for.
    `LayerArray::admit` takes `label` and `report` only to print two tripwire
    lines; returning the event in `ArrayAdmission` and letting the caller
    report would remove both arguments and the exemption.

## Lenses that found nothing

- **Shared GPU resource lifetime (lens 1).** Nothing found. Traced: texture
  and card arrays are destroyed only by `SharedArray::drop` on the last `Arc`
  (`Models.albedo`/`surface`, `Cards.array`); geometry buffers only by
  `Drawable::drop` on the last `Arc<ModelGeometry>`; both indexes are `Weak`
  and upgrade only while a row holds the payload. Layer and material slots are
  append-only and never reassigned, so a pending generation writing into the
  live array only touches layers no live material row or card row names. A
  dropped or failed pending generation leaves reservations and written layers,
  and changes no live table, report or scenery/grass owner. Stale work is
  rejected in `commit_assets` by device, pipelines and catalog revision.
- **Panics and locks.** Nothing reachable found. The art store lock is never
  held across an `await` in `bake_cards` and is not taken re-entrantly: the
  `build` closure run under the lock in `ArtStore::geometry` (`store.rs:217`)
  takes no store lock, and the two back-to-back locks in
  `admission.rs:96`–`98` are separate statements. The index and `expect` sites
  (`texture_array.rs:107`, `:121`; `card_store.rs:82`; `admission.rs:113`,
  `:264`; `plan.rs:86`; `textures.rs:108`) each rest on a reservation made
  earlier in the same admission. A lost device surfaces as an `Err` from the
  readback, not a panic.
- **Unbounded growth (lens 2), apart from finding 1.** `MaterialSlots.ranges`,
  `LayerArray.slots` and `CardStore.frames` are never pruned but are bounded
  by the fresh-store tripwires; the weak geometry, rest-pose and library
  texture indexes are pruned on use. On the page, `FieldingOwner` holds one
  result and is freed with its battle, the two `WeakMap`s are keyed by objects
  that die with their art, and `SceneAppearances` holds one content string per
  request.
- **Reuse keyed by name, row or address (lens 3), apart from finding 4.**
  Texture layers are keyed by `Texture.id`, a content address by the schema's
  definition; geometry, cards, decoded bundles and presentation facts by
  `ContentId`; scenery reuse by ordered name and content routes.
- **Page races (lens 4), apart from findings 6 and 7.** One candidate at a
  time; a superseded candidate is freed when its promise settles; `missing`
  is computed and the draft filled in one synchronous turn, so a commit cannot
  land between them. A failed admission is not retried for the same content
  (the ledger's decision); it reaches `reportFailure`, which stops the
  viewport, so it is not silent.
- **Stand-in (lens 5), apart from finding 5.** A body draws as a block only
  when `UnitsSection::resolve` finds no installed look for it, and that
  section is swapped in the same commit as the art. With a stand-in supplied,
  every catalog type has a body box, so there is no unit with neither art nor
  a pick. The block is never given the impostor tier, and its bundle has all
  four tiers.
- **Leftovers (lens 6), apart from findings 2, 3 and 10.** No `console.log`,
  debug timers, `dbg!`, scratch files or lineage comments in the added
  production lines.
