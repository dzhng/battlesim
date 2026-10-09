// @vitest-environment node
import { createServer, type Server } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import * as filesystem from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { SoundWorkbenchStore, soundWorkbenchMiddleware } from "../../apps/sound-workbench/server";
import { validateSoundCatalog, type SoundCatalog } from "@packages/battle-audio/src/catalog";
import { SOUNDS } from "@packages/battle-audio/src/synth";

vi.mock("node:fs/promises", async (importOriginal) => ({
  ...(await importOriginal<typeof import("node:fs/promises")>()),
  rename: vi.fn((await importOriginal<typeof import("node:fs/promises")>()).rename),
}));

const roots: string[] = [];
const servers: Server[] = [];
afterEach(async () => {
  vi.restoreAllMocks();
  vi.mocked(filesystem.rename).mockImplementation(
    (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises")).rename,
  );
  await Promise.all(
    servers.splice(0).map((server) => new Promise<void>((done) => server.close(() => done()))),
  );
  await Promise.all(roots.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function fixture() {
  const root = await mkdtemp(join(tmpdir(), "sound-workbench-"));
  roots.push(root);
  await mkdir(join(root, "fixtures"));
  const baseline: SoundCatalog = {
    sources: {},
    clips: {},
    sounds: Object.fromEntries(
      Object.entries(SOUNDS).map(([id, sound]) => [
        id,
        { label: `Synth · ${id}`, clips: [], synth: id, synth_gain: 1, gain: 1, loop: sound.loop },
      ]),
    ),
    defaults: {},
    impacts: {},
    effects: {},
  };
  const text = JSON.stringify(baseline); // Preserve unformatted bytes on no-op.
  await writeFile(join(root, "fixtures/sounds.json"), text);
  await writeFile(
    join(root, "fixtures/game.json"),
    JSON.stringify({
      weapons: { rifle: { speed_mps: 900 }, carbine: { extends: "rifle", speed_mps: 850 } },
      presentation: {
        audio: {
          shots: {
            default: { near: "rifle", far: "rifle_far", gain: 0.35, far_m: 300 },
            rifle: { near: "rifle", far: "rifle_far", gain: 0.35, far_m: 300 },
          },
          impacts: {
            ground: { sound: "impact_ground", gain: 0.2 },
            hull: { sound: "impact_hull", gain: 0.4 },
          },
          vehicles: {},
          footsteps: { sound: "footstep", gain: 0.07, stride_m: 0.8, max_step_m: 2 },
        },
        pose: { gait: { run_mps: 2.2 } },
      },
    }),
  );
  const store = new SoundWorkbenchStore(root, validateSoundCatalog, Object.keys(SOUNDS));
  const middleware = soundWorkbenchMiddleware(store);
  const server = createServer(
    (req, res) =>
      void middleware(req, res, () => {
        res.statusCode = 404;
        res.end();
      }),
  );
  servers.push(server);
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const address = server.address();
  if (!address || typeof address === "string") throw new Error("No local server address");
  const origin = `http://127.0.0.1:${address.port}`;
  async function request(endpoint: string, body?: unknown, headers: Record<string, string> = {}) {
    const response = await fetch(
      `${origin}/__sound-workbench/${endpoint}`,
      body === undefined
        ? undefined
        : {
            method: "POST",
            headers: { Origin: origin, "Content-Type": "application/json", ...headers },
            body: JSON.stringify(body),
          },
    );
    return { status: response.status, body: await response.json() };
  }
  return { root, text, store, request };
}

test("HTTP preview is read-only; a reviewed save gives a derived weapon row its own choice", async () => {
  const { root, text, request } = await fixture();
  const initial = (await request("snapshot")).body;
  // The snapshot carries each row's lineage, not the rest of its row.
  expect(initial.weapons).toEqual({ rifle: {}, carbine: { extends: "rifle" } });
  const catalog: SoundCatalog = structuredClone(initial.catalog);
  catalog.defaults.carbine = { near: "hmg", far: "hmg_far", gain: 0.7 };
  const preview = await request("preview", { revision: initial.revision, catalog });
  expect(preview.status).toBe(200);
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
  expect(preview.body.files[0].after).toContain('"carbine"');
  expect(
    (await request("save", { candidateId: preview.body.candidateId, revision: initial.revision }))
      .status,
  ).toBe(200);
  const saved = (await request("snapshot")).body;
  expect(saved.catalog.defaults.carbine.near).toBe("hmg");
  expect(saved.catalog.defaults.rifle).toBeUndefined();
  expect(
    (await request("save", { candidateId: preview.body.candidateId, revision: initial.revision }))
      .status,
  ).toBe(200);
});

test("no-op preview/save preserves source bytes and stale reviewed saves preserve outside edits", async () => {
  const { root, text, request } = await fixture();
  const snapshot = (await request("snapshot")).body;
  const noop = (
    await request("preview", { revision: snapshot.revision, catalog: snapshot.catalog })
  ).body;
  expect(noop.files).toEqual([]);
  await request("save", { candidateId: noop.candidateId, revision: snapshot.revision });
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
  const catalog = structuredClone(snapshot.catalog);
  catalog.defaults.rifle = { near: "hmg", far: "hmg_far", gain: 1 };
  const review = (await request("preview", { revision: snapshot.revision, catalog })).body;
  const outside = `${text}\n`;
  await writeFile(join(root, "fixtures/sounds.json"), outside);
  expect(
    (await request("save", { candidateId: review.candidateId, revision: snapshot.revision }))
      .status,
  ).toBe(409);
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(outside);
});

test("HTTP refuses invalid identity, malformed clips, changed baselines and client paths", async () => {
  const { root, text, request } = await fixture();
  const snapshot = (await request("snapshot")).body;
  for (const mutate of [
    (c: SoundCatalog) => {
      // A per-unit override the game no longer reads.
      Object.assign(c, {
        units: { alpha: { rifle: { near: "rifle", far: "rifle_far", gain: 1 } } },
      });
    },
    (c: SoundCatalog) => {
      c.defaults.unknown = { near: "rifle", far: "rifle_far", gain: 1 };
    },
    (c: SoundCatalog) => {
      c.sounds.rifle.gain = 0.2;
    },
    (c: SoundCatalog) => {
      c.clips.unknown = {} as never;
    },
    (c: SoundCatalog) => {
      c.impacts.unknown = { rifle: "impact_ground" };
    },
  ]) {
    const catalog = structuredClone(snapshot.catalog);
    mutate(catalog);
    expect((await request("preview", { revision: snapshot.revision, catalog })).status).toBe(400);
  }
  expect(
    (
      await request("preview", {
        revision: snapshot.revision,
        catalog: snapshot.catalog,
        path: "fixtures/game.json",
      })
    ).status,
  ).toBe(400);
  expect(
    (await request("save", { candidateId: "unknown", revision: snapshot.revision })).status,
  ).toBe(409);
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
});

test("HTTP local-origin and request-size bounds apply before editing", async () => {
  const { request } = await fixture();
  expect((await request("preview", {}, { Origin: "https://example.com" })).status).toBe(403);
  expect((await request("preview", { oversized: "x".repeat(1024 * 1024 + 1) })).status).toBe(413);
});

test("snapshot recovers an interrupted sound publication before exposing sources", async () => {
  const { root, text, store } = await fixture();
  await mkdir(join(root, "throwaway"));
  const after = `${text}\n`;
  await writeFile(join(root, "fixtures/sounds.json"), after);
  await writeFile(
    join(root, "throwaway/sound-workbench-transaction.json"),
    JSON.stringify({
      pid: 2147483647,
      files: [{ path: "fixtures/sounds.json", before: text, after }],
    }),
  );
  await store.snapshot();
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
});

test("outside changes during publication survive its rollback", async () => {
  const { root, request, text } = await fixture();
  const initial = (await request("snapshot")).body;
  const catalog = structuredClone(initial.catalog);
  catalog.defaults.rifle = { near: "hmg", far: "hmg_far", gain: 1 };
  const review = (await request("preview", { revision: initial.revision, catalog })).body;
  const rename = (await vi.importActual<typeof import("node:fs/promises")>("node:fs/promises"))
    .rename;
  const outside = `${text}\n\n`;
  let changed = false;
  vi.mocked(filesystem.rename).mockImplementation(async (from, to) => {
    await rename(from, to);
    if (!changed && String(to) === join(root, "fixtures/sounds.json")) {
      changed = true;
      await writeFile(join(root, "fixtures/sounds.json"), outside);
    }
  });
  expect(
    (await request("save", { candidateId: review.candidateId, revision: initial.revision })).status,
  ).toBe(409);
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(outside);
});

async function installRecording(root: string) {
  const document = JSON.parse(
    await readFile(join(root, "fixtures/sounds.json"), "utf8"),
  ) as SoundCatalog;
  const bytes = Buffer.from("immutable cleaned clip bytes");
  document.sources.master = {
    label: "Master",
    author: "Test",
    license: "CC0-1.0",
    url: "https://example.com/master",
    path: "assets/third-party/audio/master.wav",
    sha256: "a".repeat(64),
    notes: "",
  };
  document.clips.reload = {
    label: "Unused reload",
    category: "mechanical",
    role: "reload",
    source: "master",
    source_rate: 48000,
    source_frames: [0, 100],
    processing: "clean",
    url: "/audio/clips/reload.wav",
    sha256: createHash("sha256").update(bytes).digest("hex"),
    sample_rate: 48000,
    frames: 100,
    loop: false,
    notes: "",
  };
  document.sounds.reload = {
    label: "Reload recipe",
    clips: ["reload"],
    synth: null,
    synth_gain: 0,
    gain: 1,
    loop: false,
  };
  await mkdir(join(root, "assets/runtime/audio/clips"), { recursive: true });
  await writeFile(join(root, "assets/runtime/audio/clips/reload.wav"), bytes);
  const source = JSON.stringify(document);
  await writeFile(join(root, "fixtures/sounds.json"), source);
  return source;
}

test("unused clean media is admitted, firing rejects reloads, and changed clip bytes refuse publication", async () => {
  const { root, request } = await fixture();
  const source = await installRecording(root);
  const initial = (await request("snapshot")).body;
  expect(initial.catalog.clips.reload.label).toBe("Unused reload");
  const invalid = structuredClone(initial.catalog);
  invalid.defaults.rifle = { near: "reload", far: "reload", gain: 1 };
  expect((await request("preview", { revision: initial.revision, catalog: invalid })).status).toBe(
    400,
  );
  const catalog = structuredClone(initial.catalog);
  catalog.effects.ricochet = "reload";
  const review = (await request("preview", { revision: initial.revision, catalog })).body;
  await writeFile(
    join(root, "assets/runtime/audio/clips/reload.wav"),
    Buffer.from("outside media edit"),
  );
  const refused = await request("save", {
    candidateId: review.candidateId,
    revision: initial.revision,
  });
  expect(refused.status).toBe(400);
  expect(refused.body.error).toContain("Clip reload");
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(source);
});

test.each(["source", "clip"])(
  "HTTP refuses a schema-valid %s provenance edit without changing bytes",
  async (kind) => {
    const { root, request } = await fixture();
    const source = await installRecording(root);
    const initial = (await request("snapshot")).body;
    const catalog: SoundCatalog = structuredClone(initial.catalog);
    if (kind === "source") catalog.sources.master.label = "Edited master";
    else catalog.clips.reload.label = "Edited reload";
    // The request must reach the immutable-provenance guard, not generic schema validation.
    expect(() => validateSoundCatalog(catalog)).not.toThrow();
    const result = await request("preview", { revision: initial.revision, catalog });
    expect(result.status).toBe(400);
    expect(result.body.error).toBe("Source and clip metadata are immutable in this editor");
    expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(source);
  },
);
