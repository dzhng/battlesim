import { expect, test } from "vitest";
import { buildFacingPreview } from "@packages/battle-renderer/src/orderOverlay";
import { villageOrderStyle, villageStroke } from "@apps/battle-lab/src/villageOverlay";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";

test("the held destination marker keeps its anchor while its arrow follows facing", () => {
  const mark = { c: [100, 200] as const, r: 8, facing: 0 };
  const draw = (facing: number) =>
    buildFacingPreview({ ...mark, facing }, () => 0, villageOrderStyle, {
      stroke: villageStroke(0.05),
    });
  const extents = (mesh: Float32Array) => {
    const xs = [],
      ys = [];
    for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
      xs.push(mesh[i]);
      ys.push(mesh[i + 1]);
    }
    return {
      left: Math.min(...xs),
      right: Math.max(...xs),
      bottom: Math.min(...ys),
      top: Math.max(...ys),
    };
  };
  const east = extents(draw(0)),
    north = extents(draw(Math.PI / 2));
  expect(east.right - 100).toBeGreaterThan(100 - east.left);
  expect(north.top - 200).toBeGreaterThan(200 - north.bottom);
  expect(east.bottom).toBeCloseTo(north.bottom, 2);
  expect(east.left).toBeCloseTo(north.left, 2);
});

test("pointer paint clears the preview when the gesture ends", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint();
  const mark = { c: [100, 200] as const, r: 8, facing: 0 };
  paint.update(null, mark, () => 0, 0.05);
  const east = paint.feed.current;
  expect(east.length).toBeGreaterThan(0);
  paint.update(null, { ...mark, facing: Math.PI / 2 }, () => 0, 0.05);
  expect(paint.feed.current).not.toEqual(east);
  paint.update(null, null, () => 0, 0.05);
  expect(paint.feed.current.length).toBe(0);
});
