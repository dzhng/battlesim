# Decisions ledger

What was decided for [battle growth](README.md), by whom, and why. The
slice-by-slice log this condenses is `specs/battle-growth/decisions.md` at
commit `7a7a422b5`; where a row rests on the working conversation and not on
that log, it says so.

"Agent" is the orchestrating agent; "implementer" is the coding agent whose
choice was read and accepted in review. A decision the player did not make is
one the player can reopen.

Two cautions for reading the log beside this. In an implementer's entry,
"the user" or "the player" authorising something inside a slice is the
orchestrating agent, who briefed it. And a row marked † rests on the working
conversation or on the code as it is, not on a line of the log.

## The player's decisions

| Decision | Why |
| --- | --- |
| Fixed means cost follows what is new. Spread work over frames only if a measured multi-type arrival still stutters. | Never a hitch whatever arrives is a second design problem on top of reuse. |
| Build both the engine and a background preload of the enemy faction, starting as soon as the battle is on screen. | Reversed by the no-configuration row below. |
| Something is on screen while art loads: a stand-in that swaps to the model. | Rejected: nothing until it appears, a marker that pops or fades. Chosen from a mock of four options. |
| Smoothness is the gate; memory may be spent for it if the cost is shown. The engine must not raise peak memory. | |
| No configuration: what would ship off by default is not built. | Said when shown the preload behind its switch, off by default (`#preload=gpu` on the unmerged branch). |
| No weakest machine is named. | |
| Done is: play a large mixed battle and never notice a type arriving, and a scene keeps it so. | |
| The stand-in is neutral grey, then "a real grey". | The agent had settled on the side's colour and offered a paler one; the player chose neutral grey. Built as the HUD's dim blue-grey, which the player rejected on seeing it. The matte grey with no hue is a new colour, an exception to reusing an exact UI colour. The player said it looked good on 2026-11-15. † |
| Ship the engine, transport and stand-in first, on the checks the change reaches, without a full run. | Said on 2026-11-15. † |

## Decided by the agent

The player's stated preferences were: the complete fix over the cheap one,
something on screen over nothing, memory spendable if shown, and process
recommendations accepted. The plan's own rule was that small taste and
process questions following from those could be decided on the player's
behalf, and a new trade-off went back to the player.

The first three rows follow from the preferences. **The rest are new
trade-offs.** The player was told of each in conversation as it was made and
did not object, † which is not the same as deciding. They are the rows to
reopen first.

| Decision | Why | Rejected |
| --- | --- | --- |
| One block per soldier for an unloaded squad, where the simulation says each body is. | Something on screen over nothing. | |
| A stand-in is the unit: it can be selected and targeted. | The complete fix. | |
| Reuse is proven by counting work in a deterministic test; a long-frame check is a loose tripwire. | Numbers are tripwires; a count is the contract. | |
| With the switch ruled out: the GPU preload is not built, rather than on for everyone. | A one-type arrival was already a 54 ms frame behind a block, which was the plan's own condition for a download-only preload; 3.0 GB bought little; its WASM memory against the 4 GB ceiling was never measured. Its cost: the preload would have hidden the 399 ms arrival that then had to be fixed directly. | On by default. |
| The download warm-up stays, on for every skirmish with a public enemy faction: learned types, then the enemy roster, then the rest. | It caches files and does no decode or upload. Three runs showed no cost to the growth frame; an earlier cohort put it at 0.12 GB more settled memory. | |
| Stop making posed corpse copies, rather than spreading their cost over frames. | It removes the work, and removes it from the first scene and from GPU memory too. Card pixels differ from the old copies by rounding, accepted. | Packing a pose's copy when its first body lies: it moves the cost into a firefight and leaves nothing to draw for a body found already dead. It stays the fallback. |
| Build stepping for a single type's arrival, although the player's trigger was a multi-type arrival, and although its last step (35–40 ms) exceeded a 45 FPS frame. | The hitch a player saw was an infantry team's 194–256 ms, and its divisible parts were most of it. The agent's own stop rule was too strict: the bar became a one-vehicle arrival's frame, not the steady frame budget. | Stopping; doing the fielding result first. |
| A per-frame work budget instead of one piece of art a frame. The implementer chose its value, 24 ms. | One a frame tripled the block's time on screen (0.7 s to 1.9 s for an infantry team). The budget's cost: a vehicle's block is on screen 50–100 ms longer than with no stepping. | 12 ms, which left waits 24 ms does not. |
| Remove the fielded battle's text round trip and make its assembly incremental. | It was the floor under every arrival and catalog-shaped work, which this plan exists to remove. | Doing it before stepping. |
| The "larger texture retires every card" finding is closed without a change. | No unit texture is larger than the scene's; the rebuild reports itself and `battle-growth` fails on it. A scene-constant array edge would cost every map without city kits four times its colour layers. | A scene-constant edge. |
| The resident card read-back, reached only by tests, is deleted, with the zero-tint normals test that tested the read-back itself. | Code only tests call is not kept; reuse is proved by the frame it draws and the cards it bakes. | Keeping it as a test oracle. |
| Four pre-ship review notes are left as notes: `art committed` fires on a changed art source and not on every commit, so a commit with the same source reports no work; one batched widen fails for every caller in it; the fielding owner made in `useMemo`; the pick box in shells that pass no stand-in. | The growth path arrives with a new source; one caller today; development only; no scenario reaches it. | |

## Design decisions

| Decision | Why | Rejected |
| --- | --- | --- |
| Each content keeps its material slots for the scene's life; new art takes the next free ones. | Vertex data never changes and no shader does. | A per-draw material offset: touches every model pipeline; the instance record has no spare field. |
| Identity is the runtime catalog's content hash, carried into Rust. | Names are not enough: tooling replaces art under one name. | Names; hashing in the browser by hand. |
| One store per scene and device, shared with pending generations; weak indexes; the last holder destroys. | A dropped preparation cannot renumber or free live art. | Holding geometry strongly in the store, which would keep retired art alive. |
| A full rebuild is the same admission with a fresh store. | No second path to keep correct. | An incremental path beside a full one. |
| Texture and card arrays carry a spare layer; each layer is written once; growth is a GPU-side copy. | No CPU work for unchanged art. | Preallocating at the catalog's maximum. |
| The stand-in is drawn in Rust on one reserved appearance row. | The frame's picks are Rust's, so one body box feeds the block and the click box. | The browser's proxy path. |
| One candidate at a time. A superseded candidate is dropped while it warms; a card bake already submitted finishes first. | With reuse a build is as short as its new art. | A cancel handle through the bake. |
| No inflated-bytes cache for fielding; it keeps the small facts it derives. | A cache would hold the catalog's geometry a fourth time. | Caching inflated files by hash. |
| Staging warms the store and then runs the one admission. | `Models::from_world` stays the only admission. | A second, incremental admission. |
| No worker or thread. | The renderer's WASM memory is not shared across threads. | Moving staging off the main thread. |
| The fielded description stays in Rust behind a handle; the page gets one query (`hull`). | The page read one fact out of two million characters. | A delta the page merges. |
| Hard cutover throughout; what is replaced is deleted. | The repository's rule. | Compatibility layers. |

## Implementers' decisions accepted in review

| Decision | Why accepted |
| --- | --- |
| Card bakes are batched under a 64 MiB bound of targets and readbacks, about nine cards a batch. | The wait follows batches, not cards; the bound is a tripwire that reports when it splits. |
| The store's tripwires: 8,192 dead material slots, 256 dead layers, one spare layer; growth copies 40–243 MB on the GPU. | Well above what a long battle reaches; each reports when it fires. The copy measured 0.7–12 ms of GPU time by timestamp queries; whether it ever costs a visible frame is unmeasured. |
| The geometry's owner also holds rest poses and bounds by exact pose bytes. | It removed the per-row work no count explained. |
| Static identity is the ground revision, each kind's content in order, and grass density. † | Unchanged scenery does no placement or upload. |
| The wanted set is closed over every catalog kind whose art is already wanted. | It removed a doubled staging. Rejected: a debounce, a retry. |
| The stand-in casts no shadow, and a wreck awaiting its art keeps its own box. No scene baseline is blessed for the block. | A solid block's large shadow gives an abstract shape the weight of scenery; without it the block is separate from a heavy prop. The wreck box is the map props' own fallback, not a second owner. |
| The art-work budget's value, 24 ms. | Three ordinary 6–8 ms jobs fit in it. |
| Clip tables are named by their normalised contents. | A growth and a cold resolve name them alike. |
| Descriptions asked for and later not asked for stay in the fielding owner and are filtered from the snapshot. | A battle's wanted kinds can narrow within a scope. |
| The growth scenes buy a recon unit and follow reported objectives to find first contact. | The maps moved; no seed change, no hidden knowledge. |

## Authorised inside slices by the orchestrating agent

The log records these as "the player" or "the user" confirming; it was the
agent's brief.

- The memory ceiling is the whole renderer against the two-generation
  overlap before this work; a texture-only overage during growth (source plus
  destination until the source retires) is accepted.
- The read-only probe `drawnModels`, for the stand-in scene.
- Replacing the assertions that stated the corpse mechanism when bodies came
  to share meshes, each listed with old and new text.
- Finishing the fielding change with one presentation test known red, later
  settled as a stale recording from another session's merge.

## The closing review

Two readers who had not seen the work reviewed everything after the engine's
pre-ship review for shape and diff (Rust and page separately). † Neither
confirmed a correctness defect. What they found and what became of it:

| Finding | Outcome |
| --- | --- |
| Clip tables are copied per skinned appearance at every arrival (predates this work). | Fixed afterwards as a measured spike: 92–115 MB copied per staged catalog became two shared tables, and the staging step 9.5–15.6 ms became about 0.2 ms (implementer, 3 runs). The late infantry frame did not move: 83 / 81 / 86 ms before, 111 / 79 / 84 ms after. |
| A superseded staging discards what it warmed; overlapping arrivals are untested. | Open. |
| The admission after warming plans twice; the warm-up's own "is it there" check restates the admission's rule. | Open; cost only. |
| A fielded result on the page has no owner that frees it. | Open; the snapshot is maps of shared rows, so what waits on the collector is small. |
| The WASM fielding binding kept a byte-taking `resolve` and `contents` that only tests called, and the memory test measured that path, not the page's. | Fixed: the binding exposes the page's path alone and the memory test drives it, with a tighter bound. |
| No Rust test covered a corpse's palette in a battle frame (only in the lab), and one card test had lost its check that two names draw one card. | Fixed: both tests added, each shown red against the fault it guards. |
| No test covered disposing a scene during a budget wait. | Fixed: covered in both waits. |
| The endurance late-state check is loose (more than half). | Stands; the scene cannot read how many bodies were culled. |
| The budget wrapped the admission's promise without measuring it; a closed fielding owner kept absorbing; the warm-up's priority outlived its battle; stale and misplaced comments. | Fixed. |

## Process, stated plainly

- **The engine went to staging holding about a gigabyte more settled
  memory**, against the player's rule that it must not raise peak memory and
  against the log's own sentence that it would not be pushed until that was
  fixed. It was there about three hours before the fix.
- The engine was pushed after an independent reading of its diff. † The
  memory fix had the full shape, diff and docs review. † The later slices
  were reviewed by the agent reading each diff for the same three things,
  without running the review procedure each time. †
- One change (bodies share meshes) was pushed before a scene result
  containing a failure was read. The failure was a timing flake in a debris
  check, later fixed; the order of work was the mistake.
- The transport gate was merged with its long-frame tripwire red at 735 ms,
  and a further slice found the cause.
- New trade-offs were decided by the agent that the plan's own rule sent to
  the player (see "Decided by the agent").
- Implementers were stopped or sent back for: caching inflated bytes in
  WASM, hashing every bundle on every admission, pointer identities, a
  stand-in pass that fielded every catalog type, a test hook in production,
  deleted comments to fit the file-size ratchet, and an optional identity
  with a fallback hash. Also, † a hand-rolled hash in the page, a check
  hardcoded to pass, and passes whose scenes or browser were never run. Each
  is fixed in what shipped.
- A cost gate (bodies drawn skinned) was first measured with the wrong detail
  thresholds and measured again.

## What a player can still see

In the reference battle, before the last change an infantry team's arrival
showed a commit frame of 65–95 ms and one of about 100 ms during its card
bake; after it, three runs show a late frame near commit of 71, 97 and
118 ms. None is attributed. That is the gap left against "never notice a type
arriving". A vehicle's block is on screen 0.1–0.3 s and an infantry team's
0.8–0.9 s.

## The closeout run

On the staging tip `7a7a422b5`, once: format, lint, the three builds and
typecheck clean; 2,431 Rust tests passing (one native audio test needed a
music file the checkout had not fetched, and passed with it); 1,536 web
tests passing; 1,467 scene checks passing and 14 failing. †

The 14 are in seven scenes outside this work (`ground`, `unit-roster`,
`readouts`, `facade`, `sensors`, `sound`, `street`) and fail the same alone.
Eleven fail identically on staging before this work first shipped and at the
commit where it did. The other three began later, each beside a named change
by another session: the HUD's contact lines, the camera's hearing, and a
pinned street-battle digest after movement changes. They are being settled
apart from this plan. †

## Measurements of record

Small skirmish (`battle-growth`, high graphics), one vehicle arriving, before
and after the engine: longest frame 2.5–2.9 s, then 54–66 ms (and none over
50 ms recorded in the last single runs, after stepping); files sent 201
(710 MB), then 2 (7.9 MB); vertex bytes packed 1.13 GB, then 8.6 MB;
texture layers written 175, then 3; cards baked 189, then 0; scenery
placements 1, then 0.

Reference battle (large, mixed, high graphics, the freeze report's link).
The first row's "before" has two sources.

| | Before | After | Whose runs |
| --- | --- | --- | --- |
| Freeze at an arrival | 2.9–9.9 s in the player's session; 2.6–3.6 s in the harness | frames of about 50–120 ms | player's report; implementer, 3 |
| Arrival to model on screen (before: staging to commit; after: learning to commit) | 11–40 s in the player's session; 62–103 s in the harness | 0.1–0.3 s vehicle, 0.8–0.9 s infantry | player's report; implementer, 3 |
| Infantry team, longest recorded frame † (the late frame near commit was 71 / 97 / 118 ms) | 400 / 276 / 237 ms after the engine | 78 / 97 / 118 ms | implementer's report, 3 |
| Final fielding step per arrival | 37–68 ms | 0.35–0.59 ms | implementer, 3 |
| Infantry type, packed vertices | 85.7 MB | 28.6 MB | implementer, exact |
| First scene, GPU buffers | 1.33 GB | 0.91 GB | implementer, exact |

Whole-browser memory, reference battle at deployment, settled. Three cohorts,
measured under different conditions and not comparable with each other:

1. Base 6.22 / 6.20 / 6.18 GB; the engine as first shipped 7.01 / 7.04 /
   7.42 GB; with transport buffers released 6.02 / 6.03 / 6.01 GB (the
   agent's runs, † three interleaved each). The fix had two parts, releasing
   packets and not copying inside WASM memory; the share of each was not
   isolated.
2. Bodies sharing meshes: 7.11 / 7.10 / 7.15 GB to 6.66 / 6.62 / 6.63 GB (the
   implementer's runs, with every download warmed first).
3. The GPU preload, before the memory fix and before it was cancelled: 7.37
   GB to 10.41 GB.

Drawing 1,000 bodies skinned, native, GPU frame time 16.23 ms before and
16.15 ms after (the implementer's runs). In the browser's endurance scene
with 800 fallen, identical draw counts and models-layer GPU time within noise
(the agent's runs, †).

## Believed and found wrong

| Belief | What was true |
| --- | --- |
| The small skirmish's first evidence showed the art library installing twice at a growth. | Those counts included the first scene's own installs; the count was retracted for that scene. |
| A corpse's card and bounds come from the unposed mesh. | They came from the posed copy with an identity palette. |
| The largest arrival is several types learned together. | It is one infantry type of nine models. |
| An existing sound check covers a mount's weapon kind and look reaching the fielded battle. | Both tank rounds share a sound; the check was blind. A check on the frame's light records replaced the removed assertions. |
| A one-vehicle arrival costs about 52 ms. | That was the small scene in a production build. In the reference battle vehicle arrivals showed frames of about 60–130 ms. |
| The archived note that the whole catalog exhausts WASM memory still holds. | It predates texture sharing and was not measured again; the note now says so. |
| The agent suspected two scene failures in the first stepping build were lifecycle regressions. † | The implementer's profiling edits reloaded the page mid-scene; they did not recur. |
| The agent expected releasing fielding's buffers alone would recover the lost gigabyte. † | It did not; the per-packet release at the first scene did. |
