# 12 The firing look follows the round fired

**Contract:** a hull's flash, firing sound and kick come from the round it
fired, not from its mount's first weapon. After this:
- the BMP-3's Bastion fires with the ATGM look;
- tank HE fires with the HE look.

**Seam:**
- `effects/launches.rs` hull launches, and the launch consumers in effects,
  pose and sound.
- **02 result: no conforming candidate.** A recovered all four BMP-3/tank
  rounds, but moved launches from ticks 2/9 to 3/10 because flight precedes
  fire (`sim/src/battle.rs`). Its effects vector moved (30 vs 38 instances
  at 0.05 s); sound/pose vectors held. Mount eligibility still needs resolving.
- B's last-fired `kind` injection kept ticks 2/9; the existing vector suites
  held, but this was an **oracle, not a production producer**. `kind` exists
  only in `effects::publication::Mount`; `sim/src/publication.rs::pose`
  publishes no round kind, and identified hulls expose no ammo. Hull counters
  and soldier segment matching also remain distinct. Prefer B's semantics
  only after approving its publication source and a shared matching rule.
- Both traces held all per-tick digests, replayed exactly, and passed
  `exact_numbers`. A moved only replay `engine_build`; B moved no replay bytes.
  This was the spike outcome before the integrator selected the mechanism
  recorded below; its unchanged-flash-tick and attachment constraints remain.

**Run / see / hear:** the look sheet's firing rows, and a clip of a BMP-3
firing its gun then its Bastion, and a tank firing AP then HE.

**Verify:**
- Write first: a presentation test in which a two-weapon mount fires its
  second row, and the launch carries that row's look. Cover:
  - a moving, turning hull;
  - a skipped publication;
  - a round whose first stretch is clipped by fog;
  - an immediate impact.
- List the expected deltas, which are hull firing effects of non-first
  weapons only. Re-bless those and nothing else.
- Battle digests unchanged.
- Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md),
  non-blocking as in the README.

**Moves:** presentation outputs for non-first hull weapons. If 02 chose to
fill a published value, the publication bytes move too; battle digests don't.

**Depends on:** [02](02-hull-launch-spike.md) and [10](10-battle-rules.md).

**Delegated:** the implementation inside the chosen mechanism.

**Would change this slice:** any battle digest movement, or launch timing or
attachment changing with the selected field.


**Done (2026-10-29):** the integrator selected the recommended mechanism on the
user's behalf after the slice-02 question went unanswered. `WeaponPose.round`
is the index of the mount's last-fired weapon row, initially zero, retained
where the simulation counts the shot. It is presentation evidence, deliberately
excluded from `Mount::digest`, and travels only with already-visible poses.
The packed pose layout and native/Wasm/TypeScript views carry it. Presentation
keeps the full ordered mount weapon list, with each row's admitted kick, for
launch effects, firing sound and pose kick. Soldier launches still match member
segments. Hull launches still use counter rises and their own muzzle.

**Proof:** `after-12/delta-report.json` in the shared proof directory compares
against `after-11-3/` and `after-08-merged/`. The complete digest stream is
byte-identical to both. Relative to 11, only BMP-3 presentation changes: its
Bastion launches now name Bastion instead of IFV HE. Launch tick, position,
direction, all flight segments and actual fired rounds are unchanged. The probe's
tank cases mount only one row, so the two-row AP/HE capture and synthetic
regressions supplement that corpus.

**Records:** `BLESS_PARITY=1` on the publication encoder tests refreshes
`publication/{stream,combat,arrangement,cover-facing,contact-lifecycle,codec-vectors}.json`:
publication hashes/words and the codec's layout change for the new field;
digests do not. The codec's logical inputs now include distinct round indices.
The publication-reader owning test rebuilds both capture streams and their
observation fixtures, preserving and checking every recorded battle digest;
it also updates the Paris capture hash in its manifest. The native count-test's
same bless flag refreshes its gzipped capture and manifest, while retaining
and verifying every browser count. The fog-sight owning test's same flag
refreshes its embedded street layout and `fog-street-publications.u32`; every
fog upload, dispatch, frame and stat vector remains identical. The pose fixture's unit section is an input
schema migration from a first weapon/kick to an ordered row list. No effects,
pose or sound output vectors were re-blessed; all still pass.

**Evidence:** shared `evidence-12/` contains all paired BMP-3 gun/Bastion and
tank AP/HE frames, launch ledgers, contact sheets, pixel comparisons, scripts
and `review.md`. Captures held the GPU lock. The before arm reconstructs the
removed first-row selection at the presentation boundary; battles are identical.
Tank AP and HE currently author identical launch flash/kick/firing sound, so
that visual pair alone cannot demonstrate row selection. The system regression
uses distinct first/second-row kicks to make it observable. Nested reviewers
were outside this implementation delegate's remit; independent unprimed visual
acceptance and the human checkpoint remain with integration.

**Tests:** the real-battle second-row regression was observed failing before
implementation. The moving/turning, skipped-publication, fog-clipped and immediate
impact regressions were also falsified together by restoring first-row selection,
then passed after restoration. Publication, presentation vectors, browser decode,
typecheck, lint/clippy, formatting and the file-size ratchet are checked in this
pass; full-suite receipts are retained with the evidence.

**Checkpoint resolution (2026-10-29, integrator; non-blocking, decided on the
evidence):** accepted.
- **BMP-3 missile launch:** an unprimed critique of `evidence-12/` found it
  no longer opens with the 100 mm gun's large fireball; it gets a compact
  flash at the muzzle. The gun shot is unchanged.
- **Tank AP against HE:** they look identical before and after, because both
  looks author the same flash, kick and sound. The selection is proven by the
  decoded launch record and the regression test, not by the pixels.
- **Look content handed to [13](13-fold-looks.md) and [15](15-content.md):**
  - the missile launch is a large glowing dust bloom that hides the missile
    for about five frames, where a viewer expects a sharp pop and a thin
    trail;
  - the missile trail shows as separate cotton-ball puffs;
  - AP and HE can't be told apart;
  - the muzzle fireball sits 40–60 px ahead of the barrel tip (also seen in
    11).
