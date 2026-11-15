# Decisions ledger

What was decided for [battle growth](README.md), by whom, and why. Add to it
whenever a slice settles something the plan left silent or confirms a fact the
plan only read from the code.

## From the planning walk, 2026-11-14

The player answered some of these; the agent answered others on the player's
behalf under the preferences below; the code answered the rest.

| # | Question | Decision and why | Closed by |
| --- | --- | --- | --- |
| 1 | What does "fixed" mean on screen? | Cost follows what is new. Work is spread over frames only if a measured multi-type arrival still stutters. The alternative, never a hitch whatever arrives, is a second design problem on top of reuse. | player |
| 2 | Preload the enemy faction? | Both: the incremental engine, then a background preload of the enemy faction. Preloading leaks nothing: the faction is in the battle link and drawing still follows what the side sees. Rejected: engine only (first contact still waits on a download), preload only (leaves the rebuild-everything path for every other art change). | player |
| 3 | What does memory have to prove? | Smoothness is the pass/fail gate. The engine must not raise peak memory. The preload's cost is measured with the archived whole-browser method and shown before the preload is on by default. | player |
| 4 | What is on screen while art loads? | A stand-in shape that swaps to the model. Rejected: nothing until it appears (today), a contact marker that pops or fades. | player |
| 5 | Is there a weakest machine to protect? | None named. The preload runs on every machine. | player |
| 6 | When does the preload run? | As soon as the battle is on screen, deployment included, one type at a time. | player |
| 7 | What does done look like? | The player plays a large mixed battle and never notices a type arriving, and `battle-growth` keeps it so. The armory and roster pages share the path and get faster without redesign. | player |
| 8 | How does geometry stop depending on the generation? | Each model keeps its material slots for the life of the scene; new art takes the next free ones. Vertex data never changes and no shader does. Rejected: a per-draw material offset in the shaders, which touches all twelve model pipelines and the instance record has no spare field. | agent, shown |
| 9 | What identifies unchanged art? | Its content hash from the runtime catalog, carried into Rust. Names are not enough: tooling replaces art under one name. | agent, shown |
| 10 | How is reuse proven? | By counting work in a deterministic test. A long-frame check is a loose tripwire. | agent, on the player's behalf (process recommendations accepted; numbers are tripwires) |
| 11 | What stands in for an infantry squad? | One block per soldier, where the simulation says each body is. | agent, on the player's behalf (something on screen over nothing) |
| 12 | Can a stand-in be selected and targeted? | Yes. It is the unit; only its art is missing. | agent, on the player's behalf (the complete fix) |
| 13 | How deep does the preload go? | By measurement after the engine: download-only if a one-type arrival then shows no visible wait, otherwise all the way to the GPU. | agent, on the player's behalf (memory is spendable, cost shown) |
| 14 | Backward compatibility or migrations? | Neither. Hard cutover; what is replaced is deleted. | the placeholder spec and the repository's own rule |

**The player's confirmed preferences:** the complete fix over the cheap one;
something on screen over nothing; memory may be spent for smoothness if the
cost is shown; process recommendations accepted as given.

**What the agent may decide on the player's behalf:** small taste and process
questions that follow from those preferences. A new trade-off, or a new visual
beyond the stand-in, goes back to the player.

## From the synthesis of three drafts

Three independent drafts (fewest slices, risk first, seam quality; two Claude,
one Codex) agreed on the store, content identity, shared lifetime and work
counters. Where they split:

| Split | Chosen | Why | The alternative |
| --- | --- | --- | --- |
| Where the stand-in is drawn | Rust presentation, on one reserved appearance row | The frame's picks are already Rust's, so one body box feeds both the block and the click box and they cannot differ. The battle's units are drawn there. | The browser's proxy path. It has no per-instance size, skips infantry, moves bodies through JSON model lists, and a new proxy name triggers an admission. |
| Ladder size | Twelve slices | One variable and one verdict each. Geometry reuse does not need stable texture layers, because the small materials table is rebuilt per generation, so the two are separate slices. | Four slices, with the whole engine as one; fourteen, with the allocators and the shared lifetime as slices of their own. |
| Cancelling an in-flight build | Not built | With reuse a build is as short as its new art. One candidate at a time stays. | A cancel handle through the bake. Revisit only if slice 08 measures a spotted type waiting behind a build. |
| Inflated-bytes cache for fielding | Rejected | It would hold the catalog's geometry a fourth time. Fielding keeps the small facts it derives instead. | Cache inflated files by hash in JS. |
| Texture growth | Arrays carry spare layers; each layer is written once; growth is a GPU-side copy | No CPU work for unchanged textures. The placeholder's rule "never overwrite a shared resource during preparation" is kept at the level of a layer: pending work writes only layers no committed table names. | Preallocate at the catalog's maximum, which spends memory unmeasured. |
| Preload to the GPU | Feed enemy types one at a time into the set `useBattleKinds` already owns | Each is then an ordinary one-type growth, with no second owner. | A separate preload admission path. |

## Facts read from the code, not yet exercised

Each is confirmed by the slice named, which then moves it to the section below.

| Fact | Slice |
| --- | --- |
| The download prefetch is idle on a battle opened by link because only the menu enables it | 09 |
| A hull's size reaches presentation only for fielded types; the unit catalog has it for every type | 10 |
| An unfielded vehicle has a pick of no size today, so it cannot be clicked; an unfielded squad's soldiers can | 10 |
| An undrawn enemy still gets a HUD readout, because readouts follow the observation | 10 |

## Confirmed during implementation

- Slice 10 probes (2026-11-14): the browser readout layer builds enemy cards from the
  observation and falls back to the published position, so an identified enemy
  still gets a HUD readout when Rust draws no model. Rust picks are the limiting
  surface: an unfielded vehicle receives a zero half box from
  `effects.types`, so it cannot be clicked; an unfielded squad still gets one
  generic soldier pick per published member. The publication sample carries
  every observed member position and member id, including for an unarted type.
**Slice 01, 2026-11-15.** What was confirmed:

- **Every fielding read everything again.** `useBattleFielding` made a fresh
  Rust `Fielding` for every request, and each one inflated all of the battle's
  contents (136 files, 635 MB in `battle-growth`). A growth widens the kinds by
  the type just met, so that happened at every growth. It was the stall before
  each freeze.
- **After the slice, measured in `battle-growth` on the merged tree:** the
  growth reads 2 files (8 MB), the new squad's bundle and its skeleton, and
  installs one art generation. The first scene reads 136 and installs two (the
  units, then the map's kits).
- **The battle is staged twice before one commit** in that scene, as in the
  reference probe's first growth. Whether those are two types met moments
  apart or one request made twice is not settled; slice 08's gate (one
  admission per growth) owns it.
- **The doubled install** came from `useMapAppearances`, which asks the
  library twice (once for the units, once when the map knows its kits). The
  two requests raced, and each installed a generation.
- **Catalog admission and fielding derive different facts** from a bundle:
  admission (`Asset::from_bundle`) derives bounds, poses and static states;
  fielding derives a unit's joints, rig and clip table and a wreck's pieces.
  Neither replaces the other, so by the slice's rule the facts stay with
  fielding.

Decisions made where slice 01 was silent:

| Decision | Why, and what was rejected |
| --- | --- |
| Fielding keeps `Facts`: per appearance, the record and clip table it derives, keyed by bundle hash and skeleton hash; per wreck, its pieces' bounds, keyed by bundle hash. The bytes are dropped when `resolve` returns | Rejected: caching the inflated bytes in Rust (the first pass did; it is the byte cache the plan rejected, inside the 4 GB heap) |
| The WASM `Fielding` object owns the facts, and a `FieldingOwner` in the page owns that object for one battle and frees it | Rejected: a module-level map of fieldings keyed by the serialized inputs, which leaked them and serialized megabytes per call |
| An owner asked to field the same inputs again returns its last result without calling Rust | An art install changes the request object but none of fielding's inputs |
| A change to the rules, either catalog or the factions makes a new Rust fielding | They are fixed when it is made; only the kinds can change under it |
| A new skinned body reads its skeleton again even when another body already used it | Keeping the parsed skeleton would hold its clips twice; it is one file, and only when kinds widen |
| `contents()` names a skeleton only for a body that wears it | It named every skeleton in the catalog before |
| Unit widenings and on-request appearances that arrive together are one install | Rejected: serializing them, which still installs once each |
| `scene installed` takes the page's counts, so the first `art committed` reports its own admission only | The counts were cumulative from page load |

**Slice 00, 2026-11-14.** One growth in `battle-growth` at the scene's own
graphics profile, from the new report:

| Count | One growth today |
| --- | --- |
| Bundles decoded in Rust | 201 |
| Vertex bytes packed | 1.13 GB |
| Model buffer bytes uploaded | 1.31 GB |
| Model texture layers written | 175 (123 MB) |
| Cards baked | 189 |
| Scenery and grass bytes uploaded | 22 MB |
| Scenery placement runs | 1 |
| Bundles encoded and sent by the page | 201 (710 MB) |
| Runtime contents inflated | 272 (1.27 GB) |
| Art library installs | 2 |

- **The art library does install twice per new type, and fielding runs
  twice:** 272 contents is two passes over the same 136. There was one
  admission for the growth. Slice 01 owns both.

Decisions made where slice 00 was silent:

| Decision | Why |
| --- | --- |
| The report's fields: bundles decoded; vertex bytes packed and buffer bytes uploaded for model geometry and the materials table; texture layers and bytes for the model texture arrays; cards baked; scenery and grass bytes; scenery placement runs; and a rebuild reason | Each engine slice needs its own number, so the kinds of work are not mixed |
| A shell hands in work it did before staging (decoding its draft, the first scenery placement) as the starting report of `Runtime::new` and `stage_assets` | Decoding happens in the library before the runtime sees the art, and the count belongs where the work is |
| The page's counts are those since the previous `art committed` | The event must say what this admission cost |
| The scene reads the event's detail from the console call's argument | It needs no test hook in the page, and `LabViewport.tsx` is over its size budget |
| The first scene publishes no `art committed` event | `scene installed` carries the baseline page work; art commits report only later admissions |
| The swap test compares the live report and the world's appearance names, not pixels | It proves a dropped, stale or failed candidate changes nothing live without a frame readback; pixel equality starts with slice 03 |

**Slice 00's probe, 2026-11-14.** The reference battle at high graphics in the
scene harness, five growths, run with
[the probe](assets/growth-probe.mjs):

| Growth | Freeze at staging | Staging to commit | Cards baked | Bundles sent | Installs |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 (staged twice) | 2.8 s, then 3.2 s | 103 s | 189 | 216 | 4 |
| 2 | 3.3 s | 66 s | 189 | 218 | 2 |
| 3 | 3.0 s | 64 s | 189 | 220 | 2 |
| 4 | 3.1 s | 62 s | 189 | 222 | 2 |
| 5 | 3.5 s | 69 s | 207 | 228 | 2 |

- **The long wait is the card bake.** After the freeze, the only work left
  before the commit is baking every card. The same 189 cards took 1.4 s in the
  small skirmish and over 60 s here, so the cost is not GPU work: each card
  waits on three GPU reads in turn, and each read waits behind the battle's
  own frames, about 0.1 s apiece. Slice 05 follows from this (see its seam).
  Inferred from what the phase contains; no timer isolated the bake.
- **A superseded build is thrown away and redone.** Growth 1 was staged twice
  six seconds apart. The first build ran to the end of its bake and was
  discarded, then the second began with its own freeze. That is the 103 s, and
  it matches the 40 s waits in the player's report. Slices 01 and 08 remove
  the doubled request; with reuse a build is short enough that a wasted one
  costs little.
- **Scenery and grass at growth:** one placement run and 24 MB uploaded, of
  1.4 GB in all. Its time on this map is not measured; slice 06 times it first.
- The player's own freezes were longer (2.9–9.9 s) on a larger window with the
  battle running in real time.

**Slice 06 initial measurement, 2026-11-15.** Temporary `js_sys::Date::now()`
timers (1 ms resolution) in the browser WASM admission timed the exact main-thread regions
in `Runtime::stage_assets`: `finest_meshes` plus `stage_scenery` (placement),
`Scenery::new` (scenery build), and `grass` (grass build). The timer and the
probe scene/fixture were removed after the runs.

| Workload | Growth | Placement | Scenery build | Grass build | Total |
| --- | ---: | ---: | ---: | ---: | ---: |
| Reference battle, high graphics | 1 | 115 ms | 6 ms | 3 ms | 124 ms |
| Reference battle, high graphics | 2 | 104 ms | 6 ms | 1 ms | 111 ms |
| Reference battle, high graphics | 3 | 94 ms | 5 ms | 1 ms | 100 ms |
| Reference battle, high graphics | 4 | 91 ms | 5 ms | 2 ms | 98 ms |
| Reference battle, high graphics | 5 | 120 ms | 6 ms | 2 ms | 128 ms |
| `battle-growth`, high graphics | 1 | 45 ms | 3 ms | 1 ms | 49 ms |
| `battle-growth`, high graphics | 2 | 45 ms | 7 ms | 1 ms | 53 ms |

The reference run took 401 s and reached five staged admissions; the small
scene passed in 29 s. Every total is clearly above one 16 ms frame, so Slice 06
was built. Review rejected the first implementation's duplicate byte hashing,
address-based reuse keys and deep-cloned placement. Its initial tests did not
cover the final identity refinement or compare with a clean final-catalog build;
the corrective pass below supersedes those implementation claims.

**Slice 06 corrective pass, 2026-11-15.** Merged slices 01 and 02 at
`3456f63e6`. Static reuse reads the canonical `ContentId` through the required
`BundleSource::content_id` seam in every source. No admission hashes transported
bytes or uses an allocation address as identity. Ground identity is its revision:
`set_ground` and `ground_mut` bump it; catalog commits do not.

A draft shares immutable scenery placement through `Arc<Scenery>` and resolves
its appearance rows again. It does not clone map placement, sizes, trunks or
lookup indexes. On an unchanged commit the live walk retains its indexes,
selection lists, dressing cache, cleared-ground and falling-tree knowledge;
changed art builds a prospective walk and adopts current knowledge at commit.
Unchanged grass retains the live GPU owner, including regrowth state that may
have advanced while asynchronous cards baked.

The same high-graphics reference probe completed in 366 s, with five admissions
and four committed growths (the first admission is superseded). Temporary
`js_sys::Date::now()` timers wrapped the original three regions plus shared
placement carry and row resolution in the presentation draft. All values below
are milliseconds; 0 means below the timer's 1 ms resolution, or a skipped region,
not an assertion of literally zero elapsed time. The before column repeats the
initial measurement above, in admission order.

| Growth admission | Before placement / scenery / grass / total | After carry | After placement | After scenery | After grass | After total |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1, superseded | 115 / 6 / 3 / 124 | 0 | 0 | 1 | 0 | 1 |
| 1, committed | 104 / 6 / 1 / 111 | 0 | 0 | 1 | 0 | 1 |
| 2 | 94 / 5 / 1 / 100 | 0 | 0 | 0 | 0 | 0 |
| 3 | 91 / 5 / 2 / 98 | 0 | 0 | 0 | 0 | 0 |
| 4 | 120 / 6 / 2 / 128 | 0 | 0 | 1 | 0 | 1 |

All four reference commits reported `sceneryPlacements: 0` and
`staticsBytesUploaded: 0`. Carry is below a millisecond on this map. The timers,
probe copy and temporary registry entry were removed after measurement.

On the timer-free final tree, `catalog` and `catalog_statics` passed (five tests
each), as did `asset_generations` (six tests), `grass` and `grass_assets` (one
each). The unchanged-admission test decodes a fresh source generation with the
same `ContentId`s and compares its committed pixels to a second runtime built
from scratch with the final catalog. Changed scenery content still places and
uploads again. No assertions were weakened.

Known imprecision: the library's decode count is cumulative, so a runtime
rebuilt after a lost GPU device reports decodes it did not redo. Slice 02
reworks the library and settles the count's owner.

**Slice 10, first pass, 2026-11-14 (not merged).** Confirmed on screen by the
implementer: an identified enemy with no art still gets its HUD readout,
because readouts follow the observation; an unfielded vehicle has a click box
of no size and cannot be clicked; an unfielded squad's soldiers can. The
observation carries every seen member's position for a type with no art.

Review findings to fix before it merges:

1. It fields every type in the unit catalog. In the real game every type's art
   is in the runtime catalog, so fielding would read the whole catalog and no
   type would ever be unfielded. Facts that need no art cover every type;
   appearance sets and anything read from art cover fielded types only.
2. Its fixture deletes the art from the catalog file through a hook in
   production code, and proves the swap by reloading the page. The real case
   is a type requested after the first scene whose files have not arrived:
   hold those responses from the scene, and swap in the same session.
3. It deleted comments from `presentation.rs` to fit the size ratchet.
4. It hardcodes the browser mesh's size inside presentation. The shell that
   builds the mesh states its size in the battle input.
5. The showcase invents a soldier's body to satisfy a parameter it never uses.
6. Hull sizes have two owners after it (the effects section and the new
   table), and the new table defaults silently when missing.
7. Its "art arrives" test starts with the art already present.

Fix-round decisions (2026-11-15):

8. The units section keeps one required `bodyHalf` table for every catalog
   type, while `types[].draws`, track gauges, appearance records, wrecks and
   skeleton contents remain scoped to `Fielding::kinds`; a second effects hull
   size was removed. The rejected alternative was retaining the first pass's
   all-catalog art derivation.
9. The browser fixture uses Playwright response interception during the real
   battle-growth admission and releases those responses in place. The rejected
   alternative was a production fetch hook, catalog rewriting, or a page reload.
10. The stand-in input carries `{ appearance, half }`, where `half` is the
    browser box primitive's authored half extents. Presentation scales from
    that value; it does not know the mesh dimensions. The rejected alternative
    was a presentation constant.
11. Showcase has no stand-in body input because it never draws a stand-in. The
    rejected alternative was inventing a soldier physics value for it.
12. `BodyBox` is the single frame-owned body-box value used by both UnitArt and
    picks; the rejected alternative was a `BodyBoxes` wrapper plus a duplicate
    `body_half` owner.
13. The loaded frame builder moved to `presentation/frame.rs` as a concept split
    so the frozen `presentation.rs` ratchet could keep all restored comments;
    the rejected alternative was shortening or deleting those comments.
**Slice 02, 2026-11-15.**

- `ContentId` reuses the shared contract crate's identity owner. It is a typed
  `{ bundle, skeleton? }` pair of canonical SHA256 strings, with the skeleton
  hash present for skinned art because clips change the model. A renderer-local
  identity type was rejected because it would duplicate validation beside the
  contract crate's exact content-hash vocabulary.
- `Library` owns two indexes: names route to `ContentId`, and each identity
  owns one decoded bundle. A replacement name therefore keeps old content only
  while another name routes to it; a second name with the same identity does
  not decode. Keeping the old name-keyed map beside the identity map was
  rejected because it would leave reuse decisions keyed by names.
- The decode counter is pending work owned by `Library` and drained by its
  mutable `take_admission_work` read. The alternative cumulative lifetime counter was
  rejected: rebuilding a runtime after device loss would report decodes that
  the rebuild did not perform. This keeps `bundles_decoded` scoped to the
  admission that consumes the source draft.
- `Library` keeps a route count per content identity, so rerouting a name drops
  an unshared decoded bundle immediately. Rescanning all names after every
  insert was rejected; the count also proves that a second name keeps shared
  content alive.
- Tooling constructors with no catalog identity hash their bytes at the point
  they make the bundle; catalog paths pass the catalog identity directly.
  Falling back to an appearance name was rejected because names route and do
  not identify content. Generated page bundles use the contract crate's
  exported WASM `bytes_hash`, cached once per bundle object in the web renderer;
  a second JavaScript digest implementation was rejected as a duplicate owner.
- `ContentId` is private-field and serde-validated through its raw wire shape,
  so browser call sites cannot deserialize an invalid hash and then patch it by
  hand. Duplicating that validation in both browser asset entrypoints was
  rejected.
- The native client takes its identity through `Runtime::content_id` when slice
  03 gives `BundleSource` its first identity consumer. Caching decoded bundles
  in the native client was rejected because it holds decoded art the client did
  not hold before.
- Confirmed from the implementation: the scene-assets runtime catalog already
  carries bundle and skeleton content hashes, while `InstalledAppearance` and
  `ModelAsset` previously dropped them before the Rust seam. Both now carry the
  same pair through `add_bundle` and `load_bundle`.

**Reslice, 2026-11-15.** "New cards bake together" left slice 05 and became
[slice 05a](slices/05a-cards-bake-together.md), depending only on slice 00.
It needs none of the reuse chain (identity, geometry, textures), it removes
the minute between staging and commit on its own, and it can be built while
that chain is. The alternative, leaving it inside 05, kept the longest wait
the player sees behind three other slices. Its batches are bounded by bytes
because one runtime card is about 3 MB across its targets and a full rebuild
bakes about 190 of them.

**Slice 07, 2026-11-15.** The binding seam is identity-first:
`Renderer::missing_content_ids(presentation, json)` accepts the presentation
and a JSON array of `ContentId` values, returning missing identities in request
order. `AppearanceDraft::declare` routes every wanted name to an identity;
`AppearanceDraft::add_bundle` supplies bytes by identity. A second name-plus-bytes
API was rejected because it would preserve the full-resend requirement beside
the delta contract. The page still sends every asset until slice 08; held
content is a hit for both decoders.

Presentation owns one immutable `Arc<Asset>` per `ContentId`. Names route to
identities instead of holding copies; stages share held facts once per identity.
These facts include static state bounds, tier counts and corners, or animated
rigs, poses and clip tables. Rows, palettes, culling, building bounds and scenery
fitting need them when no bytes arrive. A second fact map keyed by name and deep
clones on every stage were rejected because unchanged rigs and clip tables would
still cost catalog-sized work.

`Library::missing_content` owns the answer to “held”: decoded bundle and retained
presentation facts must both exist. The shared `SourceDraft` used by the browser
binding and native runtime tests uses this same answer. Initial renderer
installation requires facts for every decoded content identity; the first scene
installs facts for all its sources before admitting its renderer. Trusting only
the renderer snapshot was rejected because it could suppress supplied facts that
presentation lacked. A mesh-only hit supplied by the shell reads its missing
facts without decoding geometry again.

Drafts retain identities, never old decoded bundle ownership. Preparation
resolves every declaration against the current committed owners. If another
commit retired held content, preparation fails before GPU work, leaving live art
intact; retry belongs to slice 08. Only declared identities survive a successful
commit, including when the shell supplied extra bytes. A replacement name drops
its old content when no other route uses it.

Placement metadata remains in each wanted name's declaration because it describes
placement rather than content. A second identity-keyed placement table was
rejected as a duplicate owner. Complete and delta admissions must yield equal
worlds (including the row lookup index) and equal committed draws; the test uses
distinct facts for each identity so wrong routing cannot pass.

Every `LabAsset` requires its creator's `ContentId`: catalog art passes the
catalog pair, and generated art uses `generatedContentId`. The binding's fallback
hash was rejected because it omitted skeleton identity and repeated work at each
install. `insert_bundle` calls `insert_content`'s single decoder and then routes
the name; decode failures identify the content.

**Slice 05a, 2026-11-15.** Confirmed first on the reference battle from the
freeze report, at high graphics, in the scene runner's 1280 × 800 viewport.
Temporary timers isolated the entire bake and each awaited read. The initial
189-card bake took 0.904 s (567 reads, 0.361 s in those waits) before battle
frames competed with it. Two later growths each baked 234 cards:

| Growth | Bake | Awaited reads | Time in reads | Other bake work | Staging to commit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Before 1 | 124.027 s | 702 | 122.914 s | 1.113 s | 127.356 s |
| Before 2 | 104.598 s | 702 | 103.521 s | 1.077 s | 108.504 s |

Over 99% of each growth's bake was in the sequential reads: the inference was
right. Individual reads averaged 175 ms and 147 ms in these growths. The
catalog and battle now produce more cards than slice 00's earlier measurement;
these are this checkout's measured counts, not new expectations for content.

After batching, the same reference route at high graphics committed its first
two growth admissions in 5.248 s and 4.542 s. They each baked 189 cards, issuing
567 GPU reads in 21 read-wait groups; the isolated bakes took 2.032 s and
1.369 s. These are admission timings, not a matched-card benchmark: the earlier
run materialized requested LFS files while loading and coalesced more arrivals
before staging (234 cards), whereas the after run had those files available
and admitted earlier (189 cards). The synthetic tests hold inputs fixed for the
work-count and byte-equality proof.

The reads no longer each add a frame. Within each card, 95% of channel wait
spreads were at most 1 ms (maxima 15 ms and 3 ms). A whole batch still waits
behind battle frames: its awaited duration averaged 94.9 ms and 63.3 ms, with
maxima 179 ms and 183 ms. Those group durations include readback copying and
coverage filtering, and sum to 1.993 s and 1.330 s; unlike the before timer,
they are not pure GPU-read waits. Batching removes repeated round trips, not
frame pacing or the synchronous staging freeze owned by the other slices.

Decisions left open by 05a:

| Decision | Why; alternative rejected |
| --- | --- |
| A 64 MiB byte bound, owned by `impostor::DEFAULT_BAKE_MEMORY_BYTES` and carried by `RuntimeConfig::art_store.card_bake_memory_bytes` into pending admissions | The runtime spec needs 7 MiB per card: three 1 MiB RGBA8 targets, three aligned 1 MiB readbacks, and 1 MiB of depth. Nine cards fit (63 MiB); a small new type fits one batch. Holding 234 cards together would account for about 1.6 GiB. A card-count cap was rejected because changing atlas dimensions would silently change its memory cost. A bound below one card is refused rather than silently exceeded. |
| Submit every card in a batch, then use `futures_util::future::join_all` across cards and across their three channels | All maps can become pending before yielding to the browser; output order remains input order. `join_all` waits for every result before propagating an error, with no cancellation handle. A custom polling executor and a second single-card API were rejected. Each card keeps the original submission and target-destruction protocol. |
| One lazily uploaded immutable authored-materials buffer per `Models` generation | Runtime batches and configurable review requests share it. Diagnostic channel controls still write the separate live materials table. Reusing that mutable table would bake diagnostic suppression; uploading per batch would still repeat stage work. `OnceLock` supplies the lifetime without another admission path, and memory diagnostics and destruction include the buffer. |
| `impostor_request` takes a slice of rows and `BakeRequest::read` returns ordered atlases | Runtime and review use one grouped request contract. The browser review caller passes one row and still returns its existing single-atlas JavaScript object. Parallel wrapper APIs were rejected. |
| `AdmissionWork::card_read_waits`, serialized as `cardReadWaits`, counts actual batch awaits | It travels through the existing generic serde/report path. The unchanged `battle-growth` logger prints it with the rest of the work. Timing assertions and a count derived only from the planned number of cards were rejected. |
| Report the first split once per runtime through `ArtStorePolicy::report`, shared with the store tripwires | The policy now carries the card byte bound beside the slot bound. The existing shell sink keeps the core platform-free; the original decision to reject a callback was superseded when slice 03 supplied this sink. A runtime-owned latch also covers dropped/stale pending bakes and survives a fresh-store fallback. Reporting each full rebuild would repeat the same 21-batch warning on every growth. A guessed new-type size threshold was rejected because 05a still bakes whole catalogs; slice 05 owns selecting new cards. No second sink was added. |

Validation: `cargo test -p renderer-core --test asset_generations card --
--nocapture` was red with the sequential implementation behind the new request
seam: four cards reported 12 waits versus one, grouped requests were refused,
and the materials upload was 640 bytes versus the fixture-derived 160 bytes.
`cargo test -p renderer-core --test asset_generations --test impostors --
--nocapture` then passed all seven admission tests and both existing impostor
tests. Deliberately routing every batched request to its first row made
`cards_baked_in_one_batch_match_each_card_baked_alone` fail on atlas identity;
the mutation was removed. `cargo check -p native-client`,
`cargo check -p renderer-browser --target wasm32-unknown-unknown` and the owning
crates' all-target Clippy check passed. Temporary timers, scene copies and the
fixture entry are excluded from the commit. No delegated decision remains open.

After restoring the deliberate fault and removing instrumentation, both named
GPU test binaries passed again. `SCENE_GRAPHICS_QUALITY=high bun run --cwd web
scene -- battle-growth` also passed and printed `cardsBaked: 189` and
`cardReadWaits: 21`. Its missing Paris homes/farmsteads kits and regional bench
were LFS setup media; their region's requested bundles/textures were materialized
before the successful run, without changing any assertion.
**Correction to slice 00's small-skirmish numbers, 2026-11-15.** The page's
counts on the first `art committed` cover everything since the page loaded:
the first scene publishes no event, so nothing reset them. The 272 contents
and 2 installs reported for "one growth" in `battle-growth` therefore include
the first scene's own. The reference probe's later growths, whose counts were
reset by the commit before them, still show two installs per growth, so the
doubled install is real. Slice 01 gives the first scene a baseline.

**Slice 02, verified on the merged tree, 2026-11-15.** `battle-growth` at
growth: 203 bundles sent, 194 decoded. The nine not decoded are names that
share content with another name. Everything else is decoded again because
each admission's draft starts with no decoded bundles; slice 07 changes that.
The scene also shows the battle staged twice before one commit, as the
reference probe did.

**Slice 10, second fix round, 2026-11-15.**

- `UnitArt` has one optional stand-in value with its mesh description and a
  reference to the frame's body box; showcase passes none. The body box still
  supplies native picks without a stand-in. Paired optional fields and an
  eight-argument constructor with a clippy exemption were rejected.
- The shared body-draw helper supplies both soldier and vehicle loops with the
  resolved appearance, physical bounds and stand-in scale. Duplicating the
  resolve/half/scale branches was rejected.
- The browser derives the stand-in half extents from the generated box bundle's
  `positionsBounds` and caches the complete asset and content hash once at
  module scope. The asset is initialized on first admission, after WASM is
  ready; eager hashing at module import was rejected because `bytes_hash`
  needs initialized WASM. Rebuilding and hashing the bundle at every admission
  was rejected.
- The user approved extending the existing lab probe with `drawnModels`: each
  instance's appearance name, placement and scale, read from the last frame's
  batches and instances only when called. It holds no state or counters and
  has no production consumer. Screenshots plus counts alone were rejected
  because they cannot identify the appearance a body was drawn with.
- The scene holds newly requested bundle responses after the first scene and
  before starting the battle. Its opponent is European, and its squad candidates
  derive from body art absent from the initial installed names. The Eastern
  opening rifle squad was rejected: it reuses rifle art already installed for
  blue, so no network response can withhold it at growth. Moving interception
  earlier did not change that. The scene waits for multiple seen members and
  frames the most compact eligible squad so the near shot shows per-body blocks.
- A commit event can belong to an earlier admission. The scene waits for both
  an event and the actual drawn stand-ins to disappear before its after shots;
  treating the first event as proof that every requested type committed was
  rejected after it left the vehicle block on screen.
- `drawnModels` reads only batches with instance records in the last Frame.
  Retained static batches name `first = NONE` when unchanged and cannot be read
  without retaining a second instance store. That store and a dynamic-only
  appearance filter were rejected: the probe simply reads all instance records
  the Frame actually contains, with no copied state between frames.
- The scene captures both subjects at the same near camera distance before and
  after commit and checks actual replacement instances at every observed body
  position. A far overview and screenshot-only inference were rejected because
  neither proves the block-to-model swap for the identified bodies.
**Slice 01 in the reference battle, 2026-11-15.** The probe again, on the
merged tree, at high graphics:

| Growth | Staging to commit | Files fielding read | Installs | Cards baked |
| --- | ---: | ---: | ---: | ---: |
| 1 (staged twice, 4 s apart) | 109 s | 2 (8 MB) | 1 | 189 |
| 2 | 65 s | 2 (5 MB) | 1 | 189 |
| 3 | 69 s | 4 (12 MB) | 1 | 207 |
| 4 | 70 s | 2 (4 MB) | 1 | 207 |

- Before the slice each growth read all 136 files (635 MB) and installed
  twice. The first scene still reads 136 and installs two generations.
- The freeze at staging is unchanged, 2.6–3.6 s, as expected: it is the
  geometry, texture and transport work of slices 03, 04, 07 and 08.
- **The doubled staging is two stagings of the same art.** Growth 1 staged
  twice four seconds apart, both times with 145 appearances, and the first
  build was thrown away after its bake. The kinds list changes only when its
  content does, so the second staging is most likely a kind that brings no
  new appearance (a contact's kind, a wreck's owner). That is not confirmed.
  Slice 08 owns it: a staging that wants the same art as the build in flight
  should not discard that build.

**Slice 03, 2026-11-15.**

- `Runtime` owns one `Arc<Mutex<ArtStore>>` per scene/device, also held by model
  generations. `MaterialSlots` owns append-only ranges per `ContentId`; the weak
  geometry index owns no buffer lifetime. Rejected: a separate incremental
  renderer, strong cache retention, or slot renumbering after cancelled work.
- The geometry key includes content, the resolved static state, an optional
  baked corpse palette (the exact joint bytes), and the ordered track-side
  signature. The corpse row/`lies` index is not an identity. Animated skinning
  keeps the unposed mesh; changing its live palette does not repack it.
- An admission resolves its complete wanted content and preflights capacity
  before any geometry allocation. Material and texture tables are rebuilt from
  the generation's decoded bundles at the store's stable material bases; the
  slot table retains addresses only, not retired CPU materials or textures.
  Rejected: retaining old bundle data in the store, or discovering exhaustion
  midway through a geometry rebuild.
- The default dead-material-slot tripwire is **8,192 slots**, counting addresses
  absent from the prospective catalog, including reservations left by dropped
  preparations. A read of this runtime catalog found 4,587 material rows across
  479 distinct appearance contents, so the bound exceeds the whole current art
  catalog and ordinary monotonic battle growth cannot hit it. Rejected: a
  time-based eviction policy or a tight percentage bound that rebuilds during
  ordinary growth. `ArtStorePolicy` supplies the bound and platform log sink;
  firing logs kind, value, bound, owner and the fresh-store outcome.
- Dead-slot pressure and exhausted historical addresses use the same admission
  into a new store. A wanted catalog larger than the 16-bit address space is an
  error; vertex packing also checks the actual material index instead of
  narrowing with a wrapping cast. Initial admission, device recreation and the
  two slot tripwires have distinct rebuild reasons; reused growth has none.
- `BundleSource::content_id` carries the identity to the store. The browser
  library reads its existing name-to-content index; the native source asks
  `Runtime::content_id` without caching decoded bundles. Every synthetic source
  supplies an explicit fixture identity rather than falling back to a name.
- Geometry lifetimes are generation-held shared references. `Models::Drop`
  still releases its own per-generation textures; it does not destroy shared
  vertex/index buffers. The final `Arc<[Drawable]>` holder drops the tiers,
  whose destructors destroy their buffers. Confirmed in wgpu's browser backend:
  dropping a buffer handle alone does not destroy its allocation, so deferring
  this cleanup to browser garbage collection was rejected. The store's
  memory visitor also counts geometry held by pending generations sharing the
  live store, without extending those lifetimes beyond the snapshot.
- Synthetic runtime admissions confirmed exact pixel equality against a clean
  build after a name-sorting insertion, content replacement and fresh-store
  fallback. The prescribed key covered those inputs without widening. Dropped
  and stale candidates also preserve live pixels, and retirement releases the
  old geometry while the store remains alive.
- At the scene's own profile and at high, `battle-growth` packs 8,648,160
  vertex bytes at growth, down from 1,141,746,672. A scratch observer of the
  committed names found just the BTR-82A and its wreck newly admitted; their
  bundle vertex counts independently sum to exactly 8,648,160 packed bytes.
  Geometry uploads are those vertices plus 1,396,896 index bytes; the other
  126,480 buffer bytes are the generation's material table. Texture/card and
  scenery work remains for the other slices.

**Slice 03, verified on the merged tree, 2026-11-15.** `battle-growth` at the
scene's own profile and at high, one growth: 8.6 MB of vertices packed and
10.2 MB of buffers uploaded, against 1.14 GB and 1.32 GB before. Still whole
at every growth: 178 texture layers (125 MB, slice 04), 189 cards (slices 05a
and 05), one scenery placement and 22 MB of scenery (slice 06), and 203
bundles sent with 194 decoded (slices 07 and 08).

**Slice 07 integration with 01 and 03, 2026-11-15.** A source draft's explicit
name routes supply `BundleSource::content_id` to the runtime's shared art store,
including held content and aliases. The delta lifecycle test uses the same
runtime fixture and pixel reader as the geometry-reuse tests; the common catalog
fixture stages wanted identities plus facts through presentation's one API.
All eleven existing admission tests retain their assertions alongside the three
delta tests. On the merged browser path, growth decoded two bundles and packed
8,648,160 vertex bytes; the page read two runtime files and installed once.
The first-scene facts guard precedes consuming the pending rebuild reason, which
is cleared only after successful renderer installation.
**Slice 05a integrated with 01 and 03, 2026-11-15.** The card byte bound now
shares `ArtStorePolicy` with the slot tripwire and the shell-supplied report
sink. A runtime-owned shared latch reports the first split once, including a
pending bake that is dropped; changing the store cannot rearm that warning.
The full bake still counts every batch in each admission. The core has no
`web-sys` dependency or platform-specific logging branch.

Card fixtures join the existing generation fixture owner and share its one
container writer, config and `Stage` runtime setup. All earlier geometry and
card assertions remain. The new sink/lifetime test first failed because the
configured sink received zero reports while the split printed to stderr; it
then passed across repeated dropped preparations and two independent runtimes.

On the merged tree, the requested renderer lib and five named test binaries
passed all 39 tests. Native-client and WASM checks, the WASM build, typecheck,
and high-graphics `battle-growth` passed. The scene reported 8,648,160 vertex
bytes packed, 10,298,016 buffer bytes uploaded, 189 cards and 21 read waits;
one growth read two runtime contents (7,942,032 bytes) and installed one art
library generation. A reused growth has no rebuild reason. The spec README
was taken from upstream without changing its status or checklist.

**Slice 05a, verified on the merged tree, 2026-11-15.** `battle-growth`: 189
cards baked in 21 read waits. The scene's timeline is now staged, committed,
staged, committed: with a short bake the first build commits before the
second staging arrives, so the doubled staging costs a second admission and
no longer a discarded one. Slice 08 still owns making it one.

**Slice 06, verified on the merged tree, 2026-11-15.** `battle-growth` at
growth: no scenery placement and no scenery or grass bytes uploaded (one
placement and 22 MB before). The placement types are not `Clone`: a draft
shares the live placement, and nothing copies it.

**Slice 07, verified on the merged tree, 2026-11-15.** `battle-growth` at
growth, with slices 01, 03, 05a, 06 and 07 together: 2 bundles decoded (194
before), 8.6 MB of vertices packed, 21 card read waits, no scenery work, 2
art files read by fielding. Still whole: 178 texture layers written (slice
04), 189 cards baked (slice 05), 203 bundles encoded and sent by the page
(slice 08). The armory scene passes.
**Slice 04, 2026-11-15.**

- `ArtStore` owns stable per-channel layer slots by texture content id and weak
  allocation records. Each generation holds `Arc<SharedTexture>`; the final
  holder destroys the array. `Models::Drop` retains the generation-local
  impostor-material buffer cleanup from 05a. Rejected: destroying shared arrays
  at generation retirement or keeping arrays alive in a strong store cache.
- A populated array reserves **one spare layer**, initially and after capacity
  growth; growth allocates the required address range plus that one layer,
  capped by the device limit. Only addresses the candidate can name need backing
  capacity; reservations belonging solely to another pending generation do not
  force an unnecessary third allocation. A regression expected zero copy bytes
  here and first observed 168. Rejected: sizing every array to the global slot
  watermark. Empty channels use one placeholder layer. This
  deliberately modest step rejects geometric capacity doubling and its larger
  overlap. Capacity pressure logs kind, value, bound, owner and GPU-copy outcome
  through `ArtStorePolicy.report`. Existing written mip layers are copied
  GPU-side; only new layers take CPU uploads.
- Cancelled preparations leave slot assignments reserved. Their written layers
  are reused while an allocation remains held. If a cancelled grown allocation
  loses its last holder, it is destroyed; its weak record cannot preserve GPU
  data. A later admission writes the needed data into its surviving/new array.
  Rejected: recycling addresses while another generation can name them, and
  retaining decoded CPU texture payloads for retired content in the store.
- The dead-layer bound is **256 addresses absent from the prospective catalog**,
  across both channels, including cancelled reservations. It tolerates discarded
  small arrivals while bounding tooling's replacement history; monotonic battle
  growth creates no dead addresses. Crossing it, or
  exhausting historical device-layer addresses while the active catalog fits,
  uses the existing fresh-store admission and reports `DeadTextureLayers` or
  `TextureLayersExhausted`. It logs through the same policy sink. Active content
  beyond the device's layer limit is refused. Rejected: unbounded dead slots or
  a texture-specific full-admission path beside the store's existing one.
- Array edge is an exact reuse input: an experiment holding a larger pending
  array while committing the old small-edge catalog changed pixels when the
  larger array was reused. An exact-edge surviving array is preferred. Edge
  changes rebuild that channel using the unchanged CPU nearest upscale and
  authored mip chain. `textureArraysRebuilt` counts these rebuilds; all their
  layer writes and CPU bytes remain in the usual counters. Capacity-copy bytes
  appear separately in `textureBytesCopied`. Rejected: a new resampling shader,
  unreported edge rebuilds, or treating a larger edge as pixel-equivalent.
- The player confirmed that the memory ceiling is the whole renderer against
  the pre-plan two-generation overlap, including the duplicated geometry removed
  by 03. Texture-only shape is asserted independently: fitting admissions add no
  texture allocation; growth holds source plus destination, and retirement
  releases the source. A synthetic small-array case measured:

  | State | Buffer bytes | Texture bytes | Total bytes |
  | --- | ---: | ---: | ---: |
  | Live | 984 | 172 | 1,156 |
  | Fitting overlap | 1,892 | 172 | 2,064 |
  | Growing overlap | 1,892 | 508 | 2,400 |
  | After retirement | 1,220 | 340 | 1,560 |
  | Pre-plan exact-fit overlap | 2,204 | 344 | 2,548 |

  A texture-only ceiling against two exact-fit arrays does **not** hold in this
  small case: one old layer plus one spare and three new wanted layers plus one
  spare occupy more than the old one-plus-three layout. The unchanged placeholder
  is shared. Geometry reuse saves 312 bytes; the texture-only overage is 164
  bytes, so total payload falls by 148 bytes. Expectations are derived from the
  fixture and production `ResourceMemory`; these figures are evidence, not pins.
- On the integrated tree (05a, 06 and 07), both `battle-growth` profiles write
  exactly **three** new layers, **2,097,148 CPU bytes**, with **40,544,784 bytes**
  copied GPU-side and no edge rebuild. A catalog probe found 175 to 178 referenced
  texture ids; the delta is one sRGB and two linear textures, independently
  matching those counts and the full upscaled mip bytes. The post-commit event
  reports 500,921,853 total texture bytes in 48 allocations, versus 500,572,329 in
  48 before this slice: the extra 349,524 bytes are one spare linear layer.
  These events are after retirement; they do not measure the overlap peak. The
  production-visitor fixture above owns that proof. Card-bake code was untouched.

**Slice 04, verified on the merged tree, 2026-11-15.** `battle-growth` at
growth, at the scene's own profile and at high: 3 texture layers written
(2 MB), against 178 (125 MB). No array was rebuilt. 40.5 MB was copied
GPU-side: with one spare layer, a type that brings two layers to one array
grows it, and growth copies the live layers on the GPU. That is no CPU work
and no upload; if the closing measurements show it in a frame, raise the
spare layers before anything else.

The gate assertion slice 04 specified for `battle-growth` (to add once slice
08's gate has merged): texture layers written equal the number of distinct
texture ids the new art references that were not resident before the
admission, and no array is rebuilt.

**Slice 08, 2026-11-15.** Confirmed first with temporary fielding-input/result
traces, removed after measurement. In `battle-growth`, learning
`eastern_rifle_squad` changes only `fieldedBattle`'s `kinds` by value (the
`factions` and `sun` references also change, with equal values). Its result
adds only `battle.units.types.eastern_rifle_squad` and
`battle.effects.types.eastern_rifle_squad`: no appearance or clip changes.
That stages 143 already wanted appearances and commits zero new bundles,
then the BTR stages 145 with its hull and wreck. The page sends 201 and 203
bundles respectively. In the reference battle the order reverses: the BTR
stages at 29,724 ms, the rifle description at 31,752 ms, both wanting exactly
the same 145 appearances and identical name/content declarations. The second
stage invalidates the build in flight, then a commit arrives at 34,053 ms.
The baseline reference diff contains only those two rifle-description entries.
This confirms the cause, rather than the earlier inference of another install.

`useBattleKinds` now closes its learned/purchase art set over catalog kinds:
any kind requiring only those appearance names is fielded with them. This
adds descriptions, not art, and keeps the returned kinds stable when such a
kind is learned. Rejecting the description, ignoring a genuinely changed
battle, or deferring it with a debounce would either lose presentation facts
or preserve the duplicate cause. A synthetic hook regression proves the
closed set keeps exactly the original art and its identity until new art is
wanted. Map/battle/observer changes still reset the learned set.

Art and staged topology can arrive on separate frames. The old scene requested
once on the source-object change (while its canonical names were still old),
then again when the staged names arrived. Even a repeated same-set request
invalidated the first candidate. Both frame orders are now exercised through
`BrowserScene`: source arrival alone with no change to drawn names requests
nothing; when both inputs are ready there is one committable admission.
`SceneAppearances` compares sorted name/content pairs and retains the same
candidate, promise and latest poses across repeated requests, including array
reordering. No cancellation, debounce or retry was added.

Delegated and discovered decisions:

| Decision | Reason; alternative rejected |
| --- | --- |
| `ModelAsset.encode()` returns borrowed bundle, texture and optional skeleton transport | Rust's missing list is consulted before any encoding. The re-encoding getter and eager texture/skeleton encoding were deleted. Caching every encoded bundle would retain another catalog-sized copy. The first scene still calls `load_bundle` for every asset; encoding its transport once also supplies its initial presentation facts. |
| One bulk `missing_content_ids` call, then one draft with every declaration and one payload per missing identity | Aliases share one payload. Property order in the identity object and order of the asset array cannot affect equality. Per-name queries and full resends were rejected. |
| Explicit grass retuning carries authored rules in the admission identity beside the art set | Grass tuning changes placement without changing art. Suppressing that existing lab operation would break the grass experiments; an arbitrary force/retry flag would bypass content equality. Normal growth keeps the same rules. |
| `battle-growth` is serial and its long-frame tripwire is 700 ms | This is about one quarter of slice 00's 2.5–2.9 s freeze, not a latency target. Another harness scene must not compete for GPU time. The fixture observes browser long-animation-frame entries and intersects their intervals with staging-to-commit, including a frame that starts just before staging. Production's timeline only logs frames above 1 s, so using those logs alone would silently miss a 700–999 ms violation. No production observation hook or diagnostic threshold change was added. |
| Packed geometry is derived from observed arriving names' runtime bundle headers | The user's initial encoded-byte upper bound was corrected: 7,942,032 source bytes become 8,648,160 bytes in the GPU vertex layout. `48` is stated once in the gate, with its Rust owner `models/geometry.rs::VERTEX_BYTES`. Every mesh tier is counted, including every articulated node. Rigid arrivals must equal that sum exactly. Skinned art may additionally pack a posed copy per corpse pose; identical palettes share geometry, so headers establish a range from the live meshes to the live meshes plus all corpse copies, rather than exact multiplicity. Already held identities are excluded from the new-content set. |

The growth gate uses existing `window.__lab.stats().models.installed` snapshots
at `scene installed` and `art committed` and catalog identities, rather than
roster pins or a new production hook. It requires one staging and commit,
exact new-content encoding/decoding, fixture-derived geometry, no scenery work,
one library install and at most the new bundles plus skeletons inflated. These
replace slice 01's two weaker page relations. Texture-layer and card delta
assertions have an explicitly marked insertion point inside the commit checks;
slices 04 and 05 still own those, and their work remains whole in these runs.


A second regression confirmed that an art-readiness false/true round trip for
an already resolved fielding request manufactured a new `PresentedBattle`
wrapper. `useBattleFielding` now retains that resolved result instead of
restarting it. Its test was red before the guard and green afterwards; this
keeps an unchanged battle's identity stable while the same art is preparing.

Closing measurements at high graphics, scene harness 1280 × 800, with textures
and cards still rebuilt for the whole catalog (223–230 texture layers,
189–234 cards and 21–26 read waits). These are evidence, not timing targets:

| Reference growth | Contents sent | Sent bytes | Stagings | Frame containing staging | Staging to commit | Other overlapping frames >100 ms |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | 2 | 7,942,032 | 1 | 784 ms | 3.695 s | 172 ms |
| 2 | 2 | 5,090,704 | 1 | 1559 ms | 3.137 s | 175 ms |
| 3 | 3 | 11,101,752 | 1 | 1165 ms | 3.661 s | 126 ms |
| 4 | 2 | 4,326,540 | 1 | 717 ms | 2.140 s | 137 ms |
| 5 | 2 | 9,068,036 | 1 | 694 ms | 2.996 s | 123 ms |
| 6 | 9 | 33,341,936 | 1 | 869 ms | 3.302 s | 119 ms |

Every reference growth committed once, with exactly its missing content sent and
no scenery work. The small one-type arrival at high graphics waited **2.605 s**
from staging to commit, including a **934 ms** staging frame. Its work was two
bundles encoded/decoded, 8,648,160 packed vertex bytes, two files inflated and
one library install; 178 layers, 189 cards and 21 card waits were still whole.
The high-quality `battle-growth` invocation therefore failed the 700 ms
tripwire while its deterministic work checks passed. The ordinary fixture's
profile passed: **568 ms** at staging and **1.296 s** to commit. The tripwire
was not relaxed to conceal the high-quality failure.

The reference still stutters by the requested >100 ms criterion: every growth
has a 694–1,559 ms staging frame plus a 119–175 ms frame before commit. The
probe reports the heavy work inside the frame loop; it does not isolate
texture, card, or other synchronous regions. No further optimization is
attributed to a cause without that isolation. Texture/card reuse has not
landed; these measurements are not final. The remaining stutter is recorded
for the reslice trigger rather than patched in this slice.


Workbench coverage now re-bakes its synthetic catalog tank with reactive armour,
reloads through the existing workbench catalog controls and selects the same
name. It compares installed names, triangle counts and canvas pixels; names must
stay the same while the new geometry draws. The existing fixture writer is
reused, rather than adding another catalog builder or a production test hook.
The grass-only scene also passes its existing tuning experiments, confirming
that explicit placement changes still prepare with held art.


A deliberate multi-type reference arrival was then measured at high: the probe
held new bundle responses after the first scene until the observation named
two new vehicle kinds, BTR and Tigr, and released them together. This is a
coalesced arrival experiment, not an ordinary network-latency sample. The
committed-name snapshot confirms both hulls and both wrecks arrived in one
staging/admission: **4 contents / 13,032,736 bytes sent**, **14,237,856 vertex
bytes packed**, **3.869 s** from staging to commit. It still produced an
**819 ms** staging frame and another **164 ms** frame. There were two loader
installs while the requests accumulated, but only one GPU/catalog commit;
no scenery work. Textures/cards were still whole: 223 layers, 189 cards,
21 waits. This confirms the README's multi-type stutter reslice trigger;
it is recorded without adding frame spreading or another optimization here.


A later ordinary-profile run kept all work counts correct but hit **895 ms**
at staging (**2.114 s** to commit). Another checkout's unlocked scene job
was active concurrently; its process ancestry had no GPU-lock owner. This
result is retained beside the earlier passing 568 ms sample, rather than
silently treating the noisy run as green. A single isolated check was queued
to distinguish that contention from a stable tripwire failure.


The final timing attempt waited for the unlocked job before joining the shared
GPU-lock queue, then still failed the unchanged **700 ms** tripwire at
**824 ms** (**817 ms** attributed to the frame loop), with **1.882 s** from
staging to commit. All work-count checks passed again. Therefore this slice
does **not** establish a reliably green long-frame gate on the 42dd5dd15
renderer base: the earlier 568 ms pass is not promoted over the later failures.
Transport and duplicate admissions are fixed; the residual timing failure is
handed back explicitly, with whole texture/card work and the multi-type
reslice evidence. No threshold was widened and no extra Rust optimization
was added to manufacture a green result.

Validation on the final code: 52 focused browser tests plus four watcher/CPU
workbench tests passed; typecheck and lint passed (one existing lint warning).
All 29 requested presentation tests and 44 renderer tests passed, as did the
native and WASM checks and WASM build. Armory, the workbench's explicit same-name
re-bake (120 to 144 triangles, changed canvas pixels), and grass retuning passed.
The red timing invocations and all commands/results are preserved in the scratch
slice report. The spec README's status and checklist are unchanged.

**Slice 08, on the merged tree, 2026-11-15.** With slice 04 merged too,
`battle-growth` at growth: one staging, one commit; 2 bundles encoded, sent
(7.9 MB) and decoded; 8.6 MB of vertices; 3 texture layers; no scenery work;
1 install. Every work check in the gate passes. The long-frame tripwire
fails: one frame of 735 ms at `battle staged` (824 ms before slice 04's
textures). The armory, workbench and stand-in scenes pass.

**Reslice, 2026-11-15.** The counts are the new art's and the frame is not,
so the time is in work nothing counts. Slice
[08a](slices/08a-the-staging-frame.md) times the frame's parts and removes
what still scales with the catalog; the first suspect, read from the code and
not timed, is the per-row rest pose and bounds that `Models::from_world`
derives for every row outside the geometry cache. Spreading work over frames
(the README's reslice trigger, which the two-type arrival also fired) is
decided after 08a's numbers: a frame that is long because of catalog-sized
work should lose that work, not be sliced thinner.

**Slice 11, stand-in look, 2026-11-15.**

- Live placeholders use the existing HUD side roles: `hud.enemy` for the
  opposing side and `hud.accent` for the observer's side, both owned by
  `fixtures/presentation.json` and read through `gameHud`. The shell passes
  those exact display-encoded values; presentation reuses `srgb_to_linear`
  for the model's linear tint. A new loading hue, a near-copy of either role,
  and a neutral colour with no side meaning were rejected.
- The form remains the existing opaque box primitive on the reserved row,
  scaled from slice 10's shared body box. Its neutral-white material takes
  the full instance tint, with metallic 1 and roughness 1. These are shading
  inputs, not a claim that the placeholder depicts a metal object. The fully
  rough response keeps the contact colour stronger than the pale matte
  versions without adding a shiny highlight, texture, emblem or animation.
  A model-shaped silhouette and an extra loading marker were rejected because
  the player chose a plain block. The hard swap remains unchanged.
- The chosen block casts no sun shadow. A solid block's large shadow gives
  an abstract representation the physical weight of scenery; removing that
  caster separates it from a heavy prop. It still receives world lighting
  and shadows, and keeps the existing unit fog and x-ray path. Authored units
  and lab models retain their casting policy; only the stand-in reads this
  optional appearance's policy. Shader code and the vertex layout did not
  change.
- The neutral `city_kit_prototype/unit_box` fallback for a known wreck is
  intentionally different. A dead hull is terrain cover, not a live unit
  carrying a combat-side selection target. Giving it the live contact colour
  would imply that it was still an active unit. Neither the wreck fallback
  nor any other prop fallback changed.
- Three candidates were captured with matched cameras: `contact-solid`
  (side colour, matte, shadow) was rejected for its prop-like mass;
  `contact-shadowless` (same matte surface without its caster) was rejected
  because the sun-facing faces remained pale and toy-like; `contact-metal`
  (fully rough metallic response, no caster) is selected for review because
  its faces retain the contact hue more clearly. Native shots, enlarged crops,
  and baseline/model comparisons remain in
  `throwaway/evidence/stand-in-look/`. The selected native frames, comparison
  sheet, strict pixel proof and verbatim fresh critique are retained in
  the look evidence (the side-coloured candidates' shots were removed when the player chose grey; they are in the history at `ffa774854`).
- Evidence uses the same paused body and camera before and after release:
  near/far views of an open-field vehicle, a vehicle at its actual sight
  boundary, and a squad at a fog edge. The squad's open control uses the
  existing lab fog suppression without changing the observed bodies. Native
  fog probes include both seen and unseen ground around the boundary views;
  the open-field vehicle samples are all seen. Moving the subjects between
  open and fog controls was rejected because it would confound the comparison.
- A strict production-route pixel check projects the physical body and its
  old shadow with the existing camera owner and the fixture's sun. Every
  changed held-frame RGB pixel is inside that region. The after-swap scene
  pixels are identical; only the existing FPS digits differ. Later meeting
  captures also have timing-dependent smoke and pose variation between runs,
  so they are not claimed as bit-identical after frames. No scene baseline
  was blessed, and no Preview checkpoint was opened; the player owns review.
- The unprimed reading recognises a deliberate side proxy and clear
  positional correspondence, but does not prove that a still image says
  specifically that art is loading. It also retains crate/post ambiguity,
  an apparent separation at the street fog boundary, and uncertainty where
  smoke hides infantry bases. These remain review limitations, not a claim
  of full visual acceptance. The physical envelope and unchanged ground
  position are fixed by slice 10; adding vehicle/infantry shape or a loading
  cue would exceed the plain-block choice. Smoke, density and other shadows
  are out of scope. The report's observation that the material does not
  visibly read as metal needs no change: a schematic side proxy is the goal,
  not a metal object. All findings are kept verbatim with the evidence.
- Model batching now owns its posed records, record runs and casting policy in a dedicated
  module. The file-size hook refused growth in frame orchestration; a
  behavior-preserving split was committed before the look change. Shortening
  comments or exempting the oversized file was rejected.

**Slice 11's review checkpoint, 2026-11-15.** The accepted look is a plain
block in the HUD's own side colour (`hud.enemy` for an enemy, `hud.accent`
for the player's side), fully rough, casting no shadow. The player was shown
it with the two rejected candidates and the choice of the paler block or a
neutral grey; the plan carried on meanwhile. Resolution, by the agent on the
player's behalf (something on screen over nothing; small taste questions):
keep the side-coloured block.

- Why not the mock's grey: a neutral block sits beside a wreck's neutral box
  and scenery, and the slice forbids a block that can be taken for scenery.
  The side colour is the one the HUD already uses for that side's contacts.
- What is not proven: an unprimed reader accepted its visibility and its
  match to the model's place, and still could not tell a temporary unit from
  a coloured crate. A saturated block is also easy to notice for the moment
  it shows, against the goal that a type's arrival goes unnoticed. Both are
  reversible by one colour and one flag in the battle input; the preload
  (slice 09) is what makes the block rare.
- No scene baseline is blessed for it. Slice 10's mechanics tests and the
  `stand-in` scene are the gate until a person who has looked blesses one.
**Slice 10, foreground block investigation and integration, 2026-11-15.**

- The second block in the vehicle close shot is a known `medium_wreck` of an
  enemy European Boxer, not the player's Abrams. The scene's observation places
  the wreck at `[1236.869873046875, 398.7654724121094, 0]`, with
  `wreckOf = europe_boxer_apc` and known-prop id `9897`. The own Abrams is at
  `[1352.411865234375, 706.9947509765625, 0]`, outside that close view. The
  nearby drawn-model and structure probes identify the foreground appearance
  as `city_kit_prototype`, state `unit_box`, with scale equal to twice the
  wreck's published half extents, the same physical hull size as the live
  Boxer; the foreground perspective makes it look larger. Its pale top and
  dark flank are the existing
  static prop material and lighting, not a blue-side unit cue.
- That box is selected by `static_world::props::Art::fit`: without the unit's
  installed wreck appearance, the known static wreck takes the existing prop
  stand-in kit, fitted to its simulation box. When wreck art arrives it takes
  the Boxer's wreck model. This static prop has no live-unit selection pick.
  It is outside the live-body stand-in contract, so changing or restyling the
  prop fallback was rejected.
- The battle view supplies `Frame.picks` and Rust's presented frame, with no
  browser proxy instances. `proxyModels` converts explicit lab `SceneInstance`
  primitives (excluding pick-only infantry); it is not a missing-unit-art
  fallback, does not resolve own versus identified units, and does not handle
  the battle's purchase or order ghosts. Those ghosts already go through Rust.
  A battle body is therefore not drawn by both mechanisms. Live unit stand-ins
  still use the shared presentation body box; lab primitive picks use their
  separate tooling shape only where a lab supplied no frame picks.
- `the-vehicle-bodies.json.models` intentionally filtered for
  `__battle_stand_in__`, so it named only the live Boxer. The full read-only
  frame probe does report the prop kit whenever its static instances are
  published, and the existing `structures` probe reports the retained prop.
  The scene now saves both nearby model names and nearby structures before
  and after release. Inferring a unit identity from colour or a filtered
  stand-in count was rejected; the probes and observed positions establish it.
- Upstream through `42dd5dd15` (including slices 03, 05a, 06 and 07) merged
  without textual conflicts. Ground admission stays only in
  `presentation/ground.rs`, and loaded frame construction stays only in
  `presentation/frame.rs`. The upstream spec README and both sets of ledger
  entries are retained. The stand-in test adopts slice 07's explicit wanted
  names plus facts API; no compatibility wrapper is added. The generated
  stand-in is an ordinary `ModelAsset` with its cached `ContentId`, so the
  existing `prepareAppearances` loop declares it before supplying bytes. A
  special stand-in declaration path was rejected as duplicate admission.

**Slice 10, verified on the merged tree, 2026-11-15.** The `stand-in` scene
passes: with the new art's responses held, the identified vehicle and each
seen soldier are drawn as one stand-in at the body's size and place, both
take an attack order by a click, and after the release the models replace
them at the same positions with no stand-in left.

- The second block in the vehicle shot is an earlier vehicle's wreck. A wreck
  whose art has not arrived is drawn as a box by the map props' own fallback
  (`static_world/props.rs`), which predates this plan and has no unit pick.
  The browser's proxy path draws nothing in a battle. So live units have one
  owner, the stand-in.
- An unprimed reading of
  [the four shots](assets/stand-in/unprimed-critique.md): position, heading
  and shadow match between each block and its model; the blocks are slightly
  larger than the art they replace (the body box, not the mesh); nothing
  floats or sinks. Its look findings go to slice 11: the default block reads
  as a model that failed to load, and the unit block and the wreck's box are
  different colours for no reason a player can see.

**Slice 05, 2026-11-15.**

- Runtime cards are addressed by `CardKey`: the canonical tier-0 `GeometryKey`
  (content, state, baked corpse palette and track signature), exact rendered
  palette bytes, serialized full bake `Spec`, and the two source model arrays'
  width, height and mip count. Living geometry is unposed, but its bake applies
  the rest palette, so the geometry key alone is insufficient. Array shape
  affects filtered pixels, as 04 confirmed. Rejected: names, appearance rows,
  name-bearing `Atlas.hash`, or geometry identity without these pixel inputs.
  The synthetic changed-palette test confirms geometry hits while cards miss.
- Content-owned authored material rows have identical values across generations:
  content fixes material factors, append-only material slots fix bases and
  texture slots fix channel layers. Only unrelated rows/holes change. The bake
  reads the immutable authored-material snapshot, never live diagnostic controls.
  Pose-derived bounds determine framing; the instance is untinted at identity,
  and the bake pipeline is fixed within the scene/device. No other varying
  generation input was found. The existing cross-platform hash remains unchanged
  and includes the requested appearance label, independently of shared routing.
- The model array owner is generalized, not duplicated: `LayerArray<K>` owns
  append-only slots and weak allocation/written-key records; `SharedArray`
  owns one or more format-specific planes of one shape and capacity. Models
  have one plane per channel; cards have two RGBA8 planes at the runtime atlas
  size, with one mip. Final shared-owner drop destroys all planes. `Cards`
  destroys only its generation-local frame and instance buffers, and rebuilds
  frame/routing tables against stable layers. Rejected: generation-owned card
  textures, a second card-array allocator, and a strong scene cache that retains
  retired GPU data.
- Card growth uses the same policy's **one spare layer** after the required
  address range, capped by the device limit. This keeps overlap modest rather
  than doubling capacity or reserving a batch of unused full atlases. Existing
  planes copy GPU-side, with no readback/re-upload of old cards; admissions that
  fit allocate no new card planes. Capacity reports use `ArtStorePolicy.report`
  with kind, value, bound, owner and outcome. `cardBytesCopied` and
  `cardBytesUploaded` distinguish these operations from model-texture counters.
- Dropped bakes are not cancelled. Their reservations remain; written layers are
  reusable while any generation holds that allocation. A dropped grown array
  with no other holder is destroyed, so a retry bakes its missing keys again.
  Rejected: address recycling, retaining CPU atlases, retaining GPU arrays
  strongly, and a second cancellation/cleanup protocol. Card dead addresses join
  the existing **256** dead-layer bound across all model/card pools; historical
  exhaustion and dead slots use the same fresh-store preflight. An active set
  beyond the device limit errors before allocations. A recreated device starts
  with the same admission and an empty store.
- Only missing representative keys enter 05a's memory-bounded batches. Alias
  rows route to one layer, while cache hits require neither read waits nor an
  authored-material upload. Batch ownership, split reporting and the one
  authored-material upload per stage remain in place. Review atlas requests can
  inspect a resident runtime layer or force a fresh configurable bake. Their
  snapshots own readbacks rather than scene borrows; the returned layer is
  separate from the unchanged pixel hash. Rejected: test-only layer accessors
  and retaining CPU pixels merely to prove reuse. Resident normal alpha stores
  tint, not coverage: readback reconstructs the original channels directly,
  without filtering them again. A zero-tint regression was observed red and
  fixed before acceptance.
- Production `ResourceMemory` measured the synthetic card lifetime scenario
  below. The test sets two spare layers so one content's rest/corpse pair fits;
  production keeps one. Figures include the tiny shared model placeholders and
  generation-local buffers, not driver residency or temporary bake targets.
  The latter still obey 05a's target/readback byte bound.

  | State | Buffers | Buffer bytes | Textures | Texture bytes |
  | --- | ---: | ---: | ---: | ---: |
  | Live | 10 | 1,440 | 4 | 2,097,160 |
  | Fitting overlap | 20 | 3,072 | 4 | 2,097,160 |
  | Growing overlap (live, fitting and grown holders) | 30 | 4,896 | 6 | 6,291,464 |
  | After retiring old holders | 18 | 2,784 | 4 | 4,194,312 |
  | Final holder dropped | 0 | 0 | 0 | 0 |

  The fitting case adds no texture allocation. Growth holds at most the source
  pair plus destination pair; retiring source holders releases the old pair.
  No cache holds GPU payload after the final generation retires. Expectations
  are relational and fixture-derived; these are observations, not pinned values.
- Both requested `battle-growth` commands pass. Growth changes from **189 cards,
  21 read waits** to **0 cards, 0 waits**, with zero card-upload/copy bytes. That
  arrival adds vehicle/wreck art with no eligible card keys. The synthetic
  positive-delta test goes from six bakes to the new content's two keys; shared
  aliases bake once. Card-distance committed pixels equal a clean runtime and
  resident atlases equal independent fresh bakes, including all channels and
  their hashes. The scene's own profile and the high environment both resolve
  this battle route to high (the runner's quality URL override applies to labs;
  an unset battle preference resolves to high).
- One high-graphics run of the reference probe recorded these page-timeline
  staging-to-commit intervals. The scratch runner/fixture and probe copy are not
  committed. Generation 3 was admitted twice; the second did no new core art
  work but still paid page transport. No admission here ran a fresh-store rebuild.

  | Generation | Appearance count | Staged ms | Committed ms | Wait ms | Cards baked | Read batches |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: |
  | 3 | 145 | 27,154 | 27,633 | 479 | 0 | 0 |
  | 3 (repeated) | 145 | 28,114 | 28,637 | 523 | 0 | 0 |
  | 4 | 147 | 42,232 | 42,762 | 530 | 0 | 0 |
  | 5 | 153 | 49,640 | 50,748 | 1,108 | 9 | 1 |
  | 6 | 155 | 78,476 | 79,788 | 1,312 | 0 | 0 |
  | 7 | 157 | 129,712 | 130,927 | 1,215 | 0 | 0 |
  | 8 | 166 | 135,622 | 137,374 | 1,752 | 27 | 3 |

  Nine new cards upload 4,718,592 bytes and copy 84,934,656 old bytes GPU-side;
  27 upload 14,155,776 and copy 89,653,248. The waits remain visible. They include
  page-wide transport and new geometry as well as cards: zero-card arrivals also
  take over a second. The page still encodes/sends the whole declared catalog in
  this checkout, which slice 08 changes. This is measurement for preload depth
  (09), not a reason to add another engine scheduling mechanism. The stand-in
  covers uncommitted art meanwhile.

**Slice 05, verified on the merged tree, 2026-11-15.** `battle-growth` at
growth, every engine slice and slice 08 together: one staging, one commit
0.4 s later; 2 bundles encoded, sent and decoded (7.9 MB); 8.6 MB of vertices
packed; 3 texture layers written; 0 cards baked (the arriving vehicle and its
wreck have none); no scenery work; 1 install. The longest frame is 465 ms,
under the gate's 700 ms tripwire (735 ms before card reuse, 2.5–2.9 s at the
start). The whole gate passes.

- The frame is still long for a player. Slice 08a times what is left.
- Still copied GPU-side at a growth: 40.5 MB of model texture layers here,
  and 85–90 MB of card layers when an arrival brings cards (slice 05's
  reference probe). One spare layer is the policy for both; slice 08a says
  whether the copies show in a frame.
- Slice 05 gave the review query a way to read a resident card back
  (`AtlasSource::PreferResident`), so its test could compare a reused layer
  with a fresh bake without a test-only accessor. It is production surface
  added for a proof; the whole-spec review decides whether it stays.

**Slice 08a, 2026-11-15.**

- Temporary browser-clock timers falsified the suspected cause: per-row skinning
  and bounds scans took 289–463 ms per growth although the GPU geometry hit the
  store. The page's requirements memo runs at initialization; its closure on
  learned art takes 0.07–0.16 ms. There was one request per arrival. Wanted assets
  and the content key together take less than 0.5 ms. Rejected: a page-side cache
  for these small tables, timers/debouncing, a worker or work split across frames.
- `ModelGeometry`, held through the existing weak `GeometryKey` index, owns
  packed tiers and a weak index of immutable `RestPose` facts. Rows share both
  with `Arc`. Geometry keeps the original content/state/baked-pose/tracks key;
  its rest facts additionally distinguish exact f32 palette bytes. A living
  far-pose change derives new bounds without repacking the unposed mesh, as the
  merged card test requires. Rejected: adding the living palette to the packed
  geometry key (it spuriously repacks), a second scene cache or strong retention
  of retired facts. Routing labels stay on rows, so aliases share facts while
  review atlases keep the requested appearance name. Final row/geometry drops
  release both payloads; no retirement protocol was added.
- `restPosesDerived` counts each actual rest-geometry scan/skin, on the miss where
  it occurs. Static state aliases and the two scenery placement routes share
  facts. The gate derives this delta from installed contents, states and sampled
  authored palettes, alongside exact packed-vertex bytes. Texture ids come only
  from referenced material channels and are deduplicated across contents/channels.
  Cards use distinct content/pose keys (sampled f32 palette bytes, not clip labels)
  and both source array shapes. The runtime spec is fixed during the scene.
  Zero-card deltas require zero read waits. No production inspection hook was
  added: installed names and the existing CPU pose owner supply the evidence.
- Geometry misses retain all tiers. Fresh/shared committed pixels and every
  returned atlas field, channel and hash match. The new growth regression was
  red with five derivations versus its fixture's one new row; an alias admission
  was red with four versus zero. Existing assertions are unchanged, including
  slice 05's changed-palette test and slice 03's geometry/memory tests.

Timing method: browser `performance.now()` around synchronous page/WASM phases,
with exclusive nested row intervals. Long Animation Frame entries supply the
whole frame; the last row is its remainder after the measured intervals (other
observation/React scripts and browser gaps), not an attribution to Rust. Values
below add to that frame; displayed rounding can differ by a few hundredths.
GPU copies use asynchronous timestamp queries around the copy commands, without
waiting on the CPU; those GPU intervals overlap CPU work and are not added to
CPU-frame totals. Commit/row-read costs are separate. All probes run serially
under the shared GPU lock at high graphics; scratch timers and registrations are
removed. The fixture's existing serial flag and 700 ms tripwire stay unchanged.

Before: accepted slice 08 tree `cd29d94a7` (04 textures, 05a batches, no 05 reuse).
After: merged `9f478bc70` (05 and 11) plus 08a. Thus the rest-derivation intervals
and new counter isolate 08a; staging-to-commit also benefits from 05 and is not
claimed as 08a alone. The small sample repeats the same one-type arrival in
three fresh pages. Reference arrivals use the supplied reference probe; a
scratch network hold coalesces two vehicle kinds for the two-type sample.
| Exclusive wall ms | small 1 | small 2 | small 3 | reference 1 | reference 2 | reference 3 | multi 1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Page battle JSON/binding | 14.64 | 17.23 | 15.63 | 17.95 | 15.41 | 15.98 | 19.44 |
| Fielding inside this frame | 0.00 | 0.00 | 18.85 | 0.00 | 18.14 | 0.00 | 0.00 |
| Presentation stage_battle | 1.16 | 1.49 | 1.23 | 1.47 | 1.30 | 1.19 | 1.29 |
| Wanted assets / content key | 0.21 | 0.25 | 0.22 | 0.27 | 0.25 | 0.35 | 0.45 |
| Missing query / declarations | 0.90 | 1.11 | 0.88 | 1.04 | 0.54 | 0.59 | 1.30 |
| Encode / supply missing art | 3.13 | 3.91 | 2.91 | 3.10 | 3.71 | 14.38 | 7.71 |
| Catalog facts | 0.17 | 0.22 | 0.21 | 0.19 | 0.08 | 0.07 | 0.23 |
| Catalog battle / rows / strings | 3.53 | 4.06 | 3.37 | 4.30 | 4.62 | 4.73 | 5.99 |
| Catalog statics | 3.28 | 3.72 | 3.61 | 5.25 | 4.42 | 4.44 | 6.36 |
| Content walk / store preflight | 0.50 | 0.65 | 0.55 | 0.66 | 0.61 | 0.62 | 0.81 |
| Materials | 1.03 | 0.48 | 0.44 | 0.47 | 0.31 | 0.45 | 0.72 |
| Rest palette / row routing | 2.94 | 3.21 | 1.91 | 2.46 | 2.39 | 3.61 | 3.94 |
| Rest facts (skin / bounds) | 424.76 | 363.52 | 289.42 | 302.14 | 305.17 | 463.08 | 388.29 |
| New geometry | 11.50 | 9.57 | 7.45 | 8.19 | 5.74 | 34.58 | 18.15 |
| Texture store (CPU) | 5.84 | 6.37 | 5.51 | 6.14 | 0.22 | 6.25 | 7.24 |
| Generation buffers / bindings | 0.06 | 0.07 | 0.05 | 0.06 | 0.06 | 0.10 | 0.10 |
| Scenery / grass carry | 0.31 | 0.39 | 0.29 | 0.34 | 0.10 | 0.10 | 0.48 |
| Source seam / request remainder | 1.86 | 2.39 | 1.99 | 2.20 | 1.05 | 1.12 | 2.73 |
| Card preflight / submit / bind | 0.56 | 0.57 | 0.66 | 0.55 | 0.57 | 0.89 | 0.62 |
| Canonical frame / render | 1.18 | 0.97 | 1.94 | 1.39 | 3.57 | 1.82 | 0.74 |
| Other browser / observation / React | 4.43 | 5.82 | 30.89 | 10.83 | 44.73 | 13.64 | 14.41 |
| **Frame total** | **482.00** | **426.00** | **388.00** | **369.00** | **413.00** | **568.00** | **481.00** |

After:

| Exclusive wall ms | small 1 | small 2 | small 3 | reference 1 | reference 2 | reference 3 | multi 1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Page battle JSON/binding | 13.84 | 17.25 | 13.62 | 14.83 | 14.39 | 15.65 | 20.13 |
| Fielding inside this frame | 0.00 | 0.00 | 0.00 | 0.00 | 0.00 | 27.87 | 0.00 |
| Presentation stage_battle | 1.18 | 1.37 | 1.13 | 1.19 | 1.17 | 1.14 | 1.53 |
| Wanted assets / content key | 0.19 | 0.25 | 0.21 | 0.23 | 0.24 | 0.23 | 0.41 |
| Missing query / declarations | 0.80 | 1.05 | 0.86 | 0.86 | 0.48 | 0.57 | 1.52 |
| Encode / supply missing art | 3.65 | 2.97 | 3.36 | 3.51 | 2.79 | 21.52 | 10.59 |
| Catalog facts | 0.16 | 0.20 | 0.16 | 0.18 | 0.07 | 0.08 | 0.27 |
| Catalog battle / rows / strings | 5.32 | 6.02 | 3.95 | 5.63 | 6.67 | 5.78 | 17.50 |
| Catalog statics | 3.17 | 3.36 | 3.22 | 4.72 | 4.20 | 4.26 | 5.90 |
| Content walk / store preflight | 1.15 | 1.31 | 1.04 | 1.16 | 1.23 | 1.23 | 2.05 |
| Materials | 0.44 | 0.49 | 0.39 | 0.42 | 0.30 | 0.52 | 0.61 |
| Rest palette / row routing | 1.62 | 2.22 | 1.51 | 1.86 | 2.04 | 1.92 | 2.89 |
| Rest facts (skin / bounds) | 0.52 | 0.53 | 0.50 | 0.52 | 0.38 | 9.50 | 1.06 |
| New geometry | 7.96 | 11.19 | 7.73 | 8.66 | 6.33 | 35.02 | 20.01 |
| Texture store (CPU) | 5.29 | 5.72 | 4.92 | 5.29 | 0.22 | 6.39 | 7.33 |
| Generation buffers / bindings | 0.06 | 0.08 | 0.06 | 0.06 | 0.09 | 0.06 | 0.09 |
| Scenery / grass carry | 0.28 | 0.34 | 0.28 | 0.30 | 0.11 | 0.10 | 0.46 |
| Source seam / request remainder | 1.76 | 2.36 | 1.68 | 1.97 | 1.13 | 0.98 | 3.33 |
| Card preflight / submit / bind | 1.12 | 1.32 | 1.10 | 1.19 | 1.31 | 1.94 | 1.95 |
| Canonical frame / render | 1.17 | 0.73 | 0.89 | 0.50 | 4.63 | 1.35 | 1.03 |
| Other browser / observation / React | 6.33 | 9.22 | 4.39 | 7.91 | 18.22 | 29.89 | 20.35 |
| **Frame total** | **56.00** | **68.00** | **51.00** | **61.00** | **66.00** | **166.00** | **119.00** |

The seventh column is the coalesced two-type arrival. The three reference
arrivals after the main comparison were also measured to include the larger
card-bearing arrival:

| Arrival | Staging frame, before → after ms | Staging → commit, before → after ms | After rest derivation ms | After cards baked |
| --- | ---: | ---: | ---: | ---: |
| Small 1 | 482 → 56 | 1,371 → 48 | 0.52 | 0 |
| Small 2 | 426 → 68 | 1,452 → 56 | 0.53 | 0 |
| Small 3 | 388 → 51 | 1,192 → 39 | 0.50 | 0 |
| Reference 1 | 369 → 61 | 2,352 → 49 | 0.52 | 0 |
| Reference 2 | 413 → 66 | 1,583 → 55 | 0.38 | 0 |
| Reference 3 | 568 → 166 | 2,163 → 173 | 9.50 | 9 |
| Reference 4 | not sampled → 127 | not sampled → 54 | 0.29 | 0 |
| Reference 5 | not sampled → 88 | not sampled → 78 | 0.59 | 0 |
| Reference 6 | not sampled → 399 | not sampled → 503 | 39.78 | 27 |
| Two-type arrival | 481 → 119 | 2,961 → 92 | 1.06 | 0 |

Every measured growth makes one staging, one request and one commit. Small
arrivals each send 2 bundles; reference sends 2, 2, 3, 2, 2, 9; the coalesced
arrival sends 4. These are observations, not pinned roster expectations.
The reference's third and sixth arrivals still exceed 100 ms, as does the
coalesced arrival. The sixth spends 141.24 ms packing **new** geometry,
39.78 ms deriving its new rest facts, 46.98 ms supplying/decoding new facts,
12.92 ms encoding, 60.78 ms fielding inside that frame, 22.09 ms in page battle
JSON/binding, 18.47 ms in catalog staging and 42.99 ms in other browser work.
That is a remaining new-art arrival stutter and fires the README's reslice
trigger; it is recorded here, not hidden with scheduling or a looser gate.

| GPU growth copy | Existing bytes copied | GPU elapsed ms | CPU encode + submit ms |
| --- | ---: | ---: | ---: |
| Small, linear model array (three runs) | 40,544,784 | 0.89 / 0.74 / 0.66 | 1.00 / 1.19 / 0.93 |
| Reference 1, model array | 50,680,980 | 0.82 | 1.06 |
| Reference 3, model arrays | 157,635,628 total | 2.91 + 0.69 | 0.72 + 1.06 |
| Reference 3, card arrays | 84,934,656 | 2.09 | 0.55 |
| Reference 6, card arrays | 89,653,248 | 12.26 | 0.64 |

The GPU intervals are real draw-queue work, including marker-pass overhead,
but queue submission does not wait for them on the main thread. They overlap
the frame and contribute GPU latency; they do not explain the former 0.3–0.5 s
CPU scan. One spare layer permits these copies. If array growth is the next
measured bottleneck, nine spare card layers fit the nine-key one-type arrival;
**27** fit the larger one-type arrival. Two spare layers in the linear model
array fit this vehicle's two new linear ids. The current shared spare policy
would need 27 to cover both card arrivals, reserving about 14 MB of unused card
planes plus the model-array spares. Rejected here: changing that policy without
an observed dominant copy stall, or doubling all array capacities.

Per-generation work retained deliberately: catalog facts/routing, battle rows,
strings and statics; content/card preflight and material rows; evaluated palettes
and row routing; small buffers and bind groups. The tables quantify them rather
than adding more caches. Commit takes 3.0–5.9 ms after the change; reading world
rows takes 0.38–0.90 ms. Before, those are 3.05–8.20 and 0.40–5.58 ms. The page
requirements pass has no growth-time rebuild, and the content key is called once
per admission. Before 05, all card submissions together took 13.3–19.7 ms spread
over the asynchronous batches; only 0.55–0.89 ms belonged to the staging frame.
After 05 the zero-delta arrivals submit no bake. No card bake or array policy is
changed by 08a.

The final uninstrumented `battle-growth` gate measures a **54 ms** staging
frame and **43 ms** staging-to-commit. The unchanged 700 ms tripwire passes.
Growth work is 2 decoded/encoded bundles (7,942,032 source bytes), 8,648,160
packed vertex bytes, **5** derived rest facts, **3** referenced texture layers,
0 texture-array rebuilds, 0 new cards/read waits, 0 scenery placements/static
uploads, and 1 page install. Existing model layers copy 40,544,784 bytes on the
GPU. The reference model-array copies were larger (50.7 MB and 157.6 MB across
both channels), reflecting the observed resident generations; neither is pinned
to the small fixture's 40.5 MB. The texture and card delta assertions now occupy both marked positions;
neither is a pinned count. This has no whole-catalog texture or card bake left.

All 64 named renderer/lib tests and 62 selected web tests pass; native-client,
WASM check/build, typecheck and lint pass. Scratch-probe failures exposed a
console-listener close race and an incorrect skeleton-container reader in the
new gate; both were corrected and rerun. A web test started before the clean
WASM build completed read the scratch profiling artifact; rerunning after that
build passed with no fallback. The report in `throwaway/slice08a-report.md`
records the exact commands, errors and evidence. The source retains no timers
or new production inspection surface.

The uninstrumented scenes `battle-growth`, `stand-in`, `armory` and `workbench`
all pass, including the armory camera extremes and workbench same-name re-bake,
stale/failed preparation and departure checks.

**Slice 08a, verified on the merged tree, 2026-11-15.** `battle-growth`: the
longest frame at the growth is 55 ms; the gate passes with its texture, card
and rest-pose assertions and its unchanged 700 ms tripwire. The armory,
workbench and stand-in scenes pass.

**The reslice trigger, open.** After 08a nothing measured in the staging
frame scales with the catalog. The reference battle's largest arrival still
took one frame of 399 ms: 141 ms packing new geometry, 61 ms fielding, 47 ms
decoding, 40 ms deriving new rest poses, the rest small. Two ways to make
that unnoticed: spread an arrival's new art over frames (the README's
trigger), or have it done before first contact (slice 09's preload, where
each step is one type). Decided when slice 09's numbers are in: if the
preload's own steps hitch, spreading is needed for the preload as much as for
first contact.

**Closeout fact, confirmed 2026-11-15.** The archived note that loading the
whole catalog exhausted WASM address space predates texture sharing: the note
was written in `966bde248` (2026-10-28 17:19) and decoded bundles began
sharing texture payloads in `e56cb964c` (22:43 the same day). The note is
corrected when the spec is archived.

**The reference battle on the finished engine, 2026-11-15.** The probe at
high graphics on the merged tree (every slice but 09), six growths, each one
staging and one commit:

| Growth | Staging to commit | Bundles sent and decoded | Vertices packed | Texture layers written | Cards baked | Copied GPU-side |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 92 ms | 2 | 8.6 MB | 3 | 0 | 51 MB |
| 2 | 57 ms | 2 | 5.6 MB | 0 | 0 | 0 |
| 3 | 216 ms | 3 | 28.6 MB | 3 | 9 | 243 MB |
| 4 | 50 ms | 2 | 4.5 MB | 3 | 0 | 52 MB |
| 5 | 47 ms | 2 | 8.6 MB | 1 | 0 | 0 |
| 6 | 443 ms | 9 | 85.7 MB | 0 | 27 | 90 MB |

- At the start each growth froze for 2.6–3.6 s and committed 62–103 s later,
  with 189–207 cards baked and 216–228 bundles sent.
- No frame of a second or more occurred after the first scene. The page's
  timeline records only frames that long, so the probe cannot give the
  shorter ones; slice 08a's timers measured 61–166 ms for typical arrivals
  and 399 ms for the sixth.
- One install and the new type's files only at every growth.
- The GPU-side copies are the arrays growing past one spare layer. They are
  not in the frame's CPU time; whether they cost the GPU a frame is not
  measured.

**The stand-in's colour, the player's decision, 2026-11-15.** Shown the
side-coloured block, the player chose neutral grey. The block is now one
colour for every side, the HUD's existing `hud.dim`; the side colours and the
observer they needed are removed from the battle input. Still fully rough and
casting no shadow. This supersedes the checkpoint's provisional resolution
above.

**Pre-ship review, 2026-11-15.** An independent reading of the whole diff
against staging ([findings](assets/pre-ship-review.md)) found nothing that
blocks shipping and nothing wrong in the shared GPU lifetimes, reachable
panics or leftovers. Its four "fix soon" findings:

| Finding | State |
| --- | --- |
| The fielding owner compared the runtime catalog through the one library object, so a catalog reloaded inside a battle kept the old Rust fielding | Fixed: the owner keeps the catalog object it was made from |
| A card's key includes the model texture arrays' edge, so a new type with a larger texture retires every card layer and rebuilds that array; two such arrivals in one battle would pass the dead-layer bound and start a fresh store mid-battle. The ledger's "monotonic growth creates no dead layers" is wrong for cards | Closed without a change: today's art cannot reach it. Every unit texture in the runtime catalog is 256 px or smaller (458 textures read: sRGB 128 ×22, 256 ×136, 512 ×2; linear 128 ×44, 256 ×254), and the two 512 px textures belong to the 16 city building kits, which are map art present from the first scene. A battle's growth is unit types, so it cannot raise an array's edge. A scene constant would also cost every map without those kits four times its colour layers. If a larger unit texture is ever authored, the array rebuild already reports itself (`art-store tripwire kind=TextureShape … outcome=array-rebuild`) and `battle-growth` fails on `textureArraysRebuilt`. The sentence "monotonic growth creates no dead layers" holds for the catalog as it is, not by construction |
| `AtlasSource::PreferResident` is reached only by the card-reuse tests; no page asks for the runtime spec | Fixed: the resident read-back is deleted (`AtlasSource`, the resident request, the channel reconstruction, the `layer` result and the frame table kept only for it, about 110 lines). Card reuse is proved by what it is for: a frame drawn from reused cards equals a clean bake's, growth bakes only new keys, and two names with one content bake once. The test of zero-tint normals went with it, since it tested the read-back's own reconstruction; a stored card's normals and tint are still checked through drawn cards in `tests/impostors.rs` |
| The scenery "needs placement" guard can no longer be true, and its only caller is a test | Fixed: `needs_scenery`, the `carried` field and the commit check that could not fire are deleted |

Its notes, 2026-11-15: the layer array now carries its own label, which
removes one of the two new `too_many_arguments` exemptions; the other (the
grass admission) says what blocks its split. The renderer READMEs' store
sections are rewritten as four rules (identity is content, addresses are
append-only, the last holder destroys, a full rebuild is the same admission)
instead of a walk through the code. Left as recorded notes: one batched widen
fails for every caller in it (one caller today), the fielding owner made in
`useMemo` (development only), and the hull-sized pick box in shells that pass
no stand-in (native capture, showcase).

**The engine's memory, 2026-11-15, found open (fixed below).** Slice 09's memory
report measured the engine without the preload against the base, in the
reference battle at deployment: 7.25 GB settled against 6.25 GB, all of the
difference in the renderer process (4.5 GB against 3.5 GB); the GPU process
is unchanged. That breaks decision 3, and the page's WASM modules have a
4 GB ceiling. Attribution (which part of the page, which slice) is in
progress; the engine is not pushed until it is found and fixed.
**The engine's memory, fixed, 2026-11-15.** Two causes, both in how the page
hands bytes to Rust, neither a leak:

- At the first scene the page encoded every model's transport packet into one
  list before either reader took any of it, and nothing released the list. The
  browser's collector had no reason to run, so about 0.9 GB sat in the page
  until it did. Each packet is now encoded, read by the renderer and by
  presentation, and released before the next (`releaseTransport`,
  `web/src/battle/transport.ts`); growth and fielding release theirs the same
  way. A released buffer is only ever one the caller just made: cached
  textures and skeletons keep their owners.
- Fielding copied every source file a second time inside WASM memory, which
  never shrinks. It now borrows the bytes the page passed in
  (`fielded::Read` returns a borrow or an owned buffer).

Whole-browser footprint, reference battle at deployment, high graphics, three
interleaved runs each, same harness and base as the report above (GB):

| | Settled, each run | Mean | Page buffers | Renderer WASM |
| --- | --- | ---: | ---: | ---: |
| Base `c61374e29` | 6.22 / 6.20 / 6.18 | 6.20 | 0.95 | 1.28 |
| Staging `3041e1cba` | 7.01 / 7.04 / 7.42 | 7.16 | 1.7–2.0 | 1.41 |
| Fielding release and borrow only | 7.03 / 6.96 / 6.99 | 7.00 | 1.8–1.9 | 1.21 |
| The whole fix | 6.02 / 6.03 / 6.01 | 6.02 | 0.95 | 1.21 |

Every run of the fix is below every run of the base, and the simulation
digest is the same in all twelve. The third row was built to see whether the
per-packet release at admission could be left out: it cannot, that is where
the gigabyte was. What the fielding release adds beside it was not isolated.
The harness borrowed the main checkout's map presets, which had moved past
the base's game module; it is pinned to the base's file now. Raw runs are in
the scratch folder (`throwaway/memory-fix/`), not the spec.


**The GPU preload is cancelled, 2026-11-15 (the player).** Shown the preload
behind its off-by-default switch, the player's answer was: no configuration;
what would be left off by default is not built; this is a zero-config game.
That supersedes decisions 2, 5, 6 and 13 where they describe a GPU preload,
and the slice's switch. Chosen between on-for-everyone and not-built, by the
agent on the measurements: not built. With the engine a one-type arrival is a
54 ms frame and a commit about 50 ms after staging, behind a stand-in, which
is decision 13's own condition for a download-only preload; the GPU preload
cost 3.0 GB settled (7.37 → 10.41 GB) and its WASM memory against the 4 GB
ceiling was never measured. Kept from the slice: the download warm-up, on for
every battle. The largest arrival's 399 ms frame, which the preload would
have hidden, is now slice 09's second half. Commit `012834b8d` holds the
cancelled code and its raw memory runs; neither is merged. If the player
wants the enemy faction on the GPU for everyone after all, that commit's
queue is the starting point and the WASM ceiling is the first thing to
measure.

**Growth scene contact after the staging merge, 2026-11-15.**

- The fixed links remain unchanged. On the moved map the farthest field is
  not a reliable meeting point. Both original purchases and moves are accepted;
  the European tank reaches the field, but its held vehicle and infantry are
  never co-visible before it dies. The Eastern tank dies to a BTR before it
  identifies new art: its last live trace has 500 m sight while the firing
  contact is over 550 m away even allowing its uncertainty. Navigation is
  progressing normally; no game regression was found in these failures.
- The scene actor uses a real recon vehicle through the ordinary purchase UI,
  initially visiting the objective nearest the reported main settlement. Once
  that move completes, it visits the nearest objective the blue observation
  reports red capturing or holding. The European failing-session replay shows
  the assault squad unloading at that field rather than the junction. Rejected:
  a new seed, hard-coded world points, reading hidden red units to drive the
  pilot, and waiting forever at one objective. The existing play limit remains.
  The shared actor in `web/scenes/_contact-battle.mjs` owns the approach for
  both fixtures; the preload scene should reuse it instead of the old tank/
  farthest-field approach.
- A second fixture assumption was a body click framed around the squad anchor.
  The outer member projected onto the army HUD, which correctly consumed the
  click despite the geometric picker finding the soldier below it. Camera and
  mouse now share the same chosen outer body. The actual mouse attack, target
  id and acknowledgement assertions are unchanged, as are the physical block,
  position, in-session swap and no-remaining-block assertions. The player's
  neutral `StandIn.tint` / `hud.dim` decision is unchanged.
- Missing growth leaves failed checks and a named `the battle never grew`
  diagnostic with the latest observation, rather than entering header analysis
  with undefined installed names. A short-sighted-tank control exercised this
  failure path. Growth work, one staging/commit, texture/card/rest derivation
  checks and the 700 ms tripwire are unchanged. The pilot stops advancing at
  the first drawn staging, rather than stepping through another minute's
  possible arrivals.
- Verification on `011977f72`: rebuilt WASM, typecheck and lint passed. Under
  the shared GPU lock, all four requested scenes passed with fixture concurrency
  one; an earlier concurrent run had timed out opening workbench's second page
  while armory was running, without a page error. No readiness timeout changed.
  The final growth/stand-in rerun passed again: staging to commit was 54 ms,
  longest overlapping long frame 66 ms against the unchanged 700 ms tripwire.
  Growth encoded/decoded 2 bundles, packed 8,648,160 vertex bytes, derived 5
  rest poses, wrote 3 texture layers with no array rebuild, baked/read 0 cards,
  did no scenery placement/static upload, and installed the page library once.
  The four requested stand-in evidence shots were refreshed with the accepted
  neutral tint. No production code changed.

**The stand-in's colour, second answer, 2026-11-15.** Shown `hud.dim` on
screen (a blue-grey), the player asked for a real grey. The block is a matte
grey with no hue, `[0.5, 0.5, 0.5]` display-encoded, defined beside the
block's mesh (`BATTLE_STAND_IN_TINT`). This is a new colour by the player's
choice, an exception to the plan's "an exact colour the UI already has". It
is not metallic any more: a metallic grey has no colour of its own to show.
The four shots in `assets/stand-in/` show it; lit by the scene it reads as
the same kind of box as a wreck awaiting its art.


**Slice 09, download warm-up, player contract change, 2026-11-15.**

- The game is zero-config. The GPU preload from `012834b8d` is rejected,
  including its fragment switch, in-flight roster steps, progress events,
  fixture and scenes. None is carried to `t3/battle-growth-warm-up`.
  The measured memory evidence remains in `throwaway/`; no raw runs are
  committed here. The memory fix is being integrated separately and is not
  reimplemented or included in this pass.
- Download warm-up is unconditional for the public-faction player battle.
  Its existing viewport-ready callback runs after the first drawn frame,
  including deployment. Using it avoids another readiness feed or renderer
  publication. The menu retains its existing delayed warm-up, sharing the same
  downloader. Smaller encounter/capture labs with no public enemy do not
  start a whole-catalog download merely by using the shared session.
- The priority is learned kinds retained for the observing session, then enabled
  kinds in the public enemy faction, then the remaining runtime files. Each
  kind resolves through the loader’s dependency selection (including faction
  looks); files already started finish, unstarted files follow the current
  priority and each transport URL is claimed once. No second downloader,
  decoded-art cache, new wanted kind or admission is introduced by warming.
- `useBattleSession.ts` was already 40,868 bytes, above the 32,000-byte source
  budget. Wiring the readiness callback would grow it, so its existing art
  composition moves intact to `useBattleArt`: wanted kinds, complete source and
  fielding stay together. Comments move with that concept. `runtimePrefetch`
  owns the existing cache/file work; forwarding exports and comment deletion
  were rejected. The session shrinks while adding the readiness composition.
- Tests first: the direct-battle readiness/order/residency test fails before
  the warm-up module exists, then passes without any fragment, roster pin or
  GPU mock. A deferred-fetch test checks a newly learned type takes the next
  unstarted download and retains priority when its contact expires. Existing
  prefetch departure/network/cache-cleanup tests consume the new owner directly.
- Large-arrival confirmation is being run before any admission policy change.
  The supplied scratch `probe-on.sh`/`probe-off.sh` do not exist on this machine;
  the committed probe’s own instructions register it temporarily instead.
  Its scratch extension records observed kinds and installed appearance names
  alongside each staging/commit, plus subsecond browser long-frame entries.
  The temporary fixture, scene and observations stay out of commits.

**Slice 09, large-arrival confirmation, before changing admission, 2026-11-15.**

- The nine-bundle, 27-card growth is **one type**,
  `eastern_rpg_team_rpg_7`, not several unit types learned together. In the
  reference battle on `3041e1cba` plus download warm-up it is growth four,
  not the historical sixth: the order changed with staging's battle/map code.
  The nine installed names are that type's `active_{a,b,c}`,
  `carried_{a,b,c}` and `guard_{a,b,c}` appearances. The admission decodes
  nine bundles, packs 85,739,040 vertex bytes and bakes 27 cards. No other
  appearance joins that commit. The observation first identifies the RPG team
  at its staging; a simultaneous own mortar description is already in the
  player's purchase roster and adds no art.
- Scratch evidence: `throwaway/evidence/growth-probe/reference-arrivals.json`
  records the side's observation and actual installed names on each event.
  The uninstrumented high-graphics run has six stages and six commits. The
  RPG-team stage-to-commit interval is 505 ms. Its admission frame is 285 ms
  (264 ms in the viewport loop, 11 ms in the React task, 10 ms browser/remainder);
  the preceding fielding/transport frame is 150 ms (140 ms Promise task).
  These browser-script intervals are not exclusive Rust phase attribution.
  The historical 399 ms breakdown remains the earlier ledger measurement:
  141 ms new geometry, 61 ms fielding, 47 ms decoding, 40 ms rest facts,
  and 110 ms other work. New nested binding measurements follow separately.
- The task's one-type branch applies. No learned-type pacing, spreading within
  one admission, worker move, switch or roster GPU preload is built. A type
  queue cannot divide this team's nine appearances. The next design decision
  is how to bound work within one type, outside this pass. Relabelling the
  current fourth growth as the sixth, or splitting units merely because the
  bundle count is nine, was rejected.

- Supplemental binding-only timing on the same single RPG-team arrival:
  357 ms whole frame, fielding 82.08 ms, `stage_battle` 13.90 ms,
  missing-content query/draft creation 0.32 ms, nine synchronous draft supplies
  43.17 ms, catalog/GPU preparation 171.38 ms, remainder 46.15 ms. The
  preparation interval contains geometry/rest/material work; it is not a
  measurement of geometry alone. Staging-to-commit is 475 ms; the commit call
  is 2.29 ms in its later 54 ms frame. These intervals were wrapped in the
  scratch scene around existing WASM methods, without production edits or
  a change to the observer. Evidence is
  `throwaway/evidence/growth-probe/reference-arrivals-profiled.json` and the
  exact scratch scene is `throwaway/warmup-reference-profiled.mjs`.
  No before/after optimization table or three-run comparison is claimed:
  the task explicitly stops at attribution for a single-type arrival.
- Verification: the rebuilt WASM, 21 focused web tests across four files,
  typecheck and lint pass. Lint retains the existing `unitGhosts.test.ts:150`
  no-explicit-any warning. High-graphics `battle-growth` and `stand-in` pass
  under the shared GPU lock. The growth's staging-to-commit is 73 ms; its
  overlapping long frame is 120 ms, below the unchanged 700 ms tripwire;
  every existing exact work assertion passes. Both reference probe runs finish
  six stages and six commits. The temporary scene and fixture are removed.
- Failure receipts, all in `throwaway/`: the initial missing-module test is
  red; priority fault injection fails both order assertions; isolated readiness
  fault injection fails on a pre-frame catalog fetch. A typecheck first refused
  a broad string passed to the faction union (TS2345/TS2322); the fake and
  comparison now use the real contract. Lint first found an unused import in
  the temporary scene; it was removed. One supplemental scene was invalidated
  by HMR during a fault-injection edit (`Execution context was destroyed`);
  it was not used as evidence, and the stable-source rerun above passes.
  No assertion, timeout, seed or acceptance threshold was relaxed.

**The warm-up as merged, 2026-11-15.** Three `battle-growth` runs with the
warm-up downloading in the background: the growth frame is 52, 52 and 51 ms,
against 54 ms without it, so it costs no smoothness there (the 120 ms in the
implementer's own run did not repeat). Slice 09's earlier memory runs had a
warm-up-only arm: 0.12 GB more settled than the engine alone. In the merge
the walk of "which kinds an observation shows" got one owner
(`observedKinds`), shared by the wanted set and the warm-up's order, and the
session computes the art's factions once for both. Noted, not changed: the
downloader also writes every file into Cache Storage (`battle-runtime-v1`),
which nothing reads back; the loader is served by the HTTP cache. It is how
the downloader remembers what it has fetched, and it is 0.83 GB of second
copy on disk. It predates this plan.

**Slice 12 added, 2026-11-15 (the README's reslice trigger).** The one hitch
left is a single type's art done in one frame. The slice warms the art store
a content per frame and then admits, so the admission path stays single.
Rejected: a worker (the renderer's WASM memory is not shared across threads
today, and that is a larger change than the hitch), and pacing by type (the
type is the unit that is too big). Its first confirm-first question may
replace it: if most of an infantry type's 85.7 MB is geometry posed for
bodies that do not exist yet, packing those when the first body lies is
smaller.

**Slice 12, first confirm-first answer stops implementation, 2026-11-15.**

- Based on `origin/staging` at `4df1c6b11`, on
  `t3/battle-growth-arrival-over-frames`. No production change was made.
  The first stop condition applies: **57,159,360 of the 85,739,040 packed
  vertex bytes (exactly two thirds) are two corpse-pose copies per bundle**.
  Living meshes total 28,579,680 bytes. There is no additional packed far-rest
  geometry: living rows share their unposed geometry, and the far pose keeps
  only its bounds/palette and supplies its card bake.

  All rows below use the `eastern_rpg_team_rpg_7_` prefix. Counts cover all
  four mesh tiers, using the production 48-byte vertex layout; index buffers,
  CPU rest facts and card pixels are not included in vertex bytes.

  | Bundle suffix | Living tier bytes | Corpse tier bytes | Living / corpse / extra far geometry keys |
  | --- | ---: | ---: | ---: |
  | active_a | 3,098,736 | 6,197,472 | 1 / 2 / 0 |
  | active_b | 2,929,776 | 5,859,552 | 1 / 2 / 0 |
  | active_c | 2,930,592 | 5,861,184 | 1 / 2 / 0 |
  | carried_a | 3,391,248 | 6,782,496 | 1 / 2 / 0 |
  | carried_b | 3,217,584 | 6,435,168 | 1 / 2 / 0 |
  | carried_c | 3,231,552 | 6,463,104 | 1 / 2 / 0 |
  | guard_a | 3,241,968 | 6,483,936 | 1 / 2 / 0 |
  | guard_b | 3,271,632 | 6,543,264 | 1 / 2 / 0 |
  | guard_c | 3,266,592 | 6,533,184 | 1 / 2 / 0 |
  | Total | 28,579,680 | 57,159,360 | 9 / 18 / 0 |

- Evidence: `throwaway/slice12-confirm/geometry.py`, `.json` and `.log`.
  The scratch reader inflates the nine committed runtime bundles, reads each
  tier's position-array length and the production vertex stride, and records
  their exact content identities, tier counts and authored poses. Every bundle
  authors `death` and `death_back` at phase 1. The current production key rule
  (`models/plan.rs`) adds a palette only for a corpse row, and
  `models/admission.rs` poses and packs every tier for those rows; living
  geometry remains unposed. `presentation/posing.rs::register_appearance`
  registers the living row and both corpse rows before a body lies.
  `models/rest_pose.rs` retains bounds and palettes, not another packed
  living far mesh.
- The calculated 85,739,040 bytes reconcile exactly with the actual reference
  admission in the preceding pass's
  `throwaway/evidence/growth-probe/reference-arrivals.json`: the commit adds
  exactly those nine names and bakes 27 cards. Its tick-5521 observation has
  three living RPG-team members and no RPG-team corpse. That is reused
  historical browser evidence from `3041e1cba` plus download warm-up, not a
  new run claimed on `4df1c6b11`. The nine runtime contents and the geometry
  key/admission code are unchanged between those trees. Simulation/mapgen are
  unchanged too; the intervening presentation fixture edit lengthens wreck
  smoke and does not affect these mesh sizes or poses.
- **Second confirm-first answer: not measured in this pass.** The task says
  to stop at the first answer if posed copies dominate. It is therefore not
  established that all fielding, supply and preparation costs divide by content,
  nor is a per-content/global-table timing invented from the previous aggregate
  measurements. No scratch timers, new reference run, stepping API, warm-store
  state, viewport scheduling or fielding/supply stepping was built. Continuing
  to implement slice 12 despite its stop condition was rejected; the next slice
  can address unnecessary corpse preparation instead, under the user's spec.
- No implementation tests, workspace Clippy, GPU tests/scenes or three-run
  before/after cohort were run: there is no implementation to verify after
  this explicit stop. The scratch calculation and its exact admission/observation
  reconciliation pass. Evidence stays in `throwaway/`; only this ledger entry
  changes. `df -h` showed 107 GiB free at setup, so no old measurement builds
  were removed. No switch, setting, shader, vertex layout, assertion, README
  status or slice text changed.

**Slice 13 replaces slice 12, 2026-11-15.** Slice 12's stop rule fired: two
thirds of an infantry type's packed geometry is corpse copies made before
anyone has died. Chosen: stop making the copies; a body draws the living mesh
held in its fall's last pose, with one shared palette per row. It removes the
work instead of moving it, and it removes the same copies from the first
scene and from GPU memory. Rejected for now: packing a pose's copy when its
first body lies. That keeps three geometries per appearance, moves the cost
to the middle of a firefight, and needs something to draw for a body found
already dead in the fog, which has no fall to wait behind. It stays the
fallback if bodies drawn skinned cost too much frame time at the corpse cap,
which the slice measures before building. Not built: slice 12's stepping.

**Slice 13, confirm first, 2026-11-15.**

- Corpses currently enter presentation's ordinary dynamic model list each
  frame, grouped by appearance row/tier/pass. They are not held static batches.
  Near and shadow draws use the existing model pipeline; a corpse's instance
  palette index is zero because admission already skins every mesh tier into
  a rigid copy. The corpse appearance row advertises zero joints. The living
  path needs retained authored joint indices/weights, the row's joint count,
  and an instance offset to a shared rest palette in Frame.palettes. The
  existing instance ABI and model shader already supply those operations.
- Corpse bounds/cards are not currently the unposed-mesh-plus-palette path:
  admission poses the finest mesh for bounds, retains an identity rest palette,
  and bakes the card from the posed copy with that identity. Living far bounds
  also scan a posed finest mesh, but retain the actual rest palette and bake
  with unposed geometry. The replacement must retain the corpse's actual rest
  palette, remove pose from geometry identity, and keep that rendered palette
  in card identity. Rest bounds can still be derived by the existing CPU skin;
  that transient scan is distinct from packing/uploading every posed tier.
- The third confirm-first measurement is pending: three baseline and candidate
  runs at the fixture's corpse cap, high graphics, using the existing renderer
  GPU timer and frame CPU duration. The minimal candidate and test oracles are
  built before finishing the implementation; the user-visible frame-time cost
  decides whether the rest proceeds. No frame-budget assertion is introduced.
- **Stopped before building the minimal candidate:** existing test
  `authored_clips_rest_corpses_and_supplied_palettes_share_the_frame_pose_contract`
  in `crates/presentation/tests/lab_models.rs:299` asserts the corpse instance's
  palette index is zero, with the message “corpse uses its baked resting mesh”.
  The same test establishes that corpse row's rest palette translates Z by 2;
  frame construction reserves palette zero for identity. With the requested
  unposed living geometry, keeping that assertion would draw the wrong picture.
  It pins the baked-copy implementation the new contract removes.
- The user explicitly requires every other existing assertion to stay unchanged
  and says to stop if one is wrong. No assertion was edited, weakened or worked
  around by changing identity, the shader, or another rendering path. The
  natural replacement would check that the corpse instance references its row's
  actual rest palette while preserving the existing ground-fog/no-xray checks,
  but that replacement requires the next instruction under the stated rule.
- Consequently the third confirm-first comparison, new regression tests,
  minimal skinned build, reference before/after, buffer/memory measurements and
  full implementation verification are not claimed. There is no production or
  interface change and nothing merge-ready about corpse rendering in this pass.
  The blocker and source evidence are recorded in `throwaway/slice13/report.md`.
  Disk had 103 GiB free at setup; no builds were removed. README and slice text
  are unchanged.
- Baseline check: `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=line-tables-only
  CARGO_PROFILE_TEST_DEBUG=line-tables-only cargo test -p presentation --test
  lab_models authored_clips_rest_corpses_and_supplied_palettes_share_the_frame_pose_contract`
  passes (one test). The contradictory assertion is therefore active coverage,
  not stale unreachable code. Receipt:
  `throwaway/slice13/existing-contract-test.log`.

**Slice 13, continuation: frozen living geometry, 2026-11-15.**

- The player authorized replacing mechanism assertions (baked corpse geometry,
  identity corpse palettes and zero corpse joints) with the new contract. Fog,
  x-ray, visibility, sinking, picks and the growth gate's other assertions stay.
  The accepted correction to confirm-first answer 2 is retained: cards/bounds
  currently use the posed copy plus identity; the replacement uses authored
  geometry plus the real palette, with CPU posing retained only for bounds.
- Confirm-first frame-cost gate: a scratch headless production `Runtime` draws
  the fixture's cap (1,000 bodies), distributed across all nine RPG-team
  appearances and both fall poses. High graphics: 1920×1080, 2048 shadows,
  2600 m reach, automatic unscaled unit detail. Every body is in view at mesh
  tier 1, no impostors; 6,967,545 model triangles. CPU measures presentation,
  prepare, encode and submit, excluding the profiling wait. GPU is the shared
  renderer's `FrameTimer`, native `AfterCompletion`; 60 warm frames then 240
  samples per fresh process. This deliberately dense local workload isolates
  the skinning cost; it is not a whole-battle browser FPS claim.
- Alternating baseline/candidate runs (all retained in `throwaway/slice13/`):

  | Run | Baseline CPU mean / p95 ms | Skinned CPU mean / p95 ms | Baseline GPU mean / p95 ms | Skinned GPU mean / p95 ms |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 0.381 / 0.409 | 1.297 / 1.880 | 21.745 / 21.824 | 22.261 / 24.557 |
  | 2 | 0.591 / 0.943 | 0.421 / 0.480 | 21.825 / 21.999 | 21.698 / 21.775 |
  | 3 | 0.438 / 0.735 | 0.447 / 0.460 | 21.755 / 21.863 | 21.683 / 21.772 |

  Candidate 1 overlapped a focused test compile and has extra CPU/GPU variance;
  it is retained, not replaced with a favorable repeat. Other host activity is
  not suppressed. Relative to the game's 45 FPS target (22.22 ms) and 60 Hz
  (16.67 ms), the repeated stable GPU results show no sustained frame penalty;
  CPU remains submillisecond in the later runs. The first run's outlier does
  not recur in the next two. Proceed with the slice. The old path already
  consumes roughly the entire 45 FPS budget at this deliberately dense cap.
- Minimal candidate proof before finishing: the new geometry test fails first
  (384 bytes rather than 192), then passes with identical mesh allocation
  bytes/counts to living-only admission. The near-frame oracle fails first
  with an identity corpse palette, then passes pixel-exact against the living
  row frozen with the corpse's real rest palette. Its independent card bakes
  also agree exactly (albedo, normal, mask, center and radius).
- Decision: keep a frame-local per-row palette cache in presentation's posing
  owner, used by battle and tooling; reject per-body sampling/appends and a
  retained cross-frame cache. Only rows actually drawn write palettes, and no
  state survives the frame. No WASM export or vertex ABI change is needed.
- Decision: keep `posed_mesh` for the CPU bounds scan; delete only pose-specific
  packing/upload/keying. Cards still key their actual rendered palette.

- **Measurement correction:** the cap table immediately above is pilot data,
  not the final high-profile gate. Review found the scratch probe used the
  synthetic test helper's LOD thresholds rather than `presentation.models`.
  Lighting/shadows matched high, but model detail did not. Repeat all three
  interleaved pairs with the authored detail, retaining the pilots and every
  sample. No acceptance claim relies on replacing that discrepancy silently.
- Review also found a shared-row composition case: battle and lab instances
  can route to one row. Their cache must therefore be owned by the outer frame
  builder, passed through both draws. A cache per caller was rejected because
  it could append that row's palette twice. Moving frame orchestration into
  the existing `presentation/frame.rs` keeps this owner together; the root
  `presentation.rs` was already beyond the size ratchet and could not grow.
  All moved comments survive verbatim; no public/WASM signature changes.

- **Corrected confirm-first gate, authored high profile:** the probe reads
  `presentation.models` directly, with the same high-quality light/shadows.
  All 1,000 bodies draw; tiers are 188 at tier 1 and 812 at tier 2, no cards,
  3,262,772 triangles. Three fresh, alternating pairs, no task compilation
  during these samples (240 frames after 60 warmup frames per process):

  | Run | Baseline CPU mean / p95 ms | Skinned CPU mean / p95 ms | Baseline GPU mean / p95 ms | Skinned GPU mean / p95 ms |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 0.617 / 0.722 | 0.689 / 0.838 | 16.233 / 16.284 | 16.151 / 16.201 |
  | 2 | 0.601 / 0.653 | 0.684 / 0.782 | 16.231 / 16.285 | 16.144 / 16.185 |
  | 3 | 0.699 / 0.928 | 0.638 / 0.666 | 16.239 / 16.319 | 16.136 / 16.180 |

  GPU mean is slightly lower in every pair; CPU changes are below a tenth of
  a millisecond in either direction. Both paths stay below 16.67 ms on the GPU
  and comfortably below the game's 22.22 ms target. Skinning does not introduce
  a frame-time cost a player would feel in this workload. Finish the slice.
  Raw samples and pictures: `throwaway/slice13/high-{baseline,candidate}-{1,2,3}/`.

- The corpse-versus-frozen-living oracle covers an actual near frame, independent
  authored card bakes (albedo/normal/mask and bounds), and relit far-card frames.
  They are pixel-exact, with nonempty controls. Palette-sharing coverage checks
  both actual sampled values and that the next frame drops rows no longer drawn;
  bypassing sharing makes the test fail (six palettes instead of three).
- Previous-card rounding was also measured on a real RPG appearance in both
  fall poses, through a scratch reproduction of the previous CPU-posed input
  (not a retained production branch). Albedo changed 203 / 301 pixels of each
  1024×256 atlas, mean absolute channel delta 0.002449 / 0.003929 (0–255).
  Normal deltas are 0.007004 / 0.010014; mask deltas 0.001728 / 0.002579.
  Centers/radii agree exactly. At alpha coverage >=32, the first silhouette
  agrees exactly; the second differs at one pixel, within one pixel of the
  other silhouette. CPU u16 skinning versus the living GPU path's unorm8
  weights and packed normal rounding account for this boundary difference.
  The contract's frozen-living oracle remains exact, with no tolerance added.
  Receipts/pairs: `throwaway/slice13/card-rounding/`.
- Unprimed picture review found no visible change in placement, scale,
  orientation, silhouettes, ground contact, lighting or shadows; the crops
  show no conspicuous missing limbs, stretched skinning or detached geometry,
  and the cards retain poses, framing and shading. Limitation retained: the
  dense cap workload hides interior ground contact and small geometry defects.
  Images do not prove hidden geometry; the independent frame/card oracles do.
  Full critique and its receipt check remain under `throwaway/slice13/`.

- Reference battle, three alternating frozen-production pairs at high graphics.
  The RPG team is one learned type (nine bundles, 27 cards), fourth growth in
  these runs. Raw per-frame entries, learned observations and admission work
  for every arrival remain in `throwaway/slice13/reference/`; the full report
  lists all six runs' six arrivals, not just means.

  | Run | Before longest frame ms | After longest frame ms | Before staging→commit ms | After staging→commit ms | Before packed vertices B | After packed vertices B |
  | --- | ---: | ---: | ---: | ---: | ---: | ---: |
  | 1 | 400.20 | 222.19 | 590 | 422 | 85,739,040 | 28,579,680 |
  | 2 | 275.78 | 193.82 | 519 | 428 | 85,739,040 | 28,579,680 |
  | 3 | 236.88 | 255.82 | 445 | 378 | 85,739,040 | 28,579,680 |

  The copies are gone: vertex work drops by two thirds every run; all 27 cards
  remain. Staging→commit is lower in every pair. Longest frame is lower in two
  pairs, higher in the third. This is still a visible 194–256 ms arrival, not a
  claim that arrival is unnoticed. No stepping is built here; the remaining
  arrival cost needs the owner's next plan decision.
- First-scene logical GPU buffers: 1,334,384,540 → 914,667,500 B (419,717,040 B
  removed), with texture payload unchanged at 521,529,117 B. This is the same
  reference deployment at tick 90, digest `bab549b4fc9795d1` in all six runs.
- Whole-browser memory follows `specs/done/one-renderer/memory.md`: isolated
  fresh Chromium trees, macOS physical-footprint sampler about every 200 ms,
  frozen production assets, high, 1280×800 DPR 1, disabled browser cache,
  all 940 runtime files warmed, ten seconds settled at tick 90. No compilation,
  tracing, screenshots or forced GC during sampling. Final-second median
  footprint and sampled peak through settle, decimal GB, every repeat:

  | Run | Before settled | After settled | Before peak | After peak |
  | --- | ---: | ---: | ---: | ---: |
  | 1 | 7.106 | 6.663 | 7.107 | 6.665 |
  | 2 | 7.098 | 6.621 | 7.099 | 6.623 |
  | 3 | 7.145 | 6.634 | 7.147 | 6.714 |

  Both fall in every pair. Logical GPU bytes overlap this footprint and are
  never added to it. Sample-gap medians are 205–208 ms, maximum 249 ms; shorter
  peaks may be missed. All raw runs stay ignored under
  `throwaway/slice13/memory-confirmation/`, with artifact hashes beside them.
  An earlier baseline artifact mixed binding imports/current WASM with aliased
  old bindings and failed before readiness (`__wbindgen_malloc` undefined).
  Fixing the scratch build's aliases/WASM URL, not production, made the six-run
  cohort valid. The failed attempt remains separate under `throwaway/slice13/memory/`.
- Mechanism expectations changed, as authorized: identity corpse palette becomes
  the row's actual Z=2 rest palette in `lab_models`; a changed corpse palette
  now packs zero new geometry in `asset_generations` (the unchanged-palette
  zero-work assertion keeps its value and receives accurate wording).
  `_growthArt` counts living tiers once and groups rest/card facts by actual
  palette without a living/corpse discriminator. Every assertion in the growth
  gate, including 700 ms, and adjacent fog/no-x-ray checks stays unchanged.
  Exact old/new assertion text is in `throwaway/slice13/assertions.md` and the
  full report. No rule, shader, vertex layout or public WASM export changed.
- The requested tests are green: 36 asset-generation, two impostor and one
  model-geometry test; 23 selected presentation tests of lab frames, poses,
  falls and observed sides; 28 selected web tests; workspace/all-target clippy,
  WASM target checks/build, typecheck and both web lint checks. The first full
  high-profile scene pass (`battle-growth`, `stand-in`, `workbench`, `armory`)
  passes; the growth frame is 54 ms. A shipped-WASM repetition is recorded in
  the final report (its growth frame is 56 ms). No scene baseline is blessed.
  Source/tests/docs remain scoped to corpses; all scratch probes are removed
  from source. Disk remains about 95 GiB free; no old builds were removed.
- Final shipped-WASM scene repetition passes all checks: `battle-growth`,
  `stand-in`, `workbench`, `armory`. The growth frame is 56 ms; no work check or
  700 ms tripwire was changed. Full hand-back, tables, exact assertion changes,
  decisions, command failures/repairs and picture receipts:
  `throwaway/slice13/report.md`. Remaining limitation is the still-visible
  reference arrival above, not corpse-cap frame cost or picture correctness.

**Slice 13 as merged, 2026-11-15.** Checked again on the merged tree: lint,
the presentation tests of falls and corpses, the renderer's GPU tests with
the new body tests, and the scenes `battle-growth` (growth frame 51 ms),
`stand-in`, `workbench`, `armory`, the street scene's soldier tour and
`consequences` (a soldier seen falling plays his death and then lies; the
fallen stay where they fell). The fallen in the `consequences` shot lie as
before.

The implementer's cost number was native. The browser's is the endurance
scene's model-cost mode, late state with 800 fallen, two runs each on staging
and on the change: the draw counts are identical (568 bodies, 1,044 cards,
137,920 triangles in the widest framing) and the models layer's GPU time is
0.48 and 0.50 ms against 0.65 and 0.51 ms in the closest framing, 0.62 and
1.16 ms against 0.42 and 0.37 ms in the widest. That is noise; bodies cost
no more drawn skinned.

In the merge: the counter of living soldiers drawn as meshes is
`soldierMeshes` (it was `skinned`, which bodies now are too), and the street
scene's soldier tour moved to its own file, as its other tours are, because
the rename grew a file already over the size budget.

Found beside it and fixed apart: that endurance mode's check on the late
state fails on staging too. It expected all 800 fallen to be drawn; a side
lays only the fallen it knows (616 there) and draws those the framing does
not cull (568). The check now says that.

**A flake seen while shipping slice 13, 2026-11-15, not fixed.** After
merging another session's staging commits, one run of `consequences` failed
"a destroyed hull draws debris" with no pieces found, while three scenes
shared the machine. It passed before that merge, and twice alone afterwards
(default and high graphics). The check reads the cook-off's pieces once, at
the instant the wreck becomes known. Slice 13 does not touch wrecks. It was
pushed before that result was read, which was a mistake in the order of
work, not a judgement that the failure did not matter. If it recurs, make
the check wait for the pieces instead of sampling one instant.

**Slice 12, confirm-first after slice 13, 2026-11-15.**

- Tree: `b1b0b8d73`, branch `t3/battle-growth-arrival-over-frames-2`. The exact reference battle in the freeze report, high graphics, release/optimized WASM through the Vite development shell. The temporary growth probe stops after the RPG team is learned. GPU work waits for and holds the main checkout’s shared lock. Scratch timers measure the existing fielding, source draft and ordinary admission; no step API or implementation is built before this gate.

- The RPG arrival is still nine contents and 27 distinct rest/card rows: 28,579,680 packed vertex bytes, nine bundles decoded, zero new texture layers and zero scenery placement/upload. Each missing content’s work does divide. Corrected attribution run 2 (ms; supply includes boundary copying, facts and decode; preparation is geometry packing/upload plus all three rest rows):

  | Content suffix (`eastern_rpg_team_rpg_7_`) | New fielding facts | Supply including decode | Geometry pack/upload | Preparation including rest bounds |
  | --- | ---: | ---: | ---: | ---: |
  | active_a | 3.500 | 3.890 | 3.460 | 6.725 |
  | active_b | 3.360 | 3.705 | 2.965 | 6.010 |
  | active_c | 3.240 | 3.715 | 2.990 | 6.010 |
  | carried_a | 3.395 | 4.055 | 3.455 | 7.170 |
  | carried_b | 3.235 | 3.965 | 3.205 | 6.490 |
  | carried_c | 3.200 | 3.755 | 3.275 | 6.690 |
  | guard_a | 3.275 | 4.445 | 3.140 | 6.600 |
  | guard_b | 3.275 | 3.760 | 4.280 | 7.845 |
  | guard_c | 3.080 | 3.555 | 3.260 | 6.480 |

- Generation-wide costs, run 2: complete fielding core resolve 50.085 ms, including 29.560 ms of new facts; its units/clip-table assembly is 45.410 ms (15.850 ms after subtracting the new facts), mount/effects assembly 0.830 ms, and other validation/catalog work 3.845 ms. Serialization of the complete result takes another 11 ms. The whole WASM resolve call takes 67.045 ms; boundary/result transfer and other wrapper work account for the remaining 5.960 ms. Contents queries take 1.455 ms; returning the two clip tables takes 0.205 ms. Merely pre-deriving the new facts leaves roughly 31.525 ms of core assembly plus serialization, before the remaining boundary work.

- Preparation’s whole-generation costs, run 2: catalog staging 12.790 ms, row planning 1.135 ms, material/texture/store tables 1.115 ms, and other model-row/table/buffer work 2.610 ms after subtracting the new content rows. `Models::from_world` is 64.880 ms including 60.020 ms of new-content geometry/rest work; the whole preparation call is 78.215 ms. Battle staging at the page boundary is another 9.925 ms; commit 2.175 ms. Those are measured separately rather than charged to one bundle.

- Pilot run 1: new facts total 27.260 ms, supply 33.825 ms including boundary, new content rows 57.535 ms; whole fielding call 61.130 ms. Logging every old model row added about 30 ms to the apparent fixed admission remainder. Run 2 suppresses those unrelated cached-row logs: the core admission is 64.880 ms rather than 95.720 ms in the pilot. Keep both raw runs and instrumentation variants; do not mistake the logging overhead for engine cost.

- A final probe will resolve the same fully cached fielding with no supplied contents, checking `contents()` is empty before each trial. That directly measures the prospective final resolve after every divisible new fact was absorbed; it determines the stop decision before implementation. Evidence and exact scratch source variants stay in `throwaway/slice12-resume/`.

- **Stop condition confirmed directly; stepping is not built.** In the third
  reference run, after the RPG admission committed, the probe retains the real
  `Fielding` instance, verifies `contents()` is empty, and calls the existing
  `resolve` with zero bytes and zero lengths on three separate animation frames.
  Every appearance fact is already cached; no new content is read or derived.
  The calls take **40.045 / 37.000 / 35.240 ms**, each returning a **2,593,598
  character** complete result. Core resolve alone is 22.745 / 20.990 / 19.780 ms;
  serialization is another 13 / 12 / 13 ms, with boundary/result handling
  accounting for the rest. Cached units/clip-table assembly alone is
  17.470 / 16.335 / 15.260 ms. This is the actual prospective final resolve,
  rather than only a subtraction from a call that also derived new contents.
- Longest remaining single step if new content work is stepped: at least this
  measured **35.240–40.045 ms** whole-result resolve, before ordinary framing
  and rendering. The existing 45 FPS target leaves 22.22 ms for the entire
  frame (60 Hz leaves 16.67 ms). I judge this itself noticeable and apply the
  user's explicit confirm-first stop rule. No new timing gate is introduced.
  The individual new contents are smaller (largest preparation 7.845 ms in
  run 2, 9.620 ms in run 3); one-content stepping alone does not remove the
  full-result rebuild/publication cost. This does not claim most of the frame
  is indivisible, or that the fielding result cannot be split under a different
  design: that publication seam needs an explicit next scope decision.
- All variance is retained. In run 3, new fact derivation totals 68.710 ms,
  supplies 46.610 ms and new geometry/rest rows 73.800 ms; the whole fielding
  call is 134.905 ms and preparation 96.775 ms. Do not substitute the faster
  run 2 for these. The direct cached calls above avoid inferring the remaining
  cost from that slower arrival. Full tables and all three raw traces remain
  in `throwaway/slice12-resume/`.
- The gate stops before feature tests/implementation: no WASM export, draft
  method, scheduling, fielding API, owner or assertion changes. The delegated
  step size and geometry lifetime choice remain open. No worker, thread,
  switch, shader/layout change or alternative admission path was added.
  Shrinking/caching final fielding assembly or changing its result publication
  is an option to scope next; neither was built silently.
- Commands/results: instrumented release WASM builds and all three high-graphics
  reference growth probes pass under the shared GPU lock; waits of several
  minutes are honored. One scratch instrumentation build fails first with
  E0433 (`js_sys` not declared in presentation-wasm), then passes with the
  temporary dependency. All timers, temporary dependencies, source edits and
  the temporary scene/fixture are removed; the clean renderer-browser WASM
  rebuild passes. Final diff is this ledger entry only. No assertions were
  edited and no baseline was blessed. The implementation tests, full check
  ladder and before/after cohort are not claimed because the confirm-first
  rule stopped the slice. Evidence/logs and exact instrumentation snapshots:
  `throwaway/slice12-resume/`; full hand-back `throwaway/slice12-resume/report.md`.
  Disk remains about 95 GiB free; no old measurement builds were removed.
