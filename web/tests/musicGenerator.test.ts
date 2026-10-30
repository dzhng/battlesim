// @vitest-environment node
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, expect, test } from "vitest";
import { musicUrl } from "@packages/battle-audio/src/music/musicCatalog";
import {
  isGenerated,
  validateSoundCatalog,
  type GeneratedSource,
  type SoundCatalog,
} from "@packages/battle-audio/src/catalog";
import { ElevenLabsClient } from "@packages/battle-audio/tools/generate/elevenlabs";
import {
  PULSE,
  STYLE,
  generateMusic,
  type MusicEncoder,
} from "@packages/battle-audio/tools/generate/music";

/** Every generation differs, as a real one does. */
let generations = 0;
/** The proxy at the network edge: a track's bytes name its request and the generation that made it. */
function fakeProxy(options: { failFirst?: number } = {}) {
  const prompts: string[] = [];
  let inFlight = 0;
  let peak = 0;
  let failures = options.failFirst ?? 0;
  const fetch = async (url: string | URL | Request, init?: RequestInit) => {
    const path = new URL(String(url)).pathname;
    if (!path.endsWith("/v1/music")) return new Response("not found", { status: 404 });
    inFlight++;
    peak = Math.max(peak, inFlight);
    await new Promise((resolve) => setTimeout(resolve, 2));
    inFlight--;
    if (failures-- > 0) return new Response("upstream error", { status: 500 });
    const body = JSON.parse(String(init?.body));
    prompts.push(body.prompt);
    return new Response(
      new TextEncoder().encode(`music:${body.model_id}:${body.music_length_ms}:${++generations}`),
    );
  };
  const client = new ElevenLabsClient({
    fetch: fetch as typeof globalThis.fetch,
    workspace: "w",
    token: "t",
  });
  return { client, prompts, peak: () => peak };
}
/** Encoding at the ffmpeg edge: served bytes follow the master, and every master measures −14 LUFS. */
const encoder: MusicEncoder = async ({ input, output }) => {
  writeFileSync(output, `served:${readFileSync(input)}`);
  return { duration_s: 150, loudness_lufs: -14 };
};

const roots: string[] = [];
afterEach(() => roots.splice(0).forEach((root) => rmSync(root, { recursive: true, force: true })));
/** A repository holding a catalog whose only music is the menu bed. */
function workspace() {
  const root = mkdtempSync(join(tmpdir(), "music-generator-"));
  roots.push(root);
  mkdirSync(join(root, "fixtures"));
  const catalog: SoundCatalog = {
    sources: {
      menu: {
        label: "Menu · bed",
        author: "Author",
        license: "CC0-1.0",
        url: "https://example.com/menu",
        path: "assets/third-party/audio/menu.mp3",
        sha256: "a".repeat(64),
        notes: "",
      },
    },
    clips: {},
    sounds: {},
    music: {
      moods: { menu: ["menu"], tension: [], combat: [] },
      tracks: { menu: { source: "menu", sha256: "b".repeat(64), gain: 0.16 } },
    },
  };
  writeFileSync(join(root, "fixtures/sounds.json"), `${JSON.stringify(catalog, null, 2)}\n`);
  return root;
}
const read = (root: string): SoundCatalog =>
  JSON.parse(readFileSync(join(root, "fixtures/sounds.json"), "utf8"));
const noWait = async () => {};
const run = (root: string, proxy: ReturnType<typeof fakeProxy>, tracks?: string[]) =>
  generateMusic({ root, client: proxy.client, encoder, tracks, log: () => {}, sleep: noWait });

test("generates 3 combat and 3 tension tracks into the catalog, two calls at a time", async () => {
  const root = workspace();
  const proxy = fakeProxy();
  const result = await run(root, proxy);
  expect(result).toMatchObject({ calls: 6, seconds: 6 * 150, failed: 0 });
  expect(proxy.peak()).toBe(2);

  const catalog = validateSoundCatalog(read(root));
  expect(catalog.music.moods.menu).toEqual(["menu"]);
  expect(catalog.music.moods.combat).toHaveLength(3);
  expect(catalog.music.moods.tension).toHaveLength(3);
  for (const mood of ["combat", "tension"] as const)
    for (const id of catalog.music.moods[mood]) {
      const track = catalog.music.tracks[id];
      const source = catalog.sources[track.source] as GeneratedSource;
      expect(isGenerated(source)).toBe(true);
      expect(source).toMatchObject({ model: "music_v1", length_s: 150, plan: "Duet ElevenLabs" });
      expect(source.prompt.startsWith(STYLE)).toBe(true);
      expect(source.prompt.includes(PULSE)).toBe(mood === "tension");
      // The raw master is kept; the served file is the 128 kbps encode of exactly it.
      const raw = readFileSync(join(root, source.path), "utf8");
      expect(raw).toMatch(/^music:music_v1:150000:/);
      const served = join(root, "assets/runtime", musicUrl(id, track));
      expect(readFileSync(served, "utf8")).toBe(`served:${raw}`);
      // −14 LUFS lowered to the −26 LUFS default: 12 dB down.
      expect(track.gain).toBeCloseTo(10 ** (-12 / 20), 3);
    }
});

test("a rerun pays for nothing; --track buys that track again, keeps its old master and drops its old served file", async () => {
  const root = workspace();
  await run(root, fakeProxy());
  const before = read(root);
  const again = fakeProxy();
  expect((await run(root, again)).calls).toBe(0);
  expect(read(root)).toEqual(before);

  const [first, second] = before.music.moods.combat;
  const regenerated = fakeProxy();
  expect((await run(root, regenerated, [first])).calls).toBe(1);
  const after = read(root);
  const oldSource = before.sources[first] as GeneratedSource;
  const newSource = after.sources[first] as GeneratedSource;
  expect(newSource.sha256).not.toBe(oldSource.sha256);
  // The paid-for raw master is kept; only its served derivation goes.
  expect(existsSync(join(root, oldSource.path))).toBe(true);
  expect(
    existsSync(join(root, "assets/runtime", musicUrl(first, before.music.tracks[first]))),
  ).toBe(false);
  expect(existsSync(join(root, "assets/runtime", musicUrl(first, after.music.tracks[first])))).toBe(
    true,
  );
  expect(after.sources[second]).toEqual(before.sources[second]);
});

test("the occasional 500 is retried rather than losing the track", async () => {
  const root = workspace();
  const proxy = fakeProxy({ failFirst: 2 });
  const result = await run(root, proxy);
  expect(result).toMatchObject({ calls: 6, failed: 0 });
});

test("an unknown track is refused before anything is spent", async () => {
  const root = workspace();
  const proxy = fakeProxy();
  await expect(run(root, proxy, ["nope"])).rejects.toThrow(/nope/);
  expect(proxy.prompts).toEqual([]);
});
