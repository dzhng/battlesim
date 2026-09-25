// @vitest-environment node
import { expect, test } from "vitest";
import { pickInstance, rayInstanceDistance } from "@packages/battle-renderer/src/picking.ts";
import type { SceneInstance } from "@packages/battle-renderer/src/scene.ts";
import { PROXY_ASSETS } from "@packages/battle-renderer/src/proxies.ts";

const tank: SceneInstance = { kind: "tank", x: 10, y: 0, z: 0, yaw: Math.PI / 2, color: [1, 1, 1] };

test("a downward ray hits a yawed tank's body but not beside its narrow side", () => {
  // Yawed 90°, the tank's long axis runs along world Y: y=3 is on the hull, x=12.5 is beside it.
  const down = (x: number, y: number) => ({
    origin: [x, y, 50] as const,
    dir: [0, 0, -1] as const,
  });
  const top = PROXY_ASSETS.tank.center[2] + PROXY_ASSETS.tank.halfExtents[2];
  expect(rayInstanceDistance(down(10, 3), tank)).toBeCloseTo(50 - top, 5);
  expect(rayInstanceDistance(down(12.5, 0), tank)).toBeNull();
});

test("the nearest hit along the ray wins", () => {
  const near = { ...tank, x: 0 };
  const ray = { origin: [-50, 0, 1] as const, dir: [1, 0, 0] as const };
  expect(pickInstance(ray, [tank, near])).toBe(1);
  expect(pickInstance({ origin: [0, 0, 50], dir: [0, 0, 1] }, [tank])).toBe(-1);
});
