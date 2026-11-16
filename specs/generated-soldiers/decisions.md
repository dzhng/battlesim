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
    a fresh procedural export still fails. *Correction (slice 06):* generated
    carried kits fail too. The French Akeron and Javelin carried kits stand
    0.68–0.71 m face down and sink 0.23–0.26 m on their backs. The earlier
    "generated carried kits pass" was only checked on a slimmer tube. The fix
    is how a big tube rides the back (its sight block faces out today); slice
    09 owns it.
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

## Slice 05 probes (fit changes they forced)

- **The fit centres the body front to back on its thighs, not its torso.**
  Sound.
  - *Why:* with a pack or ruck, the torso band's centre sits behind the body.
    The whole soldier then landed half a pack-depth too far forward, which
    left the head forward of its joint and shrank the scout to 1.618 m in
    `stand_aim`. The thighs never carry kit.
- **The head band for the height fit excludes everything 5% of his height
  behind the torso centre.** Sound: a ruck's top rising behind the head can't
  set his height.
- **Leg slices are centred on the midpoint of their 15th and 85th
  percentiles**, from the leg's own vertices (within `LEG_REACH`). Moves are
  smoothed over ±4 slices, and the whole leg region moves together. Sound.
  - *Why:* the middle of the extent followed protrusions (knee pads,
    pockets), and filtering which vertices moved tore the shins. The median
    followed vertex density, not shape.
- **Back kit rides one bone whole** (`_strap_packs`). Sound, and a physical
  rule.
  - *Kit:* above the crotch, behind the back plane, or standing more than
    `PACK_STANDOFF` off the body behind the back or beside the head (a whip).
  - *Which bone:* kit joined to what hangs from the shoulders rides `spine_03`,
    down to a ruck's bottom. Separate kit lower down (an armour flap, a dump
    pouch) rides the pelvis.
  - *The seam:* two rings of body vertices round the kit take 50% and 25% of
    its bone (`KIT_SEAM`). A ruck pivoting off the buttocks stretches the skin
    over a few faces, not one; before this it tore into a spike.
  - *Before:* a ruck's top nodded with the head, its sides swung with the arms,
    a whip stood upright when he lay down, and a flap swung out with a thigh.
- **The OCP rifleman was regenerated** with the composed prompt, so his pack
  and radio whip are in the input. The pilot's body stays committed as an old
  take. Sound.
- **The scout's pale fringe under the ruck is not regenerated.** Sound: it
  doesn't show at battle distance in the rear and side renders, and a
  regeneration costs $0.64.
- **The EMR body's helmet has a sharp corner at the back of its cover.**
  Provisional. It is in the generated mesh, so no wearer fix would be general.
  Slice 07's contact sheet judges whether the EMR body is regenerated.

## Slice 03b (the aim hold)

- **The eye sits at the top of the sight, and the weapon errs low.** This is
  the user's rule: "make the eye look at the absolute top of the gun … I rather
  have it be too low than too high … the butt of the gun needs to be aligned
  with the shoulder".
  - *How:* the neck cranes up to `NECK_CRANE_DEG` (36°), and the butt slides at
    most `POCKET_SLIDE` (4 cm) on the shoulder.
  - *Result:* on the M4s the eye is 0.2–1 cm above the scope's top. On the
    AK-12 it is 3 cm above, because one hold serves every rifle and the AK has
    no optic. The user accepts a low weapon.
- **The eye socket moves to the pupil**, 1.5 cm below spike 03's brow point,
  measured on three generated heads. Sound.
  - *Effect:* the simulation's eye stands 1.5 cm lower; `fit.eye` still holds.
  - *Until slice 09:* procedural exports keep the old socket until they are
    re-exported.
- **`fit.cheek`'s floor is skeleton data** (`aim_reference.sight_over_bore_m`:
  0.05 on the rifle skeleton, absent on the launcher's). Sound: a launcher's
  sight sits beside its tube, so the eye can be level with or below the bore.
- **The support hand slides back to reach** (`within_reach`), and the aimed
  poses swing the support shoulder forward 20° (`lean_in`). Sound.
  - *Why:* with the butt in the pocket, the handguard was 67–72 cm from the
    left shoulder, and the arm is 49.5 cm long.
  - *Ruled out:* putting the clavicle in the IK chain. Blender's legacy solver
    ignored both stiffness and limits, and hunched the shoulders up beside
    the helmet.
- **The walk and run low-ready support hand is 1.7 cm short** of its target.
  Accepted (5.3 cm before): the walk animation swings the shoulder back, and
  the hand is already at the magazine well.
- **The procedural Kornet carried kit stands 1.752 m in stand_aim**, over the
  1.75 m limit. Provisional, like the `fit.lies` set: it predates this work,
  and slice 09 replaces the file.

## Slice 06 (the PLA and French armies)

- **The PLA army is `pla_type21`**, Type 21 (Xingkong) woodland. Sound: shippable
  photos of the uniform, helmet and carrier together exist (CC-BY-4.0 frames
  from the 2021 International Army Games, and CC0 flat lays of the uniform).
  - *Caveats, in the reference notes:* the field frames don't settle Type 19
    against Type 21. The helmet is a covered high-cut shell, not confirmed as a
    QGF-11. There is no front view.
  - *Why the prompt says "high-cut helmet with side rails and ear defenders",*
    not "QGF-11": that is what the photos show.
- **Licences:** CC-BY-SA references are allowed, as the existing library
  allows them (`references.ts` has no review-only tier; the whole library is
  review-only). The French Army's own Commons photos (Licence Ouverte) are
  refused by the checker, so the French references are public-domain US DoD
  photos from Saber Junction 23. Sound.
- **A new army's `cloth_mean` is its look `a` body's own measured cloth mean**
  (French `[0.0965, 0.0735, 0.0461]`, PLA `[0.088, 0.0799, 0.0593]`). Sound:
  there is no print recipe to match, so look `a` keeps its colours and looks `b`
  and `c` grade to it.
- **The new rifles (`hk416f`, `qbz191`) follow the M4/M27 construction**, with
  their optic's top at the shared hold's sight height. The eye-at-the-top-of-
  the-sight rule then holds for them with no new hold. Sound; new rifle models
  were out of scope.
- **Until slice 08, a generated-only army borrows its faction's procedural look
  table** for the kit's materials, and refuses to export without a pin.
  Superseded in slice 08, which removes the tables.
- **Catalog rows:** `rifle_squad_pla_active_*` and `rifle_squad_french_active_*`
  (army, not faction, since two armies now share each faction).
  - The FN-6 team's rows name the PLA riflemen, the Akeron team's name the
    French riflemen, and the Javelin guard's `europe` variant is the French
    rifleman.
  - The Akeron guard rows (German scouts) had no user left, so they are
    deleted, along with the six German Javelin exports no row names.
  - Sound.

## Slice 07 (every body generated and pinned)

- **A face-down corpse rolls 30° onto one side** (`CORPSE["roll_down"]`; the
  face-up corpse keeps 10°). Sound, and a physical rule: a dead man carrying a
  large rucksack doesn't lie flat on his face.
  - *Why:* rigid on the upper back, a scout's rucksack stood 0.60–0.65 m on a
    flat face-down corpse, against the 0.6 m `fit.lies` limit.
  - *What it fixes:* every rifle and recon kit now lies under the limit.
  - *Range:* 40° raised a knee or arm to 0.64 m; rolling the other way buried
    one side 0.4 m.
  - *What it doesn't fix:* slung launchers stand too high, and sink face up,
    whatever the roll. Slice 09 owns that.
- **Rejected candidates are regenerated, never patched.**
  - OCP recon `a` and EMR recon `c`: rucksacks too deep to lie flat. Each was
    redrawn from a new image after two meshes failed.
  - PLA `a`: came back untextured.
  - EMR recon `a`: the generator's arms drooped so far that its left hand hung
    below the forearm after seating.
  - Sound.
- **The PLA prompt was corrected after the unprimed critique.** It said "pale
  grey-green", from washed-out field frames; it now gives the flat lay's green
  with brown and reddish-brown patches and fine dark specks, and three more
  distinct faces. All three PLA bodies were regenerated, and `cloth_mean` was
  re-measured from the new look `a`. Sound: national identity comes first.
- **Critique notes accepted, not acted on.** Provisional, re-judged at slice 09's
  in-game review.
  - *Generator limits:* blurry sleeves, paddle gloves and dark renders under the
    offline light.
  - *By design:* the same look scheme across armies (a night-vision mount on `a`,
    the darker-skinned soldier on `b`).
- **HTTP 409 refusals at submit were transient and charged nothing.** The tool
  now prints the service's reason. Two jobs a disk-full failure left unresolved
  were resumed, not resubmitted.
- **Spend: $25.44** over 30 bodies and their retakes, against the spec's $16–22
  estimate. The overrun is the retakes above.
- **Probe exports are committed as they stand** (the current code's output);
  slice 09 re-exports everything once more on the final code. Sound.
