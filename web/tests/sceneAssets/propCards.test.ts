// @vitest-environment node
import { expect, test } from "vitest";
import { bakeCatalog, runtimeCatalogText } from "@packages/scene-assets/src/bake";
import { AppearanceLibrary, memoryFetch } from "@packages/scene-assets/src/loader";
import { staticCardPose } from "@packages/scene-assets/src/propCards";
import { restingModelPose } from "@packages/battle-renderer/src/models/modelInstances";
import { farPoseBounds } from "@packages/scene-assets/src/pose";
import { AUTHORITY, testCatalog, testSources } from "./synthetic";

test("asset opt-in survives bake and load, using the opaque default pose and its bounds", async () => {
  const catalog = testCatalog();
  catalog.appearances.crate.far_card_px = 2;
  const sources = testSources();
  const result = await bakeCatalog(
    catalog,
    async (path) => sources[path],
    { authority: AUTHORITY },
    ["crate"],
  );
  expect(result.ok).toBe(true);
  const files = new Map(result.files);
  files.set("catalog.json", new TextEncoder().encode(runtimeCatalogText(result.runtime)));
  const installed = await new AppearanceLibrary(memoryFetch(files, "/art/")).load("/art/");
  const art = installed.appearances.get("crate")!;
  expect(staticCardPose(art)).toEqual({
    pose: { kind: "static", state: "default" },
    bounds: farPoseBounds(art.bundle, null),
  });
  expect(staticCardPose(art)!.pose).toEqual(restingModelPose(art.bundle, null));
  expect(staticCardPose({ ...art, farCardPx: 0 })).toBeNull();
  expect(
    staticCardPose({
      ...art,
      bundle: {
        ...art.bundle,
        materials: art.bundle.materials.map((m) => ({
          ...m,
          coverage: { kind: "blended" as const },
        })),
      },
    }),
  ).toBeNull();
  if (art.bundle.kind !== "static") throw new Error("expected static test art");
  expect(
    staticCardPose({
      ...art,
      bundle: { ...art.bundle, states: art.bundle.states.map((s) => ({ ...s, name: "wreck" })) },
    }),
  ).toBeNull();
});
