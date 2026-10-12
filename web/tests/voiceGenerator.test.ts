// @vitest-environment node
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { afterEach, describe, expect, it } from "vitest";
import { takeId, type VoiceCatalog } from "@packages/battle-audio/src/voices/voiceCatalog";
import { ElevenLabsClient } from "@packages/battle-audio/tools/generate/elevenlabs";
import { Pool } from "@packages/battle-audio/tools/generate/pool";
import { matches } from "@packages/battle-audio/tools/generate/transcript";
import { readLock } from "@packages/battle-audio/tools/generate/lock";
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

/** The proxy at the network edge: speech is the text's bytes, a transcript echoes the speech. */
function fakeProxy(options: { failAfter?: number; transcript?: (spoken: string) => string } = {}) {
  /** Requests the proxy answered, so the ones that cost credits. */
  const calls: string[] = [];
  let requests = 0;
  const fetch = async (url: string | URL | Request, init?: RequestInit) => {
    const path = new URL(String(url)).pathname;
    if (options.failAfter !== undefined && ++requests > options.failAfter)
      return new Response('{"detail":{"status":"invalid_api_key"}}', { status: 401 });
    calls.push(path);
    if (path.includes("/text-to-speech/")) {
      const { text } = JSON.parse(String(init?.body));
      return new Response(new TextEncoder().encode(`speech:${text}`));
    }
    if (path.endsWith("/speech-to-text")) {
      const file = (init!.body as FormData).get("file") as Blob;
      const spoken = (await file.text()).replace(/^speech:/, "");
      return Response.json({ text: (options.transcript ?? ((s) => s))(spoken) });
    }
    if (path.endsWith("/sound-generation")) return new Response(new Uint8Array([1, 2, 3]));
    return new Response("not found", { status: 404 });
  };
  const client = new ElevenLabsClient({
    fetch: fetch as typeof globalThis.fetch,
    workspace: "w",
    token: "t",
  });
  return { client, calls, tts: () => calls.filter((c) => c.includes("/text-to-speech/")).length };
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
    expect(selectTakes(c, { beds: {}, takes: {} }, select).takes).toHaveLength(4);
    expect(
      selectTakes(c, { beds: {}, takes: {} }, { ...select, voices: ["lead"] }).takes.map(takeId),
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

  it("flags a misheard keyword even when the rest of the line is right", () => {
    const result = matches("Gunner, sabot, tank!", "Gutter, sabot, tank!", { callsign: "Oscar 1" });
    expect(result).toMatchObject({ flagged: true, missing: ["gunner"] });
  });

  it("flags a take longer than its trigger's talk time", () => {
    const line = "Roger, moving out.";
    expect(matches(line, line, { callsign: "", durationS: 2, talkS: 3 }).flagged).toBe(false);
    expect(matches(line, line, { callsign: "", durationS: 4, talkS: 3 })).toMatchObject({
      flagged: true,
      overlong: true,
    });
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
