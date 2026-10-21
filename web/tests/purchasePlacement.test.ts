// @vitest-environment node
import { expect, test, vi } from "vitest";
import { PurchasePlacementControl } from "@web/battle/input/purchasePlacement";
import type { SimClient } from "@web/battle/sim/client";
import type { PurchasePlacement } from "@web/battle/sim/protocol";
const settle = async () => {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
};
test("a changed variant/destination cannot reuse an earlier valid preview", async () => {
  const resolve: ((p: PurchasePlacement) => void)[] = [];
  const client = {
    previewPurchase: vi.fn(() => new Promise<PurchasePlacement>((r) => resolve.push(r))),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_tank");
  expect(control.at([20, 30], client, "1")?.valid).toBeNull();
  control.choose("test_scout");
  expect(control.at([40, 50], client, "1")?.valid).toBeNull();
  resolve[0]({ Ok: { kind: 0, destination: [20, 30], spots: [] } });
  await settle();
  expect(control.at([40, 50], client, "1")?.valid).toBeNull();
  expect(client.previewPurchase).toHaveBeenCalledTimes(2);
  resolve[1]({ Ok: { kind: 1, destination: [40, 50], spots: [] } });
  await settle();
  expect(control.at([40, 50], client, "1")?.valid).toBe(true);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null, refused: [] }));
  expect(await control.confirm(send)).toBeTruthy();
  expect(send).toHaveBeenCalledExactlyOnceWith({
    kind: "confirm_purchase",
    variant: "test_scout",
    destination: [40, 50],
  });
  expect(control.variant).toBeNull();
});

test("a replaced authority cannot inherit a valid preview", async () => {
  const resolvers: ((p: PurchasePlacement) => void)[] = [];
  const first = {
    previewPurchase: () => new Promise<PurchasePlacement>((r) => resolvers.push(r)),
  } as unknown as SimClient;
  const second = {
    previewPurchase: () => new Promise<PurchasePlacement>((r) => resolvers.push(r)),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  control.at([10, 20], first, "1");
  control.at([10, 20], second, "1");
  resolvers[0]({ Ok: { kind: 1, destination: [10, 20], spots: [] } });
  await settle();
  expect(control.at([10, 20], second, "1")?.valid).toBeNull();
  resolvers[1]({ Err: { reason: "wrong_faction" } });
  await settle();
  expect(control.at([10, 20], second, "1")?.valid).toBe(false);
  expect(await control.confirm(vi.fn())).toBeNull();
});

test("a completed preview can confirm without another pointer movement", async () => {
  let reply!: (placement: PurchasePlacement) => void;
  const client = {
    previewPurchase: () =>
      new Promise<PurchasePlacement>((resolve) => {
        reply = resolve;
      }),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  control.at([10, 20], client, "1");
  reply({ Ok: { kind: 1, destination: [10, 20], spots: [] } });
  await settle();
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null, refused: [] }));
  expect(await control.confirm(send)).toBeTruthy();
  expect(send).toHaveBeenCalledExactlyOnceWith({
    kind: "confirm_purchase",
    variant: "test_scout",
    destination: [10, 20],
  });
});

test("deployment ghosts face the opposing edge by default and track a drag bearing", () => {
  const client = {
    previewPurchase: vi.fn(async (_variant: string, at: [number, number]) => ({
      Ok: { kind: 1, destination: at, spots: [] },
    })),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  expect(control.at([10, 20], client, "1")?.facing).toBeCloseTo(-Math.PI / 2);
  expect(control.at([10, 20], client, "1", Math.PI / 2)?.facing).toBeCloseTo(Math.PI / 2);
});

test("turning a placed ghost keeps its preview, and confirming reports the ghost as placed", async () => {
  const client = {
    previewPurchase: vi.fn(async (_variant: string, at: [number, number]) => ({
      Ok: { kind: 1, destination: at, spots: [] },
    })),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  control.at([10, 20], client, "1");
  await settle();
  // The drag only turns the ghost: where it stands is unchanged, so is its admission.
  expect(control.at([10, 20], client, "1", 0.5)?.valid).toBe(true);
  expect(client.previewPurchase).toHaveBeenCalledTimes(1);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null, refused: [] }));
  expect(await control.confirm(send)).toEqual({
    variant: "test_scout",
    destination: [10, 20],
    facing: 0.5,
    valid: true,
    spots: [],
  });
});

test("a squad's ghost stands its soldiers where the preview spread them, and keeps that shape while the cursor moves on", async () => {
  const client = {
    previewPurchase: vi.fn(async (_variant: string, [x, y]: [number, number]) => ({
      Ok: {
        kind: 1,
        destination: [x, y] as [number, number],
        spots: [
          [x - 1, y],
          [x + 1, y + 2],
        ] as [number, number][],
      },
    })),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_squad");
  expect(control.at([10, 20], client, "1")?.spots).toEqual([]);
  await settle();
  expect(control.at([10, 20], client, "1")?.spots).toEqual([
    [9, 20],
    [11, 22],
  ]);
  // The next destination's preview has not answered: the squad keeps its shape.
  expect(control.at([30, 40], client, "1")?.spots).toEqual([
    [29, 40],
    [31, 42],
  ]);
});

test("a ghost over blocked ground stands where the authority will send it, and confirms there", async () => {
  // The authority snaps the cursor at (10, 20) to open ground 6 m east.
  const client = {
    previewPurchase: vi.fn(async () => ({
      Ok: {
        kind: 1,
        destination: [16, 20] as [number, number],
        spots: [
          [15, 20],
          [17, 21],
        ] as [number, number][],
      },
    })),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_squad");
  control.at([10, 20], client, "1");
  await settle();
  const ghost = control.at([10, 20], client, "1");
  expect(ghost?.destination).toEqual([16, 20]);
  expect(ghost?.spots).toEqual([
    [15, 20],
    [17, 21],
  ]);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null, refused: [] }));
  await control.confirm(send);
  expect(send).toHaveBeenCalledExactlyOnceWith({
    kind: "confirm_purchase",
    variant: "test_squad",
    destination: [16, 20],
  });
});

test("a ghost stays placeable while the side's knowledge changes faster than previews answer", async () => {
  // In a team battle the side's knowledge changes every tick or two, so the
  // revision moves on before each preview answers.
  const replies: ((placement: PurchasePlacement) => void)[] = [];
  const client = {
    previewPurchase: vi.fn(() => new Promise<PurchasePlacement>((r) => replies.push(r))),
  } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  for (let revision = 1; revision <= 4; revision++) {
    control.at([10, 20], client, `${revision}`);
    replies.at(-1)!({ Ok: { kind: 1, destination: [12, 20], spots: [] } });
    await settle();
  }
  const ghost = control.at([10, 20], client, "5");
  expect(ghost?.valid).toBe(true);
  expect(ghost?.destination).toEqual([12, 20]);
  // The newer knowledge is still asked about.
  expect(client.previewPurchase).toHaveBeenCalledTimes(5);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null, refused: [] }));
  expect(await control.confirm(send)).toBeTruthy();
  expect(send).toHaveBeenCalledExactlyOnceWith({
    kind: "confirm_purchase",
    variant: "test_scout",
    destination: [12, 20],
  });
});
