// @vitest-environment node
// The voice bank: a battle's sets load from their packs in the background, a
// line whose take isn't loaded is simply not there, and one set's failure
// never silences another.
import { expect, test } from "vitest";
import { VoiceBank, type BankListener } from "@packages/battle-audio/src/voices/voiceBank";
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

/** A context whose decode turns the fetched bytes' length into a buffer's duration. */
const context = {
  async decodeAudioData(bytes: ArrayBuffer) {
    return { duration: bytes.byteLength } as AudioBuffer;
  },
} as unknown as BaseAudioContext;

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

test("loading runs in the background; a take is there only once its audio is decoded", async () => {
  const { asked, fetcher, flush } = server({ tank: TANK });
  const { told, listener } = listening();
  const bank = new VoiceBank(context, fetcher, undefined, { listener });
  const loaded = bank.load(["tank"]);
  expect(bank.line("tank/lead/move/steady", 0)).toBeNull();
  expect(told).toEqual([]);
  await flush();
  await loaded;
  expect(asked[0]).toBe(packUrl("tank"));
  // The set's voices first, then each take as it decodes.
  expect(told).toEqual([
    "tank: lead, gunner",
    "tank/lead/move/steady/0 3 s",
    "tank/lead/move/stressed/1 3 s",
  ]);
  expect(bank.line("tank/lead/move/steady", 0)?.duration).toBe(3);
  expect(bank.line("tank/lead/move/stressed", 1)?.duration).toBe(3);
  // Not in the pack: never fetched, never there.
  expect(bank.line("tank/lead/move/steady", 1)).toBeNull();
  expect(asked.filter((url) => url.endsWith(".mp3"))).toHaveLength(2);
});

test("a set without a pack stays silent and the others still load", async () => {
  const { fetcher, flush } = server({ tank: TANK });
  const { told, listener } = listening();
  const bank = new VoiceBank(context, fetcher, undefined, { listener });
  const loaded = bank.load(["absent", "tank"]);
  await flush();
  await loaded;
  expect(bank.line("tank/lead/move/steady", 0)).not.toBeNull();
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
  expect(bank.line("tank/lead/move/steady", 0)).toBeNull();
  expect(told).toEqual(["tank: lead, gunner"]);
  expect(bank.errors.size).toBe(0);
});
