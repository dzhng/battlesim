# 16 Closeout

**Contract:** the system as a whole matches the map, and the plan becomes a
record.

**Do:**
- Pull the LFS files a full run needs: runtime and third-party audio, roster
  and infantry sources, scene baselines and the city.
- Delete 01's probe.
- Audit for leftovers. None of these may remain:
  - the retired combined fixture filename anywhere;
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

## Done: implementation handoff

The transitional proof executables are removed. The six-category audit is
complete: the catalog owns inheritance and editor ancestry, weapon looks are
admitted without a default, shared CPU/GPU luminance has one owner, and the dead
forest body tables are absent. Required LFS media is materialized. Fixtures and
development docs describe the independent rules, presentation and mix contracts.

The required full runs completed once. `check` passed (2,107 Rust test executions
and 1,314 web tests); `visual` passed all 452 assertions without changing a
baseline. The original `verify` run recorded 1,037 passing assertions and 17
failures. Those failures were repaired in their owners and checked through the
affected scenes and tours; this is not a claim of a second, all-green full
`verify` run. The initial balance report had six failed requests; its final full
rerun completed all 120. Later source changes have focused native, browser,
lifetime and rebuilt-Wasm parity proof, rather than another full `check` run.

The final proof retains all 63,653 baseline digest lines. Sound, effects and poses
remain byte-identical to the merged slice 15 proof. Generator repairs deliberately
advance its layout identity and paired map records; they do not change that battle
proof. No approved visual picture was re-blessed.

Full logs, failure-to-fix commits, audit searches, visual inspection, final proof
and the commit/diff inventory are in
`/Users/server/dev/battlesim/throwaway/split-game-json/evidence-16/closeout-report.md`.
The implementation-child restriction prevented an independent screenshot agent;
the documented adversarial self-review fallback was used. The integrator still
owns `/review`, the choices ledger, independent acceptance and `close-spec`.
Slice 16 remains unticked in the README; no archive or push was performed.

## Whole-spec review follow-ups (closed)

- Presentation admission is Rust-only for contacts, buildings and the camera:
  air-mark dimensions must be positive and zoom bounds distinct. The other
  browser constraints already existed in Rust, including integral building
  residency; refusal coverage now lives at document admission. Browser readers
  trust admitted values and retain their geometry and control behavior tests.
- Look-sheet review carriers take their magazine from a resolved weapon naming
  the sampled look, so recorded bursts meet the same fielding contract as a
  battle while the controlled HE trajectory and authored looks stay intact.
- Native ground, fog sensors and building remains consume the battle's resolved rules. A saved-map regression failed with the base sensor height (1 instead of the encounter's 3.25), then passed with matching sensors and forest/prop geometry. The category-based rules helper remains for scenario construction and its existing test callers.
