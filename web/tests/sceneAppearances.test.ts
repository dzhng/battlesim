import { afterEach, beforeAll, beforeEach, expect, test, vi } from "vitest";
import type { Presentation, PreparedAppearances, Renderer } from "@wasm/renderer_browser";
import type { ScenePresentation } from "../src/battle/sim/presentation";
import type { ModelAsset } from "../src/battle/render/modelAssets";
import { prepareAppearances, type LabModel } from "../src/battle/sim/presentationModels";
import { SceneAppearances } from "../src/battle/render/sceneAppearances";

beforeAll(async () => {
  const { loadPresentation } = await import("../src/battle/sim/presentation");
  await loadPresentation();
});
let testFrame = 0;
beforeEach(() => vi.stubGlobal("document", { timeline: { currentTime: ++testFrame } }));
afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

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
    begin_appearances(_presentation: unknown, draft: { declared: Map<string, string> }) {
      return { declared: draft.declared, step: () => false, free() {} };
    },
    prepare_appearances(draft: { declared: Map<string, string> }) {
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
    prepareRendererAppearances(
      _renderer: Renderer,
      assets: readonly ModelAsset[],
      _detail: unknown,
      nextFrame: () => Promise<boolean>,
    ) {
      return prepareAppearances(
        renderer as unknown as Renderer,
        {} as Presentation,
        assets,
        detail,
        nextFrame,
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

async function frames(owner: SceneAppearances, count = 12) {
  for (let i = 0; i < count; i++) {
    Object.assign(document.timeline, { currentTime: Number(document.timeline.currentTime) + 1 });
    owner.presented();
    await Promise.resolve();
    await Promise.resolve();
  }
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
  await frames(scene.owner);
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
  await frames(scene.owner);
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
  await frames(scene.owner);
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
    begin_appearances: () => ({ step: () => false, free() {} }),
    prepare_appearances: async () => ({ free() {} }),
  } as unknown as Parameters<typeof prepareAppearances>[0];
  await prepareAppearances(renderer, {} as Presentation, art, detail, async () => true);
  expect(copied.map((bytes) => Array.from(bytes))).toEqual(input.map((bytes) => Array.from(bytes)));
  expect(owned.map((packet) => packet.byteLength)).toEqual(input.map(() => 0));
  expect(peak).toBe(Math.max(...input.map((packet) => packet.byteLength)));
});

test("geometry transport retires its packet when the draft refuses it", async () => {
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
  await expect(
    prepareAppearances(renderer, {} as Presentation, art, detail, async () => true),
  ).rejects.toThrow("test draft refuses the source");
  expect(packet.byteLength).toBe(0);
  expect(freed).toBe(1);
});

test("a growth spends its frame budget before yielding, then finishes the same generation", async () => {
  const { loadPresentation } = await import("../src/battle/sim/presentation");
  const { art_work_budget_ms } = await import("@wasm/renderer_browser");
  await loadPresentation();
  const budget = art_work_budget_ms();
  let now = 0;
  const clock = vi.spyOn(performance, "now").mockImplementation(() => now);
  let frame = 0;
  vi.stubGlobal("document", {
    timeline: {
      get currentTime() {
        return frame;
      },
    },
  });
  const steps: { frame: number; operation: string }[] = [];
  let release: ((go: boolean) => void) | undefined;
  const nextFrame = (yieldFrame = true) =>
    !yieldFrame
      ? Promise.resolve(true)
      : new Promise<boolean>((resolve) => {
          release = resolve;
        });
  const art = [0, 1, 2].map((i) => ({
    name: `body-${i}`,
    contentId: { bundle: String(i).repeat(64), skeleton: null },
    encode: () => ({ bundle: Uint8Array.of(i), textureIds: [], textureBytes: [] }),
  }));
  let warmed = 0;
  const renderer = {
    missing_content_ids: () => JSON.stringify(art.map((a) => a.contentId)),
    appearance_draft: () => ({
      declare() {},
      add_bundle(id: string) {
        now += budget / 5;
        steps.push({ frame, operation: `supply ${JSON.parse(id).bundle}` });
      },
      free() {},
    }),
    begin_appearances() {
      now += budget / 5;
      steps.push({ frame, operation: "catalog" });
      return {
        step() {
          now += budget / 5;
          steps.push({ frame, operation: "warm" });
          return warmed++ < art.length;
        },
        free() {},
      };
    },
    async prepare_appearances() {
      now += budget / 5;
      steps.push({ frame, operation: "admit" });
      return { free() {} };
    },
  } as unknown as Parameters<typeof prepareAppearances>[0];
  const pending = prepareAppearances(renderer, {} as Presentation, art, detail, nextFrame);
  for (let i = 0; i < 10; i++) {
    for (let task = 0; task < 30; task++) await Promise.resolve();
    frame++;
    release?.(true);
  }
  const result = await pending;
  expect(result).not.toBeNull();
  expect(steps.filter((s) => s.operation.startsWith("supply")).map((s) => s.operation)).toEqual(
    art.map((a) => `supply ${a.contentId.bundle}`),
  );
  expect(steps.slice(0, 3).map((s) => s.frame)).toEqual(
    steps.slice(0, 3).map(() => steps[0].frame),
  );
  expect(new Set(steps.map((s) => s.frame)).size).toBeLessThan(steps.length);
  expect(new Set(steps.map((s) => s.frame)).size).toBeGreaterThan(1);
  clock.mockRestore();
  expect(steps.at(-1)?.operation).toBe("admit");
});

test("superseding a partial supply or warm discards it and commits only the latest catalog", async () => {
  const { art_work_budget_ms } = await import("@wasm/renderer_browser");
  let now = 0;
  vi.spyOn(performance, "now").mockImplementation(() => now);
  for (const supersedeAt of ["supply", "warm"]) {
    let phase = "";
    let visible = ["live"];
    const committed: string[][] = [];
    const retired: string[] = [];
    const errors: unknown[] = [];
    const art = (name: string, id: string): ModelAsset => ({
      name,
      contentId: { bundle: id.repeat(64), skeleton: null },
      encode: () => ({ bundle: Uint8Array.of(id.charCodeAt(0)), textureIds: [], textureBytes: [] }),
    });
    const renderer = {
      missing_content_ids: (_: unknown, ids: string) => ids,
      appearance_draft() {
        const names: string[] = [];
        return {
          names,
          declare(name: string) {
            names.push(name);
          },
          add_bundle() {
            phase = "supply";
            now += art_work_budget_ms();
          },
          free() {
            retired.push(`supplied ${names.join()}`);
          },
        };
      },
      begin_appearances(_: unknown, draft: { names: string[] }) {
        let remaining = draft.names.length;
        return {
          names: draft.names,
          step() {
            phase = "warm";
            now += art_work_budget_ms();
            return remaining-- > 0;
          },
          free() {
            retired.push(`warmed ${draft.names.join()}`);
          },
        };
      },
      async prepare_appearances(warming: { names: string[] }) {
        return {
          names: warming.names,
          free() {
            retired.push(`ready ${warming.names.join()}`);
          },
        };
      },
    } as unknown as Renderer;
    const presentation = {
      prepareRendererAppearances(
        r: Renderer,
        a: readonly ModelAsset[],
        d: typeof detail,
        next: (yieldFrame: boolean) => Promise<boolean>,
      ) {
        return prepareAppearances(r, {} as Presentation, a, d, next);
      },
      commitRendererAppearances(_: unknown, ready: { names: string[] }) {
        committed.push(ready.names);
        visible = ready.names;
        return true;
      },
      setLabModels() {},
    } as unknown as ScenePresentation;
    const owner = new SceneAppearances(
      renderer,
      presentation,
      detail,
      () => {},
      (error) => errors.push(error),
    );
    try {
      const old = owner.request([art("a", "a"), art("b", "b")], []);
      for (let frame = 0; phase !== supersedeAt && frame < 20; frame++) {
        await frames(owner, 1);
      }
      expect(phase).toBe(supersedeAt);
      expect(owner.flush()).toBe(false);
      expect(visible).toEqual(["live"]);
      const latest = owner.request([art("new", "c")], []);
      expect(await old).toBe(false);
      await frames(owner, 20);
      expect(visible).toEqual(["live"]);
      expect(owner.flush()).toBe(true);
      expect(await latest).toBe(true);
      expect(owner.flush()).toBe(false);
      expect(visible).toEqual(["new"]);
      expect(committed).toEqual([["new"]]);
      expect(retired).toEqual([`${supersedeAt === "supply" ? "supplied" : "warmed"} a,b`]);
      expect(errors).toEqual([]);
    } finally {
      owner.dispose();
    }
  }
});

test.each(["supply", "warm"])(
  "departure during a %s budget wait settles false and frees its held candidate once",
  async (phase) => {
    const { art_work_budget_ms } = await import("@wasm/renderer_browser");
    let now = 0;
    vi.spyOn(performance, "now").mockImplementation(() => now);
    const draftFreed = vi.fn();
    const warmingFreed = vi.fn();
    const commit = vi.fn();
    const prepare = vi.fn();
    const redraw = vi.fn();
    const errors = vi.fn();
    const renderer = {
      missing_content_ids: (_: unknown, ids: string) => ids,
      appearance_draft: () => ({
        declare() {},
        add_bundle() {
          if (phase === "supply") now += art_work_budget_ms();
        },
        free: draftFreed,
      }),
      begin_appearances: () => ({
        step() {
          now += art_work_budget_ms();
          return true;
        },
        free: warmingFreed,
      }),
      prepare_appearances: prepare,
    } as unknown as Renderer;
    const presentation = {
      prepareRendererAppearances(
        r: Renderer,
        art: readonly ModelAsset[],
        d: typeof detail,
        next: (yieldFrame: boolean) => Promise<boolean>,
      ) {
        return prepareAppearances(r, {} as Presentation, art, d, next);
      },
      commitRendererAppearances: commit,
    } as unknown as ScenePresentation;
    const owner = new SceneAppearances(renderer, presentation, detail, redraw, errors);
    const requested = owner.request(assets("arriving"), []);
    // A redraw request means the real admission helper spent its budget and
    // is waiting for SceneAppearances to hear that a frame was presented.
    await vi.waitFor(() => expect(redraw).toHaveBeenCalledTimes(1));
    expect(draftFreed).not.toHaveBeenCalled();
    expect(warmingFreed).not.toHaveBeenCalled();
    owner.dispose();
    expect(await requested).toBe(false);
    await vi.waitFor(() =>
      expect(phase === "supply" ? draftFreed : warmingFreed).toHaveBeenCalledTimes(1),
    );
    owner.dispose();
    owner.presented();
    expect(owner.flush()).toBe(false);
    expect(draftFreed).toHaveBeenCalledTimes(phase === "supply" ? 1 : 0);
    expect(warmingFreed).toHaveBeenCalledTimes(phase === "warm" ? 1 : 0);
    expect(prepare).not.toHaveBeenCalled();
    expect(commit).not.toHaveBeenCalled();
    expect(errors).not.toHaveBeenCalled();
  },
);
