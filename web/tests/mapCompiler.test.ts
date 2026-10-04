// @vitest-environment node
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { beforeAll, expect, test } from "vitest";
import { GAME_RULES } from "@apps/battle-lab/src/scenarios";
import { initSync, compile_map, WorldView, world_layout } from "@wasm/game_wasm.js";
import type { WorldLayout } from "@packages/battle-renderer/src/worldMesh";

beforeAll(() => {
  initSync({ module: readFileSync(new URL("../src/wasm/game_wasm_bg.wasm", import.meta.url)) });
});

test("the compiled artifact reaches the existing public world without a plan interpreter", () => {
  const request = readFileSync(
    new URL("../../fixtures/parity/map-compiler/request.json", import.meta.url),
    "utf8",
  );
  const descriptor = readFileSync(
    new URL("../../fixtures/parity/templates/asymmetric.json", import.meta.url),
    "utf8",
  );
  const original = JSON.parse(
    readFileSync(
      new URL("../../fixtures/parity/templates/native-asymmetric.json", import.meta.url),
      "utf8",
    ),
  );
  const compiled = JSON.parse(compile_map(request, `[${descriptor}]`)).result;
  const rules = JSON.stringify(GAME_RULES);
  const world = new WorldView(JSON.stringify(compiled.map), rules);
  try {
    const layout = JSON.parse(world_layout(rules)) as WorldLayout;
    const expected = original.materialized.parts.flatMap(
      (
        part: {
          center: [number, number];
          yaw: number;
          half_extents: [number, number, number];
          base_z: number;
        },
        id: number,
      ) => [
        id,
        0,
        layout.propKinds.indexOf("building"),
        ...part.center,
        part.yaw,
        ...part.half_extents,
        part.base_z,
      ],
    );
    expected.push(
      2,
      0,
      layout.propKinds.indexOf("tooth"),
      40.000000000168804,
      100,
      0,
      0.6,
      0.6,
      0.6,
      0,
    );
    expect(Array.from(world.props())).toEqual(expected.map(Math.fround));
    expect(JSON.parse(world.buildings())).toEqual({
      catalogueHash: compiled.identity.template_catalog_hash,
      regionalFamily: "api_fixture",
      buildings: [
        {
          owner: 0,
          kind: "building",
          templateId: "asymmetric-api-compound",
          category: "attached_home",
          regionalFamily: "api_fixture",
          frame: original.materialized.frame,
          parts: [
            { part: "main", prop: 0 },
            { part: "wing", prop: 1 },
          ],
        },
      ],
    });
    const wing = original.materialized.parts[1];
    expect(world.raycast(wing.center[0], wing.center[1], 20, 0, 0, -1, 30)[7]).toBe(1);
  } finally {
    world.free();
  }
});

test("implicit ground height cannot turn a finite input into an infinite physical box", () => {
  const request = readFileSync(
    new URL("../../fixtures/parity/map-compiler/request.json", import.meta.url),
    "utf8",
  ).replace("[0.6, 0.6, 0.6]", "[0.6, 0.6, 1e308]");
  const descriptor = readFileSync(
    new URL("../../fixtures/parity/templates/asymmetric.json", import.meta.url),
    "utf8",
  );
  expect(JSON.parse(compile_map(request, `[${descriptor}]`))).toEqual({
    status: "error",
    diagnostics: [
      {
        code: "invalid_bounds",
        feature: null,
        location: "$.plan.authored_parts[2]",
        message: "physical box requires finite positive extents and a finite pose",
      },
    ],
  });
});

test("native CLI and WASM emit the same complete compiler records and refusals", () => {
  const cases: { name: string; request_json: string; native_sha256: string }[] = JSON.parse(
    readFileSync(
      new URL("../../fixtures/parity/map-compiler/paired-records.json", import.meta.url),
      "utf8",
    ),
  ).cases;
  const descriptor = readFileSync(
    new URL("../../fixtures/parity/templates/asymmetric.json", import.meta.url),
    "utf8",
  );
  for (const record of cases) {
    const outcome = compile_map(record.request_json, `[${descriptor}]`);
    expect(createHash("sha256").update(outcome).digest("hex"), record.name).toBe(
      record.native_sha256,
    );
  }
});
