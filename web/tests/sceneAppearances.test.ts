import { expect, test } from "vitest";
import type { Presentation, PreparedAppearances, Renderer } from "@wasm/renderer_browser";
import type { ScenePresentation } from "../src/battle/sim/presentation";
import type { ModelAsset } from "../src/battle/render/modelAssets";
import { prepareAppearances, type LabModel } from "../src/battle/sim/presentationModels";
import { SceneAppearances } from "../src/battle/render/sceneAppearances";

const detail = { lod_px: [100, 50, 25] as const, impostor_px: 10, far_px: 2 };
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
    contentId: { bundle: "a".repeat(64), skeleton: null },
    encode: () => ({ bundle: new Uint8Array(), textureIds: [], textureBytes: [] }),
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

/** Exercise admission scheduling and the real page transport across a fake WASM seam. */
function transportStage() {
  const encoded: string[] = [];
  const sent: { content: string; bytes: number[] }[] = [];
  const pending: { names: string[]; finish: () => void }[] = [];
  let committed: string[] = ["A"];
  let held = new Set(["a".repeat(64)]);
  const asset = (name: string, hash: string, byte: number): ModelAsset => ({
    name,
    contentId: { bundle: hash.repeat(64), skeleton: null },
    encode() {
      encoded.push(name);
      return { bundle: Uint8Array.of(byte), textureIds: [], textureBytes: [] };
    },
  });
  const renderer = {
    missing_content_ids(_presentation: unknown, json: string) {
      return JSON.stringify(
        JSON.parse(json).filter((id: { bundle: string }) => !held.has(id.bundle)),
      );
    },
    appearance_draft() {
      const declared = new Map<string, string>();
      return {
        declared,
        declare(name: string, json: string) {
          declared.set(name, JSON.parse(json).bundle);
        },
        add_bundle(json: string, bytes: Uint8Array) {
          sent.push({ content: JSON.parse(json).bundle, bytes: [...bytes] });
        },
        free() {},
      };
    },
    prepare_appearances(_presentation: unknown, draft: { declared: Map<string, string> }) {
      return new Promise<PreparedAppearances>((resolve) =>
        pending.push({
          names: [...draft.declared.keys()],
          finish: () =>
            resolve({ declared: draft.declared, free() {} } as unknown as PreparedAppearances),
        }),
      );
    },
  };
  const presentation = {
    prepareRendererAppearances(_renderer: Renderer, assets: readonly ModelAsset[]) {
      return prepareAppearances(
        renderer as unknown as Renderer,
        {} as Presentation,
        assets,
        detail,
      );
    },
    commitRendererAppearances(_renderer: Renderer, ready: { declared: Map<string, string> }) {
      committed = [...ready.declared.keys()];
      held = new Set(ready.declared.values());
      return true;
    },
    setLabModels() {},
  };
  const owner = new SceneAppearances(
    renderer as unknown as Renderer,
    presentation as unknown as ScenePresentation,
    detail,
    () => {},
    (error) => {
      throw error;
    },
    { assets: [asset("A", "a", 1)], models: [] },
  );
  return { owner, asset, encoded, sent, pending, committed: () => committed };
}

test("fresh objects with the same name/content set leave committed art current", async () => {
  const scene = transportStage();
  const result = scene.owner.request([scene.asset("A", "a", 1)], []);
  expect(scene.pending).toEqual([]);
  expect(await result).toBe(true);
  expect(scene.owner.current()).toBe(true);
  expect(scene.encoded).toEqual([]);
  expect(scene.sent).toEqual([]);
  scene.owner.dispose();
});

test("growth and replacement send only missing content, and a duplicate in flight stays committable", async () => {
  const scene = transportStage();
  const wanted = [
    scene.asset("A", "a", 1),
    scene.asset("B", "b", 2),
    scene.asset("B-alias", "b", 2),
  ];
  const growth = scene.owner.request(wanted, []);
  const repeated = scene.owner.request(
    [scene.asset("B-alias", "b", 2), scene.asset("B", "b", 2), scene.asset("A", "a", 1)],
    [],
  );
  expect(scene.pending.map((p) => p.names)).toEqual([["A", "B", "B-alias"]]);
  expect(scene.encoded).toEqual(["B"]);
  expect(scene.sent).toEqual([{ content: "b".repeat(64), bytes: [2] }]);
  scene.pending[0].finish();
  await Promise.resolve();
  await Promise.resolve();
  expect(scene.owner.flush()).toBe(true);
  expect(await growth).toBe(true);
  expect(await repeated).toBe(true);
  expect(scene.committed()).toEqual(["A", "B", "B-alias"]);
  expect(scene.owner.work()).toEqual({ bundlesEncoded: 1, bundleBytesSent: 1 });

  const replacement = scene.owner.request([scene.asset("A", "c", 3), wanted[1], wanted[2]], []);
  expect(scene.pending.map((p) => p.names)).toEqual([
    ["A", "B", "B-alias"],
    ["A", "B", "B-alias"],
  ]);
  expect(scene.encoded).toEqual(["B", "A"]);
  expect(scene.sent).toEqual([
    { content: "b".repeat(64), bytes: [2] },
    { content: "c".repeat(64), bytes: [3] },
  ]);
  scene.pending[1].finish();
  await Promise.resolve();
  await Promise.resolve();
  expect(scene.owner.flush()).toBe(true);
  expect(await replacement).toBe(true);
  expect(scene.committed()).toEqual(["A", "B", "B-alias"]);
  scene.owner.dispose();
});

test("explicit grass-rule changes still prepare placement with held art", async () => {
  const scene = transportStage();
  const assets = [scene.asset("A", "a", 1)];
  const tuned = scene.owner.request(assets, [], { density: 2 });
  const repeated = scene.owner.request([scene.asset("A", "a", 1)], [], { density: 2 });
  expect(scene.pending.map((p) => p.names)).toEqual([["A"]]);
  expect(scene.encoded).toEqual([]);
  scene.pending[0].finish();
  await Promise.resolve();
  await Promise.resolve();
  expect(scene.owner.flush()).toBe(true);
  expect(await tuned).toBe(true);
  expect(await repeated).toBe(true);
  expect(scene.committed()).toEqual(["A"]);
  scene.owner.dispose();
});

test("geometry transport holds one owned packet and releases it after the draft copies it", async () => {
  const input = [new Uint8Array([1, 2]), new Uint8Array([3, 4, 5]), new Uint8Array([6, 7, 8, 9])];
  const owned: Uint8Array[] = [];
  const copied: Uint8Array[] = [];
  let peak = 0;
  const art: ModelAsset[] = input.map((source, i) => ({
    name: `test-${i}`,
    contentId: { bundle: String(i).repeat(64), skeleton: null },
    encode() {
      const bundle = new Uint8Array(source);
      owned.push(bundle);
      peak = Math.max(
        peak,
        owned.reduce((sum, packet) => sum + packet.byteLength, 0),
      );
      return { bundle, textureIds: [], textureBytes: [] };
    },
  }));
  const draft = {
    declare() {},
    add_bundle(_id: string, bundle: Uint8Array) {
      copied.push(new Uint8Array(bundle));
    },
    free() {},
  };
  const renderer = {
    missing_content_ids: () => JSON.stringify(art.map((asset) => asset.contentId)),
    appearance_draft: () => draft,
    prepare_appearances: async () => ({ free() {} }),
  } as unknown as Parameters<typeof prepareAppearances>[0];
  await prepareAppearances(renderer, {} as Presentation, art, detail);
  expect(copied.map((bytes) => Array.from(bytes))).toEqual(input.map((bytes) => Array.from(bytes)));
  expect(owned.map((packet) => packet.byteLength)).toEqual(input.map(() => 0));
  expect(peak).toBe(Math.max(...input.map((packet) => packet.byteLength)));
});

test("geometry transport retires its packet when the draft refuses it", () => {
  const packet = new Uint8Array([7, 9, 11]);
  let freed = 0;
  const art: ModelAsset[] = [
    {
      name: "test-rejected",
      contentId: { bundle: "a".repeat(64), skeleton: null },
      encode: () => ({ bundle: packet, textureIds: [], textureBytes: [] }),
    },
  ];
  const renderer = {
    missing_content_ids: () => JSON.stringify(art.map((asset) => asset.contentId)),
    appearance_draft: () => ({
      declare() {},
      add_bundle() {
        throw new Error("test draft refuses the source");
      },
      free() {
        freed++;
      },
    }),
  } as unknown as Parameters<typeof prepareAppearances>[0];
  expect(() => prepareAppearances(renderer, {} as Presentation, art, detail)).toThrow(
    "test draft refuses the source",
  );
  expect(packet.byteLength).toBe(0);
  expect(freed).toBe(1);
});
