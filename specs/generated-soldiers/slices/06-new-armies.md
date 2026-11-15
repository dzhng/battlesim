# 06 The PLA and French armies, and the teams that move to them

About $1.30 of generation: look `a` for each new army. Looks `b` and `c` come
in slice 07.

## Contract

Two new armies exist as data with real references. Every soldier of the teams
that move to them resolves to the new army, crews included, and no team
fields a mix of armies.

## Seam

1. **References first.** Collect `assets/references/` photos for each kit that
   names a new army, with licences, as the `make-3d-models` workflow requires.
   `references.ts:291` refuses an army look whose views are all generated.
   - Sources: public-domain US DoD and DVIDS photos of joint exercises, and
     CC-BY Wikimedia Commons. Share-alike photos stay review-only.
   - PLA: Type 21 ("Xingkong") woodland with the QGF-11 helmet. If no
     shippable Type 21 photos exist, use Type 07 woodland digital with the
     QGF-03 helmet, and record why.
   - French: CCE woodland, with the helmet and carrier the dated photos show.
   - `asset check` (references) must pass.
2. **Army data.** Add `pla_type21` (or `pla_type07`) and `french_cce` to
   `armies.json`, each with:
   - `faction`: `eastern` or `europe`;
   - `rifle`: `qbz191` or `hk416f`;
   - `gloves`, `cloth_mean` (measured from the generated cloth of look `a`,
     since there is no recipe), `prompt`, and three look clauses;
   - `crew_kit`: the French army's crew is its rifleman;
   - no `lmg` unless a kit needs one.
3. **Rifles.** Add `weapons.RIFLES["qbz191"]` and `["hk416f"]`, built on the
   M4A1's construction with proportions from the references. Same grip,
   support and sight anchors, so the shared rifle hold fits. This is a named
   decision: new rifle models are out of scope.
4. **Who moves:**
   - `eastern_air_defense_team_fn_6.armies = ("pla_…",)`
   - `europe_atgm_team_akeron_mp.armies = ("french_cce",)`
   - `fgm_148_javelin_team.armies = ("us_army_ocp", "french_cce")`
   - `rifle_squad` gains both armies, as the teams' riflemen and guards.
5. **Team riflemen are named looks, not faction looks.** The runtime picks a
   faction look by faction only (`appearanceCatalog.ts:116-118`), so a team's
   guard rows in `fixtures/units/roster/infantry-appearances.json` name the
   new army's `rifle_squad` appearances explicitly:
   - FN-6: today `eastern_atgm_team_kornet_guard_*`, which are EMR scouts;
   - Akeron: today `europe_atgm_team_akeron_mp_guard_*`, which are Flecktarn
     scouts;
   - the Javelin's Europe guards: `fgm_148_javelin_team_guard_*`.

   Add the catalog rows (`assets/catalog.json`) for each new appearance, then
   rebuild the resolved `fixtures/catalog.json` with its owning tool.
6. **Crews.** `vbci.py` and `vbl.py` get `crew_army="french_cce"`.

## Steps

1. Do the references, then the data and the rifles.
2. Generate look `a` for each new army, using slice 05's image-then-mesh steps.
3. Export the FN-6 kit (active and carried), the Akeron (active and carried),
   the Europe Javelin (active and carried), the new `rifle_squad` looks, and
   the VBCI and VBL with their French crew.

## What you can see

- `/labs/showcase?unit=eastern_air_defense_team_fn_6` and
  `?unit=europe_atgm_team_akeron_mp`, each a whole team in one army.
- `?unit=fgm_148_javelin_team&faction=europe`.
- The VBL and VBCI in the yard (`&yard=1`) with French crews.

## Verification

- `asset validate` has no findings, every export repeats byte for byte, and
  `asset check` passes (references included).
- The sim catalog test passes: every appearance a unit names exists.
- One narrow digest test shows outcomes unchanged; appearance ids aren't
  battle rules.
- **compare-screenshots:** each new army's body against its references.
  Judge national identity first (camouflage, helmet, carrier), then fit.
- **screenshot-critique** as the last check: the team showcases, the sheets,
  crops of helmets and camouflage, and the crews.
- **Non-blocking checkpoint:** open the shots with preview-shots, wait about
  5 minutes, decide, record and close.

## Decisions delegated

The exact helmet and carrier the references show, rifle proportions, prompt
wording, and catalog row naming (following the existing pattern).

## Must stay green

Existing armies' sources, variant persistence, the shared holds, and the sim
catalog and digest tests.

## Feedback that would change it

"Use BME", "use Type 07" or "the wrong era" changes the references and
prompts, and regenerates 3 bodies per army in slice 07.
