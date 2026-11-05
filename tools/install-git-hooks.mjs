// Installs this repository's hooks beside Git LFS's in the shared hooks
// directory, so one installation covers every worktree. It never sets
// core.hooksPath: that would hide LFS's hooks in every checkout.
import { execFileSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const marker = "# Installed by tools/install-git-hooks.mjs";
const preCommit = `#!/bin/sh
${marker}; runs this checkout's pre-commit checks.
# A checkout whose branch predates the checks commits unchanged.
[ -f lint-staged.config.mjs ] || exit 0
[ -x node_modules/.bin/lint-staged ] || {
  echo "pre-commit: the checks aren't installed here; run bun run setup" >&2
  exit 1
}
exec bun run lint:staged
`;
// The release order first, then LFS's own pre-push, which uploads the objects
// the pushed commits name. Both read the push's ref updates from stdin.
const prePush = `#!/bin/sh
${marker}; checks the main/staging order, then runs Git LFS's pre-push.
command -v git-lfs >/dev/null 2>&1 || {
  echo "pre-push: this repository uses Git LFS, but git-lfs is not on your path" >&2
  exit 2
}
updates=$(cat)
[ -z "$updates" ] || updates="$updates
"
printf '%s' "$updates" | node "$(dirname "$0")/check-branch-order.mjs" "$@" || exit 1
printf '%s' "$updates" | git lfs pre-push "$@"
`;
/** The pre-push \`git lfs install\` writes, which ours replaces and runs. */
const lfsPrePush = /^#!\/bin\/sh\ncommand -v git-lfs [^\n]*\ngit lfs pre-push "\$@"\n?$/;

const git = (...args) =>
  execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();

let common;
try {
  common = git("rev-parse", "--path-format=absolute", "--git-common-dir");
} catch {
  process.exit(0); // Not a Git checkout (an exported tree): nothing to install.
}

let hooksPath = "";
try {
  hooksPath = git("config", "--get", "core.hooksPath");
} catch {
  // Unset: Git uses the common hooks directory.
}
if (hooksPath === ".husky/_") git("config", "--unset", "core.hooksPath");
else if (hooksPath) {
  console.warn(`core.hooksPath is ${hooksPath}; not installing the Git hooks.`);
  process.exit(0);
}

const hooks = join(common, "hooks");
mkdirSync(hooks, { recursive: true });
// A copy, not a path into a checkout: a branch that predates the checker, or
// a checkout that is removed, still pushes under the guard.
const checker = readFileSync(new URL("./check-branch-order.mjs", import.meta.url), "utf8");
writeFileSync(join(hooks, "check-branch-order.mjs"), checker);
for (const [name, hook, replaces] of [
  ["pre-commit", preCommit, null],
  ["pre-push", prePush, lfsPrePush],
]) {
  const path = join(hooks, name);
  const current = existsSync(path) ? readFileSync(path, "utf8") : null;
  if (current !== null && !current.includes(marker) && !replaces?.test(current)) {
    console.warn(`${path} exists and is not ours; not replacing it.`);
    continue;
  }
  writeFileSync(path, hook);
  chmodSync(path, 0o755);
}
