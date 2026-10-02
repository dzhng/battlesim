// @vitest-environment node
import { expect, test } from "vitest";
import { loadMap, shipped } from "@web/maps/node";
import { compile_map } from "@web/wasm/game_wasm";

// A catalogue cutover preserves the compiled arena and its original PropIds.
test("the camera catalogue map preserves the compiled arena and original PropIds", () => {
  const resolved = loadMap("camera-lab");
  const expected = JSON.parse(
    compile_map(shipped.document("camera-lab", "request.json"), shipped.library("camera-lab")),
  );
  expect(expected.status).toBe("ok");
  expect(resolved.definition).toEqual(expected.result.map);
});
