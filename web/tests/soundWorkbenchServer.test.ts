// @vitest-environment node
import { check_presentation, check_battle_sounds } from "@wasm/renderer_browser";
import authoredPresentation from "@fixtures/presentation.json";
import authoredAudio from "@fixtures/audio.json";
import authoredRules from "@fixtures/rules.json";
import { standInPresentationRecordings } from "./support/soundStandIns";
import type { AudioMix } from "@packages/battle-audio/src/audioMix";
import { createServer, type Server } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import * as filesystem from "node:fs/promises";
import { createHash } from "node:crypto";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test, vi } from "vitest";
import { SoundWorkbenchStore, soundWorkbenchMiddleware } from "../../apps/sound-workbench/server";
import {
  validateSoundCatalog,
  type SoundCatalog,
  type SoundSource,
} from "@packages/battle-audio/src/catalog";
import { SOUNDS } from "@packages/battle-audio/src/synthSounds.gen";

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
    music: { moods: { menu: [], tension: [], combat: [] }, tracks: {} },
  };
  const presentation = structuredClone(authoredPresentation);
  presentation.pose.gait.run_mps = 2.2;
  const audio = {
    ...authoredAudio,
    impacts: {
      default: { gain: 0.2 },
      ground: { gain: 0.2 },
      hull: { gain: 0.4 },
    },
    vehicles: { default: authoredAudio.vehicles.default },
    footsteps: { sound: "footstep", gain: 0.07, stride_m: 0.8, max_step_m: 2 },
  } as unknown as AudioMix;
  standInPresentationRecordings(baseline, audio);
  const text = JSON.stringify(baseline); // Preserve unformatted bytes on no-op.
  await writeFile(join(root, "fixtures/sounds.json"), text);
  await writeFile(
    join(root, "fixtures/rules.json"),
    JSON.stringify({
      weapons: authoredRules.weapons,
    }),
  );
  await writeFile(join(root, "fixtures/presentation.json"), JSON.stringify(presentation));
  await writeFile(join(root, "fixtures/audio.json"), JSON.stringify(audio));
  const store = new SoundWorkbenchStore(
    root,
    validateSoundCatalog,
    Object.keys(SOUNDS),
    (presentation, audio, catalog, weapons) => {
      check_presentation(presentation);
      check_battle_sounds(presentation, audio, JSON.stringify(catalog), weapons);
    },
  );
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

test("HTTP preview is read-only; a reviewed save publishes a library recipe", async () => {
  const { root, text, request } = await fixture();
  const initial = (await request("snapshot")).body;
  const catalog: SoundCatalog = structuredClone(initial.catalog);
  catalog.sounds.custom = { ...catalog.sounds.hmg, label: "Custom recipe" };
  const preview = await request("preview", { revision: initial.revision, catalog });
  expect(preview.status).toBe(200);
  expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
  expect(preview.body.files[0].after).toContain('"custom"');
  expect(
    (await request("save", { candidateId: preview.body.candidateId, revision: initial.revision }))
      .status,
  ).toBe(200);
  const saved = (await request("snapshot")).body;
  expect(saved.catalog.sounds.custom.label).toBe("Custom recipe");
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
  catalog.sounds.custom = { ...catalog.sounds.hmg, label: "Custom recipe" };
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
      Object.assign(c, { defaults: { unknown: { near: "rifle", far: "rifle_far", gain: 1 } } });
    },
    (c: SoundCatalog) => {
      c.sounds.rifle.gain = 0.2;
    },
    (c: SoundCatalog) => {
      c.clips.unknown = {} as never;
    },
    (c: SoundCatalog) => {
      Object.assign(c, { impacts: { unknown: { rifle: "impact_ground" } } });
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
        path: "fixtures/rules.json",
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
  catalog.sounds.custom = { ...catalog.sounds.hmg, label: "Custom recipe" };
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
  invalid.defaults = { rifle: { near: "reload", far: "reload", gain: 1 } };
  expect((await request("preview", { revision: initial.revision, catalog: invalid })).status).toBe(
    400,
  );
  const catalog = structuredClone(initial.catalog);
  catalog.sounds.custom = { ...catalog.sounds.reload, label: "Custom reload" };
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
    if (kind === "source") (catalog.sources.master as SoundSource).label = "Edited master";
    else catalog.clips.reload.label = "Edited reload";
    // The request must reach the immutable-provenance guard, not generic schema validation.
    expect(() => validateSoundCatalog(catalog)).not.toThrow();
    const result = await request("preview", { revision: initial.revision, catalog });
    expect(result.status).toBe(400);
    expect(result.body.error).toBe("Source and clip metadata are immutable in this editor");
    expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(source);
  },
);

test("snapshots refuse misspelled presentation and mix fields before editing", async () => {
  for (const document of ["presentation", "audio"]) {
    const { root, request } = await fixture();
    const path = join(root, `fixtures/${document}.json`);
    const value = JSON.parse(await readFile(path, "utf8"));
    value.misspelled_field = 1;
    await writeFile(path, JSON.stringify(value));
    const refusal = await request("snapshot");
    expect(refusal.status).toBe(400);
    expect(refusal.body.error).toContain("misspelled_field");
  }
});

test.each(["missing", "loop"])(
  "preview refuses a %s firing recipe used by a look",
  async (fault) => {
    const { root, text, request } = await fixture();
    const initial = (await request("snapshot")).body;
    const catalog = structuredClone(initial.catalog);
    const shot = authoredPresentation.weapon_looks.tank_sabot.sound.shot.near;
    if (fault === "missing") delete catalog.sounds[shot];
    else catalog.sounds[shot] = { ...catalog.sounds.rotor };
    const result = await request("preview", { revision: initial.revision, catalog });
    expect(result.status).toBe(400);
    expect(result.body.error).toContain("weapon_looks");
    expect(await readFile(join(root, "fixtures/sounds.json"), "utf8")).toBe(text);
  },
);

test("a library with external provenance saves while its receipts stay immutable", async () => {
  const { root, request } = await fixture();
  const path = join(root, "fixtures/sounds.json");
  const catalog = JSON.parse(await readFile(path, "utf8")) as SoundCatalog;
  catalog.provenance = {
    unused: {
      label: "External receipt",
      author: "Test",
      license: "CC0-1.0",
      url: "https://example.com/source",
      sha256: null,
      source_frames: [0, 100],
      processing: "clean",
      download: { tool: "test", version: "1", status: "candidate-not-downloaded" },
      notes: "Unassigned receipt",
    },
  };
  await writeFile(path, JSON.stringify(catalog));
  const snapshot = (await request("snapshot")).body;
  const before = await readFile(path, "utf8");
  const draft = { revision: snapshot.revision, catalog: snapshot.catalog };
  const review = await request("preview", draft);
  expect(review.status).toBe(200);
  expect(
    (await request("save", { candidateId: review.body.candidateId, revision: snapshot.revision }))
      .status,
  ).toBe(200);
  expect(await readFile(path, "utf8")).toBe(before);
  const saved = (await request("snapshot")).body;
  const invalid = { revision: saved.revision, catalog: saved.catalog };
  invalid.catalog = structuredClone(invalid.catalog);
  invalid.catalog.provenance!.unused.notes = "Changed receipt";
  const refused = await request("preview", invalid);
  expect(refused.status).toBe(400);
  expect(refused.body.error).toContain("metadata are immutable");
});
