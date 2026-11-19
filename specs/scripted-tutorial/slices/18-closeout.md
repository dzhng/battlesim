# 18 Closeout

**Unlocks:** the feature is done and documented.

- **The full repository closeout run, once** (`development.md` check ladder step 3).
  First pull the LFS assets the full run needs (runtime and third-party
  audio, roster and infantry sources, scene baselines, city). No balance report
  is needed: no rule changed beyond the offered-shop refusal, playbooks and the
  director, all proved by digests and tests.
- **[review](../../../.agents/skills/review/SKILL.md)**: shape, diff and docs.
- **Docs:**
  - `fixtures/README.md`: scenario maps and documents, coach and solution files,
    and the content-owner paragraph;
  - `development.md`: content owners go from three to four; playbooks join the
    opponent bullet in the game rules list;
  - `crates/sim/README.md`: the director and resume;
  - `crates/contract/README.md`: the scenario document;
  - `apps/battle-lab/README.md`: the Guide and the tutorial route;
  - `web/README.md`: the coach and UI signals;
  - `packages/battle-audio/README.md`: narration and the `bento` transport.
- **Delete** `throwaway/tutorial-drafts/` and `throwaway/tutorial-assets/`, and
  check the spec's review takes all reached the pipeline (or were replaced).
- **[close-spec](../../../.agents/skills/close-spec/SKILL.md):** archive to
  `specs/done/scripted-tutorial/` as a durable rationale.
