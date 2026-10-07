// @vitest-environment jsdom
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import * as wasm from "@wasm/game_wasm.js";
import { TEST_RULES } from "./catalog";
import { useStaticWorld } from "@apps/battle-lab/src/useStaticWorld";

vi.mock("@web/battle/sim/module", () => ({ loadWasm: async () => wasm }));
beforeAll(() => {
  wasm.initSync({ module: readFileSync(join(process.cwd(), "src/wasm/game_wasm_bg.wasm")) });
});
afterEach(cleanup);

test("the page's physical world uses its scenario's captured rules", async () => {
  const rules = structuredClone(TEST_RULES);
  const props = (rules.catalog as { props?: Record<string, unknown> }[]).find(
    (doc) => doc.props && "tooth" in doc.props,
  )!.props!;
  props["replay-only-body"] = structuredClone(props.tooth);
  const map = {
    size: [100, 100],
    height_grid_m: 10,
    fog_cell_m: 8,
    slope_cutoff_deg: 35,
    relief: [],
    bridges: [],
    props: [
      { id: 0, kind: "replay-only-body", center: [50, 50], yaw: 0, half_extents: [0.6, 0.6, 0.6] },
    ],
    surfaces: [],
    forests: [],
    buildings: [],
  };
  const { result } = renderHook(() => useStaticWorld(map, rules));
  await waitFor(() => expect(result.current?.layout.propKinds).toContain("replay-only-body"));
  expect(result.current!.view.raycast(50, 50, 20, 0, 0, -1, 30)[7]).toBe(0);
});
