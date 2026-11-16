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
/** The site as a reader reaches it. */
export const productionSite = `https://${productionDomain}`;
/** The runner is a process boundary: rehearsals never invoke a hosting CLI. */
export function promoteMain({ yes: ship = false, runner = run, report = console.log } = {}) {
  function fail(message) {
    throw new Error(message);
  }
  function git(...args) {
    const result = runner("git", args);
    if (result.status !== 0) fail(`git ${args[0]} failed`);
    return result.stdout.trimEnd();
  }
  function vercel(...args) {
    const result = runner("bun", [...vercelCli, ...args, ...vercelScope]);
    if (result.status !== 0) fail(`vercel ${args[0]} failed`);
    return result.stdout;
  }
  try {
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
    if (runner("git", ["merge-base", "--is-ancestor", main, staging]).status !== 0)
      fail("main is not contained by staging; merge main into staging and push staging first");
    const deployments = JSON.parse(
      vercel("list", vercelProject, "--environment", "production", "--limit", "100", "--json"),
    ).deployments;
    const servedUrl = JSON.parse(vercel("inspect", productionDomain, "--json")).url;
    const served = deployments.find((d) => d.url === servedUrl)?.meta?.githubCommitSha;
    if (!served) fail(`could not tell which commit ${productionDomain} serves`);
    const staged = deployments.find(
      (d) => d.meta?.githubCommitSha === staging && d.meta?.githubCommitRef === "staging",
    );
    if (served === staging) report(`production already serves staging ${staging}`);
    else {
      if (!staged) fail(`Vercel has no production deployment of staging ${staging}`);
      if (staged.state === "ERROR" || staged.state === "CANCELED")
        fail(`the deployment of ${staging} is ${staged.state}: ${staged.url}`);
      report(
        `staging ${staging.slice(0, 9)} ships:\n${git("log", "--format=  %h %an  %s", `${served}..${staging}`)}`,
      );
      if (ship) {
        vercel("inspect", staged.url, "--wait", "--timeout", "30m");
        vercel("promote", staged.url, "--yes");
        report(`promoted staging ${staging} (${staged.url}) to production`);
      } else report("rerun with --yes to promote these commits to production");
    }
    // Production serves staging once it has been promoted, now or earlier.
    if (served !== staging && !ship) {
      report("The lobby is checked against the site once it is promoted.");
      return 0;
    }
    // Fast-forwards origin's main to the commit production now serves.
    let mainRecorded = true;
    if (main !== staging) {
      mainRecorded = runner("git", ["push", "origin", `${staging}:refs/heads/main`]).status === 0;
      if (mainRecorded) report(`main fast-forwarded to ${staging}`);
      else report("promote:main: git push failed; site promotion is complete");
    }
    // A staged deployment's own address asks for a sign-in. The production
    // domain is the site that anyone, and the lobby's deploy, can read.
    const site = productionSite;
    const deploy = ["run", "deploy:server", "--site", site, "--commit", staging];
    const checked = runner("bun", [...deploy, "--check"]);
    if (checked.status === 0) {
      report("Lobby is in step with the site; no deploy needed.");
      return mainRecorded ? 0 : 1;
    }
    if (checked.status !== 2) fail("could not check the lobby against the site's build");
    if (!ship) {
      report(`Lobby would deploy for site commit ${staging}; rerun with --yes.`);
      return mainRecorded ? 0 : 1;
    }
    report(`Lobby needs a deploy for promoted commit ${staging}.`);
    const deployed = runner("bun", [...deploy, "--push"]);
    if (deployed.status !== 0) {
      report("Site promotion is complete. Online play is refused until the lobby is deployed.");
      report(`Repair: bun run deploy:server --site ${site} --commit ${staging} --push`);
      return 1;
    }
    report(`Lobby deployed for promoted commit ${staging}.`);
    return mainRecorded ? 0 : 1;
  } catch (error) {
    report(`promote:main: ${error instanceof Error ? error.message : "promotion failed"}`);
    return 1;
  }
}

function run(command, args) {
  return spawnSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"] });
}

if (import.meta.main) process.exitCode = promoteMain({ yes: process.argv.includes("--yes") });
