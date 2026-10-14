import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { EFFECT_MASTERS, type GeneratedSource, type SoundCatalog } from "../../src/catalog";
import { sha256 } from "./cacheKey";
import type { ElevenLabsClient } from "./elevenlabs";
import { writeAtomic } from "./lock";
import { writeCatalog } from "./music";
import { LIMITS, Pool } from "./pool";

export const EFFECT_MODEL = "eleven_text_to_sound_v2";
const SOUNDS = "fixtures/sounds.json";
/** What every effect prompt rules out: the game's recordings are of machines, never of people. */
const NOTHING_HUMAN =
  "Only the machine: no voices, no people, no crowd, no talking, no shouting, no cheering, no radio chatter, no music.";

/**
 * Sound effects the game has no clean recording of, bought from ElevenLabs as
 * `candidates` raw masters each. Every candidate becomes a generated source in the
 * sound catalog, cut into a clip by `assets.py` like any recording; a recipe picks one.
 */
export const SOUND_EFFECTS: Record<
  string,
  { prompt: string; length_s: number; loop: boolean; candidates: number }
> = {
  "airwar-jet-engine": {
    prompt: `A fighter jet's turbofan engine heard from the ground as the aircraft holds at altitude: a steady deep roar with a high turbine whine, even level throughout, seamless loop. ${NOTHING_HUMAN}`,
    length_s: 10,
    loop: true,
    candidates: 3,
  },
  "airwar-flare-burst": {
    prompt: `A military aircraft fires a salvo of countermeasure flares: a rapid string of sharp pyrotechnic pops, each followed by a short fizzing burn, close by, then silence. ${NOTHING_HUMAN}`,
    length_s: 3,
    loop: false,
    candidates: 3,
  },
};
/** The source a candidate is kept as: `<effect>-<n>`, 1-based. */
export const effectSource = (effect: string, n: number) => `${effect}-${n}`;

export interface EffectRun {
  root: string;
  client: ElevenLabsClient;
  dryRun?: boolean;
  log: (line: string) => void;
}

const today = () => new Date().toISOString().slice(0, 10);

/**
 * `generate effects`: buys each effect's missing or changed candidates. A raw master is
 * paid for once and kept while its prompt, model and length stand; every candidate is
 * written to the catalog's sources at once, so a kill keeps what was bought.
 */
export async function generateEffects(run: EffectRun): Promise<{ calls: number; failed: number }> {
  const { root, client, log } = run;
  const path = join(root, SOUNDS);
  const catalog = JSON.parse(readFileSync(path, "utf8")) as SoundCatalog;
  const pool = new Pool(LIMITS.music);
  const result = { calls: 0, failed: 0 };
  const jobs = Object.entries(SOUND_EFFECTS).flatMap(([effect, spec]) =>
    Array.from({ length: spec.candidates }, (_, k) => ({
      effect,
      spec,
      name: effectSource(effect, k + 1),
    })),
  );
  await Promise.all(
    jobs.map(async ({ spec, name }) => {
      const held = catalog.sources[name] as GeneratedSource | undefined;
      const fresh =
        held?.kind === "Generated" &&
        held.prompt === spec.prompt &&
        held.model === EFFECT_MODEL &&
        held.length_s === spec.length_s &&
        existsSync(join(root, held.path)) &&
        sha256(readFileSync(join(root, held.path))) === held.sha256;
      if (fresh) return log(`effect ${name} cached`);
      if (run.dryRun) return log(`effect ${name} would generate ${spec.length_s}s`);
      try {
        const bytes = await pool.run(() =>
          client.sound(spec.prompt, spec.length_s, spec.loop, EFFECT_MODEL),
        );
        result.calls++;
        const sha = sha256(bytes);
        const raw = `${EFFECT_MASTERS}/${name}.${sha.slice(0, 8)}.mp3`;
        writeAtomic(join(root, raw), bytes);
        catalog.sources[name] = {
          kind: "Generated",
          generator: "ElevenLabs sound effects",
          model: EFFECT_MODEL,
          prompt: spec.prompt,
          length_s: spec.length_s,
          date: today(),
          plan: "Duet ElevenLabs",
          path: raw,
          sha256: sha,
        };
        writeCatalog(path, catalog);
        log(`effect ${name} generated ${bytes.length} bytes`);
      } catch (error) {
        result.failed++;
        log(`effect ${name} failed: ${error instanceof Error ? error.message : error}`);
      }
    }),
  );
  return result;
}
