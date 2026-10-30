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
import {
  ALL_TRIGGERS,
  validateVoiceCatalog,
  type Trigger,
  type VoiceCatalog,
} from "@packages/battle-audio/src/voices/voiceCatalog";
import { recordVerdicts, type TakeRecord } from "@packages/battle-audio/tools/generate/lock";
import {
  GenerationQueue,
  musicWorkbenchMiddleware,
  type GenerateRunner,
} from "../../apps/sound-workbench/generation";
import {
  VoiceWorkbenchStore,
  voiceWorkbenchMiddleware,
} from "../../apps/sound-workbench/voiceStore";

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

// Voices: the review grid's store. The generation tool is the lock's only writer, so the
// store runs it as a child process; tests replace only that process boundary.
const LEAD_MOVE = "tank/lead/move/steady/0";
const LEAD_MOVE_STRESSED = "tank/lead/move/stressed/0";
function voiceCatalog(): VoiceCatalog {
  const line = (text: string) => ({
    steady: [`${text}.`, `${text}, steady.`] as [string, string],
    stressed: [`${text}!`, `${text}, now!`] as [string, string],
  });
  return {
    roles: {
      crew: {
        lines: Object.fromEntries(
          ALL_TRIGGERS.map((trigger) => [trigger, line(trigger)]),
        ) as Record<Trigger, ReturnType<typeof line>>,
      },
    },
    sets: {
      tank: {
        role: "crew",
        country: "us",
        callsign: "Oscar 55",
        processing: "voice-radio-light",
        voices: [
          { id: "lead", speaks: "responses", gender: "male", description: "Lead", voice_id: "v1" },
          { id: "soldier", speaks: "barks", gender: "male", description: "Crew", voice_id: null },
        ],
      },
    },
    assignments: {},
    retakes: {},
  };
}
function takeRecord(text: string, served: string, flagged = false): TakeRecord {
  return {
    text,
    generation_key: "g".repeat(64),
    raw: "assets/source/audio/voices/tank/raw.mp3",
    raw_sha256: "r".repeat(64),
    processing_key: "p".repeat(64),
    served_sha256: served,
    duration_s: 1.2,
    transcript: text,
    match: { score: flagged ? 0.5 : 1, missing: [], overlong: false, flagged },
    provenance: {
      kind: "Generated",
      generator: "test",
      model: "test",
      text,
      date: "2026-10-12",
      plan: "Duet ElevenLabs",
    },
  };
}

async function voiceFixture(
  run: GenerateRunner = async () => {
    throw new Error("unexpected generation run");
  },
) {
  const root = await mkdtemp(join(tmpdir(), "voice-workbench-"));
  roots.push(root);
  await mkdir(join(root, "fixtures"));
  const text = `${JSON.stringify(voiceCatalog(), null, 2)}\n`;
  await writeFile(join(root, "fixtures/voices.json"), text);
  await writeFile(
    join(root, "fixtures/voices.lock.json"),
    JSON.stringify({
      beds: {},
      takes: {
        [LEAD_MOVE]: takeRecord("move.", "a".repeat(64)),
        [LEAD_MOVE_STRESSED]: takeRecord("move!", "b".repeat(64), true),
      },
    }),
  );
  const open = async () => {
    const store = new VoiceWorkbenchStore(
      root,
      (value) => validateVoiceCatalog(value, null),
      new GenerationQueue(run),
    );
    const middleware = voiceWorkbenchMiddleware(store);
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
    return async (endpoint: string, body?: unknown, raw = false) => {
      const response = await fetch(
        `${origin}/__sound-workbench/voices/${endpoint}`,
        body === undefined
          ? undefined
          : {
              method: "POST",
              headers: { Origin: origin, "Content-Type": "application/json" },
              body: JSON.stringify(body),
            },
      );
      return {
        status: response.status,
        type: response.headers.get("content-type"),
        body: raw ? await response.text() : await response.json(),
      };
    };
  };
  return { root, text, open };
}
/** The generation tool's `verdict` command, run in-process against the fixture's lock. */
const verdictTool =
  (root: string): GenerateRunner =>
  async ([command, value, ...judged], line) => {
    if (command !== "verdict") throw new Error(`unexpected generate ${command}`);
    return recordVerdicts(join(root, "fixtures/voices.lock.json"), value, judged, line) ? 0 : 1;
  };
const lockOf = async (root: string) =>
  JSON.parse(await readFile(join(root, "fixtures/voices.lock.json"), "utf8"));

test("a voice verdict is recorded by the generation tool and survives a reload", async () => {
  let root = "";
  const fixture = await voiceFixture((args, line) => verdictTool(root)(args, line));
  root = fixture.root;
  const request = await fixture.open();
  const before = (await request("snapshot")).body;
  expect(before.takes[LEAD_MOVE].verdict).toBeNull();
  expect(before.counts.tank).toMatchObject({ takes: 2, unreviewed: 2, flagged: 1, bad: 0 });
  const judged = await request("verdict", {
    verdict: "ok",
    takes: [{ take: LEAD_MOVE, sha256: before.takes[LEAD_MOVE].sha256 }],
  });
  expect(judged.status).toBe(200);
  expect(judged.body.takes[LEAD_MOVE].verdict).toBe("ok");
  expect(judged.body.counts.tank).toMatchObject({ unreviewed: 1, ok: 1 });
  // The verdict is given on the bytes heard and bound to the recording they were made from.
  expect((await lockOf(root)).takes[LEAD_MOVE].verdict).toEqual({
    value: "ok",
    raw_sha256: "r".repeat(64),
  });
  const reloaded = (await (await fixture.open())("snapshot")).body;
  expect(reloaded.takes[LEAD_MOVE].verdict).toBe("ok");
});

test("a regenerated take falls back to unreviewed, and a verdict on the old bytes is refused", async () => {
  let root = "";
  const fixture = await voiceFixture((args, line) => verdictTool(root)(args, line));
  root = fixture.root;
  const request = await fixture.open();
  const heard = (await request("snapshot")).body.takes[LEAD_MOVE].sha256;
  await request("verdict", { verdict: "bad", takes: [{ take: LEAD_MOVE, sha256: heard }] });
  expect((await request("snapshot")).body.counts.tank.bad).toBe(1);
  // The tool regenerates the take: a new recording and new served bytes, the old
  // verdict kept with the recording it judged.
  const lock = await lockOf(root);
  lock.takes[LEAD_MOVE].raw_sha256 = "s".repeat(64);
  lock.takes[LEAD_MOVE].served_sha256 = "c".repeat(64);
  await writeFile(join(root, "fixtures/voices.lock.json"), JSON.stringify(lock));
  const after = (await request("snapshot")).body;
  expect(after.takes[LEAD_MOVE]).toMatchObject({ sha256: "c".repeat(64), verdict: null });
  expect(after.counts.tank).toMatchObject({ bad: 0, unreviewed: 2 });
  const stale = await request("verdict", {
    verdict: "ok",
    takes: [{ take: LEAD_MOVE, sha256: heard }],
  });
  expect(stale.status).toBe(200);
  expect(stale.body.changed).toEqual([LEAD_MOVE]);
  expect(stale.body.takes[LEAD_MOVE].verdict).toBeNull();
  expect((await lockOf(root)).takes[LEAD_MOVE].verdict.value).toBe("bad");
});

test("generate spawns the tool's voices command with the requested scope and reports its progress", async () => {
  const runs: string[][] = [];
  let finish = () => {};
  const fixture = await voiceFixture((args, line) => {
    runs.push(args);
    line("[1/4] tank/lead/move/steady/1 ok 1.4s match 1.00");
    return new Promise((done) => (finish = () => done(0)));
  });
  const request = await fixture.open();
  const started = await request("generate", {
    scope: {
      sets: ["tank"],
      voices: ["lead"],
      triggers: ["move"],
      intensities: ["steady"],
      take: 1,
    },
  });
  expect(started.status).toBe(200);
  await vi.waitFor(() => expect(runs).toHaveLength(1));
  expect(runs[0]).toEqual([
    "voices",
    "--set",
    "tank",
    "--voice",
    "lead",
    "--trigger",
    "move",
    "--intensity",
    "steady",
    "--take",
    "1",
  ]);
  const running = (await request("jobs")).body.jobs;
  expect(running).toMatchObject([
    {
      id: started.body.job.id,
      status: "running",
      lines: ["[1/4] tank/lead/move/steady/1 ok 1.4s match 1.00"],
    },
  ]);
  finish();
  await vi.waitFor(async () => expect((await request("jobs")).body.jobs[0].status).toBe("done"));
});

test("generate refuses a scope without a known set; no tool runs", async () => {
  const fixture = await voiceFixture();
  const request = await fixture.open();
  for (const scope of [
    {},
    { sets: [] },
    { sets: ["unknown"] },
    { sets: ["tank"], take: 2 },
    { sets: ["tank"], triggers: ["dance"] },
  ])
    expect((await request("generate", { scope })).status).toBe(400);
  expect((await request("jobs")).body.jobs).toEqual([]);
});

test("voice edits publish line text, descriptions and retakes only", async () => {
  const fixture = await voiceFixture();
  const request = await fixture.open();
  const initial = (await request("snapshot")).body;
  const catalog: VoiceCatalog = structuredClone(initial.catalog);
  catalog.roles.crew.lines.move.steady[0] = "Moving out.";
  catalog.sets.tank.voices[1].description = "Young loader";
  catalog.retakes[LEAD_MOVE] = 1;
  // "None of these" asks for a voice's design retake.
  catalog.retakes["tank/soldier/design"] = 1;
  const review = await request("preview", { revision: initial.revision, catalog });
  expect(review.status).toBe(200);
  expect(await readFile(join(fixture.root, "fixtures/voices.json"), "utf8")).toBe(fixture.text);
  const saved = await request("save", {
    candidateId: review.body.candidateId,
    revision: initial.revision,
  });
  expect(saved.status).toBe(200);
  expect(saved.body.catalog.roles.crew.lines.move.steady[0]).toBe("Moving out.");
  expect(saved.body.catalog.retakes).toEqual({ [LEAD_MOVE]: 1, "tank/soldier/design": 1 });
  // Who speaks, with which voice and where, is not this editor's to change.
  for (const mutate of [
    (c: VoiceCatalog) => (c.sets.tank.voices[0].voice_id = "other"),
    (c: VoiceCatalog) => (c.sets.tank.callsign = "Other 1"),
    (c: VoiceCatalog) => (c.assignments.rifle = "tank"),
    (c: VoiceCatalog) => (c.retakes["tank/lead/dance/steady/0"] = 1),
    (c: VoiceCatalog) => (c.retakes[LEAD_MOVE] = -1),
  ]) {
    const changed = structuredClone(saved.body.catalog);
    mutate(changed);
    expect(
      (await request("preview", { revision: saved.body.revision, catalog: changed })).status,
    ).toBe(400);
  }
});

test("a design candidate is played through the tool's preview, and pick runs the tool's pick", async () => {
  const runs: string[][] = [];
  let root = "";
  const fixture = await voiceFixture(async (args) => {
    runs.push(args);
    if (args[0] === "preview") {
      await mkdir(join(root, "throwaway/voice-design/tank/soldier"), { recursive: true });
      await writeFile(join(root, "throwaway/voice-design/tank/soldier/2.mp3"), "candidate-two");
    }
    if (args[0] === "pick") {
      // The tool saves the voice and writes its id into voices.json.
      const catalog = voiceCatalog();
      catalog.sets.tank.voices[1].voice_id = "picked-id";
      await writeFile(join(root, "fixtures/voices.json"), JSON.stringify(catalog, null, 2));
    }
    return 0;
  });
  root = fixture.root;
  const lock = await lockOf(root);
  const candidate = (n: number) => ({
    generated_voice_id: `g${n}`,
    raw: `assets/source/audio/voices/design/tank/soldier/candidate-${n}.mp3`,
    raw_sha256: "r".repeat(64),
    duration_s: 8 + n,
    transcript: "heard",
    match: { score: 0.9, missing: [], overlong: false, flagged: false },
  });
  lock.designs = {
    "tank/soldier": {
      description: "Crew",
      text: "audition",
      model: "m",
      retake: 0,
      design_key: "k",
      date: "2026-10-12",
      candidates: [1, 2, 3].map(candidate),
    },
  };
  await writeFile(join(root, "fixtures/voices.lock.json"), JSON.stringify(lock));
  const request = await fixture.open();
  const snapshot = (await request("snapshot")).body;
  expect(snapshot.designs["tank/soldier"].candidates[1]).toEqual({
    transcript: "heard",
    score: 0.9,
    duration_s: 10,
  });

  const audio = await request("candidate/tank/soldier/2", undefined, true);
  expect(audio).toMatchObject({ status: 200, type: "audio/mpeg", body: "candidate-two" });
  // Rendered once per voice per session.
  await request("candidate/tank/soldier/2", undefined, true);
  expect(runs).toEqual([["preview", "tank/soldier"]]);
  expect((await request("candidate/tank/nobody/1", undefined, true)).status).toBe(400);

  const picked = await request("pick", { voice: "tank/soldier", candidate: 2 });
  expect(picked.status).toBe(200);
  expect(runs.at(-1)).toEqual(["pick", "tank/soldier", "2"]);
  expect(picked.body.catalog.sets.tank.voices[1].voice_id).toBe("picked-id");
  // Replacing a voice already picked runs the tool's pick with --replace, then
  // remakes that voice's takes (the old voice's takes are superseded) and the set's pack.
  const replaced = await request("pick", { voice: "tank/soldier", candidate: 3, replace: true });
  expect(replaced.status).toBe(200);
  await vi.waitFor(() => expect(runs.at(-1)?.[0]).toBe("pack"));
  expect(runs.slice(-3)).toEqual([
    ["pick", "tank/soldier", "3", "--replace"],
    ["voices", "--set", "tank", "--voice", "soldier"],
    ["pack", "--set", "tank"],
  ]);
  expect((await request("pick", { voice: "tank/soldier", candidate: 4 })).status).toBe(400);

  const designed = await request("design", { voice: "tank/soldier" });
  expect(designed.body.job.args).toEqual(["design", "tank/soldier"]);
});

test("the Music tab regenerates one named battle track through the generation tool's queue", async () => {
  const ran: string[][] = [];
  let finish = () => {};
  const queue = new GenerationQueue(
    (args) =>
      new Promise((resolve) => {
        ran.push(args);
        finish = () => resolve(0);
      }),
  );
  const middleware = musicWorkbenchMiddleware(queue);
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
  const post = (body: unknown) =>
    fetch(`${origin}/__sound-workbench/music/regenerate`, {
      method: "POST",
      headers: { Origin: origin, "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });

  expect((await post({ track: "battlefield2_menu" })).status).toBe(400);
  const response = await post({ track: "combat_2" });
  expect(response.status).toBe(200);
  expect((await response.json()).job.args).toEqual(["music", "--track", "combat_2"]);
  // A second command waits for the first: the tool's files keep one writer.
  await post({ track: "tension_dark_1" });
  await vi.waitFor(() => expect(ran).toHaveLength(1));
  finish();
  await vi.waitFor(() => expect(ran).toHaveLength(2));
  finish();
  await vi.waitFor(async () => {
    const { jobs } = await (await fetch(`${origin}/__sound-workbench/music/jobs`)).json();
    expect(jobs.map((job: { status: string }) => job.status)).toEqual(["done", "done"]);
  });
});

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
