# 16 Closeout

**Contract:** the system as a whole matches the map, and the plan becomes a
record.

**Do:**
- Pull the LFS files a full run needs: runtime and third-party audio, roster
  and infantry sources, scene baselines and the city.
- Delete 01's probe.
- Audit for leftovers. None of these may remain:
  - `game.json` anywhere;
  - weapon-keyed presentation tables;
  - `extends` walks outside the catalog;
  - `"default"` look fallbacks;
  - duplicated shader constants;
  - forest-floor code.
- Run everything once: `bun run check`, `bun run verify`, `bun run visual`
  and the full balance report.
- Run `/review` (shape, diff, docs). Update `fixtures/README.md` and
  `development.md` where they describe the old file.
- Archive with [`close-spec`](../../../.agents/skills/close-spec/SKILL.md).

**Moves:** nothing new. A failure here is fixed in its owning slice's area,
with a named commit.

**Depends on:** every other slice.
