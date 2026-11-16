// @vitest-environment node
import { expect, test, vi } from "vitest";
vi.mock("node:child_process", () => ({
  spawnSync: () => {
    throw new Error("Real promotion process execution forbidden");
  },
}));
import { promoteMain, productionSite } from "../../../tools/promote-main.mjs";

function rehearsal({
  inStep = true,
  deployFails = false,
  alreadyServed = false,
  recordFails = false,
} = {}) {
  const main = "a".repeat(40);
  const staging = "b".repeat(40);
  const site = "https://staged.example.invalid";
  const events: string[] = [];
  const lines: string[] = [];
  let promoted = alreadyServed;
  let deployedCommit: string | undefined;
  const runner = (command: string, args: string[]) => {
    const ok = (stdout = "", status = 0) => ({ status, stdout });
    if (command === "git") {
      if (args[0] === "ls-remote")
        return ok(`${main}\trefs/heads/main\n${staging}\trefs/heads/staging\n`);
      if (["fetch", "merge-base"].includes(args[0])) return ok();
      if (args[0] === "log") return ok("  change to ship");
      if (args[0] === "push") {
        events.push(recordFails ? "main record failed" : "main recorded");
        return ok("", recordFails ? 1 : 0);
      }
    }
    if (command === "bun" && args[0] === "x") {
      if (args.includes("list"))
        return ok(
          JSON.stringify({
            deployments: [
              {
                url: "served.example.invalid",
                meta: { githubCommitSha: alreadyServed ? staging : main },
              },
              {
                url: site,
                state: "READY",
                meta: { githubCommitSha: staging, githubCommitRef: "staging" },
              },
            ],
          }),
        );
      if (args.includes("inspect")) return ok(JSON.stringify({ url: "served.example.invalid" }));
      if (args.includes("promote")) {
        promoted = true;
        events.push("site promoted");
        return ok();
      }
    }
    if (command === "bun" && args[0] === "run" && args[1] === "deploy:server") {
      expect(args[args.indexOf("--commit") + 1]).toBe(staging);
      // A staged deployment's own address cannot be read without signing in.
      expect(args[args.indexOf("--site") + 1]).toBe(productionSite);
      expect(promoted).toBe(true);
      if (args.includes("--check")) {
        events.push("lobby checked");
        return ok("", inStep ? 0 : 2);
      }
      if (args.includes("--push")) {
        expect(promoted).toBe(true);
        deployedCommit = args[args.indexOf("--commit") + 1];
        events.push("lobby deploy");
        return ok("", deployFails ? 1 : 0);
      }
    }
    throw new Error(`Unexpected rehearsal command: ${command} ${args.join(" ")}`);
  };
  return {
    runner,
    lines,
    events,
    staging,
    site,
    promoted: () => promoted,
    deployed: () => deployedCommit,
  };
}

test("a promoted site whose shared build is admitted leaves the lobby alone", async () => {
  const r = rehearsal();
  expect(
    await promoteMain({ yes: true, runner: r.runner, report: (line) => r.lines.push(line) }),
  ).toBe(0);
  expect(r.promoted()).toBe(true);
  expect(r.deployed()).toBeUndefined();
  expect(r.events).toEqual(["site promoted", "main recorded", "lobby checked"]);
  expect(r.lines.join("\n")).toContain("Lobby is in step");
});

test("an incompatible lobby deploys the promoted site's commit", async () => {
  const r = rehearsal({ inStep: false });
  expect(
    await promoteMain({ yes: true, runner: r.runner, report: (line) => r.lines.push(line) }),
  ).toBe(0);
  expect(r.deployed()).toBe(r.staging);
  expect(r.events).toEqual(["site promoted", "main recorded", "lobby checked", "lobby deploy"]);
});

test("a failed lobby deploy exits nonzero with a repair command and never undoes the promoted site", async () => {
  const r = rehearsal({ inStep: false, deployFails: true });
  expect(
    await promoteMain({ yes: true, runner: r.runner, report: (line) => r.lines.push(line) }),
  ).toBe(1);
  expect(r.promoted()).toBe(true);
  expect(r.events).toEqual(["site promoted", "main recorded", "lobby checked", "lobby deploy"]);
  expect(r.lines.join("\n")).toContain("Online play is refused until the lobby is deployed");
  expect(r.lines.join("\n")).toContain(
    `bun run deploy:server --site ${productionSite} --commit ${r.staging} --push`,
  );
});

test("a dry promotion promotes, records, checks and deploys nothing, and says the lobby is checked after", async () => {
  const r = rehearsal({ inStep: false });
  expect(await promoteMain({ runner: r.runner, report: (line) => r.lines.push(line) })).toBe(0);
  expect(r.promoted()).toBe(false);
  expect(r.deployed()).toBeUndefined();
  expect(r.events).toEqual([]);
  expect(r.lines.join("\n")).toContain("once it is promoted");
});

test.each([true, false])(
  "without --yes, a site already promoted has its lobby checked (%s) and never deployed",
  async (inStep) => {
    const r = rehearsal({ inStep, alreadyServed: true });
    expect(await promoteMain({ runner: r.runner, report: (line) => r.lines.push(line) })).toBe(0);
    expect(r.deployed()).toBeUndefined();
    expect(r.events).toEqual(["main recorded", "lobby checked"]);
    expect(r.lines.join("\n")).toContain(inStep ? "Lobby is in step" : "Lobby would deploy");
  },
);

test("rerunning promotion repairs an already promoted site", async () => {
  const r = rehearsal({ inStep: false, alreadyServed: true });
  expect(
    await promoteMain({ yes: true, runner: r.runner, report: (line) => r.lines.push(line) }),
  ).toBe(0);
  expect(r.deployed()).toBe(r.staging);
  expect(r.events).toEqual(["main recorded", "lobby checked", "lobby deploy"]);
});

test("a failed main record still checks and synchronizes the already promoted site", async () => {
  const r = rehearsal({ inStep: false, recordFails: true });
  expect(
    await promoteMain({ yes: true, runner: r.runner, report: (line) => r.lines.push(line) }),
  ).toBe(1);
  expect(r.promoted()).toBe(true);
  expect(r.deployed()).toBe(r.staging);
  expect(r.events).toEqual([
    "site promoted",
    "main record failed",
    "lobby checked",
    "lobby deploy",
  ]);
  expect(r.lines.join("\n")).toContain("git push failed");
});
