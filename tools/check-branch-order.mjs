// The pre-push guard on the release order: every commit on main has been on
// staging first, and staging never moves behind main. Git hands it the push's
// ref updates on stdin and the destination as its second argument; it asks the
// remote for the tips the push leaves alone. `install-git-hooks.mjs` copies it
// into the shared hooks directory, so a checkout whose branch predates it is
// guarded too.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";

const mainRef = "refs/heads/main";
const stagingRef = "refs/heads/staging";
const updates = new Map(
  readFileSync(0, "utf8")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => {
      const [, sha, ref] = line.split(/\s+/);
      return [ref, sha];
    }),
);
if (!updates.has(mainRef) && !updates.has(stagingRef)) process.exit(0);

// The push destination, not the remote's name: a push URL can differ from the
// fetch URL, and a push may name no remote at all.
const remote = process.argv[3];
if (!remote) fail("could not determine the push destination");
const tips = new Map(
  git("ls-remote", "--refs", remote, mainRef, stagingRef)
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => {
      const [sha, ref] = line.split(/\s+/);
      return [ref, sha];
    }),
);
const main = updates.get(mainRef) ?? tips.get(mainRef);
const staging = updates.get(stagingRef) ?? tips.get(stagingRef);

if (main === undefined) process.exit(0); // Staging can be created before main.
if (/^0+$/.test(main)) fail("refusing to delete main");
if (!staging || /^0+$/.test(staging))
  fail("staging must exist while main exists. Push to staging first.");

if (updates.has(mainRef)) {
  const publishedStaging = tips.get(stagingRef);
  if (!publishedStaging || !contains(publishedStaging, main))
    fail("main must be contained by remote staging. Push to staging first, then promote main.");
}
if (updates.has(stagingRef)) {
  // A non-atomic push may land staging while the remote rejects main's update.
  for (const production of new Set([main, tips.get(mainRef)]))
    if (production !== undefined && !contains(staging, production))
      fail("staging must contain main; refusing to move staging behind or away from production");
}

function contains(tip, ancestor) {
  for (const sha of [tip, ancestor])
    if (run("cat-file", "-e", `${sha}^{commit}`).status !== 0)
      git(
        "fetch",
        "--quiet",
        "--no-tags",
        "--no-write-fetch-head",
        "--no-recurse-submodules",
        remote,
        sha,
      );
  const result = run("merge-base", "--is-ancestor", ancestor, tip);
  if (result.status !== 0 && result.status !== 1) fail("could not check main/staging ancestry");
  return result.status === 0;
}

function run(...args) {
  return spawnSync("git", args, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 30_000,
  });
}

function git(...args) {
  const result = run(...args);
  if (result.status !== 0) fail(`git ${args[0]} failed; could not verify main/staging order`);
  return result.stdout;
}

function fail(message) {
  console.error(`pre-push: ${message}`);
  process.exit(1);
}
