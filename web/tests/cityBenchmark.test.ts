// @vitest-environment node
import { readFileSync } from "node:fs";
import { expect, test, vi } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { CITY_CONTACT, VILLAGE_CONTACT } from "@web/battle/benchmark/presets";
import { prepareBenchmark } from "@apps/battle-lab/src/benchmark/prepare";
import { createBenchmarkRun } from "@apps/battle-lab/src/benchmark/run";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { prepare } from "@web/battle/prepare/prepare";
import type { PrepareMessage } from "@web/battle/prepare/protocol";
import type { BenchmarkReport } from "@web/battle/benchmark/report";
import { loadEncounter, loadMap } from "@web/maps/node";

const memory = wasm.initSync({
  module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
}).memory;

test("city-contact prepares the pinned full generated world and the existing central contact script, unlike the village control", async () => {
  let asked: PrepareMessage | null = null;
  class Worker {
    onmessage: ((e: { data: unknown }) => void) | null = null;
    onerror: ((e: { message: string }) => void) | null = null;
    terminate() {}
    postMessage(message: PrepareMessage) {
      asked = message;
      void prepare(
        wasm,
        memory,
        message.request,
        message.documents,
        { loadMap, loadEncounter },
        undefined,
        undefined,
        message.stress,
      ).then(
        ({ world, ...battle }) => {
          world.free();
          this.onmessage?.({ data: { type: "prepared", battle } });
        },
        (error: Error) => this.onerror?.({ message: error.message }),
      );
    }
  }
  vi.stubGlobal("Worker", Worker);
  try {
    const built = await prepareBenchmark(wasm, CITY_CONTACT, new AbortController().signal);
    const report = built.prepared!.report;
    expect(asked!.request).toEqual({
      map_source: {
        kind: "generated",
        request: expect.objectContaining({ type: "metro", size: "large", seed: "4" }),
      },
      recipe_id: "assault",
      encounter_seed: "1",
      battle_seed: 4,
    });
    expect(asked!.stress).toEqual({ kind: "city-arena-2", late: false });
    expect(report.request).toEqual(asked!.request);
    expect(report.identity.kind).toBe("generated");
    expect(report.stress).toEqual({
      kind: "city-arena-2",
      late: false,
      livingUnits: { blue: 100, red: 100 },
    });
    const city = JSON.parse(built.scenario);
    const village = JSON.parse(
      wasm.village_scenario(
        JSON.stringify({ ...GAME_RULES, map: loadMap("village").definition }),
        VILLAGE_CONTACT.variant,
      ),
    );
    expect(city.units[4].position[0]).toBeLessThan(report.size[0] / 2 - 1000);
    expect(city.units[104].position[0]).toBeGreaterThan(report.size[0] / 2 + 1000);
    expect(city.map.size).toEqual(report.size);
    expect(city.map.size[0]).toBeGreaterThan(village.map.size[0]);
    expect(city.map.buildings.length).toBeGreaterThan(village.map.buildings.length);
    expect(city.units.filter((u: { side: string }) => u.side === "blue")).toHaveLength(100);
    expect(
      city.scripts.some(
        (s: { tick: number; order: { kind: string } }) =>
          s.tick === 150 && s.order.kind === "attack_move",
      ),
    ).toBe(true);
    expect(built.workload.tour.keyframes[0].slice(1, 3)).toEqual(report.size.map((n) => n / 2));
    const reports: BenchmarkReport[] = [];
    const run = createBenchmarkRun(built.workload, "short", 30, (r) => reports.push(r), report);
    expect(run.scripted.script).toBeUndefined();
    expect(run.scripted.warmTo).toBe(150);
    run.cancel();
    expect(reports[0].scenario.fingerprint).toBe("fe532f0e");
    expect(reports[0].tour).toEqual(built.workload.tour);
    expect(reports[0].preparation).toEqual(report);
  } finally {
    vi.unstubAllGlobals();
  }
}, 60_000);

test("leaving city benchmark preparation closes its worker and a late answer cannot start a run", async () => {
  const workers: Worker[] = [];
  let stopped = false;
  class Worker {
    onmessage: ((e: { data: unknown }) => void) | null = null;
    onerror = null;
    constructor() {
      workers.push(this);
    }
    terminate() {
      stopped = true;
    }
    postMessage() {}
  }
  vi.stubGlobal("Worker", Worker);
  try {
    const controller = new AbortController();
    let settled = false;
    void prepareBenchmark(wasm, CITY_CONTACT, controller.signal).then(() => (settled = true));
    controller.abort();
    expect(stopped).toBe(true);
    workers[0].onmessage?.({ data: { type: "prepared", battle: { scenario: "{}", report: {} } } });
    await Promise.resolve();
    expect(settled).toBe(false);
  } finally {
    vi.unstubAllGlobals();
  }
});
