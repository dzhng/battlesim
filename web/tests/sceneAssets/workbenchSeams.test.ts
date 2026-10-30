// @vitest-environment node
// The workbench's CPU seams: a dropped file judged as the CLI judges it and
// installed through the one loader.
import { expect, test } from "vitest";
import { previewRuntime } from "@packages/scene-assets/src/bake.ts";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader.ts";
import { validateLoose } from "@packages/scene-assets/src/loose.ts";
import type { SkinnedBundle } from "@packages/scene-assets/src/schema.ts";
import {
  AUTHORITY,
  SOLDIER_CLIPS,
  TOLERANCES,
  soldierGlb,
  tankGlb,
  testCatalog,
} from "./synthetic";

const context = { authority: AUTHORITY, tolerances: TOLERANCES };
const empty = { skeletons: {}, appearances: {} };

test("a dropped GLB outside the catalog is judged, and its preview installs through the loader", async () => {
  // Not a catalog source: it is rigged like its type's own model ("tank").
  const catalog = { skeletons: {}, appearances: { tank: testCatalog().appearances.tank } };
  const result = await validateLoose("drop.glb", tankGlb({ muzzleX: 5.9 }), catalog, context, {
    type: "tank",
  });
  expect(result.unit).toBe("vehicle");
  expect(result.mounts).toEqual({ cannon: "gun", HMG: "hmg" });
  const codes = result.appearance!.findings.map((f) => f.code);
  expect(codes).toContain("fit.vehicle_muzzle");
  expect(result.appearance!.bundle).toBeNull();
  const preview = result.appearance!.preview!;
  expect(preview.kind).toBe("articulated");
  const files = await previewRuntime(
    [{ name: "drop.glb", unit: result.unit, bundle: preview as never }],
    testCatalog().sides,
  );
  const installed = await new AppearanceLibrary(memoryFetch(files, "mem:/")).load("mem:/");
  const got = installed.appearances.get("drop.glb")!.bundle;
  expect(got.kind === "articulated" && got.nodes.map((n) => n.name)).toEqual(
    preview.kind === "articulated" && preview.nodes.map((n) => n.name),
  );
});

test("a dropped skinned body brings its own clips, installed under their id", async () => {
  const result = await validateLoose("man.glb", soldierGlb(), empty, context, {
    loops: ["idle", "walk"],
    yaw: 90,
  });
  expect(result.yaw).toBe(90);
  expect(result.clips!.preview!.clips.find((c) => c.name === "walk")!.loop).toBe(true);
  expect(result.clips!.preview!.clips.find((c) => c.name === "run")!.loop).toBe(false);
  const body = result.appearance!.preview as SkinnedBundle;
  const files = await previewRuntime(
    [{ name: "man.glb", unit: "soldier", bundle: body, clips: result.clips!.preview! }],
    testCatalog().sides,
  );
  const installed = await new AppearanceLibrary(memoryFetch(files, "mem:/")).load("mem:/");
  expect(installed.skeletons.get("adhoc")!.clips.length).toBe(SOLDIER_CLIPS.length);
});

test("a dropped kit preview uses the same gzip transport and installs its actual module geometry", async () => {
  const { kitGlb, cityCatalog, KIT_SOURCE } = await import("./city");
  const { encodeBundle } = await import("@packages/scene-assets/src/codec.ts");
  const result = await validateLoose(KIT_SOURCE, kitGlb(), cityCatalog(), context, { unit: "kit" });
  const preview = result.appearance!.preview!;
  if (preview.kind !== "static") throw new Error(preview.kind);
  const files = await previewRuntime(
    [{ name: "kit.glb", unit: "kit", bundle: preview }],
    testCatalog().sides,
  );
  const loader = new AppearanceLibrary(memoryFetch(files, "mem:/"));
  await loader.load("mem:/");
  const installed = await loader.withAppearances(["kit.glb"]);
  expect(encodeBundle(installed.appearances.get("kit.glb")!.bundle)).toEqual(encodeBundle(preview));
});
