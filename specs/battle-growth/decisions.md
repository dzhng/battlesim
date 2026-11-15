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
| An undrawn enemy still gets a HUD readout, because readouts follow picks | 10 |
| The archived WASM exhaustion predates texture sharing | closeout |

## Confirmed during implementation

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
