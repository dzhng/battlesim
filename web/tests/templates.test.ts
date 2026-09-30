// @vitest-environment node
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { initSync, template_catalogue_json, materialize_template } from "@wasm/game_wasm.js";

const read = (name: string) =>
  readFileSync(
    new URL(`../../specs/city-maps/assets/template-geometry/${name}.json`, import.meta.url),
    "utf8",
  );
beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

test("native and wasm share exact physical catalogue and materialization records", () => {
  for (const name of ["asymmetric", "rotated", "precision"]) {
    const input = read(name);
    const expected = JSON.parse(read(`native-${name}`));
    expect(JSON.parse(template_catalogue_json(`[${input}]`)), `${name} identity`).toEqual(
      expected.catalogue,
    );
    expect(
      JSON.parse(materialize_template(input, JSON.stringify(expected.materialized.frame))),
      `${name} geometry`,
    ).toEqual(expected.materialized);
  }
});

test("wasm rejects the frozen invalid joins and unrepresentable lattice", () => {
  for (const name of ["vertical", "gap", "lattice"]) {
    const input = read(`rejected-numeric/${name}`);
    const frame = JSON.parse(read(`rejected-numeric/native-${name}`)).materialized.frame;
    expect(() => template_catalogue_json(`[${input}]`), name).toThrow();
    expect(() => materialize_template(input, JSON.stringify(frame)), name).toThrow();
  }
});

test("canonical rotations preserve every native value across the finite angle corpus", () => {
  const descriptor = read("asymmetric");
  const records: { frame_json: string; native_report: string }[] = JSON.parse(
    read("runtime-rotation/paired-records"),
  ).cases;
  for (const record of records) {
    expect(
      JSON.parse(materialize_template(descriptor, record.frame_json)),
      record.frame_json,
    ).toEqual(JSON.parse(record.native_report).materialized);
  }
});
