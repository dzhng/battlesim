# Slice 04 — Contextual Attack Ground UI and artillery lab

## Contract unlocked

When the selection contains an artillery weapon, the existing Attack Ground command receives a tactical emphasis. The command still serializes the existing `Order::Attack` + `TargetRef::Ground`; ordinary right-click, Attack Move, range reads, hold fire and mixed selections retain their current semantics.

The browser has a focused artillery lab/scene where a reviewer can see indirect fire over a ridge, minimum-range holding, long-range spread and ordinary target attacks using real observed data.

## API and visual ownership

- `web/src/battle/input/commandReach.ts` remains command eligibility owner.
- `web/src/battle/input/pointerIntent.ts` and `useUnitControl.ts` remain pointer/keyboard command owners.
- `web/src/battle/present/readouts.tsx` owns the command bar and reads the session catalog's resolved weapon metadata.
- `web/src/hud.css` owns the tactical emphasis style.
- The range ruler/readout remains the single nominal range visualization; do not add a second artillery ring or panel.
- Register the focused route/fixture in `apps/battle-lab/src/fixtures.json` and pair it with a scene in `web/scenes/`.

## Visual contract

The one variable judged here is **contextual command emphasis**: Attack Ground should become more legible when artillery is selected while remaining the same tactical control. The crop/mask is the command bar and the selected-unit context; terrain, projectile art, fog, model silhouette and range-ruler geometry are out of scope except where needed to make the command state understandable.

Use the existing icon, shortcut and button shape. Choose a thin border/inset glow or equivalent existing focus treatment; do not add a permanent label, dashboard panel, emoji, new hue role or per-unit ground ring. Mixed selections use the existing armed-unit reach semantics.

## Tests and scene

- Add a browser unit test proving artillery selection adds the emphasis and non-artillery selection does not.
- Prove the existing G/left-click path still emits the same ground order and that right-click Attack/Attack Move serialization is unchanged.
- Add a fixed test-map scene/fixture with an artillery unit and target positions that show:
  - high arc over a ridge;
  - real obstacle blocking;
  - minimum-range hold;
  - long-range scatter;
  - normal identified-target attack and Attack Ground;
  - hold-fire state.

Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) against the approved pre-change command-bar shot, using the command-bar crop only. Run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) as the final visual check on the candidate. Open the resulting shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) for a non-blocking human review window of about five minutes; if there is no response, record the evidence-based decision and close the shots before proceeding.

## Verification

```bash
bun run --cwd web test -- tests/unitControl.test.tsx tests/commandDeck.test.tsx
bun run --cwd web scene -- --list
bun run --cwd web scene -- artillery
```

Use the repository's GPU lock for scene runs and put captures/diffs in `throwaway/evidence/`.

## Firewalls and delegated choices

Do not create an artillery-only button, panel, command, cursor glyph, range geometry or browser-side simulation. Internal CSS token choice and fixture coordinates are delegated within the visual contract. If the scene exposes a gameplay defect, send it back to slice 01/02 rather than hiding it in the renderer.

## Acceptance and feedback

The slice is accepted when the command bar communicates the existing Attack Ground action without changing command serialization, the focused scene proves the player-facing moments, and the visual gates pass or their approved rationale is recorded.

## Pass evidence

The command-bar unit suite passes with an indirect-weapon selection receiving a
thin inset accent border and a direct-weapon selection retaining the existing
button. The `artillery` lab route and scene are registered and reuse the
production flight bench; browser scene execution is blocked in this checkout by
pre-existing Git LFS runtime bundle pointers.

