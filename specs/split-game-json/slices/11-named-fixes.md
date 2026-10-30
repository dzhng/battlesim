# 11 Named fixes: kick, impact size, RPG level

**Contract:** three staging bugs the audit found are fixed, each as its own
commit with its own named delta.

1. **Tank guns kick.** No unit fires `tank_ap` directly, so derived tank
   rounds never kicked (`kick_of`'s exact-id match). With looks, every
   weapon on the tank looks gets the tank's hull kick, and the soldier kick
   covers ATGM, RPG, Igla and FN-6.
2. **Tank-round ground impacts at their intended size**, 5x size and 5x
   lifetime (`impacts.*.round_scale`). Today they show at 3x and 1x.
3. **RPGs back to their own loudness.** They have played the grenade's rows
   since the 2026-10-16 rename (`b4410901c`), about 11 dB quieter.

Each fix edits the values of [08](08-looks.md)'s transient bug looks
(`tank_sabot_no_kick` and the like) to match their siblings. No weapon
changes which look it names, so no hash moves. The looks that are now
identical are folded by [13](13-fold-looks.md).

**Run / see / hear:**
- a cropped clip of a tank firing, before and after (hull rock);
- impact plates from the look sheet;
- `firefight.wav` before and after, with the RPG's level in the ledger.

**Verify:**
- Before each re-bless, list the expected deltas: which rows, which outputs.
- Re-bless only those: `UPDATE_SOUND_VECTORS`, `BLESS_PARITY`, native plates.
- Battle digests unchanged.
- For the visual fixes, run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint: open the clips and plates with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md). It is
  non-blocking: about 5 minutes, then decide on the evidence, record it here,
  close the windows and go on.

**Moves:** presentation outputs only. `rules_hash` doesn't move, because no
weapon changes which look it names.

**Depends on:** [10](10-battle-rules.md).

**Delegated:** clip framing.

**Would change this slice:** the user finding a tank's rock or an impact too
big. Retune the look's values. That's a data edit and moves no hash.

## Named delta 1 — recoil (declared before changing values)

The seven derived tank-gun rows (`ground_tank_ap`, `ground_tank_he`,
`advanced_tank_ap`, `advanced_tank_he`, `light_tank_ap`, `light_tank_he`,
`ifv_he`) gain the base tank look's hull kick. The rocket pod keeps its
explicit zero hull kick; a helicopter crash is not a firing gun.

The ATGM-derived looks (`tow`, `bastion`, `kornet`, `spike`, `javelin`,
`akeron_mp`, `heli_atgm`, `air_ground_missile`) get ATGM soldier recoil;
`igla_s` and `fn_6` get MANPADS recoil; `rpg_at_light`, `rpg_at_heavy`,
`rpg_general` get grenade-launcher recoil. These fix default-strength
soldier recoil, rather than an absence of all soldier motion.

Allowed outputs: hull/soldier poses and effects/sound positions carried by
those poses, only for those weapons (including `bmp-3`, whose gun fires
`ifv_he`). The existing all-weapon probe mounts missiles/RPGs on hulls,
so their soldier-only multiplier changes need not move its output.
Sound levels, impact styles, recordings, rules and all battle digests stay
unchanged. Re-bless only affected presentation vectors, then run unflagged.

### Receipt 1

The battle-driven regression failed on the stationary firing hull, then passed
with the data edit. Relative to `after-08-merged/`, the probe changes 459 effect
rows and 3,348 pose rows: each of the seven derived gun cases has 17 effect
and 227 pose rows changed; `bmp-3` has 340 and 1,759. All sound commands,
events, hull publications/layouts and battle digests remain byte-identical.
The report is `after-11-1/delta-report.txt` in the shared proof directory.

Only the corresponding kick inputs change in the effect-vector fixture;
recorded outputs do not change. `UPDATE_SOUND_VECTORS` deliberately exits
with its rerun reminder after writing, then the sound, effects, poses and
look tests all pass unflagged. `BLESS_PARITY` leaves effect outputs unchanged.
Native plates were captured under the shared GPU lock for integration review.

## Named delta 2 — ground impacts (declared before changing values)

For `ground_tank_ap`, `ground_tank_he`, `advanced_tank_ap`,
`advanced_tank_he`, `light_tank_ap`, `light_tank_he` and `ifv_he`, ground
contacts use the base tank round's surface multiplier: 5× material size
and 5× material lifetime. Water and other surfaces do not change. Rocket
pods and helicopter crashes keep their distinct looks for the later content
pass.

Allowed additional probe deltas are effect instances and impact cast lights
for those seven gun cases and the mixed `bmp-3` case. Poses, sound commands,
authoritative events, hull publications/layouts and battle digests must stay
byte-identical to fix 1. The effect-vector migration changes only those
looks' ground-surface inputs before `BLESS_PARITY`; sound is expected to
remain unchanged under `UPDATE_SOUND_VECTORS` and its unflagged rerun.

### Receipt 2

The controlled ground-contact regression first failed on the rendered puff,
then passed for early size, survival beyond the unscaled material lifetime,
and eventual expiration. Only the seven ground-surface inputs change in the
stored effect fixture; its recorded base-weapon output stays unchanged.
Sound/effect re-blessing and all focused sound, effects, pose, look and effect
behaviour checks pass unflagged.

The standard probe has no tank ground-contact rows: its tank shots hit hulls.
Consequently every stream is byte-identical to fix 1, including all battle
digests. This is a coverage limit, not evidence of a visible change;
`after-11-2/delta-report.txt` names it explicitly. A separate controlled ground
capture supplies visual review evidence. Native capture is queued under the
shared GPU lock; review remains with the integrator.

## Named delta 3 — shared RPG level (declared before changing values)

`rpg_at_light`, `rpg_at_heavy` and `rpg_general` keep their existing shared
look. Per the user's decision, that look takes the stranded light RPG row's
levels: shot gain 0.45 / far 350 m, blast gain 0.55 / far 350 m, contact
loudness multiplier 2.5. Recordings and visual parts do not change. These
values come from the stranded `rpg_light` rows in the last mixed fixture;
the `b4410901c` rename predates those authored sound rows.

Only those three weapons' sound-command rows may change beyond fix 2.
Effects, poses, events, hull publications/layouts, rules and all battle
digests stay identical. The committed sound vectors do not fire RPGs, so
their output and the standard sound-lab firefight are expected to remain
unchanged. Separate RPG shot/blast/contact WAVs make the named audible
delta reviewable. Re-bless the corresponding sound inputs, run the focused
sound/effect re-blesses, and rerun unflagged.

The heavy RPG's original shot/blast gains (0.55 / 0.8) and far distances
(400 / 450 m) are deferred to 13, which may split appearance assignments.
The user explicitly chose no `rules_hash` change here.

### Receipt 3

The RPG launch regression first failed because it played at the grenade's
level, then passed after the data edit. Against fix 2 the exact probe changes
two sound rows each for `rpg_at_light`, `rpg_at_heavy` and `rpg_general`,
six rows total. Every other stream is identical, and all battle digests still
match `after-08-merged/` byte for byte. Reports for each fix are beside its
`after-11-<n>/` dump in the shared proof directory.

Separate before/after RPG WAVs use the same recordings, listener, clock and
offline audio graph. Measured RMS changes are +11.480626 dB (launch),
+5.264828 dB (blast), and +1.938201 dB (ground contact). Their level ledger is
`evidence-11/rpg-level-ledger.txt`. The standard firefight has no RPG event;
its before/after peak is identical and 55 of 768,000 PCM samples differ by
one 16-bit least-significant bit (RMS difference 0.000000083 dB).

**Heavy RPG interim difference:** this slice deliberately uses the shared
light row, so `rpg_at_heavy` remains 1.743 dB quieter on launch and 3.255 dB
quieter on blast than its stranded authored levels (contacts 2.923 dB
quieter). Far distances remain 350/350 m here instead of 400/450 m. The
user-authorized heavy look split and the original values are recorded as an
input in [13](13-fold-looks.md). This preserves `rules_hash` in 11.

## Done — three presentation data fixes

Each cause is isolated in its own commit. No weapon changes its appearance
reference; `rules.json` and all battle digest streams are byte-identical.
The shared dump digest SHA-256 remains
`2ee49f6b869abe7bd7841615026c6a4149753b9f06073b9d33bed78c8a7938c1`.
The heavy RPG's intentional interim level difference is documented above and
assigned to 13.

The three regressions were observed failing on the old data, then passing.
Final `cargo test -p presentation -p sim --no-fail-fast` passes 1,640 tests
across 46 targets. Rebuilt Wasm passes the 54 selected sound/projectile/capture
web tests; a further 62 audio/workbench/visual-publication tests pass. Typecheck,
ESLint, Oxlint, workspace/all-target Clippy, formatting and the size ratchet
pass. The sound scene passes before and after. Missing LFS assets were
materialized locally; no source, baseline or requirement was changed to fix
those setup failures. Review of shape, diff and documentation found no new
production abstraction, dependency or runtime API.

Evidence lives under
`/Users/server/dev/battlesim/throwaway/split-game-json/evidence-11/`:
`before/` and `after/` contain native plates, tank clips, ground-impact
captures, `firefight.wav`, and isolated RPG WAVs; the root holds exact delta,
identity and level reports. Native captures ran under the shared GPU lock and
all six frozen plates remain byte-identical. The look sheet recaptures the
seven hull-visible changed looks with baseline cameras, and explicitly reuses
the other thirty whose hull-visible inputs did not change. The capture-scope
receipt names every reuse. The baseline is complete for all 37 looks.

The implementation brief reserves screenshot critique and listening acceptance
for the integrator. Only framing/nonblank evidence is checked here; no artistic
acceptance, Preview checkpoint or child-agent critique is claimed. The probe's
missing tank-ground case is covered by the controlled ground-contact regression
and real ground-attack captures. All scratch evidence and scripts remain ignored.

Capture receipt: the tank clips each hold 91 frames at matching ticks/camera
(launch tick 108). The seven changed-look captures pass the scene, and 56
before/after detail pairs have comparison artifacts. The full sheet uses a
wide viewport to retain all four comparison columns. The ground pair hits the
same point at tick 185; its supplementary isolated version disables the HE
carrier's blast radius in both arms, explicitly disclosed in `capture.json`,
to expose the contact puff. It changes no shipped rule. An AP capture attempt
was discarded because it selected an enemy shell and could not supply a
matching own ground hit. `evidence-11/capture-receipt.txt` identifies the
reviewable captures and their controls.

**Checkpoint resolution (2026-10-29, integrator; non-blocking, decided on the
evidence):** accepted.
- **Ground impacts:** an unprimed critique of `evidence-11/` found the impact
  dust now lasts and reads at 1.4 s, and is a little larger. Nothing broken.
- **Tank recoil:** it restores the intended value, but at the evidence
  camera (18 m) the hull rock is under a pixel (0.2–0.8 px, rocking back and
  forth over frames 15–24), so a viewer can't see it. If it should read on
  screen, raising the tank look's `kick.hull` is a data edit and moves no
  hash. It's flagged to the user, not changed here.
- **The standard firefight WAV** differs by one 16-bit step in 55 samples,
  though it has no RPG event. That's within rounding, and the Rust command
  oracle is byte-identical.
- **Seen in both versions, out of scope:** the muzzle fireball sits about
  30 px ahead of the barrel for several frames; the tank's lighting jumps
  across frames 13–15; a heavy round into dirt reads as an orange fireball
  with no thrown soil. These are candidates for the content pass
  ([15](15-content.md)).

**Review follow-ups (closed):**

- The tank-impact regression pinned the shipped ×5 lifetime; it now authors
  its own look's ground size and lifetime and judges against them.
- The RPG loudness regression pinned the shipped RPG-over-grenade ordering;
  it now authors both looks' levels, swaps them, and requires each launch to
  follow its own look.
