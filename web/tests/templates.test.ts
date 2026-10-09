// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, template_catalogue_json } from "@wasm/game_wasm.js";

const read = (name: string) =>
  readFileSync(new URL(`../../fixtures/parity/templates/${name}.json`, import.meta.url), "utf8");
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

test("native and wasm share one exact physical catalogue", () => {
  // The native half: `crates/mapgen/tests/compiler.rs` lowers to this record.
  const native = JSON.parse(read("native-asymmetric"));
  expect(JSON.parse(template_catalogue_json(`[${read("asymmetric")}]`))).toEqual(native.catalogue);
});

test("wasm rejects the frozen invalid joins and unrepresentable lattice", () => {
  const refusals = { vertical: /join faces/, gap: /join faces/, lattice: /bay lattice indices/ };
  for (const [name, refusal] of Object.entries(refusals))
    expect(() => template_catalogue_json(`[${read(`rejected-numeric/${name}`)}]`), name).toThrow(
      refusal,
    );
});
