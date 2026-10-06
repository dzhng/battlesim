// @vitest-environment node
import { expect, test } from "vitest";
import { restingModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import type { Bundle, SkeletonClips } from "@packages/scene-assets/src/schema";

test("placement uses the appearance's authored resting pose", () => {
  const body = { kind: "skinned", far_pose: { clip: "standing", phase: 0.25 } } as Exclude<
    Bundle,
    SkeletonClips
  >;
  expect(restingModelPose(body)).toEqual({
    kind: "skinned",
    clip: "standing",
    phase: 0.25,
    blend: null,
  });
  const vehicle = { kind: "articulated" } as Exclude<Bundle, SkeletonClips>;
  const first = restingModelPose(vehicle);
  const second = restingModelPose(vehicle);
  expect(first).toEqual(second);
  if (first.kind === "articulated" && second.kind === "articulated") {
    first.articulation.deploy = 1;
    expect(second.articulation.deploy).toBe(0);
  }
  const prop = { kind: "static", states: [{ name: "intact" }] } as Exclude<Bundle, SkeletonClips>;
  expect(restingModelPose(prop)).toEqual({ kind: "static", state: "intact" });
});
