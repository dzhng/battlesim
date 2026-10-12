# 16 — Closeout

1. **Run everything once:** `bun run check` and `bun run verify` (AGENTS.md: once, when the spec's implementation is finished). Fix what only the full run finds.
2. **Run [`review`](../../../.agents/skills/review/SKILL.md) over the feature's whole diff.** Confirm the single-owner list in the README holds:
   - no `cue_voices`;
   - no music WAV decode path;
   - no voice clip in `sounds.json`;
   - no shout state on `Unit`;
   - no AI reads `shouts`.
3. **Docs:**
   - `packages/battle-audio/README.md`: voices, music, the generation tool, and the provenance kinds;
   - `apps/sound-workbench/README.md`: the Voices and Music tabs and the review flow;
   - `assets/README.md`: the layout `source/audio/{voices,music}` → `runtime/audio/{voices,music}`;
   - `fixtures/README.md`: `voices.json` and `voices.lock.json`, and the `music` section;
   - `web/README.md`: the Vercel LFS sharp edge, if slice 07 didn't already add it.
4. **Hand over the spec:** [`close-spec`](../../../.agents/skills/close-spec/SKILL.md) moves it to `specs/done/` and rewrites it as rationale.
