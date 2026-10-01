// @vitest-environment node
import { readFileSync } from "node:fs";
import { VILLAGE_RULES } from "@apps/battle-lab/src/scenarios";
import { beforeAll, expect, test } from "vitest";
import {
  initSync,
  template_catalogue_json,
  materialize_template,
  WorldView,
} from "@wasm/game_wasm.js";

const read = (name: string) =>
  readFileSync(
    new URL(`../../fixtures/parity/templates/${name}.json`, import.meta.url),
    "utf8",
  );
/** Only C01's added owner-emitted local interval enriches the old output oracle. */
function withLocalSpans(materialized: { edges: { id: string }[] }, descriptorJson: string) {
  const source = JSON.parse(descriptorJson).edges as { id: string; span_m: [number, number] }[];
  return {
    ...materialized,
    edges: materialized.edges.map((edge) => ({
      ...edge,
      span_m: source.find((s) => s.id === edge.id)!.span_m,
    })),
  };
}
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
    ).toEqual(withLocalSpans(expected.materialized, input));
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
    ).toEqual(withLocalSpans(JSON.parse(record.native_report).materialized, descriptor));
  }
});

test("materialization and the public loader admit the same representable part pose", () => {
  const descriptor = JSON.parse(
    readFileSync(new URL("../../fixtures/building-templates.json", import.meta.url), "utf8"),
  ).templates[0];
  descriptor.parts[0].yaw = 0.4;
  const hash = JSON.parse(template_catalogue_json(JSON.stringify([descriptor]))).hash;
  const materialize = (yaw: number) =>
    JSON.parse(
      materialize_template(
        JSON.stringify(descriptor),
        JSON.stringify({ translation: [50, 50, 0], yaw }),
      ),
    );
  const geometry = materialize(0.7);
  const world = new WorldView(
    JSON.stringify({
      size: [128, 128],
      fog_cell_m: 8,
      height_grid_m: 4,
      slope_cutoff_deg: 35,
      template_catalog_hash: hash,
      buildings: [
        {
          owner: 0,
          kind: "building",
          category: descriptor.category,
          regional_family: descriptor.regional_family,
          parts: [{ part: "body", prop: 0 }],
          geometry,
        },
      ],
    }),
    JSON.stringify(VILLAGE_RULES),
  );
  try {
    expect(world.props()[5]).toBe(Math.fround(1.1));
  } finally {
    world.free();
  }
  expect(() => materialize(1e20)).toThrow(/cannot represent physical geometry/);
});
