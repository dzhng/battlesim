import { scrubCredentials } from "../../../packages/multiplayer/src/identity.ts";
import { readFile } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { setTimeout as delay } from "node:timers/promises";
import type { BattleMachines } from "../src/machines/index.ts";
import { CONTROL_HEADER } from "../src/http/controlProtocol.ts";
import settings from "./settings.json";

export interface SandboxOptions {
  apiUrl: string;
  appKey: string;
  archive: string;
  capacity: number;
  ttlSeconds: number;
  startupMs?: number;
  pollMs?: number;
  error?: (text: string) => void;
}
type Sandbox = { id: string; state: string; labels: Record<string, string> };
const labels = { battlegame: "battle" };

/** Host-specific allocation and reclamation. Call before accepting lobby traffic. */
export async function sandboxBattleMachines(options: SandboxOptions): Promise<BattleMachines> {
  const pollMs = options.pollMs ?? settings.pollMs;
  const startupMs = options.startupMs ?? settings.startupMs;
  const error = options.error ?? ((text: string) => process.stderr.write(text));
  const archive = await readFile(options.archive);
  const api = `${options.apiUrl.replace(/\/$/, "")}/v1/services/boat`;
  const scrub = (text: string) => scrubCredentials(text.split(options.appKey).join("[credential]"));
  async function request(
    path: string,
    method = "GET",
    body?: unknown,
    headers = {},
    signal = AbortSignal.timeout(10_000),
  ) {
    const reply = await fetch(`${api}${path}`, {
      method,
      headers: {
        Authorization: `Bearer ${options.appKey}`,
        "Content-Type": "application/json",
        ...headers,
      },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      signal: AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
      redirect: "error",
    });
    // Never read or reflect service error bodies: they may echo the create environment.
    if (reply.status === 404) throw new Gone();
    if (!reply.ok) throw new ServiceRefusal();
    return (await reply.json()).data;
  }
  async function reclaim(id: string, processId?: string) {
    if (processId) {
      try {
        const result = await request(`/sandboxes/${id}/commands/${processId}`);
        if (result.stderr) error(scrub(String(result.stderr)));
      } catch {
        error("Battle diagnostics unavailable\n");
      }
    }
    // Shutdown is acknowledged before a failed start returns; removal is detached.
    let stopped = false;
    try {
      await request(`/sandboxes/${id}/stop`, "POST");
      stopped = true;
    } catch (cause) {
      if (cause instanceof Gone) return;
      void remove();
      throw cause;
    }
    void remove();
    /** A sandbox made without snapshots is erased by its stop and then answers
     *  "gone"; one that kept a disk is archived and must be deleted. The
     *  service's list can keep a row for either after it has gone. */
    async function remove() {
      const deadline = performance.now() + options.ttlSeconds * 1000 + 180_000;
      while (performance.now() < deadline) {
        try {
          const { sandbox } = await request(`/sandboxes/${id}`);
          if (!stopped && sandbox.state !== "archived") {
            await request(`/sandboxes/${id}/stop`, "POST");
            stopped = true;
          }
          if (sandbox.state === "archived") {
            await request(`/sandboxes/${id}`, "DELETE", undefined, {
              "X-Ascii-Confirm-Delete": id,
            });
            return;
          }
        } catch (cause) {
          if (cause instanceof Gone) return;
          /* The finite rental remains the final bound during a service outage. */
        }
        await delay(pollMs);
      }
      error("Battle machine deletion deadline exceeded\n");
    }
  }
  const { sandboxes } = await request(
    `/sandboxes?label=${encodeURIComponent("battlegame=battle")}`,
  );
  const swept = await Promise.allSettled(
    (sandboxes as Sandbox[])
      .filter((s) => s.labels.battlegame === labels.battlegame)
      .map((s) => reclaim(s.id)),
  );
  if (swept.some((result) => result.status === "rejected"))
    throw new Error("Battle machine cleanup failed");
  let running = 0;
  return {
    capacity: options.capacity,
    async start(key) {
      if (running >= options.capacity) throw new Error("battle capacity exhausted");
      running++;
      let id: string | undefined;
      let processId: string | undefined;
      let released = false;
      const release = () => {
        if (!released) {
          released = true;
          running--;
        }
      };
      const dispose = async () => {
        if (!id) return;
        try {
          await reclaim(id, processId);
        } catch {
          error("Battle machine stop failed; rental lease remains in force\n");
        }
      };
      const deadline = performance.now() + startupMs;
      const signal = AbortSignal.timeout(startupMs);
      const token = randomUUID();
      let uncertain = false;
      try {
        const body = {
          type: settings.machine,
          ttlSeconds: options.ttlSeconds,
          noEnv: true,
          visibility: "private",
          snapshots: false,
          labels: { ...labels, allocation: token },
          env: { BATTLE_KEY: key, HOST: "0.0.0.0", PORT: String(settings.port) },
        };
        // A lost reply can mean a successful allocation. Replay exactly the same request.
        for (;;) {
          try {
            const { sandbox } = await request(
              "/sandboxes",
              "POST",
              body,
              { "Idempotency-Key": token },
              signal,
            );
            id = sandbox.id;
            break;
          } catch (cause) {
            if (!(cause instanceof ServiceRefusal)) uncertain = true;
            if (cause instanceof ServiceRefusal || signal.aborted) throw cause;
            await delay(pollMs, undefined, { signal });
          }
        }
        while (true) {
          const { sandbox } = await request(`/sandboxes/${id}`, "GET", undefined, {}, signal);
          if (["ready", "idle", "running"].includes(sandbox.state)) break;
          await delay(pollMs, undefined, { signal });
        }
        const upload = await request(
          `/sandboxes/${id}/files/upload-url`,
          "POST",
          { path: "battle.tar.gz", ttlSeconds: Math.ceil(startupMs / 1000) },
          {},
          signal,
        );
        const sent = await fetch(upload.url, {
          method: upload.method,
          headers: { "Content-Length": String(archive.length) },
          body: archive,
          signal,
          redirect: "error",
        });
        if (!sent.ok) throw new ServiceRefusal();
        ({ processId } = await request(
          `/sandboxes/${id}/commands`,
          "POST",
          { command: "tar -xzf battle.tar.gz && exec node battle.mjs", detached: true },
          {},
          signal,
        ));
        const { url: address } = await request(
          `/sandboxes/${id}/host`,
          "POST",
          { port: settings.port, public: true, title: "Battle" },
          {},
          signal,
        );
        while (performance.now() < deadline) {
          try {
            const reply = await fetch(`${address}/control/ping`, {
              method: "POST",
              headers: { [CONTROL_HEADER]: key },
              signal: AbortSignal.any([signal, AbortSignal.timeout(1000)]),
              redirect: "error",
            });
            if (reply.ok)
              return {
                address,
                stop() {
                  if (released) return;
                  release();
                  void dispose();
                },
              };
          } catch {
            /* The command can be running before its listener opens. */
          }
          await delay(pollMs, undefined, { signal });
        }
        throw new Error("startup deadline");
      } catch {
        if (!id && uncertain) {
          try {
            const found = await request(
              `/sandboxes?label=${encodeURIComponent(`allocation=${token}`)}`,
            );
            const allocated = (found.sandboxes as Sandbox[]).filter(
              (s) => s.labels.battlegame === labels.battlegame,
            );
            await Promise.allSettled(allocated.map((s) => reclaim(s.id)));
          } catch {
            error("Battle allocation reconciliation unavailable; rental lease remains in force\n");
          }
        }
        await dispose();
        release();
        throw new Error("battle machine unavailable");
      }
    },
  };
}
class ServiceRefusal extends Error {}
/** The service no longer has what was asked for. */
class Gone extends ServiceRefusal {}
