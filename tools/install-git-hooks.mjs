// Installs the pre-commit hook beside Git LFS's hooks in the shared hooks
// directory. It never sets core.hooksPath: that would hide LFS's pre-push in
// every checkout of this repository.
import { execFileSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const marker = "# Installed by tools/install-git-hooks.mjs";
const hook = `#!/bin/sh
${marker}; runs this checkout's pre-commit checks.
# A checkout whose branch predates the checks commits unchanged.
[ -f lint-staged.config.mjs ] || exit 0
[ -x node_modules/.bin/lint-staged ] || {
  echo "pre-commit: the checks aren't installed here; run bun run setup" >&2
  exit 1
}
exec bun run lint:staged
`;

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
  console.warn(`core.hooksPath is ${hooksPath}; not installing the pre-commit hook.`);
  process.exit(0);
}

const path = join(common, "hooks", "pre-commit");
if (existsSync(path) && !readFileSync(path, "utf8").includes(marker)) {
  console.warn(`${path} exists and is not ours; not replacing it.`);
  process.exit(0);
}
mkdirSync(dirname(path), { recursive: true });
writeFileSync(path, hook);
chmodSync(path, 0o755);
