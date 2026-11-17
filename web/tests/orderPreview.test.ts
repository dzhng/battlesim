import { expect, test, vi } from "vitest";
import { OrderPreview, type OrderPreviewSnapshot } from "@apps/battle-lab/src/orderPreview";
import {
  cursorForIntent,
  pointerCursor,
  previewForIntent,
} from "@apps/battle-lab/src/pointerPaint";
import type { MoveDestination } from "@web/battle/sim/protocol";
import type { PointerIntent } from "@web/battle/input/pointerIntent";

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

test("a building shows the garrison badge at once; only the simulation's answer takes it away", async () => {
  type BuildingAnswer = {
    building: number;
    entries: { unit: number; approach: [number, number] }[];
  };
  const blocked = deferred<BuildingAnswer>();
  const admitted = deferred<BuildingAnswer>();
  const sim = client();
  sim.previewBuilding.mockReturnValueOnce(blocked.promise).mockReturnValueOnce(admitted.promise);
  const preview = new OrderPreview();
  const intent = {
    kind: "building" as const,
    request: { units: [1], building: 4 },
  };
  const occupy: PointerIntent = { kind: "occupy_building", units: [1], building: 4, queued: false };

  // Asked, not yet answered: entering shows at once, as boarding does.
  const asked = preview.update(intent, sim, 1, "building");
  expect(cursorForIntent(occupy, asked)).toBe("garrison");
  // No squad can get there: the answer takes the badge away.
  blocked.resolve({ building: 4, entries: [] });
  await blocked.promise;
  await settled();
  expect(cursorForIntent(occupy, preview.update(intent, sim, 1, "building"))).toBe("blocked");

  const queued = { ...intent, request: { ...intent.request, queued: true } };
  preview.update(queued, sim, 1, "building");
  admitted.resolve({ building: 4, entries: [{ unit: 1, approach: [10, 10] }] });
  await admitted.promise;
  await settled();
  expect(cursorForIntent(occupy, preview.update(queued, sim, 1, "building"))).toBe("garrison");
});

test("the garrison badge holds from one enterable building to the next while it is asked", async () => {
  const first = deferred<{
    building: number;
    entries: { unit: number; approach: [number, number] }[];
  }>();
  const sim = client();
  sim.previewBuilding.mockReturnValueOnce(first.promise).mockReturnValueOnce(deferred().promise);
  const preview = new OrderPreview();
  const into = (building: number) => ({
    intent: { kind: "building" as const, request: { units: [1], building } },
    occupy: { kind: "occupy_building", units: [1], building, queued: false } as PointerIntent,
  });
  const a = into(4);
  const b = into(5);

  preview.update(a.intent, sim, 1, "building");
  first.resolve({ building: 4, entries: [{ unit: 1, approach: [10, 10] }] });
  await first.promise;
  await settled();
  expect(cursorForIntent(a.occupy, preview.update(a.intent, sim, 1, "building"))).toBe("garrison");

  // The pointer crosses to the next house: asked, not yet answered.
  expect(cursorForIntent(b.occupy, preview.update(b.intent, sim, 1, "building"))).toBe("garrison");
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

test("moving the goal keeps the cursor's availability until the new goal is placed", async () => {
  const here = deferred<MoveDestination[]>();
  const there = deferred<MoveDestination[]>();
  const sim = client();
  sim.previewMove.mockReturnValueOnce(here.promise).mockReturnValueOnce(there.promise);
  const preview = new OrderPreview();
  const unload = (goal: [number, number]) => ({
    kind: "unload" as const,
    units: [1],
    goal,
    route: "shortest" as const,
    queued: false,
  });

  preview.update(move([10, 10]), sim, 1, "goal-a");
  here.resolve([destination([10, 10])]);
  await here.promise;
  await settled();
  expect(cursorForIntent(unload([10, 10]), preview.update(move([10, 10]), sim, 1, "goal-a"))).toBe(
    "unload",
  );

  // The pointer moves on: the old marks go, but the badge stays while the
  // new goal is in flight.
  const moving = preview.update(move([20, 20]), sim, 1, "goal-b");
  expect(moving.destinations).toEqual([]);
  expect(cursorForIntent(unload([20, 20]), moving)).toBe("unload");

  // A goal nobody can reach settles to the plain arrow.
  there.resolve([{ ...destination([20, 20]), placed: false }]);
  await there.promise;
  await settled();
  expect(cursorForIntent(unload([20, 20]), preview.update(move([20, 20]), sim, 1, "goal-b"))).toBe(
    "default",
  );
});

test("unloading into a host badges the host at once, over another carrier too, with no hover placement", () => {
  const into = (
    host: { kind: "building"; building: number } | { kind: "carrier"; carrier: number },
  ): PointerIntent => ({
    kind: "unload",
    units: [1],
    goal: [10, 10],
    route: "shortest",
    queued: false,
    into: host,
  });
  const building = into({ kind: "building", building: 3 });
  const carrier = into({ kind: "carrier", carrier: 4 });
  // The simulation picks where to set down beside the host: nothing to place.
  expect(previewForIntent(building)).toBeNull();
  const unanswered = { state: "pending", building: null } as const;
  expect(cursorForIntent(building, unanswered)).toBe("unload_garrison");
  expect(cursorForIntent(carrier, unanswered)).toBe("unload_board");
  const preview = new OrderPreview().update(null, null, 0, "none");
  expect(pointerCursor(carrier, true, null, { intent: carrier, preview })).toBe("unload_board");
});
