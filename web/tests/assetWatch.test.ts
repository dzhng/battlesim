// @vitest-environment node
import { EventEmitter } from "node:events";
import { PassThrough } from "node:stream";
import { afterEach, expect, test, vi } from "vitest";
import type { Plugin, ViteDevServer } from "vite";
import config from "../vite.config";

const processState = vi.hoisted(() => ({ spawn: vi.fn() }));
vi.mock("node:child_process", () => ({ spawn: processState.spawn }));
afterEach(() => {
  vi.useRealTimers();
  vi.resetAllMocks();
});

test("asset edits share one bake writer and publish only the latest completed bake", async () => {
  vi.useFakeTimers();
  const runs: (EventEmitter & { stdout: PassThrough; stderr: PassThrough; kill: () => void })[] =
    [];
  processState.spawn.mockImplementation(() => {
    const run = Object.assign(new EventEmitter(), {
      stdout: new PassThrough(),
      stderr: new PassThrough(),
      kill: vi.fn(),
    });
    runs.push(run);
    return run;
  });
  const watcher = new EventEmitter() as EventEmitter & { add: () => void };
  watcher.add = () => {};
  const server = new EventEmitter();
  const messages: unknown[] = [];
  const plugin = (config.plugins as Plugin[]).find((p) => p.name === "asset-watch")!;
  const hook = plugin.configureServer!;
  const configure = typeof hook === "function" ? hook : hook.handler;
  configure.call(
    {} as ThisParameterType<typeof configure>,
    {
      watcher,
      httpServer: server,
      ws: { send: (value: unknown) => messages.push(value) },
    } as unknown as ViteDevServer,
  );
  const catalog = new URL("../../assets/catalog.json", import.meta.url).pathname;
  watcher.emit("change", catalog);
  await vi.advanceTimersByTimeAsync(200);
  watcher.emit("change", catalog);
  await vi.advanceTimersByTimeAsync(200);
  expect(runs).toHaveLength(1);
  runs[0].emit("close", 0);
  expect(messages).toEqual([]);
  expect(runs).toHaveLength(2);
  runs[1].stdout.write("latest bake");
  runs[1].emit("close", 0);
  expect(messages).toEqual([
    { type: "custom", event: "assets:rebaked", data: { ok: true, output: "latest bake" } },
  ]);
  server.emit("close");
});
