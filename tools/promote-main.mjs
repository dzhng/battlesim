// Promotes production. Vercel builds each staging push as a staged production
// deployment (staging is its production branch, and custom domains are not
// auto-assigned) and never builds main, so promotion builds nothing: it
// promotes the deployment Vercel already built from origin's staging, then
// fast-forwards origin's main to that commit as the record of what production
// serves. It pushes that commit itself, so local branches and uncommitted
// files are untouched, and without force, so Git refuses anything but a
// fast-forward. Pushing main by hand changes nothing that is served.
//
// Promotion ships every commit production lacks, whoever pushed it, so a first
// run lists them and only `--yes` ships.
import { spawnSync } from "node:child_process";

const vercelCli = ["x", "vercel@65.16.0"];
const vercelScope = ["--scope", "david-zhangs-projects-6456877a"];
const vercelProject = "battlegame";
const productionDomain = "www.armata.game";
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
git("fetch", "--quiet", "--no-tags", "origin", "main", "staging");
if (run("git", ["merge-base", "--is-ancestor", main, staging]).status !== 0)
  fail("main is not contained by staging; merge main into staging and push staging first");

const deployments = JSON.parse(
  vercel("list", vercelProject, "--environment", "production", "--limit", "100", "--json"),
).deployments;
const servedUrl = JSON.parse(vercel("inspect", productionDomain, "--json")).url;
const served = deployments.find((d) => d.url === servedUrl)?.meta?.githubCommitSha;
if (!served) fail(`could not tell which commit ${productionDomain} serves`);
if (served === staging) {
  console.log(`production already serves staging ${staging}`);
  recordOnMain();
  process.exit(0);
}

const staged = deployments.find(
  (d) => d.meta?.githubCommitSha === staging && d.meta?.githubCommitRef === "staging",
);
if (!staged) fail(`Vercel has no production deployment of staging ${staging}`);
if (staged.state === "ERROR" || staged.state === "CANCELED")
  fail(`the deployment of ${staging} is ${staged.state}: ${staged.url}`);
console.log(`staging ${staging.slice(0, 9)} ships:\n${git("log", "--format=  %h %an  %s", `${served}..${staging}`)}`);
if (!ship) {
  console.log("rerun with --yes to promote these commits to production");
  process.exit(0);
}

vercel("inspect", staged.url, "--wait", "--timeout", "30m");
vercel("promote", staged.url, "--yes");
console.log(`promoted staging ${staging} (${staged.url}) to production`);
recordOnMain();

/** Fast-forwards origin's main to the commit production now serves. */
function recordOnMain() {
  if (main === staging) return;
  git("push", "origin", `${staging}:refs/heads/main`);
  console.log(`main fast-forwarded to ${staging}`);
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
