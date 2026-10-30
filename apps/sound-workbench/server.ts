import { randomUUID } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { Plugin } from "vite";
import { FixturePublication } from "../fixture-publication/publication";
import type { SoundCatalog } from "../../packages/battle-audio/src/catalog";
import { validateVoiceCatalog } from "../../packages/battle-audio/src/voices/voiceCatalog";
import type { AudioMix } from "../../packages/battle-audio/src/audioMix";
import type { Draft, Review, Save, Snapshot } from "./src/protocol";
import { WorkbenchError, equal, hash, jsonRoutes, object } from "./http";
import { GenerationQueue, generateTool, musicWorkbenchMiddleware } from "./generation";
import { VoiceWorkbenchStore, voiceWorkbenchMiddleware } from "./voiceStore";

interface Inputs {
  texts: string[];
  catalog: SoundCatalog;
  audio: AudioMix;
  looks: Snapshot["looks"];
  runMps: number;
  media: [string, string][];
  revision: string;
}
interface Candidate {
  review: Review;
  inputs: Inputs;
  catalog: SoundCatalog;
  after: string;
}

export class SoundWorkbenchStore {
  private readonly publication: FixturePublication;
  private readonly candidates = new Map<string, Candidate>();
  constructor(
    private readonly root: string,
    private readonly validateCatalog: (value: unknown) => SoundCatalog,
    private readonly baselines: readonly string[],
    private readonly admitSettings: (
      presentation: string,
      audio: string,
      catalog: SoundCatalog,
      weapons: string,
    ) => void,
  ) {
    this.publication = new FixturePublication(
      root,
      "sounds",
      (message, status) => new WorkbenchError(message, status),
    );
  }

  private async inputs(): Promise<Inputs> {
    const texts = await Promise.all(
      ["sounds", "rules", "presentation", "audio"].map((name) =>
        readFile(join(this.root, `fixtures/${name}.json`), "utf8"),
      ),
    );
    let catalog: SoundCatalog;
    try {
      catalog = this.validateCatalog(JSON.parse(texts[0]));
      this.admitSettings(texts[2], texts[3], catalog, JSON.stringify(JSON.parse(texts[1]).weapons));
    } catch (error) {
      throw new WorkbenchError(error instanceof Error ? error.message : String(error));
    }
    const presentation = JSON.parse(texts[2]);
    const audio = JSON.parse(texts[3]) as AudioMix;
    const media: [string, string][] = [];
    for (const [id, clip] of Object.entries(catalog.clips)) {
      let bytes;
      try {
        bytes = await readFile(join(this.root, "assets/runtime", clip.url.slice(1)));
      } catch {
        throw new WorkbenchError(`Clip ${id} is missing; fetch or rebuild its media`);
      }
      const digest = hash(bytes);
      if (digest !== clip.sha256)
        throw new WorkbenchError(`Clip ${id} does not match its admitted media hash`);
      media.push([id, digest]);
    }
    return {
      texts,
      catalog,
      audio,
      looks: presentation.weapon_looks,
      runMps: presentation.pose.gait.run_mps,
      media,
      revision: hash(JSON.stringify([texts, media])),
    };
  }

  private snapshotOf(input: Inputs): Snapshot {
    return {
      revision: input.revision,
      catalog: input.catalog,
      looks: input.looks,
      effects: input.audio.effects,
      vehicles: input.audio.vehicles,
      footsteps: input.audio.footsteps,
      runMps: input.runMps,
      music: input.audio.music,
    };
  }

  snapshot(): Promise<Snapshot> {
    return this.publication.capture(async () => this.snapshotOf(await this.inputs()));
  }

  private validate(candidate: unknown, input: Inputs): SoundCatalog {
    object(candidate, ["sources", "clips", "sounds", "music"]);
    let catalog;
    try {
      catalog = this.validateCatalog(candidate);
      this.admitSettings(
        input.texts[2],
        input.texts[3],
        catalog,
        JSON.stringify(JSON.parse(input.texts[1]).weapons),
      );
    } catch (error) {
      throw new WorkbenchError(error instanceof Error ? error.message : String(error));
    }
    if (
      !equal(catalog.sources, input.catalog.sources) ||
      !equal(catalog.clips, input.catalog.clips)
    )
      throw new WorkbenchError("Source and clip metadata are immutable in this editor");
    if (!equal(catalog.music, input.catalog.music))
      throw new WorkbenchError("Music is immutable in this editor");
    for (const name of this.baselines)
      if (!equal(catalog.sounds[name], input.catalog.sounds[name]))
        throw new WorkbenchError(`Baseline ${name} is immutable; clone it to a new recipe`);
    return catalog;
  }

  preview(value: Draft): Promise<Review> {
    return this.publication.capture(async () => {
      const draft = object(value, ["revision", "catalog"]);
      const input = await this.inputs();
      if (draft.revision !== input.revision)
        throw new WorkbenchError("Sources changed; reload before reviewing this draft", 409);
      const catalog = structuredClone(this.validate(draft.catalog, input));
      const after = equal(catalog, input.catalog)
        ? input.texts[0]
        : `${JSON.stringify(catalog, null, 2)}\n`;
      const review: Review = {
        candidateId: randomUUID(),
        revision: input.revision,
        files:
          after === input.texts[0]
            ? []
            : [{ path: "fixtures/sounds.json", before: input.texts[0], after }],
      };
      if (this.candidates.size >= 8) this.candidates.delete(this.candidates.keys().next().value!);
      this.candidates.set(review.candidateId, { review, inputs: input, catalog, after });
      return review;
    });
  }

  async save(value: Save): Promise<Snapshot> {
    const request = object(value, ["candidateId", "revision"]);
    const candidate = this.candidates.get(request.candidateId as string);
    if (!candidate || request.revision !== candidate.review.revision)
      throw new WorkbenchError("Review expired; preview the draft again", 409);
    const verify = async (phase: "before" | "after") => {
      const current = await this.inputs();
      const expected = [...candidate.inputs.texts];
      if (phase === "after") expected[0] = candidate.after;
      if (!equal(current.texts, expected) || !equal(current.media, candidate.inputs.media))
        throw new WorkbenchError("Sources changed after review; outside edits were preserved", 409);
      this.validate(candidate.catalog, { ...current, catalog: candidate.inputs.catalog });
    };
    // A repeated reviewed save succeeds only when every captured input is still its exact published generation.
    const already = await this.publication.capture(async () => {
      const current = await this.inputs();
      return (
        candidate.after !== candidate.inputs.texts[0] &&
        current.texts[0] === candidate.after &&
        equal(current.texts.slice(1), candidate.inputs.texts.slice(1)) &&
        equal(current.media, candidate.inputs.media)
      );
    });
    if (!already) await this.publication.publish(candidate.review.files, verify);
    return this.snapshot();
  }
}

export function soundWorkbenchMiddleware(store: SoundWorkbenchStore) {
  return jsonRoutes(
    "/__sound-workbench/",
    { snapshot: () => store.snapshot() },
    {
      preview: (body) => store.preview(body as Draft),
      save: (body) => store.save(body as Save),
    },
  );
}

/** Repository documents the workbench saves; a save never hot-replaces a running battle's. */
const SAVED_DOCUMENTS = [
  "fixtures/sounds.json",
  "fixtures/voices.json",
  "fixtures/voices.lock.json",
];

export function soundWorkbenchPlugin(root: string): Plugin {
  return {
    name: "sound-workbench",
    apply: "serve",
    configureServer(server) {
      type Middleware = ReturnType<typeof jsonRoutes>;
      let sounds: Promise<Middleware> | undefined;
      let voices: Promise<Middleware> | undefined;
      // Voices and music share one tool queue, so the tool's files keep one writer.
      const queue = new GenerationQueue(generateTool(root));
      const music = musicWorkbenchMiddleware(queue);
      // Config loading precedes Vite aliases; shared runtime validation loads through its own module graph.
      const load = (path: string) => server.ssrLoadModule(resolve(root, path));
      server.middlewares.use(async (request, response, next) => {
        if (!request.url?.startsWith("/__sound-workbench/")) return next();
        if (request.url.startsWith("/__sound-workbench/music/"))
          return music(request, response, next);
        const voice = request.url.startsWith("/__sound-workbench/voices/");
        const middleware = voice
          ? (voices ??= rosterRows(root).then((roster) =>
              voiceWorkbenchMiddleware(
                new VoiceWorkbenchStore(
                  root,
                  (value) => validateVoiceCatalog(value, roster),
                  queue,
                ),
              ),
            ))
          : (sounds ??= Promise.all([
              load("packages/battle-audio/src/catalog.ts"),
              load("packages/battle-audio/src/synthSounds.gen.ts"),
              load("web/src/wasm/renderer_browser/renderer_browser.js"),
            ]).then(([catalog, synth, admission]) =>
              soundWorkbenchMiddleware(
                new SoundWorkbenchStore(
                  root,
                  catalog.validateSoundCatalog,
                  Object.keys(synth.SOUNDS),
                  (presentation, audio, library, weapons) => {
                    admission.check_presentation(presentation);
                    admission.check_battle_sounds(
                      presentation,
                      audio,
                      JSON.stringify(library),
                      weapons,
                    );
                  },
                ),
              ),
            ));
        try {
          await (
            await middleware
          )(request, response, next);
        } catch (error) {
          if (voice) voices = undefined;
          else sounds = undefined;
          response.statusCode = 500;
          response.end(
            JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
          );
        }
      });
    },
    handleHotUpdate(context) {
      if (SAVED_DOCUMENTS.some((path) => resolve(context.file) === resolve(root, path))) {
        for (const module of context.modules) context.server.moduleGraph.invalidateModule(module);
        return [];
      }
    },
  };
}

/** Every roster row, so a voice edit is refused when a playable unit loses its set. */
async function rosterRows(root: string): Promise<Record<string, unknown>> {
  const folder = join(root, "fixtures/units/roster");
  const rows = {};
  for (const name of (await readdir(folder)).filter((file) => file.endsWith(".json")).sort())
    Object.assign(rows, JSON.parse(await readFile(join(folder, name), "utf8")).units);
  return rows;
}
