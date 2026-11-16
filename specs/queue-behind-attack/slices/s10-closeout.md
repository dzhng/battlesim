# S10 — docs, review, closeout run, push part 1

- Docs: rewrite `specs/done/move-validity/README.md` (the refusal line) and
  `choices.md` (the indefinite-attack paragraph) to the new promise; a reversal note in
  `specs/done/transport/choices.md` (step records were once rejected); one sentence in
  `crates/sim/README.md` on volley-unless-last and on queued orders taking effect at
  their turn.
- `/review` over part 1's full diff; apply the fixes.
- The full closeout run once (pull the LFS sources it needs), and the balance report once.
- Merge `origin/staging`, confirm the net diff is only this work, push `staging`, and
  confirm `git merge-base --is-ancestor HEAD origin/staging`.
