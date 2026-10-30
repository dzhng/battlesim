# 15 Content pass

**Contract:** every look sounds and looks like its weapon. The firing sound
always fits the weapon, and burst recordings match each weapon's rate of fire.

Each item below is its own sub-slice: one variable, one verdict, one commit.
None moves a hash. The looks already exist from [13](13-fold-looks.md), so
only their values change.

**Sound** (sources from [03](03-sound-sources.md)):
1. firing recordings for `artillery`, `bomb` (release), the rocket salvo for
   `rocket_pod`, and the strike missile's look. Today they fall back to the
   rifle shot or the tank cannon;
2. mortar, if a mortar weapon uses the artillery look and the user can hear
   the difference;
3. burst recordings at cadence: the marksman rifle, the heavy sniper and
   `aa_gun`;
4. **the cadence check:** at fielding, a look whose burst recording doesn't
   match its weapon's rate of fire is refused (`sound/frame.rs:846-866`).
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

## Visual items 10 and 11 — resumed separately

The first visual pass stopped at item 9 and did not reach these; they resume
in their own pass from this point.

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
