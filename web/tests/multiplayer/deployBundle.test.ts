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
import settings from "../../../apps/game-server/deploy/settings.json";
import { gunzipSync } from "node:zlib";
import { readFile, readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { expect, test, vi } from "vitest";
import { releaseRepository } from "../../../apps/game-server/deploy/repository.testSupport.ts";
import { deployServer } from "../../../apps/game-server/deploy/deployment.ts";
import { localSite } from "../../../apps/game-server/deploy/localSite.testSupport.ts";
import { serviceStandIn } from "../../../apps/game-server/deploy/sandbox.testSupport";
import { httpRecords } from "../../../packages/multiplayer/src/stream/http";
import { readJson } from "../../../packages/multiplayer/src/stream/index";
import type { RoomView } from "../../../packages/multiplayer/src/wire";

const root = join(import.meta.dirname, "../../..");
test("the deploy dry run lists its complete release and its hosted entry seats a player on a rented machine", async () => {
  const repository = await releaseRepository(root);
  const site = await localSite(repository.folder, repository.sourceCommit);
  expect(repository.head).not.toBe(site.build.commit);
  let lobby: Awaited<ReturnType<Awaited<ReturnType<typeof serviceStandIn>>["lobby"]>> | undefined;
  let service: Awaited<ReturnType<typeof serviceStandIn>> | undefined;
  const abort = new AbortController();
  try {
    const lines: string[] = [];
    expect(
      await deployServer(
        { site: site.address },
        { root: repository.folder, report: (line) => lines.push(line) },
      ),
    ).toBe(0);
    const output = lines.join("\n");
    await writeFile(join(root, "throwaway/multiplayer/deploy-dry-run.log"), output);
    const folder = join(repository.folder, /Prepared folder: (.+)/.exec(output)![1]);
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
          // As the host's proxy states each client's address.
          headers: {
            "X-Player": who,
            "Content-Type": "application/json",
            [settings.clientAddressHeader]: "198.51.100.7",
          },
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
    // A request that reached the hosted lobby without the proxy's word on
    // who sent it is refused rather than counted against nobody.
    expect(
      (
        await fetch(`${lobby.address}/rooms`, {
          method: "POST",
          headers: { "X-Player": who, "Content-Type": "application/json" },
          body: JSON.stringify({ visibility: "private" }),
        })
      ).status,
    ).toBe(400);
    const proxied = { "X-Player": who, [settings.clientAddressHeader]: "198.51.100.7" };
    for await (const record of httpRecords(
      `${lobby.address}/rooms/${code}`,
      proxied,
      abort.signal,
    )) {
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
    await repository.close();
  }
}, 60_000);

test.each(["data", "wire"] as const)(
  "preparation refuses a site's false %s identity before offering any upload",
  async (part) => {
    const { createHash } = await import("node:crypto");
    const repository = await releaseRepository(root);
    const site = await localSite(repository.folder, repository.sourceCommit);
    try {
      const falseBuild = {
        ...site.build,
        [part]: createHash("sha256")
          .update(site.build[part] + "different source")
          .digest("hex"),
      };
      await writeFile(join(site.folder, "build.json"), JSON.stringify(falseBuild));
      const lines: string[] = [];
      expect(
        await deployServer(
          { site: site.address },
          { root: repository.folder, report: (line) => lines.push(line) },
        ),
      ).toBe(1);
      expect(lines.join("\n")).toContain("Bundled server identity differs from site");
      expect(lines.join("\n")).not.toContain("Would push:");
    } finally {
      await site.close();
      await repository.close();
    }
  },
);
