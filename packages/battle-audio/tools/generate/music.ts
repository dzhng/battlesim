import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import type { GeneratedSource, SoundCatalog } from "../../src/catalog";
import { musicUrl } from "../../src/music/musicCatalog";
import { sha256 } from "./cacheKey";
import type { ElevenLabsClient } from "./elevenlabs";
import { writeAtomic } from "./lock";
import { LIMITS, Pool } from "./pool";

/** The approved opening of every prompt: a restrained underscore, never a lead. */
export const STYLE =
  "Background underscore for a modern military real-time strategy game, in the style of hybrid analog-synth and orchestral game scores. It must sit underneath gunfire, explosions and radio dialogue: no lead melody, no hooks, no solos, no big hits or drops, consistent low-mid energy throughout, dark and wide. Instrumental.";
/** The combat score's palette, which a tension track names so it reads as the same score. */
export const PULSE =
  "Steady pulsing analog synth bass ostinato in D minor, low sustained strings, distant brass swells, same palette as a 125 BPM hybrid combat score.";
const COMBAT =
  "Steady pulsing analog synth bass ostinato in D minor, subdued electronic drums and low taiko kept far back in the mix, soft sustained low strings, distant brass swells. 125 BPM.";
const SPARSE =
  "Calm reconnaissance version of that score: the bass ostinato at half intensity, filtered and soft, no drums at all, only a quiet ticking pulse, long low string pads. 125 BPM.";
const DARK =
  "Brooding pre-battle version of that score: a slow low synth pulse and sub-bass heartbeat, faint distant taiko every few bars, dark pads and occasional low brass. 125 BPM.";

export const MUSIC_MODEL = "music_v1";
/** Integrated loudness every battle track is set to by its gain (choices.md T12). */
export const MUSIC_LUFS = -26;
const LENGTH_S = 150;
const RAW = "assets/source/audio/music";
const SOUNDS = "fixtures/sounds.json";
const ASSETS = fileURLToPath(new URL("../assets.py", import.meta.url));
const OXFMT = fileURLToPath(new URL("../../../../web/node_modules/.bin/oxfmt", import.meta.url));

/** The battle score: each track's mood and its prompt after `STYLE` (and `PULSE` for tension). */
export const MUSIC_TRACKS: Record<string, { mood: "combat" | "tension"; direction: string }> = {
  combat_1: {
    mood: "combat",
    direction: `${COMBAT} The bass ostinato and low strings carry it; the drums are a soft, distant pulse; one flat, even intensity from start to end.`,
  },
  combat_2: {
    mood: "combat",
    direction: `${COMBAT} The low taiko and electronic drums lead the texture, still far back; brass swells every eight bars; a slow build over the first minute, then a steady plateau.`,
  },
  combat_3: {
    mood: "combat",
    direction: `${COMBAT} Darker and more urgent: lower-register strings and a syncopated variation of the ostinato, gentle swells and recessions every thirty seconds, never peaking.`,
  },
  tension_sparse_1: {
    mood: "tension",
    direction: `${SPARSE} Wide and still, the ticking pulse alone for long stretches.`,
  },
  tension_sparse_2: {
    mood: "tension",
    direction: `${SPARSE} Slightly uneasy: the filtered ostinato drifts in and out under slow string swells.`,
  },
  tension_dark_1: { mood: "tension", direction: DARK },
};
export const musicPrompt = (id: string) => {
  const { mood, direction } = MUSIC_TRACKS[id];
  return [STYLE, ...(mood === "tension" ? [PULSE] : []), direction].join(" ");
};

/** Encodes a raw master into `output` and measures it. */
export type MusicEncoder = (job: {
  input: string;
  output: string;
}) => Promise<{ duration_s: number; loudness_lufs: number }>;

export interface MusicRun {
  root: string;
  client: ElevenLabsClient;
  encoder: MusicEncoder;
  /** Tracks to buy again even when held; without it, only missing or changed tracks are made. */
  tracks?: string[];
  dryRun?: boolean;
  log: (line: string) => void;
  sleep?: (ms: number) => Promise<void>;
}
export interface MusicRunResult {
  calls: number;
  /** Seconds of music requested from the proxy. */
  seconds: number;
  failed: number;
  stopped: Error | null;
}

const today = () => new Date().toISOString().slice(0, 10);
/** `record` with `entry` set, the score's tracks kept after everything else in table order. */
function inOrder<T>(record: Record<string, T>, entry: Record<string, T>): Record<string, T> {
  const merged = { ...record, ...entry };
  const rank = (key: string) => (MUSIC_TRACKS[key] ? Object.keys(MUSIC_TRACKS).indexOf(key) : -1);
  return Object.fromEntries(Object.entries(merged).sort(([a], [b]) => rank(a) - rank(b)));
}
/**
 * Writes the catalog as it is kept: non-ASCII escaped as `assets.py build` writes it,
 * laid out by the repo's formatter, so an added track never reflows the rest of the file.
 */
export function writeCatalog(path: string, catalog: SoundCatalog) {
  const text = JSON.stringify(catalog, null, 2).replace(
    /[\u0080-\uffff]/g,
    (c) => `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`,
  );
  const temporary = join(dirname(path), `.${basename(path, ".json")}.${process.pid}.json`);
  writeFileSync(temporary, `${text}\n`);
  const formatted = spawnSync(OXFMT, [temporary], { encoding: "utf8" });
  if (formatted.status !== 0) {
    rmSync(temporary, { force: true });
    throw new Error(`oxfmt ${path}: ${formatted.stderr || formatted.error}`);
  }
  renameSync(temporary, path);
}

/**
 * `generate music`: brings each battle track up to date. A raw master is paid for once
 * and kept while its prompt, model and length stand; its served encode and gain follow
 * it locally. Each finished track is written to the catalog at once, so a kill keeps it.
 */
export async function generateMusic(run: MusicRun): Promise<MusicRunResult> {
  const { root, client, log } = run;
  for (const id of run.tracks ?? [])
    if (!MUSIC_TRACKS[id]) throw new Error(`music track ${id}: not in MUSIC_TRACKS`);
  const path = join(root, SOUNDS);
  const catalog = JSON.parse(readFileSync(path, "utf8")) as SoundCatalog;
  const pool = new Pool(LIMITS.music, { sleep: run.sleep });
  const result: MusicRunResult = { calls: 0, seconds: 0, failed: 0, stopped: null };
  const ids = Object.keys(MUSIC_TRACKS);
  let done = 0;

  async function produce(id: string): Promise<string> {
    const prompt = musicPrompt(id);
    const held = catalog.sources[id] as GeneratedSource | undefined;
    const fresh =
      !run.tracks?.includes(id) &&
      held?.kind === "Generated" &&
      held.prompt === prompt &&
      held.model === MUSIC_MODEL &&
      held.length_s === LENGTH_S &&
      existsSync(join(root, held.path)) &&
      sha256(readFileSync(join(root, held.path))) === held.sha256;
    let source = held!;
    if (!fresh) {
      result.seconds += LENGTH_S;
      if (run.dryRun) return `would generate ${LENGTH_S}s`;
      const bytes = await pool.run(() => client.music(prompt, LENGTH_S * 1000, MUSIC_MODEL));
      result.calls++;
      const sha = sha256(bytes);
      const raw = `${RAW}/${id}.${sha.slice(0, 8)}.mp3`;
      writeAtomic(join(root, raw), bytes);
      source = {
        kind: "Generated",
        generator: "ElevenLabs music",
        model: MUSIC_MODEL,
        prompt,
        length_s: LENGTH_S,
        date: today(),
        plan: "Duet ElevenLabs",
        path: raw,
        sha256: sha,
      };
    } else if (run.dryRun) return "cached";
    const previous = catalog.music.tracks[id];
    const servedAt = (sha: string) =>
      join(root, "assets/runtime", musicUrl(id, { ...previous, sha256: sha }));
    if (fresh && previous && existsSync(servedAt(previous.sha256))) return "cached";
    const output = join(root, "assets/runtime/audio/music", `.${id}.${source.sha256}.mp3`);
    mkdirSync(dirname(output), { recursive: true });
    const measured = await run.encoder({ input: join(root, source.path), output });
    const served = sha256(readFileSync(output));
    renameSync(output, servedAt(served));
    if (previous && previous.sha256 !== served) rmSync(servedAt(previous.sha256), { force: true });
    const gain = Number((10 ** ((MUSIC_LUFS - measured.loudness_lufs) / 20)).toFixed(3));
    catalog.sources = inOrder(catalog.sources, { [id]: source });
    catalog.music.tracks = inOrder(catalog.music.tracks, {
      [id]: { source: id, sha256: served, gain },
    });
    for (const mood of ["combat", "tension"] as const)
      catalog.music.moods[mood] = ids.filter(
        (name) => MUSIC_TRACKS[name].mood === mood && catalog.music.tracks[name],
      );
    writeCatalog(path, catalog);
    return `${fresh ? "re-encoded" : "generated"} ${measured.duration_s}s ${measured.loudness_lufs} LUFS gain ${gain}`;
  }

  await Promise.all(
    ids.map(async (id) => {
      try {
        log(`[${++done}/${ids.length}] music ${id} ${await produce(id)}`);
      } catch (error) {
        result.failed++;
        result.stopped ??= pool.stopped;
        log(
          `[${++done}/${ids.length}] music ${id} failed: ${error instanceof Error ? error.message : error}`,
        );
      }
    }),
  );
  return result;
}

/** Encoding through `assets.py music`, the only owner of ffmpeg. */
export const assetsEncoder: MusicEncoder = ({ input, output }) =>
  new Promise((resolve, reject) => {
    const child = spawn("python3", [ASSETS, "music", "--input", input, "--output", output]);
    let stdout = "";
    let stderr = "";
    child.stdout.on("data", (chunk) => (stdout += chunk));
    child.stderr.on("data", (chunk) => (stderr += chunk));
    child.on("error", reject);
    child.on("close", (code) =>
      code === 0
        ? resolve(JSON.parse(stdout))
        : reject(new Error(`assets.py music: ${stderr.trim()}`)),
    );
  });
