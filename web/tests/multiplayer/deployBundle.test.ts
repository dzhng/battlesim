// @vitest-environment node
vi.mock("node:child_process", async (original) => {
  const actual = await original<typeof import("node:child_process")>();
  const permitted = (command: unknown) => {
    if (!["bun", "node", "git", "tar", "/bin/sh"].includes(String(command)))
      throw new Error("External process execution forbidden in deploy tests");
  };
  return {
    ...actual,
    execFileSync: (...args: Parameters<typeof actual.execFileSync>) => {
      permitted(args[0]);
      return actual.execFileSync(...args);
    },
    spawn: (...args: Parameters<typeof actual.spawn>) => {
      permitted(args[0]);
      return actual.spawn(...args);
    },
    exec: () => {
      throw new Error("Shell execution forbidden in deploy tests");
    },
    execSync: () => {
      throw new Error("Shell execution forbidden in deploy tests");
    },
    execFile: () => {
      throw new Error("Process execution forbidden in deploy tests");
    },
    fork: () => {
      throw new Error("Process execution forbidden in deploy tests");
    },
  };
});
import { gunzipSync } from "node:zlib";
import { spawn } from "node:child_process";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { expect, test, vi } from "vitest";
import { localSite } from "../../../apps/game-server/deploy/localSite.testSupport.ts";
import { serviceStandIn } from "../../../apps/game-server/deploy/sandbox.testSupport";
import { httpRecords } from "../../../packages/multiplayer/src/stream/http";
import { readJson } from "../../../packages/multiplayer/src/stream/index";
import type { RoomView } from "../../../packages/multiplayer/src/wire";

const root = join(import.meta.dirname, "../../..");
test("the deploy dry run lists its complete release and its hosted entry seats a player on a rented machine", async () => {
  const site = await localSite(root);
  let lobby: Awaited<ReturnType<Awaited<ReturnType<typeof serviceStandIn>>["lobby"]>> | undefined;
  let service: Awaited<ReturnType<typeof serviceStandIn>> | undefined;
  const abort = new AbortController();
  try {
    const output = await new Promise<string>((resolve, reject) => {
      const child = spawn("bun", ["run", "deploy:server", "--site", site.address], {
        cwd: root,
        stdio: ["ignore", "pipe", "pipe"],
      });
      let text = "";
      child.stdout.on("data", (b) => {
        text += b;
      });
      child.stderr.on("data", (b) => {
        text += b;
      });
      child.once("error", reject);
      child.once("exit", (code) => (code === 0 ? resolve(text) : reject(new Error(text))));
    });
    await writeFile(join(root, "throwaway/multiplayer/deploy-dry-run.log"), output);
    const folder = join(root, /Prepared folder: (.+)/.exec(output)![1]);
    const files = (await readdir(folder)).sort();
    const reported = [...output.matchAll(/Would push: ([^\n]+) \(\d+ bytes\)/g)]
      .map((m) => m[1])
      .sort();
    expect(reported).toEqual(files);
    expect(output).toContain("Push is OFF");
    expect(JSON.parse(await readFile(join(folder, "build.json"), "utf8"))).toEqual(site.build);
    const archive = join(folder, "battle.tar.gz");
    const compressed = await readFile(archive);
    expect(gunzipSync(compressed).length).toBeGreaterThan(compressed.length);
    service = await serviceStandIn(archive);
    lobby = await service.lobby(folder);
    const who = "d".repeat(32);
    async function post(path: string, body: unknown, address = lobby!.address) {
      return (
        await fetch(`${address}${path}`, {
          method: "POST",
          headers: { "X-Player": who, "Content-Type": "application/json" },
          body: JSON.stringify(body),
        })
      ).json();
    }
    expect(await post("/hello", { build: site.build, callsign: "SCOUT" })).toEqual({
      callsign: "SCOUT",
    });
    const { code } = await post("/rooms", { visibility: "private" });
    for (const act of [
      { kind: "set_seat", seat: "seat-2", to: "ai" },
      { kind: "settings", patch: { map: { type: "open", size: "small", seed: "1" } } },
      { kind: "ready", ready: true },
      { kind: "start" },
    ])
      expect(await post(`/rooms/${code}/act`, act)).toEqual({ ok: true });
    for await (const record of httpRecords(`${lobby.address}/rooms/${code}`, who, abort.signal)) {
      if (record.kind === "end") throw new Error("Hosted battle failed before seating");
      if (record.kind !== "state") continue;
      const view = readJson<RoomView>(record);
      if (view.phase.kind !== "battle") continue;
      const ticket = view.phase.ticket;
      expect(ticket.address).not.toBe(lobby.address);
      expect(
        await post(
          `/battles/${ticket.battle}/up`,
          { seq: 1, items: [{ id: 1, order: { kind: "ready" }, queued: false }] },
          ticket.address,
        ),
      ).toMatchObject({ results: [{ ack: { error: null } }] });
      break;
    }
    abort.abort();
    await lobby.close();
    lobby = undefined;
    await vi.waitFor(
      () => expect([...service!.machines.values()].map((m) => m.state)).toEqual(["deleted"]),
      { timeout: 10_000 },
    );
    expect(service.appCredentialEscaped()).toBe(false);
  } finally {
    abort.abort();
    await lobby?.close();
    await service?.close();
    await site.close();
  }
}, 60_000);
