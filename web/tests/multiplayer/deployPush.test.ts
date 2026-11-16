// @vitest-environment node
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { expect, test, vi } from "vitest";
vi.mock("node:child_process", () => ({
  execFileSync: () => {
    throw new Error("Host execution forbidden in this test");
  },
}));
import { push } from "../../../apps/game-server/deploy/push.ts";
import { pushStandIn } from "../../../apps/game-server/deploy/push.testSupport.ts";

test("deployment registers with service access, a second deploy updates files and restarts the same app, and neither makes it public", async () => {
  const folder = await mkdtemp(join(tmpdir(), "battle-push-"));
  const host = pushStandIn();
  try {
    await mkdir(join(folder, "fixtures"));
    await writeFile(join(folder, "lobby.mjs"), "entry");
    await writeFile(join(folder, "fixtures/rules.json"), "rules");
    await writeFile(join(folder, "battle.tar"), "archive");
    await push(folder, "test-battles", "local-workspace", host.runner);
    expect(host.apps.get("test-battles")).toMatchObject({
      apiScopes: ["services"],
      command: "node lobby.mjs",
    });
    expect([...host.files.entries()]).toEqual(
      expect.arrayContaining([
        ["apps/test-battles/lobby.mjs", "entry"],
        ["apps/test-battles/fixtures/rules.json", "rules"],
        ["apps/test-battles/battle.tar", "archive"],
      ]),
    );
    await writeFile(join(folder, "lobby.mjs"), "new entry");
    await push(folder, "test-battles", "local-workspace", host.runner);
    expect(host.files.get("apps/test-battles/lobby.mjs")).toBe("new entry");
    expect([...host.apps.keys()]).toEqual(["test-battles"]);
    expect(host.restarts()).toBeGreaterThan(0);
    // Who can reach the app is its operator's decision, on both deploys.
    expect(host.apps.get("test-battles")!.visibility).toBe("private");
  } finally {
    await rm(folder, { recursive: true, force: true });
  }
});
