# Native rendering choices

## User decisions — 2026-10-08

- Mac-first, platform-neutral spike.
- Godot versus the browser renderer first.
- Full main-menu reel as the reference workload.
- Graphics test is exposed in Settings for the current client only.
- Report average FPS, 1% low, absolute minimum, maximum and a frame-time chart.
- No hard performance threshold.
- Same gameplay and visual target; engine-specific rendering is allowed.
- Browser is the release baseline; Godot is experimental until broad parity.
- Binding architecture is itself the first spike; no random bridge is accepted.

## Delegated decisions

- Candidate binding seams are compared before selection: direct native/GDExtension, offline replay/presentation capture and process boundary. The winner is the smallest seam that preserves fixed-tick authority, side-filtered bulk observations, replay/digest parity and clean lifecycle.
- The live menu reel is not the timing driver. A canonical playback freezes its scene identities and camera data, ignores reduced-motion preferences and advances independently of render stalls.
- The existing synthetic benchmark is preserved as a separate workload because its identity and reports are already part of the repository's contract.

## Rejected or deferred

- Pixel-level cross-engine screenshots: too costly for the agreed visual target.
- A hard 2× or 25% performance gate: the verdict includes maintenance, startup, platform and visual evidence.
- Full browser/Godot live side-by-side comparison: each client reports its own current run.
- A general renderer backend abstraction before the spike proves a shared seam.
- Native JavaScript WebGPU and Rust `wgpu` in the first comparison: follow-up probes only if Godot evidence makes the host question material.

## Spike findings — 2026-10-09

- Godot 4.7.2 stable is pinned in ignored tooling for the spike and passes
  headless editor/GDExtension scanning. The workspace still has no native
  binding or Godot project. The first native pass therefore measures the
  existing direct Rust concepts and offline replay seam before a live window
  probe. This keeps the benchmark honest while the extension is built.
- Godot 4.7.2 is paired with godot-rust 0.5.5 `api-4-7`; the official
  compatibility guidance says the API version must be no newer than the runtime.
- The first shared menu workload owner is a thin identity module over
  `fixtures/menu-backdrop.json`; it does not duplicate scene content or alter
  the live menu's clock.
- The first live binding seam is a Godot 4 GDExtension smoke crate using
  godot-rust 0.5.5 `api-4-7`, with an extension-owned `Node` and no simulation
  methods yet. It compiles for the target Mac; simulation transfer remains a
  later measured step rather than being hidden behind a fake API.
- The GDExtension smoke loads in Godot 4.7.2 headless and prints
  `rust-simulation-probe 0.1.0`; this validates the loader, entry symbol and
  Rust-owned node lifecycle. It does not yet prove simulation transfer.

## Audit entries — implementation passes

### Needs-user (low confidence)

- **The first live Godot binding returns a copied `PackedFloat32Array`.** The
  Godot API is convenient for a smoke method, so the pass chose to copy the
  simulation publisher's slice into Godot's packed-array value. The alternative
  would be a borrowed pointer or a shared native buffer. This is sound for the
  small probe but not yet a production transfer decision. Provisional call:
  keep the copy for the smoke and measure a bulk/shared-buffer alternative in
  the real binding slice; reverse it if transfer cost is material. The gap was
  that the spec required bulk transfer but did not select an ownership model.
- **The native readability probe enlarges and slightly emissively lights observed unit markers, and raises terrain/fog contrast.** The current shots leave tactical contacts too small to read at the captured camera distance, so this reversible probe makes the unit question legible without changing positions or visibility. The exact scale and palette remain presentation choices for the matched visual gate; provisional call: keep these values while browser crops are gathered, then tune or revert from comparison evidence. Confidence: low.
- **Atmospheric fog is applied by the Godot world environment while captured visibility cells remain the authority for hidden ground.** The hard sky/terrain horizon made depth unreadable, but using the fog bitfield as a renderer oracle would change gameplay ownership. The provisional environment fog only grades distance; it can be tuned or removed after matched browser crops without changing Rust visibility. Confidence: medium.

### Sound (high confidence)

- **The offline native probe is a separate executable.** It advances the real
  Rust battle, copies one side publication, and replays the result to the same
  digest without opening a window. This keeps simulation authority evidence
  independent from Godot's renderer and makes a failed Godot installation
  diagnosable. The alternative would have mixed engine startup and sim timing.
- **Godot API 4.7 is pinned to the runtime.** The extension uses godot-rust
  0.5.5 with `api-4-7` because the verified runtime is Godot 4.7.2. The
  alternative of `api-custom` was rejected because the official compatibility
  guidance gives it no guarantee. This constrains the first Mac spike to the
  pinned runtime and leaves broader runtime support for a later decision.
- The existing `game-wasm::BattleHandle` already exposes the required seam
  concepts: ordered commands, fixed stepping, side publication, digest,
  replay and observation resync. Native work should reuse those simulation
  owners rather than create a parallel authority.

## Evidence decisions — 2026-10-09

- **The browser exporter copies packed publications lazily at the existing credit boundary.** The graphics-test reel receives a `copyPacked()` accessor and calls it before the worker publication is released, then stores the digest, tick and canonical camera pose in one per-scene `battle-presentation-capture/v1` artifact. This keeps the simulation publication as the owner and avoids copying every publication for ordinary battles; a direct retained view would become invalid when credit returns. The choice is sound for the renderer-only capture seam; a future shared-buffer implementation may replace the copy only after measuring it.
- **Godot follows the capture's camera samples without treating them as a second clock.** When a scene capture is present, the probe selects the latest sample at `warmTick + sceneElapsed × tickHz` and uses its canonical pose; otherwise it keeps the authored reel interpolation. This lets the native report prove it consumed the browser-owned presentation stream while leaving simulation time in Rust. The choice is sound for the playback checkpoint; authored world geometry still must replace the synthetic proxy before comparison-ready evidence.
- **The authored asset seam is an explicit environment-selected PackedScene.** The probe attempts `GODOT_AUTHORED_SCENE` and only falls back to its synthetic field when the path is absent or not a Godot scene. This keeps missing Git-LFS assets diagnosable and avoids baking a duplicate asset catalog into the spike; the report records whether the authored scene loaded. The choice is sound for the probe, while the next pass must provide and review a real scene before any visual or performance verdict.
- **A full reel consumes one capture file per menu scene.** The browser report already exports scene-sized JSON strings, so Godot accepts a directory keyed by map name and reports the number loaded. A single-file environment remains for decoder smoke tests, but a complete reel cannot silently reuse one scene's publication stream for another. This is sound because scene identity stays owned by the menu fixture and missing files remain visible in the report.
- **The authored-scene probe places a bounded kit instance at each admitted building frame.** The map JSON remains the placement authority, while `GODOT_AUTHORED_BUILDING_LIMIT` bounds the deliberately incomplete probe so thousands of repeated GLB instances cannot masquerade as a complete renderer. The report exposes the count and keeps `comparison_ready=false` until terrain, props and observed units are rendered. This is sound as a staged asset path; it is not a visual-equivalence decision.
- **Authored map families select their own kit scene.** The probe accepts a comma-separated family-keyed scene list, so China and Paris menu maps no longer reuse one unrelated house kit. Map `regional_family` remains the selector and the placement cap remains explicit. A foreground uncapped run now loads both kits and all 3,153 admitted building rows; terrain, materials, and visual equivalence remain separate gates.
- **Capture publications are exported as raw u32 words and include Rust's layout JSON.** The producer stores compact fields in lossless bit carriers inside a float buffer; converting those carriers to JavaScript numbers loses the wire contract. The browser now copies each word through `Uint32Array`, validates the range, and carries the exact layout string in the capture. This is sound because a native decoder can reconstruct the same schema instead of inventing a second one; baseline/preroll and presentation-timeline replay remain the next corrections.
- **Warm-up publications are retained as capture preroll.** The graphics test still starts performance measurement after warm-up, but the capture receives publications from the first authority snapshot so a fresh decoder can establish ground/fog/group baselines. This separates measurement timing from replay completeness and is sound for the renderer-only artifact.
- **Displayed camera frames are a separate capture stream.** Publication ticks describe authority output, while the camera actually displayed is driven by wall-time and interpolation. The artifact now records elapsed frame time, presented tick and actual camera pose; Godot selects that stream directly. This is sound because stalls no longer make native presentation drift from the browser's displayed path.
- **Godot validates the layout object and walks every raw publication word before rendering.** Until a full native observation decoder exists, this makes the current seam honest: the report proves the capture is structurally consumed without claiming that raw words are already units, terrain or fog. The remaining semantic decoder can be added behind the same Rust-owned layout rather than inventing a parallel schema.
- **Replayability is first proven through the existing browser decoder before native decoding.** A focused test copies three real Rust publications as raw u32 words and feeds them to a fresh `ObservationDecoder`; this catches bit-carrier, baseline and ordering errors without making Godot a second simulation authority. Native semantic rendering still consumes the same layout after this gate.

- **Keep the interactive seam as a GDExtension, while using offline presentation capture for renderer comparison.** Same-input blue/red runs now match native and Godot digests and packed publication bytes, including replay, duplicate-command rejection, retained-buffer immutability and teardown. A minimal stdin/stdout process candidate also completes cleanly, but it adds process framing without improving the interactive contract. The offline capture is a measurement control, not a second gameplay authority; it must freeze the presentation history that raw observations alone do not contain.
- **Do not call the browser route a frozen reference yet.** The route now runs the full two-scene menu workload and uses the menu's subject smoothing, but its camera clock and plate composition still need to be aligned and its full production run and visual gates have not passed. Headless Godot and the 4096-cube scene remain smoke evidence only.
- **A completed preview run is lifecycle evidence, not a performance sample.** Both menu scenes completed in 156 seconds, but the shared preview was background-throttled to roughly one frame per second. The report is retained for route and identity proof; performance runs require a foreground, GPU-capable harness.
- **The first Godot reel artifact is a camera/workload probe only.** It reads the browser-owned menu fixture and reproduces scene/shot timing over a synthetic field, while reporting `comparison_ready=false`. This preserves an honest checkpoint until the presentation capture decoder and authored assets exist.

- **Native semantic decoding reuses the published group grammar.** Godot now bitcasts raw u32 carriers, reconstructs packed/snapshot/copy/replacement group baselines, and extracts own-unit pose fields from the layout's first group. The decoder reports complete publication/unit counts without advancing simulation or inventing a parallel schema. This is a valid semantic checkpoint, while terrain materials, authored placement quality, and visual equivalence remain required for comparison-ready rendering.

- **The native composition pass consumes saved-map exports before authored materials.** The Godot probe reads map size, surface strokes, building transforms and prop extents from the authoritative `map.json` exports, draws bounded primitive geometry, and records the limits. This keeps placement and coordinate ownership in map generation while making missing authored assets visible; it does not claim material or visual parity.
- **Graphics-test evidence is downloadable from the results page.** The browser report now exposes the frozen workload fingerprint, scene identity, host viewport and adapter labels, and the Settings result offers the complete JSON as a download. This makes a finished run portable for native comparison instead of leaving it only in in-memory `window.__graphicsTest` state.

- **Fog payloads are consumed at the native boundary before rendering.** The decoder validates the published dimensions, full/delta payload length and carrier location for each sampled publication, then leaves visibility semantics in the Rust-owned bitfield. Rendering still needs a display-facing fog pass; native code does not infer visibility from map geometry.

- **Native semantic replay uses the complete capture stream.** The decoder follows the browser/Rust grammar for packed groups, raw replacement/copy carriers, fog and ground tails. Full market-town and paris-corner captures now decode with every publication accepted and zero invalid ground payloads; the reel may still bound presentation work separately from this validation pass.

- **Fog presentation uses captured visibility cells, not a native visibility oracle.** Godot bitcasts the Rust publication fog words, applies full snapshots and deltas, and draws a bounded dark-cell layer from the resulting bitfield. The simulation still owns what is visible; the bound limits presentation geometry only.
- **Fog cells follow each decoded presentation frame.** The initial fog snapshot still seeds the bounded cell mesh, but each captured publication updates cell visibility as the replay advances. This keeps the dark layer a view of Rust's full/delta bitfield instead of a static first-frame overlay; the 4,096-cell draw bound remains a presentation limit, never a visibility rule.
- **Saved-map geometry is complete by default and bounded only by an explicit diagnostic cap.** A foreground authored run must not silently replace a 3,000-building or 50,000-prop export with a small proxy subset. The reel now treats an unset or zero map limit as “load every exported row”; a positive environment value is an intentional probe cap and is recorded in the report. This keeps the map export authoritative while preserving a cheap lifecycle path when a caller explicitly asks for one.
- **The native reel resolves the checked-in map export by default.** A normal Godot run should consume the same `fixtures/maps` source as the browser without a shell-only environment recipe; `GODOT_AUTHORED_MAP_DIR` remains an override for alternate exports and lifecycle fixtures. This keeps the map owner singular and makes a missing override observable rather than silently reverting to a synthetic field.
- **Authored building limits are parsed before kit selection.** The multi-kit path and the single-kit path share one placement cap, so a foreground diagnostic cannot accidentally load the full city while its report claims a bounded run. The cap is still a positive, explicit diagnostic value; zero or unset remains the complete catalog contract.
- **Each authored placement shows only the shell module named by its template family.** The hydrated GLB is a module catalog, not a complete building instance; instantiating every child at every map frame produced overlapping walls and camera occlusion. The probe maps slab, point, U-block, and court template ids to their shell node and hides the remaining catalog modules. This keeps map placement authoritative while making the catalog usable as a staged renderer input; facade details and exact material recipes remain open.
- **Unsupported authored templates hide their catalog and appear in the report.** The reviewed kits cover apartment shell families, while saved maps also contain homes, shops, farms, and industrial templates without a matching hydrated kit. Leaving the full catalog visible would recreate the overlap bug; hiding those modules preserves truthful placement and `authored_unresolved_templates` makes the missing art explicit for the next asset pass.
- **Catalog selection follows the map template family across multiple kits.** Apartment, home, farmstead, industrial, and tower GLBs are loaded as separate family resources; each saved `template_id` chooses its matching shell before placement. This keeps all 3,153 menu buildings admitted without pretending one apartment kit can represent shops or warehouses; unresolved ids remain a hard report blocker.
- **Observed units get one friendly-colored contact disk under the decoded capsule.** The native semantic feed carries own-unit poses without a per-row side field, so the marker uses the capture's authoritative blue/red side once for the whole observed feed; alternating colors would falsely imply ownership differences. The disk answers “where is this observed unit on the ground?” without inventing per-row team data or changing the Rust observation.
- **Named-cut counts are unique scene/shot paths.** A fast foreground run can schedule more than one save while a frame is settling; counting each successful overwrite overstated coverage. The report now increments only when a new `scene-NN-shot-NN.png` path is first written and compares that set with the fixture's expected shot count.
- **Screenshots are sampled after `frame_post_draw`.** A deferred callback before rendering read the previous viewport image, which made a moving reel appear frozen and hid camera/asset defects. Each queued cut now waits for Godot's rendered-frame signal, then the final report waits for all pending saves; this preserves the captured camera contract while making visual evidence correspond to the frame the user would see.
- **Forest regions become low native terrain patches.** The saved map owns polygon membership, so the Godot composition draws one dark ground patch per admitted forest region without inventing tree placement or visibility. This adds terrain context while keeping foliage identity and gameplay authority in the map/simulation owners; it is still a material approximation until the authored terrain pass lands.
- **Authored kit families are derived from their resource paths, and missing families are reported.** Shipped GLBs commonly use the basename `kit.glb`, so the loader derives apartment and kit family keys from the filename or parent directory. A template with no matching family increments `authored_unresolved_templates` before placement is skipped; catalog completeness must never be inferred from the rows that happened to instantiate.
- **The native catalog caches selected shell prototypes instead of instantiating each full GLB per row.** A kit resource contains hundreds of catalog nodes, while a map placement needs one named shell. The loader instantiates each family/shell once, duplicates that subtree for placements, and keeps the full authored row count and unresolved-template report; this preserves the catalog contract without making the complete export unrenderable.
