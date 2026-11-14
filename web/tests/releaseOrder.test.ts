// @vitest-environment node
// The release order, run by real pushes to a scratch remote: work reaches
// staging first and `promote:main` fast-forwards main to it; the hooks setup
// installs refuse anything else, and Git LFS's own pre-push still runs on every
// push they allow.
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test } from "vitest";

const ROOT = join(import.meta.dirname, "../..");
const roots: string[] = [];
afterEach(() => {
  for (const root of roots.splice(0)) rmSync(root, { recursive: true, force: true });
});

/** A clone of a scratch remote with main and staging on one commit, Git LFS's
 *  stock pre-push installed the way `git lfs install` leaves it, a stand-in
 *  `git-lfs` on the path that records what each push handed it, and a
 *  stand-in Vercel (`bun x vercel@…`) serving `vercel.json`'s deployments and
 *  recording each promotion, so the test never reaches the real project. */
function fixture() {
  const root = mkdtempSync(join(tmpdir(), "git-hooks-"));
  roots.push(root);
  const bin = join(root, "bin");
  mkdirSync(bin);
  const lfsLog = join(root, "lfs.log");
  writeFileSync(
    join(bin, "git-lfs"),
    `#!/bin/sh\n[ "$1" = pre-push ] || exit 0\necho "push $2" >> "${lfsLog}"\ncat >> "${lfsLog}"\n`,
  );
  chmodSync(join(bin, "git-lfs"), 0o755);
  const vercel = join(root, "vercel.json");
  const realBun = spawnSync("which", ["bun"], { encoding: "utf8" }).stdout.trim();
  writeFileSync(
    join(root, "vercel.mjs"),
    `import { readFileSync, writeFileSync } from "node:fs";
const [command, target] = process.argv.slice(2);
const state = JSON.parse(readFileSync(${JSON.stringify(vercel)}, "utf8"));
if (command === "list") console.log(JSON.stringify({ deployments: state.deployments }));
else if (command === "inspect" && target === "www.armata.game") console.log(JSON.stringify({ url: state.served }));
else if (command === "promote") writeFileSync(${JSON.stringify(vercel)}, JSON.stringify({ ...state, served: target, promoted: [...state.promoted, target] }));
`,
  );
  writeFileSync(
    join(bin, "bun"),
    `#!/bin/sh\ncase "$2" in vercel@*) [ "$1" = x ] && shift 2 && exec node "${join(root, "vercel.mjs")}" "$@";; esac\nexec "${realBun}" "$@"\n`,
  );
  chmodSync(join(bin, "bun"), 0o755);
  const env = {
    ...process.env,
    PATH: `${bin}:${process.env.PATH}`,
    GIT_CONFIG_GLOBAL: "/dev/null",
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_AUTHOR_NAME: "t",
    GIT_AUTHOR_EMAIL: "t@example.invalid",
    GIT_COMMITTER_NAME: "t",
    GIT_COMMITTER_EMAIL: "t@example.invalid",
  };
  const run = (cwd: string, ...args: string[]) => {
    const r = spawnSync(args[0], args.slice(1), { cwd, env, encoding: "utf8" });
    return { status: r.status, output: `${r.stdout}${r.stderr}` };
  };
  const remote = join(root, "remote.git");
  const work = join(root, "work");
  run(root, "git", "init", "-q", "--bare", "-b", "main", remote);
  run(root, "git", "init", "-q", "-b", "main", work);
  run(work, "git", "remote", "add", "origin", remote);
  mkdirSync(join(work, "tools"));
  for (const file of ["install-git-hooks.mjs", "check-branch-order.mjs", "promote-main.mjs"])
    copyFileSync(join(ROOT, "tools", file), join(work, "tools", file));
  const hooks = join(work, ".git", "hooks");
  mkdirSync(hooks, { recursive: true });
  writeFileSync(
    join(hooks, "pre-push"),
    `#!/bin/sh\ncommand -v git-lfs >/dev/null 2>&1 || { echo "no git-lfs"; exit 2; }\ngit lfs pre-push "$@"\n`,
  );
  chmodSync(join(hooks, "pre-push"), 0o755);
  run(work, "git", "commit", "-q", "--allow-empty", "-m", "first");
  run(work, "git", "push", "-q", "origin", "main", "main:staging");
  const tip = (ref: string) => run(remote, "git", "rev-parse", ref).output.trim();
  /** Vercel's staged production builds, one per staging commit, and the one served. */
  const deploy = (served: string, shas: string[]) =>
    writeFileSync(
      vercel,
      JSON.stringify({
        served: `${served}.vercel.app`,
        promoted: [],
        deployments: shas.map((sha) => ({
          url: `${sha}.vercel.app`,
          state: "READY",
          meta: { githubCommitSha: sha, githubCommitRef: "staging" },
        })),
      }),
    );
  const promoted = () => JSON.parse(readFileSync(vercel, "utf8")).promoted;
  return { work, lfsLog, run, tip, deploy, promoted };
}

test("main is refused until staging holds it, promote fast-forwards it there, staging is refused behind main, and Git LFS still sees every allowed push", () => {
  const { work, lfsLog, run, tip, deploy, promoted } = fixture();
  expect(run(work, "node", "tools/install-git-hooks.mjs").status).toBe(0);
  writeFileSync(lfsLog, "");

  // A commit pushed straight to main is refused, and LFS is never asked.
  run(work, "git", "commit", "-q", "--allow-empty", "-m", "second");
  const second = run(work, "git", "rev-parse", "HEAD").output.trim();
  const before = tip("main");
  const early = run(work, "git", "push", "origin", "main");
  expect(early.status).not.toBe(0);
  expect(early.output).toMatch(/Push to staging first/);
  expect(tip("main")).toBe(before);
  expect(readFileSync(lfsLog, "utf8")).toBe("");

  // Staging first, then the promotion: a first run only lists what would
  // ship; with --yes Vercel promotes staging's own build, main records it,
  // the checkout's own work is untouched, and LFS saw each push's refs.
  expect(run(work, "git", "push", "origin", "main:staging").status).toBe(0);
  deploy(before, [before, second]);
  writeFileSync(join(work, "work-in-progress.txt"), "keep me\n");
  const status = run(work, "git", "status", "--porcelain").output;
  const listed = run(work, "bun", "tools/promote-main.mjs");
  expect(listed.status, listed.output).toBe(0);
  expect(listed.output).toMatch(/rerun with --yes/);
  expect(tip("main")).toBe(before);
  expect(promoted()).toEqual([]);
  const shipped = run(work, "bun", "tools/promote-main.mjs", "--yes");
  expect(shipped.status, shipped.output).toBe(0);
  expect(promoted()).toEqual([`${second}.vercel.app`]);
  expect(tip("staging")).toBe(second);
  expect(tip("main")).toBe(second);
  expect(run(work, "git", "status", "--porcelain").output).toBe(status);
  expect(run(work, "bun", "tools/promote-main.mjs").output).toMatch(/already serves staging/);
  const lfs = readFileSync(lfsLog, "utf8");
  expect(lfs).toMatch(new RegExp(`push origin\\n\\S+ ${second} refs/heads/staging`));
  expect(lfs).toMatch(new RegExp(`push origin\\n\\S+ ${second} refs/heads/main`));

  // Staging may never fall behind what main already published.
  const back = run(work, "git", "push", "--force", "origin", "HEAD~1:staging");
  expect(back.status).not.toBe(0);
  expect(back.output).toMatch(/staging must contain main/);
  expect(tip("staging")).toBe(second);

  // Any other branch pushes freely.
  expect(run(work, "git", "push", "origin", "HEAD~1:refs/heads/feature").status).toBe(0);
  expect(readFileSync(lfsLog, "utf8")).toMatch(/ refs\/heads\/feature /);

  // A commit that reached main some other way (a merge on GitHub, a push past
  // the hook) is never buried by a promotion: staging must take it first.
  run(work, "git", "commit", "-q", "--allow-empty", "-m", "hotfix");
  expect(run(work, "git", "push", "--no-verify", "origin", "HEAD:main").status).toBe(0);
  const hotfix = tip("main");
  run(work, "git", "checkout", "-q", "-b", "next", "HEAD~1");
  run(work, "git", "commit", "-q", "--allow-empty", "-m", "next");
  expect(run(work, "git", "push", "origin", "next:staging").status).not.toBe(0);
  expect(run(work, "git", "push", "--no-verify", "origin", "next:staging").status).toBe(0);
  const diverged = run(work, "bun", "tools/promote-main.mjs");
  expect(diverged.status).not.toBe(0);
  expect(diverged.output).toMatch(/merge main into staging/);
  expect(tip("main")).toBe(hotfix);
});
