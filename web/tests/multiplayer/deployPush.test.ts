// @vitest-environment node
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { expect, test, vi } from "vitest";
vi.mock("node:child_process", () => ({
  execFileSync: vi.fn(() => {
    throw new Error("Host execution forbidden in this test");
  }),
}));
import { push } from "../../../apps/game-server/deploy/push.ts";
import settings from "../../../apps/game-server/deploy/settings.json";
import { pushStandIn } from "../../../apps/game-server/deploy/push.testSupport.ts";

test("deployment registers with service access, a second deploy updates files and restarts the same app, and neither makes it public", async () => {
  const folder = await mkdtemp(join(tmpdir(), "battle-push-"));
  const host = pushStandIn();
  try {
    await writeFile(join(folder, "settings.json"), JSON.stringify(settings));
    await mkdir(join(folder, "fixtures"));
    await writeFile(join(folder, "lobby.mjs"), "entry");
    await writeFile(join(folder, "fixtures/rules.json"), "rules");
    await writeFile(join(folder, "battle.tar.gz"), "archive");
    await push(folder, host.runner);
    expect(host.apps.get(settings.app)).toMatchObject({
      apiScopes: ["services"],
      command: "node lobby.mjs",
    });
    expect([...host.files.entries()]).toEqual(
      expect.arrayContaining([
        [`apps/${settings.app}/lobby.mjs`, "entry"],
        [`apps/${settings.app}/fixtures/rules.json`, "rules"],
        [`apps/${settings.app}/battle.tar.gz`, "archive"],
      ]),
    );
    await writeFile(join(folder, "lobby.mjs"), "new entry");
    await push(folder, host.runner);
    expect(host.files.get(`apps/${settings.app}/lobby.mjs`)).toBe("new entry");
    expect([...host.apps.keys()]).toEqual([settings.app]);
    expect(host.restarts()).toBeGreaterThan(0);
    // Who can reach the app is its operator's decision, on both deploys.
    expect(host.apps.get(settings.app)!.visibility).toBe("private");
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});

test("an app the host cannot start at first is asked again until it is up, and a deploy that never starts fails", async () => {
  const folder = await mkdtemp(join(tmpdir(), "battle-push-"));
  try {
    await writeFile(join(folder, "settings.json"), JSON.stringify(settings));
    await writeFile(join(folder, "lobby.mjs"), "entry");
    const slow = pushStandIn();
    slow.refuseStarts(2);
    await push(folder, slow.runner, async () => {});
    expect(slow.restarts()).toBeGreaterThan(0);
    const dead = pushStandIn();
    dead.refuseStarts(Number.POSITIVE_INFINITY);
    await expect(push(folder, dead.runner, async () => {})).rejects.toThrow();
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});

test("host process diagnostics cannot escape in deployment errors", async () => {
  const folder = await mkdtemp(join(tmpdir(), "battle-push-"));
  const credential = randomBytes(16).toString("hex");
  try {
    await writeFile(join(folder, "settings.json"), JSON.stringify(settings));
    vi.mocked(execFileSync).mockImplementationOnce(() => {
      throw new Error(credential);
    });
    await expect(push(folder)).rejects.not.toThrow(credential);
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});
