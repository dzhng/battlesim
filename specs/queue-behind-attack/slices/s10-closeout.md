# S10 — docs, review, closeout run, push part 1

- Docs: rewrite `specs/done/move-validity/README.md` (the refusal line) and
  `choices.md` (the indefinite-attack paragraph) to the new promise; a reversal note in
  `specs/done/transport/choices.md` (step records were once rejected); one sentence in
  `crates/sim/README.md` on volley-unless-last and on queued orders taking effect at
  their turn.
- `/review` over part 1's full diff; apply the fixes.
- No full closeout run (the user's call, 2026-11-16): run the tests of what part 1
  changed — the sim modules it touched (`order_queue`, `order_markers`, `formation`,
  `publication*`, the jet run tests), the web vitest files it touched, the presentation
  tests reading the layout, and the re-recorded parity captures.
- Merge `origin/staging`, confirm the net diff is only this work, push `staging`, and
  confirm `git merge-base --is-ancestor HEAD origin/staging`.
