import { expect, test } from "vitest";
import type { PreparedAppearances, Renderer } from "@wasm/renderer_browser";
import type { ScenePresentation } from "../src/battle/sim/presentation";
import type { ModelAsset } from "../src/battle/render/modelAssets";
import type { LabModel } from "../src/battle/sim/presentationModels";
import { SceneAppearances } from "../src/battle/render/sceneAppearances";

const detail = { lod_px: [100, 50, 25] as const, impostor_px: 10 };
const model = (name: string, x = 0): LabModel => ({
  appearance: name,
  x,
  y: 0,
  z: 0,
  yaw: 0,
  pose: { kind: "rest" },
});
const assets = (name: string): ModelAsset[] => [
  {
    name,
    bundle: new Uint8Array(),
    textureIds: [],
    textureBytes: [],
  },
];

function stage() {
  let visible = "C";
  let requests: readonly LabModel[] = [model("C")];
  const pending: {
    name: string;
    finish: () => void;
    fail: (error: Error) => void;
  }[] = [];
  const freed: string[] = [];
  const errors: unknown[] = [];
  const presentation = {
    prepareRendererAppearances(_renderer: unknown, assets: readonly ModelAsset[]) {
      const name = assets[0].name;
      return new Promise<PreparedAppearances>((resolve, reject) => {
        pending.push({
          name,
          finish: () =>
            resolve({ name, free: () => freed.push(name) } as unknown as PreparedAppearances),
          fail: reject,
        });
      });
    },
    commitRendererAppearances(
      _renderer: unknown,
      ready: PreparedAppearances,
      models: readonly LabModel[],
    ) {
      visible = (ready as unknown as { name: string }).name;
      requests = models;
      return true;
    },
    setLabModels(models: readonly LabModel[]) {
      requests = models;
    },
  };
  const owner = new SceneAppearances(
    {} as Renderer,
    presentation as unknown as ScenePresentation,
    detail,
    () => {},
    (error: unknown) => errors.push(error),
  );
  return { owner, pending, freed, errors, visible: () => visible, models: () => requests };
}

test("a complete C remains active while A prepares and B supersedes it; only latest B commits", async () => {
  const scene = stage();
  const first = scene.owner.request(assets("A"), [model("A")]);
  const latest = scene.owner.request(assets("B"), [model("B")]);
  scene.owner.models([model("B", 7)]);
  expect(scene.visible()).toBe("C");
  expect(scene.models()).toEqual([model("C")]);
  expect(scene.pending.map((p) => p.name)).toEqual(["A"]);
  scene.pending[0].finish();
  await first;
  await Promise.resolve();
  expect(scene.visible()).toBe("C");
  expect(scene.freed).toEqual(["A"]);
  expect(scene.pending.map((p) => p.name)).toEqual(["A", "B"]);
  scene.pending[1].finish();
  await Promise.resolve();
  expect(scene.visible()).toBe("C");
  expect(scene.owner.flush()).toBe(true);
  expect(await latest).toBe(true);
  expect(scene.visible()).toBe("B");
  expect(scene.models()).toEqual([model("B", 7)]);
  scene.owner.dispose();
});

test("a failed newest generation retains committed art and poses without retrying", async () => {
  const scene = stage();
  const committed = scene.owner.request(assets("C"), [model("C", 2)]);
  scene.pending[0].finish();
  await Promise.resolve();
  expect(scene.owner.flush()).toBe(true);
  await committed;
  const requested = scene.owner.request(assets("B"), [model("B")]);
  scene.owner.models([model("B", 11)]);
  const error = new Error("B atlas failed");
  scene.pending[1].fail(error);
  expect(await requested).toBe(false);
  expect(scene.owner.flush()).toBe(false);
  expect(scene.visible()).toBe("C");
  expect(scene.models()).toEqual([model("C", 2)]);
  expect(scene.errors).toEqual([error]);
  expect(scene.pending.map((p) => p.name)).toEqual(["C", "B"]);
  await expect(scene.owner.settled()).rejects.toThrow("B atlas failed");
  scene.owner.dispose();
});

test("departure discards pending and ready assets without committing or starting queued work", async () => {
  const scene = stage();
  const first = scene.owner.request(assets("A"), [model("A")]);
  const latest = scene.owner.request(assets("B"), [model("B")]);
  scene.owner.dispose();
  scene.pending[0].finish();
  await Promise.resolve();
  expect(await first).toBe(false);
  expect(await latest).toBe(false);
  expect(scene.freed).toEqual(["A"]);
  expect(scene.pending.map((p) => p.name)).toEqual(["A"]);
  expect(scene.owner.flush()).toBe(false);
  expect(scene.visible()).toBe("C");
  expect(scene.errors).toEqual([]);

  const ready = stage();
  const waiting = ready.owner.request(assets("A"), [model("A")]);
  ready.pending[0].finish();
  await Promise.resolve();
  ready.owner.dispose();
  expect(await waiting).toBe(false);
  expect(ready.freed).toEqual(["A"]);
  expect(ready.owner.flush()).toBe(false);
  expect(ready.visible()).toBe("C");
});
