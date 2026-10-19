// @vitest-environment node
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { afterEach, describe, expect, it } from "vitest";
import {
  ALL_TRIGGERS,
  takeId,
  validateVoiceCatalog,
  type VoiceCatalog,
} from "@packages/battle-audio/src/voices/voiceCatalog";
import { ElevenLabsClient } from "@packages/battle-audio/tools/generate/elevenlabs";
import { Pool } from "@packages/battle-audio/tools/generate/pool";
import { matches } from "@packages/battle-audio/tools/generate/transcript";
import {
  readLock,
  recordVerdicts,
  verdictOf,
  type CandidateRecord,
  type DesignRecord,
} from "@packages/battle-audio/tools/generate/lock";
import {
  auditionText,
  autoPicks,
  designVoices,
  pickVoice,
} from "@packages/battle-audio/tools/generate/design";
import { checkPacks, writePacks } from "@packages/battle-audio/tools/generate/packs";
import {
  checkVoices,
  generateVoices,
  selectTakes,
  type Processor,
  type VoiceProfiles,
} from "@packages/battle-audio/tools/generate/voices";

const LEAD = "voice-lead-id";
const lines = (steady: [string, string], stressed: [string, string]) => ({ steady, stressed });
/** One set, one saved voice, one trigger: four takes. */
function catalog(): VoiceCatalog {
  return {
    roles: {
      crew: {
        lines: {
          move: lines(
            ["Roger, moving out.", "Driver, move out."],
            ["[urgent] Driver, go, go, go!", "[shouting] Moving! Driver, step on it!"],
          ),
        },
      },
    },
    sets: {
      tank: {
        role: "crew",
        country: "us",
        callsign: "Oscar 55",
        processing: "voice-radio-light",
        voices: [
          {
            id: "lead",
            speaks: "responses",
            gender: "male",
            description: "a tank commander",
            voice_id: LEAD,
          },
          { id: "soldier", speaks: "barks", gender: "male", description: "", voice_id: null },
        ],
      },
    },
    assignments: {},
    retakes: {},
  } as unknown as VoiceCatalog;
}
const profiles = (compression = 4): VoiceProfiles => ({
  encoder: { codec: "libmp3lame", bitrate: "40k" },
  profiles: {
    "voice-radio-light": {
      bed: { generated: "tank_engine", filter: "anull", volume: 0.3 },
      ratio: compression,
    },
    "voice-intercom": {
      bed: { clip: "rotor", filter: "lowpass=f=900", volume: 0.6 },
      ratio: compression,
    },
  },
});

/** The proxy at the network edge: speech is the text's bytes (marked by `take`, as a
 *  real retake speaks new bytes), a transcript echoes the speech. */
function fakeProxy(
  options: { failAfter?: number; transcript?: (spoken: string) => string; take?: number } = {},
) {
  /** Requests the proxy answered, so the ones that cost credits. */
  const calls: string[] = [];
  /** Descriptions sent to Voice Design, and bodies sent to save a voice. */
  const designs: string[] = [];
  const saved: { voice_name: string; generated_voice_id: string; labels: object }[] = [];
  let requests = 0;
  const fetch = async (url: string | URL | Request, init?: RequestInit) => {
    const path = new URL(String(url)).pathname;
    if (options.failAfter !== undefined && ++requests > options.failAfter)
      return new Response('{"detail":{"status":"invalid_api_key"}}', { status: 401 });
    calls.push(path);
    if (path.includes("/text-to-speech/")) {
      const { text } = JSON.parse(String(init?.body));
      return new Response(new TextEncoder().encode(`speech${options.take ?? ""}:${text}`));
    }
    if (path.endsWith("/speech-to-text")) {
      const file = (init!.body as FormData).get("file") as Blob;
      const spoken = (await file.text()).replace(/^speech\d*:/, "");
      return Response.json({ text: (options.transcript ?? ((s) => s))(spoken) });
    }
    if (path.endsWith("/sound-generation")) return new Response(new Uint8Array([1, 2, 3]));
    // Voice Design: three previews speaking the audition text, each with its own bytes.
    if (path.endsWith("/text-to-voice/design")) {
      const { text, voice_description } = JSON.parse(String(init?.body));
      designs.push(voice_description);
      return Response.json({
        previews: [1, 2, 3].map((n) => ({
          generated_voice_id: `generated-${designs.length}-${n}`,
          audio_base_64: Buffer.from(`speech${n}:${text}`).toString("base64"),
          duration_secs: 6 + n,
        })),
      });
    }
    if (path.endsWith("/text-to-voice")) {
      const body = JSON.parse(String(init?.body));
      saved.push(body);
      return Response.json({ voice_id: `saved-${body.generated_voice_id}` });
    }
    return new Response("not found", { status: 404 });
  };
  const client = new ElevenLabsClient({
    fetch: fetch as typeof globalThis.fetch,
    workspace: "w",
    token: "t",
  });
  return {
    client,
    calls,
    designs,
    saved,
    tts: () => calls.filter((c) => c.includes("/text-to-speech/")).length,
  };
}
/** Processing at the ffmpeg edge: the served bytes are a function of raw bytes and profile. */
function fakeProcessor() {
  const jobs: string[] = [];
  const processor: Processor = async ({ profile, input, bed, output }) => {
    jobs.push(profile);
    const raw = readFileSync(input);
    writeFileSync(output, `${profile}:${bed ? readFileSync(bed).length : "-"}:${raw}`);
    return { duration_s: 1.5 };
  };
  return { processor, jobs };
}

const roots: string[] = [];
afterEach(() => roots.splice(0).forEach((root) => rmSync(root, { recursive: true, force: true })));
function workspace() {
  const root = mkdtempSync(join(tmpdir(), "voice-generator-"));
  roots.push(root);
  // The game's own rotor loop is a catalog clip; the engine bed is already generated.
  writeClip(root, "rotor-loop");
  const bed = "assets/source/audio/voices/beds/tank_engine.0102.mp3";
  mkdirSync(dirname(join(root, bed)), { recursive: true });
  writeFileSync(join(root, bed), "engine");
  writeFileSync(
    join(root, "fixtures/voices.lock.json"),
    JSON.stringify({
      beds: {
        tank_engine: {
          raw: bed,
          sha256: createHash("sha256").update("engine").digest("hex"),
          provenance: { kind: "Generated", prompt: "engine" },
        },
      },
      takes: {},
    }),
  );
  return root;
}
/** The sound catalog's `rotor` clip and its prepared runtime file. */
function writeClip(root: string, bytes: string) {
  const url = "/audio/clips/rotor.wav";
  mkdirSync(join(root, "assets/runtime/audio/clips"), { recursive: true });
  writeFileSync(join(root, "assets/runtime", url), bytes);
  mkdirSync(join(root, "fixtures"), { recursive: true });
  writeFileSync(
    join(root, "fixtures/sounds.json"),
    JSON.stringify({
      clips: { rotor: { url, sha256: createHash("sha256").update(bytes).digest("hex") } },
    }),
  );
}
const quiet = () => {};
function run(
  root: string,
  proxy: ReturnType<typeof fakeProxy> | null,
  more: Partial<Parameters<typeof generateVoices>[0]> = {},
) {
  return generateVoices({
    root,
    catalog: catalog(),
    client: proxy?.client ?? null,
    processor: fakeProcessor().processor,
    profiles: profiles(),
    select: {},
    log: quiet,
    ...more,
  });
}

describe("voice generation", () => {
  it("records every take served, transcribed and matched, and a rerun makes no call", async () => {
    const root = workspace();
    const first = fakeProxy();
    const result = await run(root, first);
    expect(result.stopped).toBeNull();
    expect(first.tts()).toBe(4);
    const lock = readLock(join(root, "fixtures/voices.lock.json"));
    const take = lock.takes["tank/lead/move/stressed/0"];
    expect(take.text).toBe("[urgent] Driver, go, go, go!");
    expect(take.match.flagged).toBe(false);
    expect(take.provenance).toMatchObject({
      kind: "Generated",
      model: "eleven_v3",
      voice_id: LEAD,
    });
    const served = join(
      root,
      "assets/runtime/audio/voices/tank",
      `lead-move-stressed-0.${take.served_sha256.slice(0, 8)}.mp3`,
    );
    expect(readFileSync(served, "utf8")).toBe(
      "voice-radio-light:6:speech:[urgent] Driver, go, go, go!",
    );
    expect(checkVoices({ root, catalog: catalog(), profiles: profiles() })).toEqual([]);

    const again = fakeProxy();
    await run(root, again);
    expect(again.calls).toEqual([]);
  });

  it("resumes a killed batch without paying again for a take it already received", async () => {
    const root = workspace();
    // The proxy refuses the key after three requests; the run stops.
    const killed = fakeProxy({ failAfter: 3 });
    const stopped = await run(root, killed);
    expect(stopped.stopped?.message).toMatch(/401/);
    // A kill before the lock flush loses the lock rows, not the received speech.
    writeFileSync(
      join(root, "fixtures/voices.lock.json"),
      JSON.stringify({ ...readLock(join(root, "fixtures/voices.lock.json")), takes: {} }),
    );
    const resumed = fakeProxy();
    await run(root, resumed);
    expect(killed.tts() + resumed.tts()).toBe(4);
    expect(Object.keys(readLock(join(root, "fixtures/voices.lock.json")).takes)).toHaveLength(4);
  });

  it("flags a take whose transcript drops the callsign", async () => {
    const root = workspace();
    const c = catalog();
    c.roles.crew.lines.move.steady[0] = "{callsign}, moving out.";
    await generateVoices({
      root,
      catalog: c,
      client: fakeProxy({ transcript: (s) => s.replace("Oscar 55", "Oscar") }).client,
      processor: fakeProcessor().processor,
      profiles: profiles(),
      select: { take: 0, intensities: ["steady"] },
      log: quiet,
    });
    const take = readLock(join(root, "fixtures/voices.lock.json")).takes["tank/lead/move/steady/0"];
    expect(take.text).toBe("Oscar 55, moving out.");
    expect(take.match).toMatchObject({ flagged: true, missing: ["55"] });
  });

  it("generates again only the take whose retake was bumped", async () => {
    const root = workspace();
    await run(root, fakeProxy());
    const c = catalog();
    c.retakes["tank/lead/move/steady/1"] = 1;
    const second = fakeProxy();
    await run(root, second, { catalog: c });
    expect(second.tts()).toBe(1);
    const lock = readLock(join(root, "fixtures/voices.lock.json"));
    expect(existsSync(join(root, lock.takes["tank/lead/move/steady/1"].raw))).toBe(true);
  });

  it("selects one voice's takes when a regeneration names its voice", () => {
    const c = catalog();
    Object.assign(c.sets.tank.voices[1], { speaks: "all", voice_id: "voice-soldier-id" });
    const select = { triggers: ["move" as const], take: 0 };
    expect(selectTakes(c, { beds: {}, designs: {}, takes: {} }, select).takes).toHaveLength(4);
    expect(
      selectTakes(
        c,
        { beds: {}, designs: {}, takes: {} },
        { ...select, voices: ["lead"] },
      ).takes.map(takeId),
    ).toEqual(["tank/lead/move/steady/0", "tank/lead/move/stressed/0"]);
  });

  it("re-processes a changed profile from the raw master without a call", async () => {
    const root = workspace();
    await run(root, fakeProxy());
    expect(checkVoices({ root, catalog: catalog(), profiles: profiles(6) })).toHaveLength(4);
    const local = fakeProcessor();
    await run(root, null, { processor: local.processor, profiles: profiles(6) });
    expect(local.jobs).toHaveLength(4);
    expect(checkVoices({ root, catalog: catalog(), profiles: profiles(6) })).toEqual([]);
  });

  it("a re-mix onto another background keeps a reviewed set approved and packed; a new recording is reviewed again", async () => {
    const root = workspace();
    const lockPath = join(root, "fixtures/voices.lock.json");
    await run(root, fakeProxy());
    const reviewed = readLock(lockPath);
    const takes = Object.keys(reviewed.takes);
    expect(takes).toHaveLength(4);
    recordVerdicts(
      lockPath,
      "ok",
      takes.map((id) => `${id}@${reviewed.takes[id].served_sha256}`),
      quiet,
    );
    const packedTakes = (c: VoiceCatalog, unreviewed = false) => {
      writePacks({ root, catalog: c, unreviewed, log: quiet });
      const pack = JSON.parse(
        readFileSync(join(root, "assets/runtime/audio/voices/tank/pack.json"), "utf8"),
      ) as { takes: { intensity: string; take: number }[] };
      return pack.takes.map((take) => `tank/lead/move/${take.intensity}/${take.take}`);
    };
    expect(packedTakes(catalog())).toEqual(takes);
    // A cleared verdict reads as unreviewed: out of the pack until judged,
    // listed by an unreviewed pack while it passes the transcript check.
    const [first] = takes;
    recordVerdicts(lockPath, "clear", [`${first}@${reviewed.takes[first].served_sha256}`], quiet);
    expect(verdictOf(readLock(lockPath).takes[first])).toBeNull();
    expect(packedTakes(catalog())).toEqual(takes.slice(1));
    expect(packedTakes(catalog(), true)).toEqual(takes);
    recordVerdicts(lockPath, "ok", [`${first}@${reviewed.takes[first].served_sha256}`], quiet);
    expect(packedTakes(catalog())).toEqual(takes);

    // The set moves onto another bed: every take is re-rendered, no speech is bought.
    const remixed = catalog();
    remixed.sets.tank.processing = "voice-intercom";
    const local = fakeProcessor();
    await run(root, null, { catalog: remixed, processor: local.processor });
    expect(local.jobs).toEqual(Array(4).fill("voice-intercom"));
    const after = readLock(lockPath);
    for (const id of takes) {
      expect(after.takes[id].served_sha256).not.toBe(reviewed.takes[id].served_sha256);
      expect(verdictOf(after.takes[id]), id).toBe("ok");
      expect(after.takes[id].match).toEqual(reviewed.takes[id].match);
    }
    expect(packedTakes(remixed)).toEqual(takes);

    // A take spoken again is a new recording: its approval no longer holds.
    const retaken = "tank/lead/move/steady/1";
    remixed.retakes[retaken] = 1;
    await run(root, fakeProxy({ take: 2 }), { catalog: remixed });
    expect(verdictOf(readLock(lockPath).takes[retaken])).toBeNull();
    expect(packedTakes(remixed)).toEqual(takes.filter((id) => id !== retaken));
  });

  it("lays a profile's bed from the game's catalog clip, and a changed clip re-renders", async () => {
    const root = workspace();
    const heli = catalog();
    heli.sets.tank.processing = "voice-intercom";
    await run(root, fakeProxy(), { catalog: heli });
    const lock = () => readLock(join(root, "fixtures/voices.lock.json"));
    const take = lock().takes["tank/lead/move/steady/0"];
    const served = (sha: string) =>
      readFileSync(
        join(root, "assets/runtime/audio/voices/tank", `lead-move-steady-0.${sha.slice(0, 8)}.mp3`),
        "utf8",
      );
    // The fake processor writes the bed's length: the clip's ten bytes, not the engine's six.
    expect(served(take.served_sha256)).toBe("voice-intercom:10:speech:Roger, moving out.");

    writeClip(root, "rotor-loop-retuned");
    const stale = checkVoices({ root, catalog: heli, profiles: profiles() });
    expect(stale).toHaveLength(4);
    expect(stale[0]).toMatch(/older profile/);
    const again = fakeProxy();
    await run(root, again, { catalog: heli });
    expect(again.calls).toEqual([]);
    expect(served(lock().takes["tank/lead/move/steady/0"].served_sha256)).toBe(
      "voice-intercom:18:speech:Roger, moving out.",
    );
    expect(checkVoices({ root, catalog: heli, profiles: profiles() })).toEqual([]);
  });

  it("reports a locked bed that no profile uses", async () => {
    const root = workspace();
    await run(root, fakeProxy());
    const unbedded = profiles();
    unbedded.profiles["voice-radio-light"].bed = null;
    expect(checkVoices({ root, catalog: catalog(), profiles: unbedded })).toContain(
      "bed tank_engine: no profile uses it",
    );
  });

  it("stops before sending more characters than the budget", async () => {
    const root = workspace();
    const proxy = fakeProxy();
    const result = await run(root, proxy, { maxChars: 40 });
    expect(proxy.tts()).toBe(2);
    expect(result.characters).toBeLessThanOrEqual(40);
  });
});

/** A crew whose every trigger has lines, so its role has an audition; its soldier awaits design. */
function designCatalog(): VoiceCatalog {
  const c = catalog();
  c.roles.crew.lines = Object.fromEntries(
    ALL_TRIGGERS.map((trigger) => [
      trigger,
      lines(
        [`Steady ${trigger} one.`, `Steady ${trigger} two.`],
        [`[shouting] Stressed ${trigger} one!`, `[urgent] Stressed ${trigger} two!`],
      ),
    ]),
  ) as VoiceCatalog["roles"][string]["lines"];
  c.sets.tank.voices[1].description = "US tank crew gunner in his twenties, raspy voice";
  return c;
}
function writeCatalog(root: string, c: VoiceCatalog) {
  writeFileSync(join(root, "fixtures/voices.json"), `${JSON.stringify(c, null, 2)}\n`);
}
const lockAt = (root: string) => readLock(join(root, "fixtures/voices.lock.json"));
const design = (root: string, proxy: ReturnType<typeof fakeProxy>, c = designCatalog()) =>
  designVoices({ root, catalog: c, client: proxy.client, log: quiet });
const pick = (
  root: string,
  proxy: ReturnType<typeof fakeProxy>,
  voice: string,
  candidate: number,
  replace = false,
) =>
  pickVoice({
    root,
    client: proxy.client,
    validate: (value) => validateVoiceCatalog(value, null),
    voice,
    candidate,
    replace,
    log: quiet,
  });

describe("voice packs", () => {
  /** A locked `tank/lead/move/<intensity>/<take>` whose served file holds `bytes`. */
  function locked(
    root: string,
    take: string,
    bytes: string,
    record: {
      verdict?: "ok" | "bad" | "stale";
      flagged?: boolean;
      onDisk?: string;
      /** The voice that spoke it; the lead's current voice by default. */
      voice?: string;
    } = {},
  ) {
    const [intensity, n] = take.split("/");
    const sha256 = createHash("sha256").update(bytes).digest("hex");
    const file = `assets/runtime/audio/voices/tank/lead-move-${intensity}-${n}.${sha256.slice(0, 8)}.mp3`;
    mkdirSync(dirname(join(root, file)), { recursive: true });
    writeFileSync(join(root, file), record.onDisk ?? bytes);
    const raw_sha256 = createHash("sha256").update(`raw:${bytes}`).digest("hex");
    const verdict =
      record.verdict === "stale"
        ? { value: "ok", raw_sha256: "f".repeat(64) }
        : record.verdict && { value: record.verdict, raw_sha256 };
    return [
      `tank/lead/move/${take}`,
      {
        raw_sha256,
        served_sha256: sha256,
        duration_s: 1.25,
        match: { flagged: record.flagged ?? false },
        provenance: { voice_id: record.voice ?? LEAD },
        ...(verdict ? { verdict } : {}),
      },
    ] as const;
  }
  function packed(unreviewed: boolean, takes: (readonly [string, object])[], root = workspace()) {
    writeFileSync(
      join(root, "fixtures/voices.lock.json"),
      JSON.stringify({ beds: {}, designs: {}, takes: Object.fromEntries(takes) }),
    );
    writePacks({ root, catalog: catalog(), unreviewed, log: quiet });
    const path = join(root, "assets/runtime/audio/voices/tank/pack.json");
    return existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) : null;
  }
  const listed = (pack: { takes: { intensity: string; take: number }[] } | null) =>
    pack?.takes.map((take) => `${take.intensity}/${take.take}`) ?? [];

  it("lists only takes judged ok on the recording served", () => {
    const root = workspace();
    const pack = packed(
      false,
      [
        locked(root, "steady/0", "ok", { verdict: "ok" }),
        // Judged on a recording since replaced; judged bad; never judged.
        locked(root, "steady/1", "regenerated", { verdict: "stale" }),
        locked(root, "stressed/0", "bad", { verdict: "bad" }),
        locked(root, "stressed/1", "unjudged"),
      ],
      root,
    );
    expect(listed(pack)).toEqual(["steady/0"]);
    const sha256 = createHash("sha256").update("ok").digest("hex");
    expect(pack).toEqual({
      set: "tank",
      voices: [
        { id: "lead", speaks: "responses" },
        { id: "soldier", speaks: "barks" },
      ],
      takes: [
        {
          voice: "lead",
          trigger: "move",
          intensity: "steady",
          take: 0,
          url: `/audio/voices/tank/lead-move-steady-0.${sha256.slice(0, 8)}.mp3`,
          duration_s: 1.25,
        },
      ],
    });
  });

  it("before review, --unreviewed adds unjudged takes the transcript check passed, never bad or flagged ones", () => {
    const root = workspace();
    const takes = [
      locked(root, "steady/0", "unjudged"),
      locked(root, "steady/1", "flagged", { flagged: true }),
      locked(root, "stressed/0", "bad", { verdict: "bad" }),
      locked(root, "stressed/1", "ok", { verdict: "ok" }),
    ];
    expect(listed(packed(true, takes, root))).toEqual(["steady/0", "stressed/1"]);
    expect(listed(packed(false, takes, root))).toEqual(["stressed/1"]);
  });

  it("check proves every packed take is the ok-judged bytes the lock names, and reports a stale or orphan pack", async () => {
    const root = workspace();
    await run(root, fakeProxy());
    const lockPath = join(root, "fixtures/voices.lock.json");
    const sha = (id: string) => lockAt(root).takes[id].served_sha256;
    const ids = ["tank/lead/move/steady/0", "tank/lead/move/steady/1"];
    recordVerdicts(
      lockPath,
      "ok",
      ids.map((id) => `${id}@${sha(id)}`),
      quiet,
    );
    writePacks({ root, catalog: catalog(), unreviewed: false, log: quiet });
    const check = () => checkVoices({ root, catalog: catalog(), profiles: profiles() });
    expect(check()).toEqual([]);

    // A verdict changed after packing: the pack ships a take no longer judged ok.
    recordVerdicts(lockPath, "bad", [`${ids[1]}@${sha(ids[1])}`], quiet);
    expect(check()).toEqual([
      `pack tank: ${ids[1]} is not judged ok on its bytes`,
      "pack tank: stale; run `generate pack`",
    ]);
    writePacks({ root, catalog: catalog(), unreviewed: false, log: quiet });
    expect(check()).toEqual([]);

    // A take judged ok since packing is missing from the pack.
    const stressed = "tank/lead/move/stressed/0";
    recordVerdicts(lockPath, "ok", [`${stressed}@${sha(stressed)}`], quiet);
    expect(check()).toEqual(["pack tank: stale; run `generate pack`"]);
    writePacks({ root, catalog: catalog(), unreviewed: false, log: quiet });

    // A packed file whose bytes changed, and a pack for a set voices.json lost.
    const served = join(
      root,
      "assets/runtime/audio/voices/tank",
      `lead-move-steady-0.${sha(ids[0]).slice(0, 8)}.mp3`,
    );
    writeFileSync(served, "edited");
    mkdirSync(join(root, "assets/runtime/audio/voices/gone"), { recursive: true });
    writeFileSync(join(root, "assets/runtime/audio/voices/gone/pack.json"), "{}");
    expect(check()).toEqual(
      expect.arrayContaining([
        `pack tank: ${ids[0]} served file differs`,
        "pack gone: set not in voices.json",
      ]),
    );
  });

  it("leaves out a take spoken by a voice voices.json no longer names, and check refuses one packed", () => {
    const root = workspace();
    // The lead's voice was replaced and regeneration stopped after one take.
    const fresh = locked(root, "steady/0", "new voice", { verdict: "ok" });
    const stale = locked(root, "steady/1", "old voice", { verdict: "ok", voice: "voice-old-id" });
    // The pack written before the swap, when the old voice was the lead's.
    const before = packed(
      false,
      [fresh, [stale[0], { ...stale[1], provenance: { voice_id: LEAD } }]],
      root,
    );
    expect(listed(before)).toEqual(["steady/0", "steady/1"]);

    const lockPath = join(root, "fixtures/voices.lock.json");
    writeFileSync(
      lockPath,
      JSON.stringify({
        beds: {},
        designs: {},
        takes: { [fresh[0]]: fresh[1], [stale[0]]: stale[1] },
      }),
    );
    expect(checkPacks(root, catalog(), readLock(lockPath))).toEqual([
      `pack tank: ${stale[0]} was spoken by another voice than voices.json names`,
      "pack tank: stale; run `generate pack`",
    ]);
    expect(listed(packed(true, [fresh, stale], root))).toEqual(["steady/0"]);
  });

  it("leaves out a take whose served file is not the bytes the lock names", () => {
    const root = workspace();
    const pack = packed(
      false,
      [locked(root, "steady/0", "ok", { verdict: "ok", onDisk: "other" })],
      root,
    );
    expect(pack).toBeNull();
  });
});

describe("voice design", () => {
  it("designs a waiting voice from its description and role audition, and keeps three transcribed candidates", async () => {
    const root = workspace();
    const proxy = fakeProxy();
    const result = await design(root, proxy);
    expect(result.failed).toBe(0);
    // Only the soldier waits for a voice; the lead is already saved.
    expect(proxy.designs).toEqual(["US tank crew gunner in his twenties, raspy voice"]);
    const record = lockAt(root).designs["tank/soldier"];
    const text = auditionText(designCatalog(), "crew");
    expect(text.length).toBeGreaterThanOrEqual(100);
    expect(record.text).toBe(text);
    expect(record.candidates.map((c) => c.generated_voice_id)).toEqual([
      "generated-1-1",
      "generated-1-2",
      "generated-1-3",
    ]);
    for (const [i, candidate] of record.candidates.entries()) {
      // The preview is a raw master: its bytes are on disk under the voice's design folder.
      expect(candidate.raw).toMatch(/^assets\/source\/audio\/voices\/design\/tank\/soldier\//);
      expect(readFileSync(join(root, candidate.raw), "utf8")).toBe(`speech${i + 1}:${text}`);
      expect(candidate.raw_sha256).toBe(
        createHash("sha256")
          .update(`speech${i + 1}:${text}`)
          .digest("hex"),
      );
      expect(candidate.transcript).toBe(text);
      expect(candidate.match?.score).toBe(1);
    }
    // A voice that already has candidates is never designed again.
    const again = fakeProxy();
    await design(root, again);
    expect(again.calls).toEqual([]);
  });

  it("designs again when the voice's design retake is bumped", async () => {
    const root = workspace();
    await design(root, fakeProxy());
    const c = designCatalog();
    c.retakes["tank/soldier/design"] = 1;
    const again = fakeProxy();
    await design(root, again, c);
    expect(again.designs).toHaveLength(1);
    expect(lockAt(root).designs["tank/soldier"].retake).toBe(1);
  });

  it("pick saves the chosen candidate permanently and writes its voice_id into voices.json", async () => {
    const root = workspace();
    writeCatalog(root, designCatalog());
    await design(root, fakeProxy());
    const proxy = fakeProxy();
    expect(await pick(root, proxy, "tank/soldier", 2)).toBe(true);
    expect(proxy.saved).toEqual([
      expect.objectContaining({
        voice_name: "battlegame tank soldier",
        generated_voice_id: "generated-1-2",
        labels: { project: "battlegame", set: "tank", voice: "soldier" },
      }),
    ]);
    const saved = JSON.parse(readFileSync(join(root, "fixtures/voices.json"), "utf8"));
    expect(saved.sets.tank.voices[1].voice_id).toBe("saved-generated-1-2");
    // The rest of the file is untouched.
    expect(saved.sets.tank.voices[0].voice_id).toBe(LEAD);
    const record = lockAt(root).designs["tank/soldier"];
    expect(record.picked).toMatchObject({ candidate: 2, voice_id: "saved-generated-1-2" });
    expect(record.candidates).toHaveLength(3);
  });

  it("pick refuses a voice that already has a voice_id unless told to replace it", async () => {
    const root = workspace();
    writeCatalog(root, designCatalog());
    await design(root, fakeProxy());
    await pick(root, fakeProxy(), "tank/soldier", 1);
    const before = readFileSync(join(root, "fixtures/voices.json"), "utf8");
    const refused = fakeProxy();
    expect(await pick(root, refused, "tank/soldier", 3)).toBe(false);
    expect(refused.calls).toEqual([]);
    expect(readFileSync(join(root, "fixtures/voices.json"), "utf8")).toBe(before);
    const replaced = fakeProxy();
    expect(await pick(root, replaced, "tank/soldier", 3, true)).toBe(true);
    const saved = JSON.parse(readFileSync(join(root, "fixtures/voices.json"), "utf8"));
    expect(saved.sets.tank.voices[1].voice_id).toBe("saved-generated-1-3");
  });

  it("auto pick takes the best speech-to-text match, then the duration nearest the role's median", () => {
    const candidate = (score: number, duration_s: number) =>
      ({ match: { score }, duration_s }) as unknown as CandidateRecord;
    const record = (candidates: CandidateRecord[]) => ({ candidates }) as unknown as DesignRecord;
    const c = designCatalog();
    c.sets.tank.voices.push({
      id: "soldier2",
      speaks: "barks",
      gender: "male",
      description: "",
      voice_id: null,
    });
    const picks = autoPicks(c, {
      beds: {},
      takes: {},
      designs: {
        // Role median of every candidate's duration: 8 s.
        "tank/soldier": record([candidate(0.9, 8), candidate(1, 4), candidate(0.7, 8)]),
        "tank/soldier2": record([candidate(1, 12), candidate(1, 8.5), candidate(0.5, 8)]),
      },
    });
    expect(picks).toEqual([
      { voice: "tank/soldier", candidate: 2 },
      { voice: "tank/soldier2", candidate: 2 },
    ]);
  });
});

describe("transcript match", () => {
  it("reads spoken numbers and ignores audio tags and punctuation", () => {
    const callsign = "Oscar 21";
    expect(
      matches("[shouting] Oscar 21, moving!", "Oscar two-one moving", { callsign }).flagged,
    ).toBe(false);
    expect(
      matches("Oscar 55, on the way.", "oscar fifty five on the way", { callsign: "Oscar 55" }),
    ).toMatchObject({ flagged: false, score: 1 });
  });

  it("reads a spelling-only difference as the same words", () => {
    const heard = (script: string, transcript: string) =>
      matches(script, transcript, { callsign: "Oscar 1" });
    for (const [script, transcript] of [
      ["Enemy in the treeline.", "Enemy in the tree line"],
      ["Armour, front.", "Armor front"],
      ["Reloading, stand by.", "Reloading. Standby"],
      ["[shouting] Bail out! Bail out, God damn it!", "Bail out. Bail out, goddammit"],
      ["Co-pilot wounded.", "Copilot wounded"],
      ["[shouting] Backblast clear!", "Back blast clear!"],
      ["Troops, four hundred metres.", "Troops, 400 meters"],
    ])
      expect(heard(script, transcript), transcript).toMatchObject({ flagged: false, score: 1 });
  });

  it("still flags a misread word or a dropped keyword once spelling is set aside", () => {
    const callsign = "Oscar 1";
    expect(matches("Rocket out.", "Rock it out", { callsign }).flagged).toBe(true);
    expect(matches("Armour, front.", "Armor hell", { callsign }).flagged).toBe(true);
    expect(
      matches("[shouting] Tank! Gunner, sabot!", "Tank, gun, sabot", { callsign }),
    ).toMatchObject({ flagged: true, missing: ["gunner"] });
    // Overlong is a flag however well the words match.
    expect(
      matches("Enemy in the treeline.", "Enemy in the tree line", {
        callsign,
        length: { durationS: 5, talkS: 3 },
      }),
    ).toMatchObject({ flagged: true, overlong: true });
  });

  it("flags a misheard keyword even when the rest of the line is right", () => {
    const result = matches("Gunner, sabot, tank!", "Gutter, sabot, tank!", { callsign: "Oscar 1" });
    expect(result).toMatchObject({ flagged: true, missing: ["gunner"] });
  });

  it("flags a take longer than its trigger's talk time", () => {
    const line = "Roger, moving out.";
    expect(matches(line, line, { callsign: "", length: { durationS: 2, talkS: 3 } }).flagged).toBe(
      false,
    );
    expect(matches(line, line, { callsign: "", length: { durationS: 4, talkS: 3 } })).toMatchObject(
      {
        flagged: true,
        overlong: true,
      },
    );
  });
});

describe("request pool", () => {
  const error = (status: number) => Object.assign(new Error(`status ${status}`), { status });
  const instant = { sleep: async () => {}, random: () => 0.5 };

  it("retries throttling and server errors with backoff", async () => {
    let attempts = 0;
    const pool = new Pool(2, instant);
    const value = await pool.run(async () => {
      attempts++;
      if (attempts < 3) throw error(attempts === 1 ? 429 : 503);
      return "ok";
    });
    expect(value).toBe("ok");
    expect(attempts).toBe(3);
  });

  it("stops every queued request on an authorisation or quota error", async () => {
    const pool = new Pool(1, instant);
    const started: number[] = [];
    const results = await Promise.allSettled(
      [0, 1, 2].map((i) =>
        pool.run(async () => {
          started.push(i);
          if (i === 0) throw error(401);
          return i;
        }),
      ),
    );
    expect(started).toEqual([0]);
    expect(results.every((r) => r.status === "rejected")).toBe(true);
    expect(pool.stopped?.message).toMatch(/401/);
  });

  it("stops after several failures in a row", async () => {
    const pool = new Pool(1, { ...instant, maxFailures: 2 });
    const started: number[] = [];
    await Promise.allSettled(
      [0, 1, 2].map((i) =>
        pool.run(async () => {
          started.push(i);
          throw error(400);
        }),
      ),
    );
    expect(started).toEqual([0, 1]);
    expect(pool.stopped).not.toBeNull();
  });

  it("never runs more requests at once than its limit", async () => {
    const pool = new Pool(3, instant);
    let running = 0;
    let peak = 0;
    await Promise.all(
      Array.from({ length: 10 }, () =>
        pool.run(async () => {
          peak = Math.max(peak, ++running);
          await new Promise((resolve) => setTimeout(resolve, 1));
          running--;
        }),
      ),
    );
    expect(peak).toBe(3);
  });
});

describe("ElevenLabs client", () => {
  it("sends the workspace as a query parameter and the token as a bearer", async () => {
    const seen: { url: string; auth: string | null }[] = [];
    const client = new ElevenLabsClient({
      fetch: (async (url: string, init?: RequestInit) => {
        seen.push({ url, auth: new Headers(init?.headers).get("authorization") });
        return new Response(new Uint8Array([1]));
      }) as unknown as typeof fetch,
      workspace: "slug",
      token: "secret",
    });
    await client.tts("abc", "Hi.", { stability: 0.5 });
    expect(seen[0].url).toMatch(
      /^https:\/\/ctl\.duet\.so\/v1\/services\/elevenlabs\/v1\/text-to-speech\/abc\?.*workspace=slug/,
    );
    expect(seen[0].auth).toBe("Bearer secret");
  });
});
