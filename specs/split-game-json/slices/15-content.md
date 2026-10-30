# 15 Content pass

**Contract:** every look sounds and looks like its weapon. The firing sound
always fits the weapon, and burst recordings match each weapon's rate of fire.

Each item below is its own sub-slice: one variable, one verdict, one commit.
Look-value edits move no hash. The directed artillery split in
[the decisions ledger](../decisions.md#slice-15) is the exception: one
appearance-assignment commit separates mortar and rocket artillery, moving
rules and replay configuration identities once while preserving battle digests.

**Sound** (sources from [03](03-sound-sources.md)):
1. firing identities for `artillery`, `rocket_artillery`, `bomb` (release),
   individual `rocket_pod` launches and the strike missile's look, replacing
   the rifle-shot or tank-cannon placeholders;
2. mortar, if a mortar weapon uses the artillery look and the user can hear
   the difference;
3. single recorded reports for the marksman rifle and heavy sniper, whose
   resolved rows have no burst group; a three-report recording at cadence
   for `aa_gun`;
4. **the cadence check:** at fielding, a look whose burst recording doesn't
   match its weapon's rate of fire is refused by
   [`SoundTables`](../../../crates/presentation/src/sound/tables.rs).
   Write it first; it fails on today's data, and passes once item 3 lands;
5. round the folded levels [13](13-fold-looks.md) wrote as raw medians
   (`3.0983866769659336`, far `825.0`) to the precision the file authors.
   Judge it by ear on `firefight.wav`, not by the ledger.

Record each library addition's provenance in `sounds.json` (source, author,
license, processing), as the catalog already does.

**Visuals** (judged on the look sheet):

6. `ifv_cannon`: distinct from the tank's HE (flash, tracer, impact);
7. `rocket_pod`: a rocket's motor, smoke trail and salvo impact, not a tank
   cannon's;
8. `rpg`: distinct from the grenade;
9. `tank_sabot` versus `tank_he`: kinetic contact without an explosive fireball
   versus an HE explosion;
10. `atgm`: reduce the launch bloom, glowing flight rod and separated trail puffs;
11. reproduce the reported detached muzzle fireball on the corrected sheet
    before changing it; leave unchanged if it does not reproduce.

**Run / see / hear:** the look sheet (flash, tracer, impact and blast per
look), `firefight.wav`, workbench auditions ([14](14-sound-workbench.md)),
and the menu reel.

**Verify, per sub-slice:**
- Visual sub-slices: state the variable and the crop (that look's row on the
  sheet only). Other looks' wrongness is out of scope. Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  against 13's sheet, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Sound sub-slices: before/after WAVs and the source receipt.
- Re-bless only that look's vector outputs.
- Battle digests unchanged. The approved menu-reel shots keep their timing.
- Human checkpoint per sub-slice with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md),
  non-blocking.

**Depends on:** [03](03-sound-sources.md), [13](13-fold-looks.md) and
[14](14-sound-workbench.md).

**Delegated:**
- crops and processing of sources;
- visual tuning within each look;
- a recipe from existing clips only where 03 found no source, recorded as a
  fallback.

**Would change this slice:** the user rejecting a recording. Pick the next
candidate from 03's ledger.

## Visual item 6 — IFV cannon

**Done:** the `ifv_cannon` visual identity has a shorter, narrower tracer,
less launch glare and dust, and a smaller explosive contact than the tank HE
values it inherited. Its launch-camera radius is retained so the corrected
sampler keeps its framing. Sound, kick and all appearance assignments stay
unchanged. Only that look's effect-vector inputs are refreshed; the recorded
vector outputs remain identical and the owning tests pass unflagged.

Evidence is in shared `evidence-15-visual/6/`: `before/ifv_cannon.png` and
`after/ifv_cannon.png` are the complete scoped row; `capture/` keeps the native
samples, `comparison/` the pixel/edge reports, and `critique.md` the unprimed
verdict. The critic accepts the compact cannon, retaining the nearly white
contact core as an observation under the excluded global bloom/exposure work.
The historical IFV flash/tracer values equal the unchanged tank HE's; the
critic did not independently compare a tank HE plate.

The flight/contact samples match the corrected reference's timing and camera.
The launch centers on a slightly different reported muzzle position, so its
pixel distance is diagnostic; attachment and size are judged against the
visible barrel in the native shot. Contact/blast stills use the same instant
and do not prove later development. The non-blocking Preview checkpoint elapsed without feedback; the scoped
change proceeds on the comparison and unprimed critique, and Preview is closed.

The exact probe changes only the IFV and mixed BMP-3 effect cases. Digests,
events, poses, sounds and hull publications/layouts match `after-13/` byte for
byte. The per-item probe and its delta report retain the proof.

## Visual item 7 — rocket pod

**Done:** `rocket_pod` has a compact motor head, joined smoke wake, restrained
launch flash and smaller explosive contact. Its disabled fireball retains its
radius with a zero lifetime, holding the sampler's launch-camera framing.
Sound and kick remain unchanged. The corrected sampler's single HE carrier
establishes style; it cannot establish actual rocket flight, blast radius,
flash duration or overlapping salvo cadence.

Shared `evidence-15-visual/7/` holds `before/rocket_pod.png`,
`after/rocket_pod.png`, the full native `capture/`, comparisons and the final
unprimed `critique.md`. The first candidate and its rejected flight treatment
are retained under `candidate-1/`: its motor was faint and its white trail
looked like a wire. The revision strengthens the compact head and softens the
wake with overlapping gray puffs. A fresh critic accepts it, including its
hierarchy against tank HE. The wider view's motor is still subdued over pale
crops; the closer view resolves it cleanly. Retain the compact head on that
evidence rather than recreating the broad glowing streak. The critic's
remaining observation is preserved for future content review.

The samples keep reference camera sides/distances and shot-relative stages;
startup timing moves the lane samples slightly, so pixel distances include
that drift and do not prove parity. Contact/blast share one instant. Full
frames preserve the trail outside the supplementary row crop. The final
non-blocking Preview checkpoint elapsed without feedback and Preview is closed.

Only the rocket look's vector inputs are refreshed; recorded outputs remain
identical and the owning tests pass unflagged. Cumulative probe changes are
confined to IFV, rocket-pod and mixed BMP-3 effects. Every non-effect stream
still matches `after-13/` byte for byte; the probe and delta report are retained.

## Visual item 8 — RPG

**Done:** `rpg` now has a compact warm motor and a quieter, softer smoke wake
instead of the grenade's long luminous streak, with a more contained explosive
contact. The separate `rpg_heavy` look is outside this named item's scope.
Sound and kick remain unchanged. Only the RPG look's vector inputs are
refreshed; recorded outputs remain identical and the owning tests pass unflagged.

Shared `evidence-15-visual/8/` holds `before/rpg.png`, `after/rpg.png`, the
complete native `capture/`, comparisons and unprimed `critique.md`. All sampled
ticks, heads, directions and camera sides/distances match the corrected
reference. The fresh critic accepts the powered-projectile cue and its
distinction from the unchanged grenade. The body remains a rounded dark speck;
the existing body authoring offers only color and size, so no silhouette fix
is claimed. Contact/blast stills share an instant and cannot prove motion,
duration or evolution. The non-blocking Preview checkpoint elapsed without
feedback, and Preview is closed.

The final probe is shared `after-15-visual/`. Digests, events, poses, sound and
hull publications/layouts remain byte-identical to `after-13/`. Effect changes
are confined to IFV, rocket-pod, light/general RPG and mixed BMP-3 cases. The
probe's delta report lists every changed case.


## Visual item 9 — closed, no change

**The game already tells them apart.** Real AP rounds make no explosion at the
target: the exact probe's AP cases publish no blasts and their HE counterparts
do (shared `evidence-15-visual/9/real-round-causes.json`). The look sheet
showed both exploding only because its sampler fires every look on a
controlled HE carrier, so its sabot row is not evidence of the game. Both
rounds leave the same gun, so a shared launch flash is the realistic choice.
A look's `blast` is a positive scale of the shared fireball, dirt and smoke;
no value is needed, and none is added. The sheet's HE carrier is a known
limit of the sheet, recorded here for anyone judging kinetic rounds on it.

## Visual item 10 — ATGM

**Done:** the variable is the `atgm` visual identity, confined to its `flash`
and `tracer` values. A compact launch pop and motor replace the broad glare
and glowing flight rod, and closely spaced softer smoke forms a joined wake.
Impact, blast, kick, every sound block and all appearance assignments stay
unchanged. No code or global rendering value changes.

Shared `evidence-15-visual/10/` holds the scoped `before/atgm.png` and
`after/atgm.png` rows, full native `capture/`, pixel/edge `comparison/`,
landmark observations and the fresh critique. The earlier candidates are retained
under `candidate-1/` and `candidate-2/`: direct inspection caught a barely
visible launch pop, and the fresh critique caught a weak default-view motor
accent. The final keeps a compact firing cue and strengthens only the short
head, retaining the narrow wake.

The sampler retains reference camera intent and shot-relative stages, but
startup timing moves ticks and lane positions; pixel distances are diagnostic,
not parity. Native close/default flight and wide aftermath supplement the row
so it does not hide the smoke tail. The controlled HE carrier cannot prove an
actual missile body's visibility at launch, trajectory, speed or temporal
occlusion. Actual shot dust is already absent from this look; no dust value
needs changing. These sampling limits are not claimed fixed.

Only this look's effect-vector inputs are refreshed. Their recorded outputs
remain identical and the owning tests pass unflagged. The exact probe in shared
`after-15-visual-2/` matches `after-13/` in digests, events, poses, sound and
hull publications/layouts. Relative to the completed item-8 pass, effect changes
are confined to ATGM-assigned weapon cases and mixed BMP-3. Both comparisons
have executable scope assertions and retained reports.

The final unprimed critique accepts the compact attached launch and joined
wake over the historical luminous bar. Its motor-head reservation was addressed
by strengthening the short head; the fresh final critic can locate it, while
retaining a modest distinction over wheat at default framing. Keep the compact
head on that evidence rather than reintroducing the rod. The unchanged impact
still has an angular white core and strong glare; no impact improvement or
animation continuity is claimed. Both critiques pass receipt validation.
The final non-blocking Preview checkpoint elapsed without feedback, and Preview
is closed.

The final presentation crate, scoped vector bless and unflagged rerun pass.
The exact probe and value/vector scope audits pass. Shape, diff and docs review
retain the existing owner with no added mechanism. The resumed pass's required
workspace and web checks are recorded with item 11's closeout below.

## Visual item 11 — closed, no change

**Done:** the variable under review is muzzle-flash attachment, scoped to the
`tank_he` and `tank_sabot` rows where the earlier reports saw a detached
fireball. It does not reproduce on the corrected sampler: native default,
close and muzzle-detail stills show fire overlapping both barrel tips, as does
the corrected historical reference. The fresh unprimed critique accepts their
attachment. No visual value, code, or vector changes for this item.

Shared `evidence-15-visual/11/` holds `before/` and `after/` rows for both
looks, full `capture/`, native `details/` muzzle crops, comparison metrics,
capture records and `critique.md`. The sabot launch retains the reference
camera record, tick, head and direction exactly; other samples have startup
or later-shot drift and are diagnostic rather than parity evidence. The
still frames do not establish attachment throughout an animation. The critic's
strong saturation and rounded flash-lobe observations remain distinct from the
unreproduced origin complaint; global exposure is outside this pass.

The non-blocking Preview checkpoint elapsed without feedback. The no-change
verdict proceeds on the corrected captures, comparison and fresh critique;
Preview is closed before continuing.

## Resumed visual pass verification

Shared `evidence-15-visual/checks-summary-2.json` and `logs-2/` hold the required
workspace Rust run and each final check receipt. The final presentation crate
and scoped effect vectors pass; vector outputs are unchanged. Wasm, typecheck,
Clippy, JavaScript lint, formatting and staged size checks pass. The full web
suite had one replay-refusal failure during concurrent build load; that owning
file passes unflagged in isolation without code changes. Required LFS media were
materialized before reruns. No rules, sound, renderer code or global settings
change. Both item-specific fresh critiques pass receipt validation, and both
non-blocking Preview checkpoints are closed.

## Visual pass verification and review

The workspace Rust suite completes successfully. The final presentation crate
run passes after all visual edits. Wasm, typecheck, Clippy, ESLint, formatting
and commit size ratchets pass; Oxlint passes with existing warnings. Full web
Vitest runs once; its icon and audio-provenance failures were LFS pointers,
resolved by materializing those source dependencies and rerunning the affected
files unflagged. No code or requirement was changed for setup failures. Shared
`evidence-15-visual/checks-summary.json` and `logs/` hold the receipts.

Shape, diff and docs review retain one settings owner and introduce no code,
abstraction or interface. Only the three named visual looks and corresponding
vector inputs change; every sound block is byte-identical in both files.
Owning-slice links resolve. The final probe and all three unprimed verdicts
stand; global bloom/exposure, small-arms contrast over wheat and the stated
visual limitations remain outside this pass.

**Sound done — `artillery` (2026-10-29):** Item 1: a recorded M777 muzzle report replaces the rifle burst. Its aligned crop excludes the target impact. Evidence: shared `evidence-15-sound/artillery/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `mortar` (2026-10-29):** Item 2: the user-confirmed distinct 82 mm launch replaces the howitzer placeholder; source impact excluded. Evidence: shared `evidence-15-sound/mortar/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `strike_missile` (2026-10-29):** Item 1: one aligned M270/ATACMS launch with sustained exhaust replaces the rifle burst. Evidence: shared `evidence-15-sound/strike/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `bomb` (2026-10-29):** Item 1: the prescribed restrained handling/air fallback replaces the rifle burst; no explosion at release. Evidence: shared `evidence-15-sound/bomb/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `rocket_pod` (2026-10-29):** Item 1: the prescribed AT4 attack/exhaust fallback plays once per rocket. Apache rotor separation was not established on listening, so its conditional recording is not admitted. Evidence: shared `evidence-15-sound/rocket-pod/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `marksman_rifle` (2026-10-29):** Item 3: one M110 report per launch; no authored burst is invented. Evidence: shared `evidence-15-sound/marksman/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `heavy_sniper` (2026-10-29):** Item 3: one Barrett report per launch; reload time is not treated as burst cadence. Evidence: shared `evidence-15-sound/sniper/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `aa_gun` (2026-10-29):** Item 3: three distinct Bofors reports relayed at 0, 300 and 600 ms, documented as the prescribed fallback. Evidence: shared `evidence-15-sound/aa/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `light_machine_gun` (2026-10-29):** Cadence gate follow-up: the inherited 100 ms rifle recording also mismatched this weapon. Existing reports are relayed at its unchanged 80 ms cadence; no rule changes. Evidence: shared `evidence-15-sound/lmg/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — `rocket_artillery` (2026-10-29):** Item 1: a short M270 ignition/exhaust crop plays per rocket; recorded as a designed fallback, not a natural salvo. Evidence: shared `evidence-15-sound/rocket-artillery/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

**Sound done — item 4 (2026-10-29):** `SoundTables` admits firing recipes
against each resolved weapon, including derived and encounter-added rows.
Multi-report clips require a magazine, fit its capacity and burst group, and
match its shot interval within 5 ms. Single reports remain valid. The regression
was written and observed red before enforcement; count and tolerance boundary
checks pass, as does the owning sound suite. Evidence: shared
`evidence-15-sound/4-cadence/`. This refusal changes no valid playback output.

**Sound done — `rifle` (2026-10-29):** Item 5: amplitude levels round to two decimals and switch distances to integer metres. The maximum amplitude move is 0.58 dB (rifle); the ledger and paired firefight retain the listening evidence. Evidence: shared `evidence-15-sound/5-levels/` (paired launch auditions from the production bank, firefight WAVs, source receipt, scene and scoped vector checks).

## Sound verification and handoff

The standard firefight does not fire the new artillery, bomb, pod, AA or
precision-rifle sounds. Each content checkpoint therefore also includes paired
launch auditions composed and calibrated through the production SoundBank.
Every content scene passes. Source receipts pin the original download, decoded
frame crop and processing; all new clips rebuild byte-for-byte through the
existing asset tool. Measured relayed attacks stay within 1 ms of AA's spacing
and 2 ms of the LMG's. The LMG correction is required by the general cadence gate,
not a new firing rule. The rebuilt Wasm admission surface refuses the original
shipped data at AA's mismatched cadence and accepts the corrected documents.

The exact probe in shared `after-15-sound/` preserves slice 13's battle digests,
events, poses, hull publications and hull layout byte-for-byte. Every numeric
effect field and bit matches; only the new artillery appearance names move
light-cause labels. The sound projection verifies every remaining command after
excluding the named firing replacements, their voice ids and consequential
ducking; all remaining gain changes match the rounding ledger. Coverage changes
only in sound-command counts from corrected AA and LMG burst coverage. The
recording commits leave the standard lab's output unchanged; level rounding also
changes its voice-budget ranking and consequent command ordering.

Both recording/level listening checkpoints elapsed without further feedback;
QuickTime was closed. Mortar has the user's explicit audible verdict. Agent-side
timbre listening remains unverified, and the Apache recording is not admitted.
The gate-only firefight uses the same unrounded look snapshot in both arms; it
changes by at most one PCM16 step in WebAudio rounding. Its controlled capture
and listening checkpoint are recorded with item 4.

The closeout shape/diff/docs review keeps one admission owner, no playback or
simulation changes, no visual retunes, and no new authoring schema. Full web
verification found one obsolete test that pinned the research ledger to three
receipts. It now checks the receipt contract on a fake catalog; its focused
suite passes. Shared `evidence-15-sound/report.md` links the paired WAVs, receipts,
ledgers, exact validation, checks and commit list. The visual half remains with
its implementing agent; 15 stays unchecked in the README for integration.

The full workspace Rust run, Wasm build, typecheck, lint/clippy, formatting and
size ratchet pass. The full web run passed every other suite; the corrected
receipt suite passes on focused rerun. Shared closeout logs record the results;
the documentation-only handoff edit does not invalidate those checks.

**Review follow-ups (closed):**

- Dropping the pinned receipt inventory also dropped the check that each shipped download receipt matches its file. A data-driven test now requires every receipt's file to exist with the recorded SHA-256, without naming receipts or counting them.
