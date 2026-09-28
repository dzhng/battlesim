# Working in this repo

Read [`README.md`](README.md) first: what the game is, how the repo fits together, and how to run it. Active plans live in `specs/<feature>/README.md`. Their "Next Agent Prompt" section says what to do next. If any folder you're working in contains a `README.md`, read it before continuing — the readmes are written for you.

Never modify `../game` (`~/dev/game`). It is a separate project we copy from, recorded file by file in [`reuse-manifest.json`](reuse-manifest.json), and never import at runtime. The battle-foundation spec's own `specs/battle-foundation/assets/reuse-manifest.json` records the earlier foundation ports; new copies go in the root one.

**Provenance.** [`reuse-manifest.json`](reuse-manifest.json) records where every reused byte came from: a file copied or adapted from `../game` under `files`, and every art source, third-party or `project-owned`, under `third_party`, pinned by its content hash with a licence from `ALLOWED_LICENCES` (`packages/scene-assets/src/schema.ts`) and the user's acceptance (`accepted_by`). The asset validator refuses a source without such an entry. Reference images are for judging our work and never ship.

## Communicating with the user

The user is very technical but doesn't read the code day-to-day. Responding in code or pointing at files is fine — just don't assume they already know what a given variable, function, or module does; introduce it briefly on first mention.

API seams and schemas are the most important things to surface. When work touches an interface between components, lead with what that contract looks like and how it changed. Examples are the simulation's commands, observation and publication layout, `Battle::digest`, fixture JSON, and module boundaries.

## Worktrees: keep them cheap

Every worktree is a full checkout. Large binaries, `node_modules` and Rust build output multiply per worktree and have run this machine out of memory and disk before. Every worktree, whether you make it or an agent harness makes it, follows this recipe.

1. **Large binaries are in Git LFS** (`*.glb`, textures, reference images under `assets/`). The repo is configured to skip downloading them on checkout, so a new worktree gets small pointer files. Pull only what your task needs:

   ```bash
   git lfs pull --include="specs/done/battle-look/assets/reference/warno/**"
   ```

   Never run a bare `git lfs pull` in a worktree.
2. **Share `node_modules`** from the main checkout. Don't install again:

   ```bash
   ln -s /Users/david/dev/battlegame/web/node_modules web/node_modules
   ```
3. **Give each worktree its own Rust build directory, under the main one.** Never point two worktrees at the same `target/`. Cargo leaves a workspace crate's path out of its build hash, so worktrees with different `contract` or `sim` sources overwrite each other's builds. The symptom is a clippy or type error from another branch's change. Keep every worktree's build under the main checkout's `target/`, so it is easy to find and delete:

   ```bash
   export CARGO_TARGET_DIR=/Users/david/dev/battlegame/target/wt/$(basename "$PWD")
   ```

   Delete that directory when you remove the worktree (step 5).
4. Build the WebAssembly once (`bun run build:wasm`) before web tests or scenes.
5. Remove the worktree when its branch is merged, along with its `target/wt/<name>` build directory.

One-time setup, kept here for fresh machines:

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
cargo test -p sim --test sim village::a_replay_matches  # one test
cargo test -p sim --test sim village::                  # one file (the sim tests are one binary)
cargo test -p sim                                       # one crate
bun run --cwd web test -- tests/observation.test.ts     # one web test file
bun run --cwd web scene -- village                      # one browser scene
bun run --cwd web scene -- --list                       # scene ids
```

Run `bun run check` and `bun run verify` once, at the end of an implementation pass, before a merge.

### Which battles a change needs

The village report (`cargo run -p sim --release --example village_report`) plays blue's comparison scripts against the red defender, one battle per (script, seed). Run only what the change can move:

| Change | Battles |
|---|---|
| Renderer, UI, sound, docs | 0 (scene checks only) |
| Sim change that shouldn't alter outcomes (refactor, perf) | 0: the digest and replay tests prove nothing moved |
| Sim rule change | `village_report -- --quick` (the flank and one ambush, 3 seeds, 600 s: about 20 s) |
| Balance tuning or spec closeout | the full report (every script, the ten seeds, 900 s), once |

- `--quick` is the feedback loop for a rule change. `--scripts flank,ambush-0.75` and `--seeds 1,2` narrow it further, and `--max-s` shortens the battles.
- Each row ends in the battle's final digest. Two runs are the same battle exactly when the digests match.
- `--compare main` sets the run beside main's. Results are cached per commit of the simulation sources, in the main checkout's `throwaway/village-report/`, so main's baseline runs once and every worktree reuses it. If main's run of those flags isn't cached yet, run the same flags once on a clean checkout of main. `--save <file>` and `--compare <file>` do the same with a file, for an uncommitted baseline.
- Run the full report once, at closeout, and only when the change can move balance or sim performance. Don't sleep-poll a long run mid-pass: start it in the background and keep working.

Simulation performance changes must leave battle digests unchanged, or be named decisions. Measure them in instructions retired, which don't move with machine load. `endurance_report` prints them per five minutes of battle, and `village_report` prints the run's total. The digest and replay tests are the proof that nothing moved. `village_report --compare` is the cross-check over whole battles.

## Visual changes

Browser scenes write evidence into gitignored `throwaway/evidence/<fixture-id>/`.

For any visual change:
- use [`screenshot-critique`](.agents/skills/screenshot-critique/SKILL.md) for an unprimed second opinion;
- use [`compare-screenshots`](.agents/skills/compare-screenshots/SKILL.md) to judge before/after shots and shots against references;
- use [`preview-shots`](.agents/skills/preview-shots/SKILL.md) to show shots to the user.

The renderer skill is [`renderer`](.agents/skills/renderer/SKILL.md). Keep it current: when a pass learns a renderer lesson (a gotcha, a pattern that paid off, a rejected approach), add it there in the same commit. General, project-agnostic architecture rules also go to the shared copy in `~/dev/skills` (`skills/graphics/renderer`).

## Game rules

Before proposing or changing a game mechanic (shooting, sight, blocking, pushing, destruction, cover, targeting), invoke [`tweak-mechanics`](.agents/skills/tweak-mechanics/SKILL.md).

## TypeScript math

Use the npm [`math`](https://github.com/pmndrs/math) package for vectors, matrices, quaternions, shapes, culling, noise, seeded randomness and easing, wherever it fits. Load the [`math`](.agents/skills/math/SKILL.md) skill before writing any. Don't add a hand-rolled equivalent.

## Skills

Repo skills live in `.agents/skills/<name>/`. `.claude/skills/<name>` is only a relative symlink to that folder.
