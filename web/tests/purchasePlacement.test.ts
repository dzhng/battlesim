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
  resolve[0]({ Ok: 0 });
  await settle();
  expect(control.at([40, 50], client, "1")?.valid).toBeNull();
  expect(client.previewPurchase).toHaveBeenCalledTimes(2);
  resolve[1]({ Ok: 1 });
  await settle();
  expect(control.at([40, 50], client, "1")?.valid).toBe(true);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null }));
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
  resolvers[0]({ Ok: 1 });
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
  reply({ Ok: 1 });
  await settle();
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null }));
  expect(await control.confirm(send)).toBeTruthy();
  expect(send).toHaveBeenCalledExactlyOnceWith({
    kind: "confirm_purchase",
    variant: "test_scout",
    destination: [10, 20],
  });
});

test("deployment ghosts face the opposing edge by default and track a drag bearing", () => {
  const client = { previewPurchase: vi.fn(async () => ({ Ok: 1 })) } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  expect(control.at([10, 20], client, "1")?.facing).toBeCloseTo(-Math.PI / 2);
  expect(control.at([10, 20], client, "1", Math.PI / 2)?.facing).toBeCloseTo(Math.PI / 2);
});

test("turning a placed ghost keeps its preview, and confirming reports the ghost as placed", async () => {
  const client = { previewPurchase: vi.fn(async () => ({ Ok: 1 })) } as unknown as SimClient;
  const control = new PurchasePlacementControl();
  control.choose("test_scout");
  control.at([10, 20], client, "1");
  await settle();
  // The drag only turns the ghost: where it stands is unchanged, so is its admission.
  expect(control.at([10, 20], client, "1", 0.5)?.valid).toBe(true);
  expect(client.previewPurchase).toHaveBeenCalledTimes(1);
  const send = vi.fn(async () => ({ seq: 1, applied_tick: 2, error: null }));
  expect(await control.confirm(send)).toEqual({
    variant: "test_scout",
    destination: [10, 20],
    facing: 0.5,
    valid: true,
  });
});
