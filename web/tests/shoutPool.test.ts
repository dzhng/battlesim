// @vitest-environment node
// The unseen enemy's shout pool: stressed barks of the enemy faction's
// shouting voices, a few per voice, loaded from the packs that exist; a set
// without a pack adds nothing.
import { expect, test } from "vitest";
import { ShoutPool, shoutTakes } from "@packages/battle-audio/src/voices/shoutPool";
import {
  factionVoices,
  packUrl,
  type PackTake,
  type VoicePack,
} from "@packages/battle-audio/src/voices/voiceCatalog";

const take = (voice: string, trigger: string, intensity: string, n: number): PackTake =>
  ({
    voice,
    trigger,
    intensity,
    take: n,
    url: `/audio/voices/rifles/${voice}-${trigger}-${intensity}-${n}.mp3`,
    duration_s: 2,
  }) as PackTake;

const RIFLES: VoicePack = {
  set: "rifles",
  voices: [
    { id: "lead", speaks: "responses" },
    { id: "rifleman", speaks: "barks" },
  ],
  takes: [
    take("lead", "move", "stressed", 0),
    take("rifleman", "under_fire", "steady", 0),
    take("rifleman", "under_fire", "stressed", 1),
    take("rifleman", "under_fire", "stressed", 0),
    take("rifleman", "man_down", "stressed", 0),
    take("rifleman", "contact_infantry", "stressed", 0),
  ],
};

test("a pack's share is its shouting voices' stressed barks, first takes first, a few per voice", () => {
  expect(
    shoutTakes(RIFLES, 3).map((t) => `${t.voice}/${t.trigger}/${t.intensity}/${t.take}`),
  ).toEqual([
    "rifleman/contact_infantry/stressed/0",
    "rifleman/under_fire/stressed/0",
    "rifleman/man_down/stressed/0",
  ]);
});

test("the pool loads only its share, and a set without a pack adds nothing", async () => {
  const fetched: string[] = [];
  const fetcher = (async (url: string) => {
    fetched.push(url);
    if (url === packUrl("rifles")) return Response.json(RIFLES);
    if (url.endsWith(".mp3")) return new Response(new Uint8Array(2));
    return new Response("missing", { status: 404 });
  }) as typeof fetch;
  const context = {
    async decodeAudioData(bytes: ArrayBuffer) {
      return { duration: bytes.byteLength } as AudioBuffer;
    },
  } as unknown as BaseAudioContext;
  const pool = new ShoutPool(context, 2, { fetcher });
  const takes = await pool.load(["absent", "rifles"]);
  expect(takes.map((t) => t.sound).sort()).toEqual([
    "rifles/rifleman/contact_infantry/stressed/0",
    "rifles/rifleman/under_fire/stressed/0",
  ]);
  expect(fetched.filter((url) => url.endsWith(".mp3"))).toHaveLength(2);
  expect(await new ShoutPool(context, 2, { fetcher }).load(["absent"])).toEqual([]);
});

test("a faction speaks with the set of each unit its roster fields", () => {
  const assignments = { rifles: { us: "us_rifles", eastern: "ru_rifles" }, tank: "ru_tank" };
  const units = [
    { id: "rifles", factions: ["us", "eastern"] },
    { id: "tank", factions: ["eastern"] },
    { id: "unvoiced", factions: ["eastern"] },
  ];
  expect(factionVoices(assignments, units, "eastern")).toEqual({
    rifles: "ru_rifles",
    tank: "ru_tank",
  });
  expect(factionVoices(assignments, units, "us")).toEqual({ rifles: "us_rifles" });
});
