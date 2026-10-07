import { expect, test } from "vitest";
import { UNITS } from "./catalog";
import { buildDestinationPreview } from "@packages/battle-renderer/src/orderOverlay";
import { gameOrderStyle, gameStroke } from "@apps/battle-lab/src/gameOverlay";
import { VERTEX_FLOATS } from "@packages/battle-renderer/src/mesh";
import { GroundView } from "../src/battle/sim/ground";
import { cellPatchRuns } from "./groundRuns";

test("the held destination marker keeps its anchor while its arrow follows facing", () => {
  const mark = { c: [100, 200] as const, r: 8, facing: 0, placed: true, opacity: 1 };
  const draw = (facing: number) =>
    buildDestinationPreview([{ ...mark, facing }], () => 0, gameOrderStyle, {
      stroke: gameStroke(0.05),
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
  const paint = new PointerPaint(UNITS);
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
  const paint = new PointerPaint(UNITS);
  const selected = [
    { id: 1, kind: "test_tank", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const requested: import("../src/battle/sim/protocol").MovePreviewRequest[] = [];
  let finish!: (marks: import("../src/battle/sim/protocol").MoveDestination[]) => void;
  const client = {
    previewBuilding: async () => {
      throw new Error("unexpected building preview");
    },
    previewMove: (move: import("../src/battle/sim/protocol").MovePreviewRequest) => {
      requested.push(move);
      return new Promise<import("../src/battle/sim/protocol").MoveDestination[]>((resolve) => {
        finish = resolve;
      });
    },
  };
  const move = { units: [1], goal: [100, 200] as [number, number], facing: 0 };
  paint.resolvePreview({ kind: "move", request: move }, selected, client, 0);
  paint.resolvePreview({ kind: "move", request: { ...move, facing: 1 } }, selected, client, 0);
  paint.resolvePreview({ kind: "move", request: { ...move, facing: 2 } }, selected, client, 0);
  expect(requested).toEqual([move]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 0 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(
    paint.resolvePreview({ kind: "move", request: { ...move, facing: 2 } }, selected, client, 0),
  ).toEqual([]);
  expect(requested).toEqual([move, { ...move, facing: 2 }]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 2 }]);
  await new Promise((r) => setTimeout(r, 0));
  const markers = paint.resolvePreview(
    { kind: "move", request: { ...move, facing: 2 } },
    selected,
    client,
    0,
  );
  expect(markers).toMatchObject([{ unit: 1, placed: true, opacity: 1, c: [100, 200], facing: 2 }]);
  paint.resolvePreview({ kind: "move", request: { ...move, facing: 3 } }, selected, client, 0);
  expect(paint.resolvePreview(null, selected, client, 0)).toEqual([]);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 3 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(paint.resolvePreview(null, selected, client, 0)).toEqual([]);
});

test("a rejected destination paints no destination marker", () => {
  const accepted = { c: [100, 200] as const, r: 8, facing: 0, placed: true, opacity: 0.5 };
  const rejected = { ...accepted, c: [140, 200] as const, placed: false };
  const draw = (marks: import("@packages/battle-renderer/src/orderOverlay").DestinationMarker[]) =>
    buildDestinationPreview(marks, () => 0, gameOrderStyle, { stroke: gameStroke(0.05) });
  expect(draw([rejected])).toEqual(new Float32Array());
  expect(draw([accepted, rejected])).toEqual(draw([accepted]));
});

test("a cancelled query cannot paint a new press at the same anchor", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint(UNITS);
  const own = [
    { id: 1, kind: "test_tank", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  let finish!: (marks: import("../src/battle/sim/protocol").MoveDestination[]) => void;
  const client = {
    previewBuilding: async () => {
      throw new Error("unexpected building preview");
    },
    previewMove: () =>
      new Promise<import("../src/battle/sim/protocol").MoveDestination[]>((resolve) => {
        finish = resolve;
      }),
  };
  const move = { units: [1], goal: [100, 200] as [number, number], facing: 1 };
  paint.resolvePreview({ kind: "move", request: move }, own, client, 0);
  paint.resolvePreview(null, own, client, 0);
  paint.resolvePreview({ kind: "move", request: { ...move, facing: 0 } }, own, client, 0);
  finish([{ unit: 1, goal: move.goal, placed: true, facing: 1 }]);
  await new Promise((r) => setTimeout(r, 0));
  expect(
    paint.resolvePreview({ kind: "move", request: { ...move, facing: 0 } }, own, client, 0),
  ).toEqual([]);
});

test("pointer paint reveals only accepted destinations from a partially blocked group", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint(UNITS);
  const own = [1, 2].map((id) => ({
    id,
    kind: "test_tank",
    position: [0, 0, 0],
    members: [],
    area: null,
    garrison: null,
  })) as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const markers = paint.markers(
    [
      { unit: 1, goal: [100, 200], facing: 0, placed: false },
      { unit: 2, goal: [140, 200], facing: 0, placed: true },
    ],
    own,
  );
  expect(markers).toMatchObject([{ unit: 2, c: [140, 200], placed: true }]);
  paint.update(null, markers, () => 0, 0.05);
  expect(paint.preview).toMatchObject([{ unit: 2, c: [140, 200] }]);
});

test("a changed building cannot inherit a late entry result, and fallback displays its accepted moves", async () => {
  const { PointerPaint } = await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint(UNITS);
  const own = [
    { id: 1, kind: "test_tank", position: [0, 0, 0], members: [], area: null, garrison: null },
    { id: 2, kind: "test_rifle", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  let finish!: (placement: import("../src/battle/sim/protocol").BuildingPlacement) => void;
  const client = {
    previewMove: async () => [],
    previewBuilding: () =>
      new Promise<import("../src/battle/sim/protocol").BuildingPlacement>((resolve) => {
        finish = resolve;
      }),
  };
  const intent = { kind: "building" as const, request: { units: [1, 2], building: 8 } };
  paint.resolvePreview(intent, own, client, 0);
  paint.resolvePreview({ ...intent, request: { ...intent.request, building: 9 } }, own, client, 0);
  finish({ building: 8, entrant: { unit: 2, approach: [100, 200] }, destinations: [] });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(paint.building).toBe(null);
  expect(
    paint.resolvePreview(
      { ...intent, request: { ...intent.request, building: 9 } },
      own,
      client,
      0,
    ),
  ).toEqual([]);
  finish({
    building: 9,
    entrant: null,
    destinations: [{ unit: 1, goal: [140, 200], placed: true, facing: 0 }],
  });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(
    paint.resolvePreview(
      { ...intent, request: { ...intent.request, building: 9 } },
      own,
      client,
      0,
    ),
  ).toMatchObject([{ unit: 1, c: [140, 200], placed: true }]);
  expect(paint.building?.entrant).toBe(null);
  expect(paint.state).toBe("ready");
});

test("a building cursor keeps a current certificate during tick refresh, clears changed intent immediately, and shows plain fallback", async () => {
  const { PointerPaint, cursorForIntent, previewForIntent } =
    await import("@apps/battle-lab/src/pointerPaint");
  const paint = new PointerPaint(UNITS);
  const own = [1, 2].map((id) => ({
    id,
    kind: "test_rifle",
    position: [0, 0, 0],
    members: [],
    area: null,
    garrison: null,
  })) as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const replies: ((p: import("../src/battle/sim/protocol").BuildingPlacement) => void)[] = [];
  const client = {
    previewMove: async () => [],
    previewBuilding: () =>
      new Promise<import("../src/battle/sim/protocol").BuildingPlacement>((finish) =>
        replies.push(finish),
      ),
  };
  const intent: import("../src/battle/input/pointerIntent").PointerIntent = {
    kind: "occupy_building",
    units: [1, 2],
    building: 7,
    queued: false,
  };
  const update = (tick: number, target = intent) => {
    paint.resolvePreview(previewForIntent(target), own, client, tick);
    return cursorForIntent(target, paint);
  };
  expect(update(0)).toBe("default");
  replies[0]({ building: 7, entrant: { unit: 1, approach: [10, 20] }, destinations: [] });
  await new Promise((r) => setTimeout(r, 0));
  expect(update(1)).toBe("garrison");
  const changed = { ...intent, building: 9 };
  expect(update(1, changed)).toBe("default");
  replies[1]({ building: 7, entrant: { unit: 1, approach: [10, 20] }, destinations: [] });
  await new Promise((r) => setTimeout(r, 0));
  expect(update(1, changed)).toBe("default");
  replies[2]({
    building: 9,
    entrant: null,
    destinations: [{ unit: 2, goal: [50, 60], placed: true, facing: 0 }],
  });
  await new Promise((r) => setTimeout(r, 0));
  expect(update(1, changed)).toBe("default");
  expect(paint.state).toBe("ready");
  expect(paint.building!.building).toBe(9);
});

test.each(["clearing", "epoch replacement"] as const)(
  "ground %s revokes a ready cursor and cannot accept its in-flight refresh",
  async (change) => {
    const { PointerPaint, cursorForIntent, previewForIntent, previewContextIdentity } =
      await import("@apps/battle-lab/src/pointerPaint");
    const paint = new PointerPaint(UNITS);
    const own = [
      { id: 1, kind: "test_rifle", position: [0, 0, 0], members: [], area: null, garrison: null },
    ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
    const ground = new GroundView({ cellM: 1, cols: 4, rows: 4 });
    const publish = (epoch: number, revision: number, cell: number, full: boolean) =>
      ground.applyRuns(
        cellPatchRuns(ground.cols, {
          epoch,
          side: "blue",
          baseRevision: full ? 0 : ground.revision,
          revision,
          full,
          cells: Uint32Array.of(cell),
          marks: new Uint8Array(4),
          cleared: Uint8Array.of(1),
        }),
      );
    publish(1, 1, 0, true);
    const replies: ((p: import("../src/battle/sim/protocol").BuildingPlacement) => void)[] = [];
    const client = {
      previewMove: async () => [],
      previewBuilding: () =>
        new Promise<import("../src/battle/sim/protocol").BuildingPlacement>((finish) =>
          replies.push(finish),
        ),
    };
    const intent = { kind: "occupy_building" as const, units: [1], building: 7, queued: false };
    const placement = {
      building: 7,
      entrant: { unit: 1, approach: [10, 20] as [number, number] },
      destinations: [],
    };
    const update = (tick: number) => {
      paint.resolvePreview(
        previewForIntent(intent),
        own,
        client,
        tick,
        previewContextIdentity(
          "blue",
          "[]",
          "same own eligibility",
          "[]",
          ground.clearedCount,
          ground.epoch,
        ),
      );
      return cursorForIntent(intent, paint);
    };
    const settle = () => new Promise((resolve) => setTimeout(resolve, 0));
    expect(update(0)).toBe("default");
    replies[0](placement);
    await settle();
    expect(update(1)).toBe("garrison");
    // Pose and tick refreshes keep the known certificate while its replacement is in flight.
    own[0] = { ...own[0], position: [1, 0, 0] };
    expect(update(2)).toBe("garrison");
    expect(paint.state).toBe("ready");
    publish(change === "clearing" ? 1 : 2, 2, 1, change === "epoch replacement");
    expect(update(2)).toBe("default");
    expect(paint.state).toBe("pending");
    expect(paint.building).toBeNull();
    replies[1](placement);
    await settle();
    expect(update(2)).toBe("default");
    expect(paint.building).toBeNull();
    replies[2](placement);
    await settle();
    expect(update(2)).toBe("garrison");
  },
);

test("unproven movement and building searches keep the cursor plain, while a known complete refusal is blocked", async () => {
  const { PointerPaint, cursorForIntent, previewForIntent } =
    await import("@apps/battle-lab/src/pointerPaint");
  const own = [
    { id: 1, kind: "test_rifle", position: [0, 0, 0], members: [], area: null, garrison: null },
  ] as unknown as import("../src/battle/sim/observation").OwnUnitView[];
  const move: import("../src/battle/input/pointerIntent").PointerIntent = {
    kind: "move",
    units: [1],
    goal: [10, 20],
    route: "shortest",
    direction: "forward",
    queued: false,
  };
  const building: import("../src/battle/input/pointerIntent").PointerIntent = {
    kind: "occupy_building",
    units: [1],
    building: 7,
    queued: false,
  };
  for (const [intent, unproven, expected] of [
    [move, false, "default"],
    [building, true, "default"],
    [building, false, "blocked"],
  ] as const) {
    const paint = new PointerPaint(UNITS);
    const client = {
      previewMove: async () => [],
      previewBuilding: async () => ({ building: 7, entrant: null, destinations: [], unproven }),
    };
    paint.resolvePreview(previewForIntent(intent), own, client, 0);
    await new Promise((r) => setTimeout(r, 0));
    expect(cursorForIntent(intent, paint)).toBe(expected);
  }
});

test("release feedback distinguishes unproven admission from a known lost attack target", async () => {
  const { cursorForAcknowledgement } = await import("@apps/battle-lab/src/pointerPaint");
  const building: import("../src/battle/input/pointerIntent").PointerIntent = {
    kind: "occupy_building",
    units: [1],
    building: 7,
    queued: false,
  };
  expect(
    cursorForAcknowledgement(building, {
      seq: 1,
      applied_tick: 10,
      error: { reason: "no_valid_destination" },
      building: { building: 7, entrant: null, destinations: [], unproven: true },
    }),
  ).toBe("default");
  const attack: import("../src/battle/input/pointerIntent").PointerIntent = {
    kind: "attack",
    units: [1],
    target: { kind: "identified", id: 2 },
    queued: false,
  };
  expect(
    cursorForAcknowledgement(attack, {
      seq: 2,
      applied_tick: 10,
      error: { reason: "unknown_target" },
    }),
  ).toBe("blocked");
});

test("a drag acknowledgement corrects the same building hover when admission falls back to movement", async () => {
  const { cursorForRelease } = await import("@apps/battle-lab/src/pointerPaint");
  const hover = { kind: "occupy_building" as const, units: [1, 2], building: 7, queued: false };
  const released = { ...hover, facing: Math.PI / 2 };
  const ack = {
    seq: 1,
    applied_tick: 11,
    error: null,
    building: {
      building: 7,
      entrant: null,
      destinations: [
        { unit: 1, placed: true, goal: [40, 50] as [number, number], facing: Math.PI / 2 },
      ],
    },
  };
  expect(cursorForRelease(hover, released, ack)).toBe("default");
  expect(cursorForRelease({ ...hover, building: 8 }, released, ack)).toBeNull();
  expect(cursorForRelease({ ...hover, queued: true }, released, ack)).toBeNull();
  expect(cursorForRelease({ ...hover, facing: 0 }, released, ack)).toBeNull();
  expect(
    cursorForRelease(
      {
        kind: "attack",
        units: [1, 2],
        target: { kind: "ground", point: [40, 50, 0] },
        queued: false,
      },
      released,
      ack,
    ),
  ).toBeNull();
});
