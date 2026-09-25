# 14 — Concurrent readiness and ammunition UI

**Status:** complete 2026-09-25. **Dependencies:** 06, 08, 10, 12, 13. **Milestone:** Village checkpoint.

## Contract and question

Can the player understand simultaneous aim/reload/deployment without inspecting debug state?

User requirements owned or exercised: U02, U03. Read their canonical entries in [requirements](../requirements.md), then the applicable [implementation contracts](../contracts.md). Do not infer rules from a fixture label.

## API seam and ownership

`packages/battle-renderer` world-anchor projections and `web` React selection panel consume WeaponReadiness/ActionReason only.

## Runnable artifact

/lab/readouts: fixed selected multiweapon unit with simultaneous timers, finite/∞ ammo, guidance and deploying supply, plus near/far zoom states. The command bar exposes all village commands and policy.



## Verification and verdict

Every displayed timer equals published state; concentric rings advance independently; no completed timer remains; cannon variants not separate guns; readable ammo counts; guidance icon; no color-only action distinction; blocked-fire explanations; no enemy hidden readiness leak. Compare same camera/tick before and after UI change.

Run relevant native Rust tests and browser/TypeScript seam tests; run typecheck/lint and the registered scene without console/GPU errors. Keep prior accepted slice contracts green. Source test names in [research](../research.md) are reuse references, not evidence that these new tests already exist. Record measured results and any scope deviation in this slice and the README handoff.

Visual variable: **Weapon-state legibility**. Review crop/mask: **Selected unit ring cluster at 2×/4× plus full 1280×800 and 900×600 frames**. Explicitly out of scope: New terrain/art direction and polishing all HUD chrome.

1. Freeze fixture seed, tick, camera, viewport and DPR. The registered scene regenerates the full frame and tight 2×–4× crops with machine/config metadata in gitignored `throwaway/evidence/<fixture-id>/`; the verdict records metrics and dispositions.
2. Use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) for candidate versus any previous/reference shot. State the target for this variable, report telemetry and a less-wrong verdict; do not reward mere baseline matching. With no predecessor, use the skill's single-image diagnostics and inspect their meaning.
3. **As the last visual acceptance check, run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed agent given only the actual candidate full frames, crops and neutral inspection task.** Inspect and record its actionable findings before accepting. Do not substitute implementer self-review.
4. Offer the relevant shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). This is a **non-blocking** course-correction window of about five minutes. Continue independent work while it is open, using waits no longer than 60 seconds. If no answer, decide reversible matters on evidence, record the choice, close the opened Preview set and proceed. Silence does not authorize changing user rules.

A visual pass is not a substitute for behavioral tests. A failing contract or serious visual defect means fix or reslice, not quietly update a golden image.

## Decision budget

Delegated: Exact colors/icons/spacing and zoom presentation within required information. Also permitted: internal naming and reversible diagnostic formatting. Every other consequential freedom is a spec gap: record and resolve it before expanding implementation. No compatibility shims, duplicate owners, historical renderer adapters or undocumented gameplay changes.

## Human feedback that changes the slice

Confusing overlapping circles or unreadable numbers require layout iteration, not deleting per-weapon state.

## Carried in from slice 04

Route ownership is ambiguous when two selected units' routes start close together, because the map carries no unit or destination labels. When world-anchored readouts land here, give destination rings and selected units a matching on-map identifier.

## Verdict — 2026-09-25

**What was built:**
- `web/src/battle/present/readouts.tsx` owns the player readouts, and reads only `ObservationView`:
  - `ReadoutLayer`: DOM ring clusters, anchored each frame through `LabViewport.onFrame`;
  - `SelectionPanel`: reason text and glyph, ammo per kind, live timers, guidance and deployment;
  - `CommandBar`.
- `useUnitControl` gained right-click attack on identified enemies, armed attack-move (A) and attack-ground (G), the fire-policy toggle (E), and Escape. `sideInstances` now reports the enemy handle per instance.

**Browser scene** (`bun run --cwd web scene -- readouts`, 16 checks):
- right-clicking an identified enemy attacks it;
- every ring's reason and timers equal the published mount state, sampled 12 times;
- completed timers disappear;
- a guiding launcher shows the icon;
- an aim ring is captured;
- the cannon's AP and HE are one ring, and unlimited reads ∞;
- the truck's separate deployment readout, shown only while it changes;
- no enemy readiness;
- zoomed out, the selection keeps its rings and the panel keeps its details;
- E switches the policy;
- A arms attack-move;
- Escape disarms;
- keys do nothing while typing;
- G then a ground right-click attacks the ground, and the mode resets;
- S stops.

**Web tests:** `web/tests/readouts.test.ts` covers:
- ring timers are the published fractions and vanish when complete;
- the cannon label follows the loaded kind, else the reloading one;
- every contract `ActionReason` has player words.

A fresh code review found problems, all fixed:
- Nothing published which kind was reloading; `MountReadiness.reloading` is new.
- Panel seconds assumed the nominal rate, which suppression slows; the panel now shows the published percentages.
- Armed modes stuck after enemy clicks, resets or an empty selection.
- The typing guard missed selects and editable content; A and G armed with nothing selected.
- Attack-move tokens could collide with move tokens; `MoveGestures::token` now issues both.
- The command bar lacked fast move and garrison; both are now modes.
- Rings lagged meshes; they now anchor to the interpolated poses.
- A layout read happened per unit per frame; there is now one projector per frame.
- Reason words were duplicated in the labs; `REASON_TEXT` is the one owner.
- `buildingUnderRay` moved to the shared lab world helper.

Every other scene stays green, and `bun run check` is green.

**Visual gate:**
- **Before/after at the same camera and tick** (readouts hidden vs shown): parity distance 0.033; edge density 0.079 → 0.089. The change is real, and small in area.
- **An unprimed critique, acted on:**
  - Clusters covered their units; there is now a fixed 22 px screen gap and an opaque box.
  - Reasons did not show on the map; there is now a lower badge.
  - The panel hid guiding behind reloading; it now shows both.
  - No aim ring had been captured; one is now.
  - The guidance icon is larger, glyphs are unique, and there is a minimum visible arc.
  - Rings now carry weapon captions, and the legend explains ▲▼✓.
  - Panel rows use a hanging indent.
- **Kept, with reason:**
  - HE's count shows only in the panel while AP is loaded.
  - At 900×600 the lab panel covers part of the scene; the lab panel is a lab convention.
  - A right-click on an approximate contact does not attack: contacts are areas, not drawn instances. Attacking a contact goes through the weapons lab, as in slice 08.
  - The A, G, E and Escape keys now work in every lab, because `useUnitControl` is shared.
  - The contract's WASD panning is not adopted: panning stays on the arrow keys, per slice 04's choice, so A and S stay free for commands.
- **Preview-shots:** not offered; the run is unattended.
