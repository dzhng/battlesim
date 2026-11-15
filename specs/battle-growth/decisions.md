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
| The archived WASM exhaustion predates texture sharing | closeout |

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
  [the look evidence](assets/stand-in/look/).
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
