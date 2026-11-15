# Decisions ledger

Decisions made where the spec was silent, appended per pass. Slice 09 rewrites
this ledger from scratch against the shipped code.

## Slice 01

- **`infantry_roster.py` lives in the Blender folder's root, not in
  `roster/`.** Sound.
  - *Why:* Blender doesn't put a script's own folder on `sys.path`, so
    `roster/` modules can't import each other. The crew code (slice 02) needs
    it too.
  - *Change:* the spec's paths updated.
- **`armies.json` holds only what the generated path and the generation tool
  need:** faction, rifle, LMG, gloves, cloth mean, prompt, looks.
  `PROCEDURAL_LOOKS` keeps the shells' own fields (print, gear colours,
  helmet, boots, per-look variants), and `army_look()` merges the two for
  `Kit`. Sound, and temporary: both the merge and `PROCEDURAL_LOOKS` go in
  slice 08.
- **`KITS` moved into `infantry_roster.py`**, keeping one owner of the
  roster's data beside `source_path`. Sound. The spec said `KITS` stays in
  Python; it does.
- **The infantry receipt lists its skins from the pins, not from
  `skin_pins.WORN`.** Sound: the receipt runs in its own Blender process after
  the exports, so `WORN` is empty there. It also hashes `armies.json`, which
  shapes every export.
- **Vehicle receipts are not refreshed in this pass**, although their scripts
  changed (`skin_pins` is now loaded). Sound: no check reads them, and slice 09
  rewrites every receipt.
- **The generator's prompt text is written now, in `armies.json`**, from
  today's look table; slice 04 composes it. Provisional: slice 05 judges
  whether the wording gets the cues.

## Slice 02

- **The crew kit is army data (`armies.json` `crew_kit`).** Sound.
  - *Why:* the Marines aren't in `rifle_squad`, so the LAV's MARPAT crew
    (slice 07) needs another kit: `us_marine_squad_close_quarters_infantry`.
  - *Values:* `rifle_squad` for the three armies in it today. The French name
    their crew kit in slice 06.
- **No unit test for the "crewed family without `crew_army`" refusal.** Sound.
  - *Why:* it is a one-line guard, and testing it would mean mocking `bpy`
    deep inside `_export`. AGENTS.md keeps unit tests for complex logic.
  - *How it was checked:* reading the code; all six families export with
    their army.
- **Proof against fresh references:** the Tigr's reference crew came from a
  freshly exported EMR soldier, which differs from the committed file (the
  pre-existing drift). Its proof re-exported that soldier first. Sound.

## Slices 03 and 04

- **`back_y` and `bone_of` live in `infantry_rig.py`**, the rig's attachment
  helpers, not on `Kit`. Sound.
  - *Why:* `infantry_kit.py` is over the size budget and may not grow.
    `infantry_rig.py` already owns where things attach (the holds).
  - *Rule:* rigid parts on `spine_03` count by their bounds, as before;
    skinned parts count by their vertices whose strongest weight is
    `spine_03`.
- **`fit.lies` measures the body and what it carries, not kit that stands on
  the ground by itself.** Sound, and a physical rule.
  - *Why:* the bake's first run flagged every tripod ATGM (TOW and Kornet,
    active) at 1.3–1.4 m. A tripod is weighted wholly to the root joint, and
    after its operator dies it stays standing, as it would in reality.
  - *Rule:* `lyingBounds` skips vertices weighted wholly to the root.
  - *Test:* a lying soldier with a 1.2 m tripod passes.
- **Procedural carried launchers and the test `at_carried` soldiers fail
  `fit.lies`, and the bake stays red until slice 09.** Provisional; slice 09
  must make the bake green.
  - *The defect:* the slung tube stands 0.60–0.74 m on a face-down corpse, or
    sinks up to 0.3 m into the ground on its back. It predates this work;
    a fresh procedural export still fails. Generated carried kits pass.
  - *The call:* the gate stays strict (0.6 m). These files are replaced in
    slice 09 (roster) and slice 08 (test soldiers), so they aren't re-exported
    procedurally first.
  - *Until then:* slices 05–08 judge through per-file `asset validate` and
    loose-GLB sheets, which don't need the bake.
- **The pilot's torso is about 7% brighter than the procedural soldier's**
  (front-crop mean red 126 against 118). Sound.
  - *Why:* the generated carrier is coyote, as the US Army photos and the
    prompt have it; the procedural one was printed in OCP. The overall cloth
    mean matches by construction. A kit difference, not a grade error.
- **The hydration hose standing off the vest at the crew tier is accepted.**
  Provisional; re-judged at slice 09's crew review.
  - *Why:* real kit has it, it is invisible at battle distance, and slice 05
    regenerates this body.
- **Slice 04's own calls** (from its report). Sound:
  - an input image is keyed by its PNG's sha256, because the image command
    prints no request id;
  - images are never retried automatically, since a paid repeat is a
    person's call;
  - `--model` is passed explicitly, so `image_model` is accurate;
  - a failure costs $0;
  - airframes keep overwriting `<appearance>.glb`, which their exports read.
  - *Accepted risk:* a submit retried after a 504 could pay twice if the first
    submit actually landed. That is rare, and the ledger would show it.
