# Working in this repo

Read [`README.md`](README.md) first: what the game is, how the repo fits together, and how to run it. Active plans live in `specs/<feature>/README.md`. Their "Next Agent Prompt" section says what to do next. If any folder you're working in contains a `README.md`, read it before continuing — the readmes are written for you.

Never modify `../game` (`~/dev/game`). It is a separate project we copy from, by reuse manifest, and never import at runtime.

## Communicating with the user

The user is very technical but doesn't read the code day-to-day. Responding in code or pointing at files is fine — just don't assume they already know what a given variable, function, or module does; introduce it briefly on first mention.

API seams and schemas are the most important things to surface. When work touches an interface between components, lead with what that contract looks like and how it changed. Examples are the simulation's commands, observation and publication layout, `Battle::digest`, fixture JSON, and module boundaries.

## Worktrees: keep them cheap

Every worktree is a full checkout. Large binaries, `node_modules` and Rust build output multiply per worktree and have run this machine out of memory and disk before. Every worktree, whether you make it or an agent harness makes it, follows this recipe.

1. **Large binaries are in Git LFS** (`*.glb`, textures, reference images under `assets/`). The repo is configured to skip downloading them on checkout, so a new worktree gets small pointer files. Pull only what your task needs:

   ```bash
   git lfs pull --include="specs/battle-look/assets/reference/warno/**"
   ```

   Never run a bare `git lfs pull` in a worktree.
2. **Share `node_modules`** from the main checkout. Don't install again:

   ```bash
   ln -s /Users/david/dev/battlegame/web/node_modules web/node_modules
   ```
3. **Share one Rust build directory.** Set it before any `cargo` or `bun run build:wasm`. Concurrent builds then wait on cargo's lock instead of each building its own multi-GB `target/`:

   ```bash
   export CARGO_TARGET_DIR=/Users/david/dev/battlegame/target
   ```
4. Build the WebAssembly once (`bun run build:wasm`) before web tests or scenes.
5. Remove the worktree when its branch is merged.

One-time setup, done by the battle-look spec's first slice and kept here for fresh machines:

```bash
brew install git-lfs
git lfs install --local --skip-smudge   # checkouts get pointers; pull on demand
git lfs pull                            # in the main checkout only
```

## Testing changes

Before implementation work on behavior changes or bug fixes, invoke [`write-tests`](.agents/skills/write-tests/SKILL.md) and follow its red/green workflow.

### Run the narrowest runner that answers your question

`bun run check` is a **closeout gate, not a feedback loop**. It covers format, clippy with oxlint, tsc, every cargo test and vitest. `bun run verify` builds the WebAssembly and runs every browser scene. Both are slow and saturate the machine. While iterating, work from the top of this ladder and stop at the first rung that covers your change:

```bash
cargo test -p sim --test village a_replay_matches     # one test
cargo test -p sim --test village                       # one file
cargo test -p sim                                      # one crate
bun run --cwd web test -- tests/observation.test.ts    # one web test file
bun run --cwd web scene -- village                     # one browser scene
bun run --cwd web scene -- --list                      # scene ids
```

Run `bun run check` and `bun run verify` once, at the end of an implementation pass, before a merge.

Simulation performance changes must leave battle digests unchanged, or be named decisions. `cargo run -p sim --release --example endurance_report` and `village_report` are the measurement tools; run one at a time on a quiet machine.

## Visual changes

Browser scenes write evidence into gitignored `throwaway/evidence/<fixture-id>/`.

For any visual change:
- use [`screenshot-critique`](.agents/skills/screenshot-critique/SKILL.md) for an unprimed second opinion;
- use [`compare-screenshots`](.agents/skills/compare-screenshots/SKILL.md) to judge before/after shots and shots against references;
- use [`preview-shots`](.agents/skills/preview-shots/SKILL.md) to show shots to the user.

The renderer skill is [`renderer`](.agents/skills/renderer/SKILL.md).
