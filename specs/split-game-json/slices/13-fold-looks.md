# 13 Fold to the final looks

**Contract:** weapons name the final set of looks. Near-duplicates are
folded, and the looks the content pass needs exist, so [15](15-content.md)
only edits look values and never moves a hash.

**The final assignment (about 18 looks):**
- **Fold** sound nudges under 2 dB into one level per look: tank AP ×4,
  ATGMs ×9, SAMs ×6, MANPADS ×3, rifle and carbine.
- **Split** where the difference is real:
  - `ifv_cannon` (the BMP-3's 100 mm main gun) from `tank_he`: 9 dB;
  - `aa_gun` from the HMG: 7 dB;
  - `bomb` from `artillery`: 20 dB;
  - `rpg` from the grenade: 11 dB.
- **Own looks for the content pass**, holding today's values until 15
  changes them: `rocket_pod` (today it looks and sounds like a tank cannon),
  and the strike missile's look (today it fires the rifle's shot).
- **Names** say what the look is (`tank_sabot`, `tank_he`, `ifv_cannon`,
  `atgm`, `manpads`, `sam`, `aa_gun`, `artillery`, `bomb`, `rpg`, `rifle`,
  …), never one of the weapons that share it.
- Every transient look from 08 is gone.

**Seam:** weapon rows' `appearance` in `rules.json`, and
`presentation.json` `weapon_looks`.

**Run / hear:**
- A level ledger: for every weapon, its old effective gain, its new gain,
  and the difference. Folds stay under 2 dB; splits are as listed.
- `firefight.wav` before and after.
- The look sheet.

**Verify:**
- The probe diff from 01 shows only the ledger's gain changes and the named
  splits.
- The fixture test: every look is used.
- Battle digests unchanged.
- Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  on the sheet, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) and the
  WAVs, non-blocking.

**Moves:** `rules_hash` and replay config digests, once. Rewrite the stored
records with `throwaway/split-proof/rewrite-look-folds.mjs`, then re-bless
the sound and effect vectors as named changes.

**Depends on:** [11](11-named-fixes.md) and [12](12-firing-look.md).

**Delegated:** the level each folded group keeps, justified by the ledger (the
loudest member's level, or the median; say which and why).

**Would change this slice:** a fold the user can hear as wrong. Split it into
its own look.

## Input from 11 — heavy RPG split

Slice 11 deliberately keeps one RPG look and restores the light RPG's levels
for all three weapons, preserving `rules_hash`. The user chose this interim
level rather than moving appearance assignments in 11.

`rpg_at_heavy`'s stranded authored values are shot gain **0.55**, far
**400 m**; blast gain **0.8**, far **450 m** (its contact loudness multiplier
was **3.5**). Against the shared light row, the heavy blast is about **3.25 dB**
louder, exceeding this slice's 2 dB fold threshold. Give the heavy RPG its
own look here and restore those levels; the shot difference alone is about
1.74 dB. The light/general RPGs may retain the light row.

## Fold decision

Use the per-weapon median in dB for shot, blast, motor and contact amplitudes;
for an even group, use the midpoint of its two central dB levels. This
minimizes total absolute level movement without making every group louder.
Near/far recording switch distances use their median in metres. Identical
values retain their exact representation. The ledger records every weapon,
including unchanged parts, and the old/new switch distances.

The final assignment is authored in `rules.json`; `weapon_looks` contains only
its used destinations. The canonical tank and ATGM visuals win their folds,
including the parents' authored water impacts and the ATGM's ground impact.
The derivative looks that lacked those surfaces acquire them. All other
visuals and every kick retain their current values. The old autocannon look
is named `autocannon`, reserving `ifv_cannon` for the BMP-3's 100 mm gun as
specified. The heavy RPG's separate look restores 11's recorded shot, blast
and contact levels. Content-pass placeholders preserve their current values.

The detailed level ledger and final assignment sheet live in shared
`throwaway/split-game-json/after-13/`; the one-shot migration is
`scripts/rewrite-look-folds.mjs` beside that directory. Stored historical
weapon aliases remain aliases in those records and name the appropriate
final looks; no battle rule is migrated.

**Exact probe result:** `after-13/digests.jsonl`, poses, events, hull
publications and their layout are byte-identical to `after-12/`. Every numeric
effect field is also identical in this corpus; only light-cause look names
change. Every changed sound gain equals a ledger ratio. SAM contact gain
crosses the existing ambience-duck threshold in one `aa_medium` impact and
four `air_missile` impacts, adding exactly five duck/release pairs. All other
sound command fields, counts and ordering hold after removing those named
ducking commands. The corpus does not land a folded derivative on the ground
or water; the controlled look sheet supplements those surface values.

**Records and regression fixtures:** the one-shot script rewrites stored
inputs, then the catalog, paired-scenario, effect and sound owners re-bless
and run unflagged. Paired records change only appearance strings; stored
effect frames remain identical; the sound vector changes only gain fields.
The hull-dust test follows each weapon's appearance. The two-row firing test
owns a distinct second-round look and kick, so shared shipped looks cannot
zero both rows when it makes the first row's kick silent.

**Capture corrections:** reviewing the paired sheets exposed two independent
harness defects beyond the removed mechanics snapshot field. Sampling any HE
projectile could select a street tank instead of the review gun; samples now
start at the review muzzle. The visibility probe now tests the latest segment,
where interpolation draws a short tracer, rather than only its published
endpoint. The initial before/after sheets and pixel distances are retained as
diagnostics, not matching-camera parity. The corrected `evidence-13/final/`
sheet covers every final look and is the reference for 15. Its source visual
blocks equal the corresponding canonical pre-fold blocks; no content retune
is hidden in these capture corrections.

**Checks:** workspace Rust tests were run once. The sole failing target was
the two-row test's shared-look assumption above; the repaired owning suite
passes all 11 tests. Full web Vitest ran once; after materializing the source
models and provenance audio that remained LFS pointers, the two affected files
pass on focused rerun. The Wasm build, typecheck, lint/clippy and formatting
pass. The staged size ratchet runs at commit.

**Done (2026-10-29):** final assignments cover all 49 weapons with 21 used
looks. Every transient key is gone. Ordinary amplitude folds stay below 2 dB;
the largest is the light tank HE contact level at +1.74 dB. The heavy RPG's
named restoration raises its blast by +3.25 dB and contact by +2.92 dB.
The shared `after-13/` holds the complete ledger, assignments, identity reports,
exact delta report and executable delta validation. Only appearance fields
change in rules and paired scenario records. Battle digests do not move.

**Visual review and checkpoint:** the unprimed critique's receipts check passes.
It confirms the corrected flight samples improve substantially, and finds no
confidently detached muzzle flash. It does not give general content sign-off:
strong bloom hides cores and silhouettes, small-arms tracers have low contrast
against wheat, the ATGM reads as a glowing rod, and background smoke competes
with samples. Contact/blast stills use the same instant and cannot prove blast
development; before/final framing differences prevent pixel-parity claims.
These are inputs to [15](15-content.md), alongside 11's and 12's observations;
13 retains the canonical visual values by contract. The complete captures,
metric reports, critique and comparison boundary are in shared `evidence-13/`.
The fold is accepted on its exact data/probe proof, without claiming the
existing content or the mismatched-camera pictures are visually approved.
The non-blocking Preview checkpoint elapsed without feedback; the scoped fold
proceeds on that proof, and Preview is closed.
