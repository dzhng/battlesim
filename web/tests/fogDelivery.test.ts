// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { createSimClient } from "../src/battle/sim/client";
import { GroundView } from "../src/battle/sim/ground";
import { initSync, Battle } from "@wasm/game_wasm.js";
import {
  ObservationDecoder,
  type ObservationLayout,
  type ObservationView,
} from "../src/battle/sim/observation";

import { originalObservation } from "./groundRuns";

const oracle = JSON.parse(
  readFileSync(
    new URL("../../specs/city-maps/assets/fog-delivery/oracle.json", import.meta.url),
    "utf8",
  ),
);
const prepared = JSON.parse(
  readFileSync(
    new URL("../../specs/city-maps/assets/building-aggregate/cutover-inputs.json", import.meta.url),
    "utf8",
  ),
);
oracle.scenario.map = prepared.fogDelivery;
// C02 relocates only the input field; frozen outputs remain the original oracle.
oracle.scenario.map.fog_cell_m = oracle.scenario.rules.sensors.fog_cell_m;
delete oracle.scenario.rules.sensors.fog_cell_m;
let memory: WebAssembly.Memory;
beforeAll(() => {
  memory = initSync({
    module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)),
  }).memory;
});

test("the incremental stream preserves every frozen observation and prior frame", () => {
  const battle = new Battle(JSON.stringify(oracle.scenario), oracle.seed);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  const decoder = new ObservationDecoder(layout);
  const retained: ObservationView[] = [];
  try {
    for (const row of oracle.rows) {
      battle.step();
      if (row.resync) battle.resync_observation();
      const length = battle.publish(row.side);
      const frame = decoder.decode(
        new Float32Array(memory.buffer, battle.publication_ptr(), length),
      )!;
      expect(battle.digest(), `tick ${row.decoded.tick}`).toBe(row.digest);
      expect(originalObservation(frame, layout), `complete tick ${frame.tick}`).toEqual(
        row.decoded,
      );
      retained.push(frame);
    }
    retained.forEach((frame, i) =>
      expect(originalObservation(frame, layout), `retained tick ${frame.tick}`).toEqual(
        oracle.rows[i].decoded,
      ),
    );
  } finally {
    battle.free();
  }
});

/** Empty groups let the decoder's ordered stream contract be exercised with
 * deliberately small fields; each payload still uses the published layout. */
function packets() {
  const battle = new Battle(JSON.stringify(oracle.scenario), oracle.seed);
  const layout = JSON.parse(battle.observation_layout()) as ObservationLayout;
  battle.free();
  const record = (
    epoch: number,
    base: number,
    revision: number,
    full: boolean,
    payload: number[],
    nx = 9,
    ny = 5,
  ) => {
    const header: Record<string, number> = {
      tick: revision,
      fogCellM: 8,
      fogNx: nx,
      fogNy: ny,
      fogFloats: payload.length,
      fogFull: Number(full),
      fogBaseLo: base % 65536,
      fogBaseHi: Math.floor(base / 65536),
      fogRevisionLo: revision % 65536,
      fogRevisionHi: Math.floor(revision / 65536),
      groundEpoch: epoch,
      groundSide: 0,
      groundFull: Number(full && base === 0),
    };
    return new Float32Array([...layout.header.map((name) => header[name] ?? 0), ...payload]);
  };
  return { layout, record };
}

test("visibility loses bits, retains earlier frames, and replaces invalidated epochs", () => {
  const { layout, record } = packets();
  const decoder = new ObservationDecoder(layout);
  const first = decoder.decode(record(1, 0, 1, true, [65535, 65535, 8191, 0]))!;
  const second = decoder.decode(record(1, 1, 2, false, [0, 0, 0]))!;
  expect(Array.from(first.fog.bits)).toEqual([0xffffffff, 8191]);
  expect(Array.from(second.fog.bits)).toEqual([0, 8191]);
  const unchanged = decoder.decode(record(1, 2, 3, false, []))!;
  expect(unchanged.fog.bits).toBe(second.fog.bits);
  decoder.invalidate();
  expect(decoder.decode(record(1, 3, 4, false, [1, 0, 0]))).toBeNull();
  const replacement = decoder.decode(record(2, 0, 0x1000001, true, [1, 0, 0, 0], 8, 8))!;
  expect(Array.from(replacement.fog.bits)).toEqual([1, 0]);
  expect(Array.from(second.fog.bits)).toEqual([0, 8191]);
});

test("rejected fog records leave the delivered baseline intact", () => {
  const { layout, record } = packets();
  const decoder = new ObservationDecoder(layout);
  expect(() => decoder.decode(record(1, 0, 1, false, []))).toThrow(/baseline/);
  decoder.decode(record(1, 0, 1, true, [1, 0, 0, 0]));
  expect(() => decoder.decode(record(1, 2, 3, false, []))).toThrow(/baseline/);
  expect(() => decoder.decode(record(1, 1, 2, false, [2, 1, 0]))).toThrow(/indices/);
  expect(() => decoder.decode(record(1, 1, 2, true, [0, 0, 0, 1]))).toThrow(/padding/);
  expect(() => decoder.decode(record(1, 1, 2, true, [0, 0, 0, 0], 8, 8))).toThrow(/baseline/);
  expect(Array.from(decoder.decode(record(1, 1, 2, false, [1, 4096, 0]))!.fog.bits)).toEqual([
    1, 4096,
  ]);
});

test("fog and ground reject a broken paired cursor before either baseline advances", () => {
  const { layout, record } = packets();
  const decoder = new ObservationDecoder(layout);
  const ground = new GroundView(layout.ground);
  const paired = (
    base: number,
    revision: number,
    groundBase: number,
    groundRevision: number,
    full = false,
  ) => {
    const data = record(1, base, revision, base === 0, base === 0 ? [1, 0, 0, 0] : []);
    const put = (name: string, value: number) => {
      data[layout.header.indexOf(name)] = value;
    };
    put("groundBase", groundBase);
    put("groundRevision", groundRevision);
    put("groundFull", Number(full));
    return data;
  };
  const first = decoder.decode(paired(0, 1, 0, 1, true))!;
  ground.applyRuns(first.groundPatch);
  expect(() => decoder.decode(paired(1, 2, 99, 2))).toThrow(/ground.*baseline/);
  expect(() => decoder.decode(paired(1, 2, 1, 2, true))).toThrow(/ground.*baseline/);
  const corrected = decoder.decode(paired(1, 2, 1, 2))!;
  expect(ground.applyRuns(corrected.groundPatch)).toBe("applied");
  expect(ground.revision).toBe(2);
  const unchanged = decoder.decode(paired(2, 3, 2, 2))!;
  expect(ground.applyRuns(unchanged.groundPatch)).toBe("applied");
  expect(Array.from(first.fog.bits)).toEqual([1, 0]);
});

test("switching views before the first callback cannot expose an old-side observation", async () => {
  const client = createSimClient({
    scenario: JSON.stringify(oracle.scenario),
    seed: oracle.seed,
    side: "blue",
    transport: "direct",
  });
  const frames: ObservationView[] = [];
  try {
    await client.ready;
    client.onPublication((p) => {
      frames.push(p.observation);
      p.release();
    });
    client.pause();
    client.start();
    const advancing = client.advance(2);
    // The authority has already queued blue records, but no consumer callback has run.
    queueMicrotask(() => client.observeAs("red"));
    expect(await advancing).toBe(2);
    expect(await client.advance(1)).toBe(3);
    expect(frames.map((f) => [f.tick, f.groundPatch.side, f.groundPatch.epoch])).toEqual([
      [3, "red", 2],
    ]);
    expect(frames[0].own.map((u) => u.id)).toEqual([2, 3]);
  } finally {
    client.dispose();
  }
});

test("a view switch returns stale in-flight credits without mixing sides or blocking advance", async () => {
  const client = createSimClient({
    scenario: JSON.stringify(oracle.scenario),
    seed: oracle.seed,
    side: "blue",
    transport: "direct",
  });
  const frames: ObservationView[] = [];
  try {
    await client.ready;
    client.onPublication((p) => {
      frames.push(p.observation);
      if (frames.length === 1) client.observeAs("red");
      p.release();
    });
    client.pause();
    client.start();
    // Both pool credits leave before the first callback switches the view.
    expect(await client.advance(2)).toBe(2);
    expect(await client.advance(1)).toBe(3);
    expect(frames.map((f) => [f.tick, f.groundPatch.side, f.groundPatch.epoch])).toEqual([
      [1, "blue", 1],
      [3, "red", 2],
    ]);
    expect(frames[1].own.map((u) => u.id)).toEqual([2, 3]);
    const reference = new Battle(JSON.stringify(oracle.scenario), oracle.seed);
    try {
      expect(originalObservation(frames[0], JSON.parse(reference.observation_layout()))).toEqual(
        oracle.rows[0].decoded,
      );
    } finally {
      reference.free();
    }
  } finally {
    client.dispose();
  }
});

test("oversized field dimensions cannot wrap past the wasm delivery bound", () => {
  const { layout, record } = packets();
  expect(() =>
    new ObservationDecoder(layout).decode(record(1, 0, 1, true, [], 65536, 65536)),
  ).toThrow(/admitted/);
});
