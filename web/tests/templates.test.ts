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
