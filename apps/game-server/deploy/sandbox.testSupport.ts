// Local service stand-in: real HTTP, uploads, shell command and Node battle process.
import { spawn, type ChildProcess } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { once } from "node:events";
import { sandboxBattleMachines, type SandboxOptions } from "./sandboxes.ts";
import settings from "./settings.json";

type Sandbox = {
  id: string;
  state: string;
  labels: Record<string, string>;
  env: Record<string, string>;
  folder: string;
  port: number;
  child?: ChildProcess;
  stderr: string;
  exitCode: number | null;
  /** Made with snapshots: a stop archives it instead of erasing it. */
  disk: boolean;
};
export async function serviceStandIn(archive: string) {
  const root = await mkdtemp(join(tmpdir(), "battle-service-"));
  const machines = new Map<string, Sandbox>();
  const seen: unknown[] = [];
  const output: string[] = [];
  const calls: string[] = [];
  let refusal = 0;
  let neverReady = false;
  let lostCreateReply = false;
  const creations = new Map<string, Sandbox>();
  async function add(
    labels: Record<string, string>,
    env: Record<string, string> = {},
    disk = false,
  ) {
    const id = `machine-${machines.size}`;
    const folder = join(root, id);
    await mkdir(folder);
    const probe = createServer();
    probe.listen(0, "127.0.0.1");
    await once(probe, "listening");
    const port = (probe.address() as { port: number }).port;
    await new Promise<void>((resolve) => probe.close(() => resolve()));
    const sandbox: Sandbox = {
      id,
      state: "ready",
      labels,
      env,
      folder,
      port,
      stderr: "",
      exitCode: null,
      disk,
    };
    machines.set(id, sandbox);
    return sandbox;
  }
  const server = createServer(async (request, response) => {
    try {
      const url = new URL(request.url!, address);
      const chunks: Buffer[] = [];
      for await (const chunk of request) chunks.push(Buffer.from(chunk));
      const bytes = Buffer.concat(chunks);
      const body =
        request.headers["content-type"] === "application/json" && bytes.length
          ? JSON.parse(bytes.toString())
          : {};
      const path = url.pathname.replace("/v1/services/boat", "");
      calls.push(`${request.method} ${path}`);
      // The one allowed credential channel (create.env) is deliberately excluded.
      seen.push(request.url, body.command, body.labels);
      const send = (data: unknown, status = 200) => {
        response.writeHead(status, { "Content-Type": "application/json" });
        response.end(JSON.stringify({ data, bento: { operationId: "operation" } }));
      };
      if (path.startsWith("/upload/")) {
        if (
          request.headers.authorization ||
          Number(request.headers["content-length"]) !== bytes.length
        )
          return send({}, 403);
        await writeFile(join(machines.get(path.split("/")[2])!.folder, "battle.tar"), bytes);
        return send({});
      }
      if (request.headers.authorization !== "Bearer stand-in-app-key") return send({}, 403);
      if (path === "/sandboxes" && request.method === "POST") {
        if (refusal) return send({}, refusal);
        if (
          !body.noEnv ||
          body.visibility !== "private" ||
          body.snapshots !== false ||
          body.type !== settings.machine ||
          !body.ttlSeconds
        )
          return send({}, 400);
        const token = String(request.headers["idempotency-key"]);
        const sandbox = creations.get(token) ?? (await add(body.labels, body.env));
        creations.set(token, sandbox);
        if (lostCreateReply) {
          lostCreateReply = false;
          response.destroy();
          return;
        }
        return send({ sandbox });
      }
      if (path === "/sandboxes") {
        const [key, value] = url.searchParams.get("label")!.split("=");
        return send({
          sandboxes: [...machines.values()]
            .filter((s) => s.labels[key] === value)
            // The list keeps a row, in its last live state, for a machine that has gone.
            .map(({ id, labels, state }) => ({
              id,
              labels,
              state: state === "deleted" ? "idle" : state,
            })),
        });
      }
      const [, , id, action] = path.split("/");
      const sandbox = machines.get(id)!;
      if (sandbox.state === "deleted") return send({}, 404);
      if (request.method === "DELETE") {
        if (sandbox.state !== "archived" || request.headers["x-ascii-confirm-delete"] !== id)
          return send({}, 409);
        sandbox.state = "deleted";
        return send({});
      }
      if (!action)
        return send({
          sandbox: { state: neverReady && sandbox.state === "ready" ? "starting" : sandbox.state },
        });
      if (action === "files") return send({ method: "PUT", url: `${address}/upload/${id}` });
      if (action === "host") return send({ url: `http://127.0.0.1:${sandbox.port}` });
      if (action === "commands" && request.method === "POST") {
        const child = spawn("/bin/sh", ["-c", body.command], {
          cwd: sandbox.folder,
          env: { PATH: process.env.PATH, ...sandbox.env, PORT: String(sandbox.port) },
          stdio: ["ignore", "pipe", "pipe"],
        });
        sandbox.child = child;
        child.stderr!.on("data", (b) => {
          sandbox.stderr += b;
        });
        child.once("exit", (code) => {
          sandbox.exitCode = code;
        });
        return send({ processId: "process" });
      }
      if (action === "commands")
        return send({
          stdout: "",
          stderr: sandbox.stderr,
          running: sandbox.child?.exitCode === null,
          exitCode: sandbox.exitCode,
        });
      if (action === "stop") {
        sandbox.child?.kill("SIGTERM");
        sandbox.state = "stopping";
        // Without snapshots, as every battle machine is made, a stop erases it.
        setTimeout(() => {
          sandbox.state = sandbox.disk ? "archived" : "deleted";
        }, 50);
        return send({});
      }
      send({}, 404);
    } catch {
      response.writeHead(500);
      response.end();
    }
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = `http://127.0.0.1:${(server.address() as { port: number }).port}`;
  return {
    appCredentialEscaped: () => JSON.stringify([seen, output]).includes("stand-in-app-key"),
    machines,
    seen,
    output,
    calls,
    finish: () => {
      for (const s of machines.values()) s.child?.kill("SIGUSR2");
    },
    diagnostic: (text: string) => {
      for (const s of machines.values()) s.stderr += `${text}:stand-in-app-key`;
    },
    refuse: (status: number) => {
      refusal = status;
    },
    stall: () => {
      neverReady = true;
    },
    loseCreateReply: () => {
      lostCreateReply = true;
    },
    leftover: (ours: boolean, disk = false) =>
      add(ours ? { battlegame: "battle" } : { other: "work" }, {}, disk),
    async lobby(folder: string) {
      const child = spawn("node", [join(folder, "lobby.mjs")], {
        cwd: root,
        env: {
          PATH: process.env.PATH,
          PORT: "0",
          BENTO_API_URL: address,
          BENTO_APP_API_KEY: "stand-in-app-key",
        },
        stdio: ["ignore", "pipe", "pipe"],
      });
      let log = "";
      child.stdout.on("data", (b) => {
        log += b;
      });
      child.stderr.on("data", (b) => {
        log += b;
        output.push(String(b));
      });
      const deadline = performance.now() + 15_000;
      while (!/Game server: http/.test(log)) {
        if (child.exitCode !== null || performance.now() >= deadline) {
          child.kill("SIGKILL");
          throw new Error("Hosted lobby did not start");
        }
        await new Promise((resolve) => setTimeout(resolve, 10));
      }
      return {
        address: /Game server: (http:\/\/127\.0\.0\.1:\d+)/.exec(log)![1],
        async close() {
          const exit = once(child, "exit");
          child.kill("SIGTERM");
          const force = setTimeout(() => child.kill("SIGKILL"), 5000);
          await exit;
          clearTimeout(force);
        },
      };
    },
    async provider(overrides: Partial<SandboxOptions> = {}) {
      return sandboxBattleMachines({
        apiUrl: address,
        appKey: "stand-in-app-key",
        archive,
        capacity: settings.capacity,
        ttlSeconds: settings.ttlSeconds,
        pollMs: 10,
        startupMs: 15_000,
        error: (text) => output.push(text),
        ...overrides,
      });
    },
    async close() {
      for (const s of machines.values()) {
        if (s.child && s.child.exitCode === null && s.child.signalCode === null) {
          const exited = once(s.child, "exit");
          s.child.kill("SIGKILL");
          await exited;
        }
      }
      server.closeAllConnections();
      await new Promise<void>((resolve) => server.close(() => resolve()));
      await rm(root, { recursive: true, force: true });
    },
  };
}
