import { expect, test } from "vitest";
import { buildDestinationPreview } from "@packages/battle-renderer/src/orderOverlay";
import { villageOrderStyle, villageStroke } from "@apps/battle-lab/src/villageOverlay";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";

test("the held destination marker keeps its anchor while its arrow follows facing", () => {
  const mark = { c: [100, 200] as const, r: 8, facing: 0, placed: true, opacity: 1 };
  const draw = (facing: number) =>
    buildDestinationPreview([{ ...mark, facing }], () => 0, villageOrderStyle, {
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
  const mark = { unit: 1, placed: true, opacity: 1, c: [100, 200] as const, r: 8, facing: 0 };
  paint.update(null, [mark], () => 0, 0.05);
  const east = paint.feed.current;
  expect(east.length).toBeGreaterThan(0);
  paint.update(null, [{ ...mark, facing: Math.PI / 2 }], () => 0, 0.05);
  expect(paint.feed.current).not.toEqual(east);
  paint.update(null, [], () => 0, 0.05);
  expect(paint.feed.current.length).toBe(0);
});

test("placement queries coalesce cursor updates and a cancelled reply cannot restore markers", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint();
  const selected = [
    { id: 1, kind: "tank", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const requested: import("../src/battle/sim/protocol").MovePreviewRequest[] = [];
  let finish!: (marks: import("../src/battle/sim/protocol").MoveDestination[]) => void;
  const client = {
    previewMove: (move: import("../src/battle/sim/protocol").MovePreviewRequest) => {
      requested.push(move);
      return new Promise<import("../src/battle/sim/protocol").MoveDestination[]>((resolve) => {
        finish = resolve;
      });
    },
  };
  const move = { units: [1], goal: [100, 200] as [number, number], facing: 0 };
  paint.resolveMove(move, selected, client, 0);
  paint.resolveMove({ ...move, facing: 1 }, selected, client, 0);
  paint.resolveMove({ ...move, facing: 2 }, selected, client, 0);
  expect(requested).toEqual([move]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 0 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(paint.resolveMove({ ...move, facing: 2 }, selected, client, 0)).toMatchObject([
    { unit: 1, c: move.goal, facing: 0 },
  ]);
  expect(requested).toEqual([move, { ...move, facing: 2 }]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 2 }]);
  await new Promise((r) => setTimeout(r, 0));
  const markers = paint.resolveMove({ ...move, facing: 2 }, selected, client, 0);
  expect(markers).toMatchObject([{ unit: 1, placed: true, opacity: 1, c: [100, 200], facing: 2 }]);
  paint.resolveMove({ ...move, facing: 3 }, selected, client, 0);
  expect(paint.resolveMove(null, selected, client, 0)).toEqual([]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 3 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(paint.resolveMove(null, selected, client, 0)).toEqual([]);
});

test("an unplaced intention uses the cannot-place color and the confirmation opacity", () => {
  const style = {
    ...villageOrderStyle,
    color: [1, 0, 0, 1] as const,
    blocked: [0, 1, 0, 1] as const,
    glow: { order: 1, selected: 1 },
  };
  const mesh = buildDestinationPreview(
    [{ c: [100, 200], r: 8, facing: 0, placed: false, opacity: 0.5 }],
    () => 0,
    style,
    { stroke: villageStroke(0.05) },
  );
  expect(mesh.length).toBeGreaterThan(0);
  for (let i = 0; i < mesh.length; i += VERTEX_FLOATS) {
    expect([...mesh.slice(i + 6, i + 10)]).toEqual([0, 1, 0, 0.5]);
  }
});

test("a cancelled query cannot paint a new press at the same anchor", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint();
  const own = [
    { id: 1, kind: "tank", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  let finish!: (marks: import("../src/battle/sim/protocol").MoveDestination[]) => void;
  const client = {
    previewMove: () =>
      new Promise<import("../src/battle/sim/protocol").MoveDestination[]>((resolve) => {
        finish = resolve;
      }),
  };
  const move = { units: [1], goal: [100, 200] as [number, number], facing: 1 };
  paint.resolveMove(move, own, client, 0);
  paint.resolveMove(null, own, client, 0);
  paint.resolveMove({ ...move, facing: 0 }, own, client, 0);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 1 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(paint.resolveMove({ ...move, facing: 0 }, own, client, 0)).toEqual([]);
});
