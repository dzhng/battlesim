# 18 · Closeout

Depends on every slice.

1. **Pull the large files with `git lfs pull`:**
   - the runtime;
   - the third-party audio;
   - the roster and infantry sources;
   - the scene baselines;
   - the city (S9).
2. **Run the full repository check ladder once.** The root task runner owns the
   commands.
3. **Run the full balance report, with depots in the maps (Q3).** Compare it
   with the last recorded report, and name any shift.
4. **Run the final visual pass over every gate's latest shot:**
   - `compare-screenshots` against its reference or prior look;
   - `screenshot-critique` last, unprimed;
   - `preview-shots` for the user, at a non-blocking checkpoint.

   Done means "looks like a shipped game's armory" (U6). A visible defect
   becomes a new focused slice; the bar is never lowered.
5. **Search the code for each "Never" entry** in the README's single-owner
   table. Each hit is a bug.
6. **Run `/review`:** shape, diff, then docs. Then push to `staging`, and verify
   with `git merge-base --is-ancestor`.
7. **Run `close-spec`.**
