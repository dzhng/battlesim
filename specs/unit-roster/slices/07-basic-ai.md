# 07 — First playable human versus basic AI checkpoint

Status: not started. Dependencies: 03,04,05,06. Follow the README handoff and repository
skills before editing. This slice is a committed checkpoint, not a stopping point.

## Contract and scope

Add only a basic observation-bound skirmish controller beside/reusing existing defensive decisions. Inputs: admitted faction catalog, own side observation, public map/objective facts. Outputs: the same accepted command envelopes as the human. Build a legal opening template from starter-balance examples, skipping disabled rows with simple role fallback. Ready during prep; every five seconds buy an affordable legal reinforcement from rifle/recon/anti-armor/armed-vehicle roles and replace a lost supply truck.

Send combat groups toward nearest non-owned objectives by known route and stable site ID; distribute new groups among sites, hold defensively when all are owned. Deploy the truck at a simple support position behind own troops. Reuse existing movement/fire/retreat behavior; no elaborate planning, hidden knowledge, learning, cheats or new combat rules. Record accepted commands and skip policy during replay. Advanced AI is a separate spec.

This is the first full human-playable loop: faction choice, zero-unit prep, real purchases/entry, credits/cap, contesting AI and score/flag victory. Temporary art during the checkpoint is explicitly unfinished; remaining models, bounty/refund/Trophy integration are still required.

## API seam and ownership

Own/public observation + catalog → ordinary command intents. Keep policy outside simulation outcome rules and avoid a second AI-only spawn/economy API.

## Narrow verification

Deterministic affordable opening/disabled fallback, cap handling, reinforcement, truck replacement and objective-order tests. Change hidden enemy poses/ammo while keeping observation fixed: policy outputs must match. Replay accepted AI commands without policy and compare digest. One focused human/basic-AI browser battle; only a quick affected combat sample.


For every visual shot produced in this slice: load game-ui and renderer as
applicable; compare candidate against its declared crop/reference and pre-change
baseline with compare-screenshots. Show the real artifact with preview-shots.
Run an **unprimed screenshot-critique as the last visual check before acceptance**.
The human checkpoint is non-blocking: leave a short (~5 minute) opportunity to
correct reversible choices while continuing independent work, decide from evidence
if silent, record rationale and close opened Preview shots. Never infer permission
for a new mechanic from silence. GPU jobs serialize under the existing harness.


## Review surface and verdict

Production generated skirmish with a small supported ground subset and full visible roster. Verdict: a player can actually buy, fight, capture and finish against the basic AI. Do not market this as sophisticated or balanced AI.

## Delegated freedoms and invariants

Internal type/function/file names and clean decomposition within the named owner
are delegated. Starter numerical tuning is delegated, with rationale and narrow
proof recorded in choices.md; physical/model facts require references. Cosmetic
spacing/icon fit may be adjusted reversibly under game-ui. Do not add mechanics,
change selected economy/victory/visibility rules or expand excluded capabilities.
If this slice exposes an unlisted material choice, record it in the choices ledger
and reslice the focused uncertainty rather than broadening implementation silently.

Keep existing developer fixtures, command sequencing, native/WASM agreement,
side-only knowledge and asset admission green. Verify changed contracts test-first;
non-behavior refactors preserve digest/replay outcomes. Review locally, update the
README pickup and commit the coherent pass, then proceed to the next independent
ready slice. No adversarial review or legacy compatibility scaffolding.

## Handoff evidence

- [ ] Red behavior/admission case observed where applicable.
- [ ] Narrow green proof and any allowed digest change recorded.
- [ ] Artifact/visual gates resolved where applicable.
- [ ] Choices, status and next pickup updated; focused pass committed.
