# 06 — Menu reel through the new path, approvals, docs

## Contract unlocked

Done means:
- the menu reel is re-recorded through the new path and approved;
- the docs describe the two concepts;
- the full run passes.

## Steps

1. **Record the menu.** Run `bun run --cwd web capture fixtures/captures/menu.json`. Watch both scene MP4s and the joined one end to end. If the sim changed after slice 01, re-record `menu_reel` (`BLESS_MENU_REEL=1`), but only after watching.
2. **Re-approve the `frame` shots.** The grade change moved `web/scenes/baselines/frame/*`. Compare old against new with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md). Slice variable: colour only; the grade is the [cooled one](../assets/grade/), with brightness kept. Bless only once there's no regression beyond the grade.
3. **Last visual check.** Run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on frames from the menu MP4 and the new `frame` shots.
4. **Human checkpoint, non-blocking.**
   - Open the shots and a few MP4 frames with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md).
   - Wait about 5 minutes.
   - If there's no reply, decide on the evidence, record it in the README, close the shots and continue.
5. **Docs** ([write-docs](../../../.agents/skills/write-docs/SKILL.md)):
   - a capture README that covers the concepts (encounter against capture), the format, a worked example and the report;
   - the categories in `fixtures/README`;
   - root README links.
6. **Cleanup.** Run [review](../../../.agents/skills/review/SKILL.md), then the full verify once.
7. **Close.** Run [close-spec](../../../.agents/skills/close-spec/SKILL.md).

## Stays green

Everything.
