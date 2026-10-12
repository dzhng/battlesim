// @vitest-environment node
// The voice bank: a battle's sets load from their packs in the background, a
// line whose take isn't loaded is simply not there, and one set's failure
// never silences another. A loaded take stays compressed; it is decoded only
// when a line plays it, and only the last few decoded stay decoded.
import { expect, test } from "vitest";
import {
  DECODED_TAKES,
  TAKES_IN_FLIGHT,
  VoiceBank,
  type BankListener,
} from "@packages/battle-audio/src/voices/voiceBank";
import {
  packUrl,
  voicePack,
  voiceUrl,
  type VoiceCatalog,
} from "@packages/battle-audio/src/voices/voiceCatalog";

const CATALOG = {
  roles: { crew: { lines: {} } },
  sets: {
    tank: {
      role: "crew",
      country: "us",
      callsign: "Oscar 5",
      processing: "voice-radio-light",
      voices: [
        { id: "lead", speaks: "responses", gender: "male", description: "", voice_id: "x" },
        { id: "gunner", speaks: "barks", gender: "male", description: "", voice_id: null },
      ],
    },
  },
  assignments: {},
  retakes: {},
} as unknown as VoiceCatalog;

const SHA = "ab".repeat(32);
const TANK = voicePack(CATALOG, "tank", (id) =>
  id === "tank/lead/move/steady/0" || id === "tank/lead/move/stressed/1"
    ? { sha256: SHA, duration_s: 1.5 }
    : null,
);

/** A context whose decode turns the fetched bytes' length into a buffer's
 *  duration, and counts its decodes. Like a browser's, it takes the bytes. */
function decoding() {
  const counted = { decodes: 0 };
  const context = {
    async decodeAudioData(bytes: ArrayBuffer) {
      counted.decodes++;
      const duration = bytes.byteLength;
      structuredClone(bytes, { transfer: [bytes] });
      return { duration } as AudioBuffer;
    },
  } as unknown as BaseAudioContext;
  return { counted, context };
}
const { context } = decoding();

/** The audio a line of take `variant` of `sound` plays, from `bank`, once decoded. */
function heard(bank: VoiceBank, sound: string, variant: number) {
  return new Promise<AudioBuffer | null>((resolve) => {
    if (!bank.play(sound, variant, 0, 1, () => 0, resolve)) resolve(null);
  });
}

/** What a bank told its listener, in order. */
function listening() {
  const told: string[] = [];
  const listener: BankListener = {
    voices: (set, voices) => told.push(`${set}: ${voices.map((v) => v.id).join(", ")}`),
    loaded: (set, voice, trigger, intensity, take, duration) =>
      told.push(`${set}/${voice}/${trigger}/${intensity}/${take} ${duration} s`),
  };
  return { told, listener };
}

function server(packs: Record<string, unknown>) {
  const asked: string[] = [];
  const release: (() => void)[] = [];
  const fetcher = (async (url: string) => {
    asked.push(url);
    await new Promise<void>((resolve) => release.push(resolve));
    const set = Object.keys(packs).find((name) => url === packUrl(name));
    if (set) return Response.json(packs[set]);
    if (url.startsWith("/audio/voices/tank/")) return new Response(new Uint8Array(3));
    return new Response("missing", { status: 404 });
  }) as typeof fetch;
  /** Answers every request made so far, then lets the bank's next steps run. */
  const flush = async (rounds = 4) => {
    for (let round = 0; round < rounds; round++) {
      for (const go of release.splice(0)) go();
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  };
  return { asked, fetcher, flush };
}

test("a pack lists a set's served takes, its voices and their roles", () => {
  expect(TANK.voices).toEqual([
    { id: "lead", speaks: "responses" },
    { id: "gunner", speaks: "barks" },
  ]);
  expect(TANK.takes).toEqual([
    {
      voice: "lead",
      trigger: "move",
      intensity: "steady",
      take: 0,
      url: voiceUrl(
        { set: "tank", voice: "lead", trigger: "move", intensity: "steady", take: 0 },
        SHA,
      ),
      duration_s: 1.5,
    },
    {
      voice: "lead",
      trigger: "move",
      intensity: "stressed",
      take: 1,
      url: voiceUrl(
        { set: "tank", voice: "lead", trigger: "move", intensity: "stressed", take: 1 },
        SHA,
      ),
      duration_s: 1.5,
    },
  ]);
});

test("loading runs in the background; a take is there once fetched, as long as its pack says", async () => {
  const { asked, fetcher, flush } = server({ tank: TANK });
  const { told, listener } = listening();
  const { counted, context } = decoding();
  const bank = new VoiceBank(context, fetcher, undefined, { listener });
  const loaded = bank.load(["tank"]);
  expect(
    bank.play(
      "tank/lead/move/steady",
      0,
      0,
      1,
      () => 0,
      () => {},
    ),
  ).toBe(false);
  expect(told).toEqual([]);
  await flush();
  await loaded;
  expect(asked[0]).toBe(packUrl("tank"));
  // The set's voices first, then each take as it arrives, with the length its pack records.
  expect(told).toEqual([
    "tank: lead, gunner",
    "tank/lead/move/steady/0 1.5 s",
    "tank/lead/move/stressed/1 1.5 s",
  ]);
  // Loaded takes are held as fetched: nothing is decoded until a line plays.
  expect(counted.decodes).toBe(0);
  expect(bank.held()).toEqual({ takes: 2, bytes: 6, decoded: 0 });
  expect((await heard(bank, "tank/lead/move/steady", 0))?.duration).toBe(3);
  expect(counted.decodes).toBe(1);
  // Not in the pack: never fetched, never there.
  expect(await heard(bank, "tank/lead/move/steady", 1)).toBeNull();
  expect(asked.filter((url) => url.endsWith(".mp3"))).toHaveLength(2);
});

test("a take decodes for each line that plays it while few others have played since", async () => {
  // Decoded audio is ~20 times its MP3 bytes: a battle keeps only the last few decoded.
  const every = voicePack(CATALOG, "tank", () => ({ sha256: SHA, duration_s: 1.5 }));
  const { fetcher, flush } = server({ tank: every });
  const { counted, context } = decoding();
  const bank = new VoiceBank(context, fetcher);
  const loaded = bank.load(["tank"]);
  while (bank.held().takes < every.takes.length) await flush(1);
  await loaded;
  const sounds = every.takes.map(
    (t) => [`tank/${t.voice}/${t.trigger}/${t.intensity}`, t.take] as const,
  );
  expect(sounds.length).toBeGreaterThan(DECODED_TAKES + 1);
  for (const [sound, take] of sounds) await heard(bank, sound, take);
  expect(counted.decodes).toBe(sounds.length);
  expect(bank.held().decoded).toBe(DECODED_TAKES);
  // The last played is still decoded; the first has to be decoded again.
  await heard(bank, ...sounds.at(-1)!);
  expect(counted.decodes).toBe(sounds.length);
  expect((await heard(bank, ...sounds[0]))?.duration).toBe(3);
  expect(counted.decodes).toBe(sounds.length + 1);
  expect(bank.held().decoded).toBe(DECODED_TAKES);
});

test("a line plays when its take is decoded, unless that is past its wait", async () => {
  const { fetcher, flush } = server({ tank: TANK });
  let decoded!: () => void;
  const slow = {
    decodeAudioData: (bytes: ArrayBuffer) =>
      new Promise<AudioBuffer>((resolve) => {
        decoded = () => resolve({ duration: bytes.byteLength } as AudioBuffer);
      }),
  } as unknown as BaseAudioContext;
  const bank = new VoiceBank(slow, fetcher);
  const loaded = bank.load(["tank"]);
  await flush();
  await loaded;
  let clock = 10;
  const played: number[] = [];
  // Asked to start at 10 s with 1 s to wait; decoded at 10.4 s, it starts then.
  expect(
    bank.play(
      "tank/lead/move/steady",
      0,
      10,
      1,
      () => clock,
      (_, at) => played.push(at),
    ),
  ).toBe(true);
  clock = 10.4;
  decoded();
  await bank.settled();
  expect(played).toEqual([10.4]);
  // Decoded at 11.5 s, past its wait: dropped.
  bank.play(
    "tank/lead/move/stressed",
    1,
    10,
    1,
    () => clock,
    (_, at) => played.push(at),
  );
  clock = 11.5;
  decoded();
  await bank.settled();
  expect(played).toEqual([10.4]);
});

test("a battle's takes download a few at a time, and all of them arrive", async () => {
  // A skirmish asks for thousands of takes; a browser refuses requests past a
  // few hundred outstanding, the battle's own downloads with them.
  const every = voicePack(CATALOG, "tank", () => ({ sha256: SHA, duration_s: 1.5 }));
  const { asked, fetcher, flush } = server({ tank: every });
  const bank = new VoiceBank(context, fetcher);
  const loaded = bank.load(["tank"]);
  let answered = 0;
  let peak = 0;
  while (answered < every.takes.length) {
    await flush(1);
    const takes = asked.filter((url) => url.endsWith(".mp3")).length;
    peak = Math.max(peak, takes - answered);
    answered = takes;
  }
  await flush();
  await loaded;
  expect(every.takes.length).toBeGreaterThan(TAKES_IN_FLIGHT * 4);
  expect(peak).toBeLessThanOrEqual(TAKES_IN_FLIGHT);
  expect(bank.held().takes).toBe(every.takes.length);
});

test("a set without a pack stays silent and the others still load", async () => {
  const { fetcher, flush } = server({ tank: TANK });
  const { told, listener } = listening();
  const bank = new VoiceBank(context, fetcher, undefined, { listener });
  const loaded = bank.load(["absent", "tank"]);
  await flush();
  await loaded;
  expect(await heard(bank, "tank/lead/move/steady", 0)).not.toBeNull();
  expect(told.filter((t) => t.startsWith("absent"))).toEqual([]);
  expect([...bank.errors.keys()]).toEqual(["absent"]);
});

test("disposing stops loading: nothing arrives after", async () => {
  const { asked, fetcher, flush } = server({ tank: TANK });
  const { told, listener } = listening();
  const bank = new VoiceBank(context, fetcher, undefined, { listener });
  const loaded = bank.load(["tank"]);
  // Dispose while the takes are being fetched: the pack has arrived, the audio hasn't.
  while (!asked.some((url) => url.endsWith(".mp3"))) await flush(1);
  bank.dispose();
  await flush();
  await loaded;
  expect(await heard(bank, "tank/lead/move/steady", 0)).toBeNull();
  expect(told).toEqual(["tank: lead, gunner"]);
  expect(bank.errors.size).toBe(0);
});
