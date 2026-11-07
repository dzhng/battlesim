import { expect, test, vi } from "vitest";
import { OrderPreview, type OrderPreviewSnapshot } from "@apps/battle-lab/src/orderPreview";
import { cursorForIntent } from "@apps/battle-lab/src/pointerPaint";
import type { MoveDestination } from "@web/battle/sim/protocol";

const move = (goal: [number, number], facing?: number) => ({
  kind: "move" as const,
  request: { units: [1], goal, ...(facing === undefined ? {} : { facing }) },
});

const destination = (goal: [number, number]): MoveDestination => ({
  unit: 1,
  goal,
  facing: 0,
  placed: true,
  spots: [],
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

function client() {
  return {
    previewMove: vi.fn(),
    previewBuilding: vi.fn(),
  };
}

async function settled() {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

test("a superseded move answer cannot repopulate the newer preview", async () => {
  const old = deferred<MoveDestination[]>();
  const next = deferred<MoveDestination[]>();
  const sim = client();
  sim.previewMove.mockReturnValueOnce(old.promise).mockReturnValueOnce(next.promise);
  const preview = new OrderPreview();

  preview.update(move([10, 10]), sim, 1, "gesture-a");
  preview.update(move([20, 20]), sim, 1, "gesture-b");
  expect(sim.previewMove).toHaveBeenCalledTimes(1);
  old.resolve([destination([10, 10])]);
  await old.promise;
  await settled();
  expect(sim.previewMove).toHaveBeenCalledTimes(1);

  const beforeNext = preview.update(move([20, 20]), sim, 1, "gesture-b");
  expect(beforeNext.destinations).toEqual([]);
  expect(sim.previewMove).toHaveBeenCalledTimes(2);

  next.resolve([destination([20, 20])]);
  await next.promise;
  await settled();
  expect(preview.update(move([20, 20]), sim, 1, "gesture-b").destinations).toEqual([
    destination([20, 20]),
  ]);
});

test("gesture and client changes revoke an already admitted answer", async () => {
  const first = deferred<MoveDestination[]>();
  const second = deferred<MoveDestination[]>();
  const sim = client();
  sim.previewMove.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);
  const preview = new OrderPreview();

  preview.update(move([10, 10]), sim, 1, "gesture-a");
  first.resolve([destination([10, 10])]);
  await first.promise;
  await settled();
  expect(preview.update(move([10, 10]), sim, 1, "gesture-a").destinations).toEqual([
    destination([10, 10]),
  ]);

  expect(preview.update(move([20, 20]), sim, 1, "gesture-b").destinations).toEqual([]);
  second.resolve([destination([20, 20])]);
  await second.promise;
  await settled();

  const old = deferred<MoveDestination[]>();
  const fresh = deferred<MoveDestination[]>();
  const oldClient = client();
  const freshClient = client();
  oldClient.previewMove.mockReturnValue(old.promise);
  freshClient.previewMove.mockReturnValue(fresh.promise);
  const clientPreview = new OrderPreview();
  const intent = move([30, 30]);
  clientPreview.update(intent, oldClient, 1, "same-gesture");
  old.resolve([destination([30, 30])]);
  await old.promise;
  await settled();
  expect(clientPreview.update(intent, oldClient, 1, "same-gesture").destinations).toEqual([
    destination([30, 30]),
  ]);
  expect(clientPreview.update(intent, freshClient, 1, "same-gesture").destinations).toEqual([]);
  fresh.resolve([destination([31, 31])]);
  await fresh.promise;
  await settled();
  expect(clientPreview.update(intent, freshClient, 1, "same-gesture").destinations).toEqual([
    destination([31, 31]),
  ]);
});

test("facing changes retain admitted destinations while the next query is pending", async () => {
  const first = deferred<MoveDestination[]>();
  const turned = deferred<MoveDestination[]>();
  const sim = client();
  sim.previewMove.mockReturnValueOnce(first.promise).mockReturnValueOnce(turned.promise);
  const preview = new OrderPreview();

  preview.update(move([10, 10], 0), sim, 1, "same-gesture");
  first.resolve([destination([10, 10])]);
  await first.promise;
  await settled();
  expect(preview.update(move([10, 10], 0), sim, 1, "same-gesture").destinations).toEqual([
    destination([10, 10]),
  ]);

  const retained = preview.update(move([10, 10], Math.PI / 2), sim, 1, "same-gesture");
  expect(retained.destinations).toEqual([destination([10, 10])]);
  expect(sim.previewMove).toHaveBeenCalledTimes(2);
  turned.resolve([destination([12, 12])]);
  await turned.promise;
  await settled();
  expect(preview.update(move([10, 10], Math.PI / 2), sim, 1, "same-gesture").destinations).toEqual([
    destination([12, 12]),
  ]);
});

test("a failed query remains pending and does not become blocked", async () => {
  const answer = deferred<MoveDestination[]>();
  const sim = client();
  sim.previewMove.mockReturnValue(answer.promise);
  const preview = new OrderPreview();
  preview.update(move([10, 10]), sim, 1, "gesture");
  answer.reject(new Error("preview failed"));
  await answer.promise.catch(() => undefined);
  await settled();

  const snapshot: OrderPreviewSnapshot = preview.update(move([10, 10]), sim, 1, "gesture");
  expect(snapshot.destinations).toEqual([]);
  expect(snapshot.building).toBeNull();
  expect(
    cursorForIntent(
      { kind: "move", units: [1], goal: [10, 10], route: "shortest", queued: false },
      snapshot,
    ),
  ).not.toBe("blocked");
});

test("building placement drives blocked, pending and garrison cursor outcomes", async () => {
  type BuildingAnswer = {
    building: number;
    entrant: { unit: number; approach: [number, number] } | null;
    destinations: MoveDestination[];
    unproven?: boolean;
  };
  const blocked = deferred<BuildingAnswer>();
  const pending = deferred<BuildingAnswer>();
  const admitted = deferred<BuildingAnswer>();
  const sim = client();
  sim.previewBuilding
    .mockReturnValueOnce(blocked.promise)
    .mockReturnValueOnce(pending.promise)
    .mockReturnValueOnce(admitted.promise);
  const preview = new OrderPreview();
  const intent = {
    kind: "building" as const,
    request: { units: [1], building: 4 },
  };
  const building = (
    entrant: { unit: number; approach: [number, number] } | null,
    unproven?: boolean,
  ) => ({
    building: 4,
    entrant,
    destinations: [{ ...destination([10, 10]), placed: false }],
    ...(unproven === undefined ? {} : { unproven }),
  });

  preview.update(intent, sim, 1, "building");
  blocked.resolve(building(null));
  await blocked.promise;
  await settled();
  const blockedSnapshot = preview.update(intent, sim, 1, "building");
  expect(
    cursorForIntent(
      { kind: "occupy_building", units: [1], building: 4, queued: false },
      blockedSnapshot,
    ),
  ).toBe("blocked");

  preview.update({ ...intent, request: { ...intent.request, facing: 1 } }, sim, 1, "building");
  pending.resolve(building(null, true));
  await pending.promise;
  await settled();
  const pendingSnapshot = preview.update(
    { ...intent, request: { ...intent.request, facing: 1 } },
    sim,
    1,
    "building",
  );
  expect(
    cursorForIntent(
      { kind: "occupy_building", units: [1], building: 4, queued: false },
      pendingSnapshot,
    ),
  ).toBe("default");

  preview.update({ ...intent, request: { ...intent.request, facing: 2 } }, sim, 1, "building");
  admitted.resolve(building({ unit: 1, approach: [10, 10] }));
  await admitted.promise;
  await settled();
  const admittedSnapshot = preview.update(
    { ...intent, request: { ...intent.request, facing: 2 } },
    sim,
    1,
    "building",
  );
  expect(
    cursorForIntent(
      { kind: "occupy_building", units: [1], building: 4, queued: false },
      admittedSnapshot,
    ),
  ).toBe("garrison");
});

test("changing client revokes the old answer", async () => {
  const old = deferred<MoveDestination[]>();
  const fresh = deferred<MoveDestination[]>();
  const first = client();
  const second = client();
  first.previewMove.mockReturnValue(old.promise);
  second.previewMove.mockReturnValue(fresh.promise);
  const preview = new OrderPreview();

  const intent = move([10, 10]);
  preview.update(intent, first, 1, "gesture");
  preview.update(intent, second, 1, "gesture");
  expect(second.previewMove).toHaveBeenCalledTimes(0);
  old.resolve([destination([10, 10])]);
  await old.promise;
  await settled();
  const beforeFresh = preview.update(intent, second, 1, "gesture");
  expect(beforeFresh.destinations).toEqual([]);
  expect(second.previewMove).toHaveBeenCalledTimes(1);
  fresh.resolve([destination([10, 10])]);
  await fresh.promise;
  await settled();
  expect(preview.update(intent, second, 1, "gesture").destinations).toEqual([
    destination([10, 10]),
  ]);
});

test("no query leaves the preview idle", () => {
  const preview = new OrderPreview();
  expect(preview.update(null, null, 1).destinations).toEqual([]);
});
