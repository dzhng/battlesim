// Promotes production: fast-forwards origin's main to the commit origin's
// staging already published, so main is always staging or behind it. It pushes
// that commit itself, so local branches and uncommitted files are untouched,
// and without force, so Git refuses anything but a fast-forward.
import { spawnSync } from "node:child_process";

const tips = new Map(
  git("ls-remote", "--heads", "origin", "main", "staging")
    .split("\n")
    .filter(Boolean)
    .map((line) => line.split(/\s+/).reverse()),
);
const main = tips.get("refs/heads/main");
const staging = tips.get("refs/heads/staging");
if (!main || !staging) fail("origin must have both main and staging");

if (main === staging) {
  console.log(`main already equals staging (${staging})`);
} else {
  git("fetch", "--quiet", "--no-tags", "origin", "main", "staging");
  if (run("merge-base", "--is-ancestor", main, staging).status !== 0)
    fail("main is not contained by staging; merge main into staging and push staging first");
  git("push", "origin", `${staging}:refs/heads/main`);
  console.log(`promoted staging ${staging} to main`);
}

function run(...args) {
  return spawnSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

function git(...args) {
  const result = run(...args);
  if (result.status !== 0) fail(`git ${args[0]} failed`);
  return result.stdout.trim();
}

function fail(message) {
  console.error(`promote:main: ${message}`);
  process.exit(1);
}
