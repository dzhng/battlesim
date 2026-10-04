# Combat audio and sound workbench

## Contract

Every existing synthesized sound remains an auditionable baseline. A persistent
sound catalog adds recorded clips, reusable recipes and editable assignments.
The battle still emits one sound for each observed launch, impact, ricochet or
blast. Sound never changes simulation rules or supplies hidden information.

The user delegated initial sound choices and extraction decisions. Choose crisp,
snappy cores for rifles and bassier cores for heavier weapons; recordings are the
core, with restrained synthesized support. Keep clean unused reloads and mechanical
actions available for later work. No user selection blocks implementation.

## Ownership

- `fixtures/sounds.json` owns source provenance, clean clip metadata, sound
  recipes, firing overrides and round-specific material impact selections.
- `packages/battle-audio/src/catalog.ts` validates that document and resolves
  selections. A firing override is keyed by exact unit type and authored mount
  name. Equivalent named physical mounts share one sound choice. The existing
  weapon-kind selection in `presentation.audio` supplies baseline mixing and a
  synthesized fallback when no override exists.
- `SoundBank` prepares recordings and synthesized support into one buffer per
  variation. `SoundFrame` continues owning timing, distance, fog cues and budgets.
- `assets/third-party/audio/` keeps pinned sources; `assets/runtime/audio/clips/`
  keeps reproducible cleaned mono WAVs. Sources and runtime media use Git LFS.
- `apps/sound-workbench` owns the developer audition/editor surface, and reuses
  `FixturePublication` for reviewed, stale-safe repository saves. Audio editing
  never rewrites mechanics or map input files.

Exact unit type and mount name already enter the observed effect feed. Preserve
them through the existing `EffectShooter`/`Launch` seam, with no new simulation or
packed-observation fields. Hull launches currently identify a mount's first weapon,
so the editor offers a cannon mount choice rather than promising AP/HE launch
discrimination that the feed cannot supply.

## Scope and preserved inputs

The first source is the supplied CC0 qubodup gunfire master, SHA256
`1eaebcb54159a0b229fc9d5ebde0b60665f38e19c30b3b15f24d419e4df0d989`.
User listening annotations supersede automatic take boundaries. Unflagged takes
are clean single-weapon passages. Split mixed takes at the stated changes, extract
clean single discharges and discard muddled intervals and the bell in take-10.
The muffled second half of take-05 is an impact candidate for hard objects as well
as an optional rifle timbre. Preserve descriptive families without invented models.

Also source and assign ATGM launch/flight, tank firing, ricochet, bullet ground
impacts, shell ground impacts and ground explosions. Each remains editable.
Reload/mechanical clips are stored and auditionable; no reload or casing event is
invented where the battle feed has no cause.

External source permissions are recorded per source. Prefer the known CC0 collection
and government recordings. If only a lower-quality public excerpt is available,
label it honestly and keep its replacement independent of the recipe/assignment.
Web Audio's asynchronous decoding resamples complete WAVs to the context rate;
use the browser API rather than adding a private decoder.

## Slice graph and review

1. [Catalog and media](slices/01-catalog.md): shared schema and durable clean assets.
2. [Playback](slices/02-playback.md): recorded preparation and observed assignment.
3. [Workbench](slices/03-workbench.md): complete library, audition and repository save.

Playback and workbench can run concurrently after the shared catalog contract.
Root integration owns final media choices and the shared screenshot/scene gates.
Use test-first red/green checks at each new behavioral seam. A visual checkpoint
requires real desktop/narrow screenshots, compare-screenshots inspection, unprimed
screenshot-critique last, and preview-shots. Scratch evidence belongs in throwaway.
Run the full check and verify once when all three slices are integrated; use the
sound scene while iterating. No played battle or balance report is required.

## Decisions and limits

The user explicitly authorized agent choices, including source selection, clean
crop selection, family labels, initial assignments and reversible UI design. Baseline
synthesis remains available and immutable in meaning; sampled recipes get new IDs.
Saving an assignment affects newly opened or explicitly restarted battles. Running
battles retain their captured audio generation. Local save APIs accept logical edits,
never file paths, and preserve outside edits. No compatibility or migration layer.

Claude Opus independently reviewed the plan. Its CLI also cannot hear audio here;
user listening annotations and source/video evidence are the auditory grounding.
Measurements verify onset, clipping and bandwidth, not timbre. Envelope peaks
flag possible extra attacks or reflections for crop review; they do not identify
individual discharges on their own.

## Next Agent Prompt

All three slices are integrated and the whole-feature review is clean. Desktop
and narrow screenshots passed an unprimed review; the tool is live on port 8794.
The recorded sound scene and all 29 baseline sample-parity checks pass. Final
review corrected unequal loop padding, aligned editor fallbacks with playback,
and canceled preparation on browser page departure, with failing regressions
first. Keep current media and delegated sound choices.

The full check ran: Rust, formatting, lint and typecheck passed. Its lone web
failure was the developer-menu expectation; the corrected router test and affected
editor/audio tests pass. The full browser run reached every scene. Missing LFS
effect PNGs and city templates have been restored; affected-scene retries passed
apart from the known staging failures below. A final sensors/ambush/village run
has no audio page error, including canceled clip requests. The final focused run
passed 78 tests in ten files; typecheck, scoped formatting/lint and production
build pass. Do not rerun the entire expensive suite unchanged.

The remaining village staging failures are resolved by admitted movement commands,
actual identification before withdrawal, and a separately staged moving HMG fight.
The village and village-watch browser scenes pass with their original assertions.
No simulation, game-rule fixture or decoder source changed. A real workbench browser
journey changed a tank mount's near/far/gain, reviewed and saved it, reloaded it,
then restored the original through the same flow, using a temporary repository root.
All original full-run failures now have passing focused replacements. Do not claim
that the original full commands themselves exited green.

The source-review map now retains the original take boundaries and user annotations;
two unused gunfire crops lost surrounding low-level audio. Rebuild/check passes for
all 88 clips and eight pinned sources. Finish the independent closeout review and
archive this plan as a rationale; no implementation slice remains open.

- [x] Durable clean source library and explicit exclusions; reproducible asset check passed.
- [x] ATGM, cannon, ricochet, small/heavy impacts and explosions assigned in catalog.
- [x] Async live/offline preparation and exact unit/mount firing overrides.
- [x] Complete catalogue audition, editable recipes and all concrete unit rows.
- [x] Preview/save/reload with shared publication and stale/outside-edit protection.
- [x] Focused tests, browser sound proof, desktop/narrow visual review.
- [x] Whole-feature review, consolidated decision audit and durable package docs.
- [x] Finish affected-scene retries and verify browser departure cancellation.
- [x] Resolve existing global staging failures without changing rules or assertions.
- [ ] Archive rationale after the independent final review.
