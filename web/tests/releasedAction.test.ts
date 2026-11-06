import { expect, test } from "vitest";
import { ReleasedAction } from "@apps/battle-lab/src/releasedAction";
import type { PointerIntent, PointerPick } from "@web/battle/input/pointerIntent";
import type { SimClient } from "@web/battle/sim/client";
import type { CommandAck, Order } from "@web/battle/sim/protocol";

const pick: PointerPick = {
  unit: null,
  button: "right",
  shift: false,
  ctrl: false,
  x: 0,
  y: 0,
  time: 0,
  ground: [10, 10],
};

const intent = (goal: [number, number]): PointerIntent => ({
  kind: "move",
  units: [1],
  goal,
  route: "shortest",
  direction: "forward",
  facing: undefined,
  queued: false,
});

const order = (goal: [number, number]): Order => ({
  kind: "move",
  units: [1],
  goal,
  route: "shortest",
  direction: "forward",
  gesture: 1,
});

const ack = (seq: number): CommandAck => ({
  seq,
  applied_tick: 5,
  refused: [],
  error: null,
});

test("a delayed answer stays attached to the released action while a new gesture is held", () => {
  const client = {} as SimClient;
  const bridge = new ReleasedAction();
  const first = order([10, 10]);
  bridge.capture(pick, intent([10, 10]), client);
  bridge.beginRelease();
  const releasedIntent = { ...intent([10, 10]), facing: 1 };
  bridge.issue(first, false, client, releasedIntent);

  bridge.capture({ ...pick, ground: [20, 20] }, intent([20, 20]), client);
  bridge.acknowledge(first, ack(1), false, client);

  expect(bridge.released).toMatchObject({
    order: first,
    intent: releasedIntent,
    ack: ack(1),
    generation: 1,
  });
  expect(bridge.capturedFor(client)?.intent).toEqual(intent([20, 20]));
});

test("an unrelated command does not consume a held right gesture", () => {
  const client = {} as SimClient;
  const bridge = new ReleasedAction();
  bridge.capture(pick, intent([10, 10]), client);
  bridge.issue({ kind: "stop", units: [1] }, false, client, null);

  expect(bridge.released?.generation).toBe(0);
  expect(bridge.capturedFor(client)?.intent).toEqual(intent([10, 10]));
});

test("orders without pointer feedback keep a null released intent", () => {
  const client = {} as SimClient;
  const bridge = new ReleasedAction();
  bridge.capture(pick, intent([10, 10]), client);
  bridge.beginRelease();
  bridge.issue({ kind: "garrison", units: [1], building: 7 }, false, client, null);

  expect(bridge.released?.intent).toBeNull();
});

test("a newer release ignores an older answer and retires its own feedback", () => {
  const client = {} as SimClient;
  const bridge = new ReleasedAction();
  const first = order([10, 10]);
  const second = { ...order([20, 20]), gesture: 2 };
  bridge.capture(pick, intent([10, 10]), client);
  bridge.beginRelease();
  bridge.issue(first, false, client, intent([10, 10]));
  const firstRecord = bridge.released!;
  bridge.capture({ ...pick, ground: [20, 20] }, intent([20, 20]), client);
  bridge.beginRelease();
  bridge.issue(second, true, client, intent([20, 20]));

  bridge.acknowledge(first, ack(1), false, client);
  expect(bridge.released?.ack).toBeNull();
  bridge.acknowledge(second, ack(2), true, client);
  const released = bridge.released!;
  expect(released.ack).toEqual(ack(2));
  bridge.retire(firstRecord);
  expect(bridge.released).toBe(released);
  bridge.retire(released);
  expect(bridge.released).toBeNull();
});

test("client identity and order identity each reject a stale acknowledgement", () => {
  const client = {} as SimClient;
  const otherClient = {} as SimClient;
  const bridge = new ReleasedAction();
  const released = order([10, 10]);
  bridge.capture(pick, intent([10, 10]), client);
  bridge.beginRelease();
  bridge.issue(released, false, client, intent([10, 10]));

  bridge.acknowledge(released, ack(1), false, otherClient);
  expect(bridge.released?.ack).toBeNull();
  bridge.acknowledge(
    { kind: "move", units: [1], goal: [10, 10], route: "shortest", gesture: 2 },
    ack(1),
    false,
    client,
  );
  expect(bridge.released?.ack).toBeNull();
  bridge.acknowledge(released, ack(1), true, client);
  expect(bridge.released?.ack).toBeNull();
});

test("session reset clears both a held gesture and released acknowledgement", () => {
  const client = {} as SimClient;
  const bridge = new ReleasedAction();
  const released = order([10, 10]);
  bridge.capture(pick, intent([10, 10]), client);
  bridge.beginRelease();
  bridge.issue(released, false, client, intent([10, 10]));
  bridge.acknowledge(released, ack(1), false, client);
  bridge.capture({ ...pick, ground: [20, 20] }, intent([20, 20]), client);

  bridge.reset();

  expect(bridge.released).toBeNull();
  expect(bridge.capturedFor(client)).toBeNull();
});
