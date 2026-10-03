# Infantry weapon use

A soldier works on one weapon at a time. Switching has no equip delay, but the selected gun still needs its ordinary aim and cycle. Vehicle mounts remain independent. An AT operator using his rifle visibly carries his launcher on his back.

The simulation owns selection; presentation observes it. Carrier assignment and active use are different facts: recovering a launcher does not permanently prevent rifle fire. Preserve existing casualty handoffs and stocked-spare assignments. Do not add a Stinger unit, target-based spare reassignment, manual weapon controls, equip timers, modular attachments, or balance tuning.

## Contracts

- Choose before any weapon advances or fires, including the first launch tick. Guidance has priority; otherwise use the assigned single weapon when it has a legal effective target, even while aiming/reloading. Fall back to a carried default gun when the single weapon cannot engage. If neither can engage, reload the assigned weapon when it needs work. Stable mount order breaks ties. Readiness must not starve a paused reload.
- Keep ownership and magazines across switches. Inactive physical cycles pause reload, clear aim and burst-start state, and allow passive cooldown to elapse. Aim belongs to the physical gun (`Cycle`), not the shared squad targeting lock. Other riflemen retain their own progress. Existing stationary-movement interruption still applies.
- Publish each visible member's active mount with their stable identity. Keep the mount's `operator` as carrier assignment. Enemy member filtering applies to active use as it does to identity; no hidden live-state reads. Include authoritative selection and aim in the digest.
- A mount's optional `operator_appearance` becomes `{ active: [...], carried: [...] }`. This is a hard cutover; no compatibility parser. The active set is launcher-held; the carried set is rifle-held/launcher-on-back. Both use the same variant identity and their own clip families. Ordinary soldiers keep their existing appearance. A dead carrier never duplicates the recovered launcher.
- Model, animation, facing, and muzzle attachment resolve the same active/carried state. An equipment-family change resets incompatible pose blending. No renderer inference from reload timers or squad-wide historical shots.

## Slice graph and review surface

1. [Simulation](slices/01-simulation.md): exclusivity and independent physical aim.
2. [Observation and posing](slices/02-observation.md): depends on 1; publish selection and consume it.
3. [Carried launcher art](slices/03-carried-art.md): generation can run alongside 1; integration depends on 2.

Independent planning drafts examined smallest shipping ladder, ordering/deadlock risks, and asset/seam ownership. Their common conclusion was to retain physical weapon identity and add authoritative activity, rather than deleting rifle cycles or changing carrier assignment to impersonate switching.

Visual checks compare matched native-size captures and feature crops against the pre-change battle and the requested equipment relationship. Run compare-screenshots, then an unprimed screenshot-critique as the last visual acceptance check, and show the result with preview-shots. Scratch captures and logs belong in `throwaway/infantry-weapon-use/` per AGENTS.md. Existing art, lighting, and terrain readability remain outside this equipment change.

## Next Agent Prompt

Finish closeout. Core weapon/publication/pose/art work is reviewed and focused gates pass. Keep production inputs frozen until the active full browser verification finishes. Full web tests pass except the generated-town test timing out under machine load; rerun that alone after competing work ends, without changing its limit. Native full tests exposed the cover test's all-rifle assumption and two unchanged movement contracts; diagnose before editing. The 50-trial balance result is saved, but flank seed 55 refuses one script order and must be explained/fixed before acceptance.

Apply the proven lab-staging patch and the ground material oracle repair after browser verification exits, then rerun only failed scenes. Native generated-map parity was refreshed because identity pins the catalog's appearance schema; Wasm publication/map parity already passes. Evidence, red probes, proposed patches and paired balance outputs live in ignored `throwaway/infantry-weapon-use/`. Do not retune outcomes or weaken checks. Finish integrated commits and archive only after every gate is resolved. Gameplay uses ordinary deaths; the unused carried-model death endpoint remains excluded.

- [x] Close mixed-target garrison regression and simulation review.
- [x] Publication schema, native privacy and packed identity codec tests.
- [x] Fresh Wasm/native publication and generated-map parity after the final native change.
- [x] Held/carried model, clip, facing and mount-aware muzzle selection.
- [x] Generated carried variants and bounded bake; unrelated runtime identities preserved.
- [x] Matched visual comparison, fresh critique and Preview checkpoint.
- [ ] Full closeout checks, paired balance evidence, integrated review/choices consolidation and archive.
