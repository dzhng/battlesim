// @vitest-environment node
import { expect, test } from "vitest";
import { loadMap, shipped } from "@web/maps/node";
import { compile_map } from "@web/wasm/game_wasm";

// A catalogue cutover preserves the compiled arena and its original PropIds.
test("the camera catalogue map preserves the compiled arena and original PropIds", () => {
  const resolved = loadMap("camera-lab");
  // The map's request pins the templates its sources select from the library.
  const { template_ids: ids } = JSON.parse(
    shipped.document("camera-lab", "SOURCES.json"),
  ).catalogue;
  const selected = JSON.parse(shipped.library("camera-lab")).filter((t: { id: string }) =>
    ids.includes(t.id),
  );
  const expected = JSON.parse(
    compile_map(shipped.document("camera-lab", "request.json"), JSON.stringify(selected)),
  );
  expect(expected.status).toBe("ok");
  expect(resolved.definition).toEqual(expected.result.map);
});
