// @vitest-environment node
import { readFileSync } from "node:fs";
import { TEST_RULES } from "./catalog";
import { beforeAll, expect, test } from "vitest";
import {
  initSync,
  template_catalogue_json,
  materialize_template,
  WorldView,
} from "@wasm/game_wasm.js";

const read = (name: string) =>
  readFileSync(new URL(`../../fixtures/parity/templates/${name}.json`, import.meta.url), "utf8");
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

test("native and wasm share one exact physical catalogue and materialization record", () => {
  const input = read("asymmetric");
  // The native half: `crates/mapgen/tests/compiler.rs` lowers to this record.
  const native = JSON.parse(read("native-asymmetric"));
  expect(JSON.parse(template_catalogue_json(`[${input}]`))).toEqual(native.catalogue);
  expect(
    JSON.parse(materialize_template(input, JSON.stringify(native.materialized.frame))),
  ).toEqual(native.materialized);
});

test("wasm rejects the frozen invalid joins and unrepresentable lattice", () => {
  // A common offset this large once hid the bad join inside the comparison tolerance.
  const frames = { vertical: -1e15, gap: -1e15, lattice: 0 };
  for (const [name, x] of Object.entries(frames)) {
    const input = read(`rejected-numeric/${name}`);
    const frame = { translation: [x, 0, 0], yaw: 0 };
    expect(() => template_catalogue_json(`[${input}]`), name).toThrow();
    expect(() => materialize_template(input, JSON.stringify(frame)), name).toThrow();
  }
});
