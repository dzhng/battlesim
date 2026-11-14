// Promotes production. Vercel builds each staging push as a staged production
// deployment (staging is its production branch, and custom domains are not
// auto-assigned), so promotion builds nothing: it promotes the deployment
// Vercel already built from origin's staging, then fast-forwards origin's main
// to that commit as the record of what production serves. It pushes that
// commit itself, so local branches and uncommitted files are untouched, and
// without force, so Git refuses anything but a fast-forward.
//
// Promotion ships every commit between main and staging, whoever pushed it, so
// a first run lists them and only `--yes` ships.
import { spawnSync } from "node:child_process";

const vercelCli = ["x", "vercel@65.16.0"];
const vercelScope = ["--scope", "david-zhangs-projects-6456877a"];
const vercelProject = "battlegame";
const ship = process.argv.includes("--yes");

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
  process.exit(0);
}

git("fetch", "--quiet", "--no-tags", "origin", "main", "staging");
if (run("git", ["merge-base", "--is-ancestor", main, staging]).status !== 0)
  fail("main is not contained by staging; merge main into staging and push staging first");
console.log(`staging ${staging.slice(0, 9)} ships:\n${git("log", "--format=  %h %an  %s", `${main}..${staging}`)}`);
if (!ship) {
  console.log("rerun with --yes to promote these commits to production");
  process.exit(0);
}

const url = stagedDeployment(staging);
vercel("inspect", url, "--wait", "--timeout", "30m");
vercel("promote", url, "--yes");
git("push", "origin", `${staging}:refs/heads/main`);
console.log(`promoted staging ${staging} (${url}) to production and main`);

/** The production deployment Vercel built from staging at `sha`. */
function stagedDeployment(sha) {
  const listed = JSON.parse(
    vercel("list", vercelProject, "--environment", "production", "--meta", `githubCommitSha=${sha}`, "--json"),
  ).deployments.filter((d) => d.meta?.githubCommitRef === "staging");
  if (listed.length === 0) fail(`Vercel has no production deployment of staging ${sha}`);
  const { url, state } = listed[0];
  if (state === "ERROR" || state === "CANCELED") fail(`the deployment of ${sha} is ${state}: ${url}`);
  return url;
}

function vercel(...args) {
  const result = run("bun", [...vercelCli, ...args, ...vercelScope]);
  if (result.status !== 0) fail(`vercel ${args[0]} failed`);
  return result.stdout;
}

function git(...args) {
  const result = run("git", args);
  if (result.status !== 0) fail(`git ${args[0]} failed`);
  return result.stdout.trimEnd();
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

function fail(message) {
  console.error(`promote:main: ${message}`);
  process.exit(1);
}
