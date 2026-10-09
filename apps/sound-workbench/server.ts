import { createHash, randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { IncomingMessage, ServerResponse } from "node:http";
import type { Plugin } from "vite";
import { FixturePublication } from "../fixture-publication/publication";
import type { SoundCatalog } from "../../packages/battle-audio/src/catalog";
import { GLANCE } from "../../packages/battle-audio/src/contacts";
import type { AudioPresentation } from "../../packages/battle-audio/src/audioPresentation";
import type { Draft, Review, Save, Snapshot } from "./src/protocol";

class WorkbenchError extends Error {
  constructor(
    message: string,
    public status = 400,
  ) {
    super(message);
  }
}
function object(value: unknown, fields: string[]): Record<string, unknown> {
  if (
    !value ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    Object.keys(value).some((key) => !fields.includes(key))
  )
    throw new WorkbenchError("Unexpected request fields");
  return value as Record<string, unknown>;
}
const hash = (bytes: string | Buffer) => createHash("sha256").update(bytes).digest("hex");
const equal = (a: unknown, b: unknown): boolean => {
  if (a === b) return true;
  if (
    !a ||
    !b ||
    typeof a !== "object" ||
    typeof b !== "object" ||
    Array.isArray(a) !== Array.isArray(b)
  )
    return false;
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length &&
    keys.every(
      (key) =>
        Object.hasOwn(b, key) &&
        equal((a as Record<string, unknown>)[key], (b as Record<string, unknown>)[key]),
    )
  );
};
interface Inputs {
  texts: string[];
  catalog: SoundCatalog;
  audio: AudioPresentation;
  runMps: number;
  weapons: Snapshot["weapons"];
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
  ) {
    this.publication = new FixturePublication(
      root,
      "sounds",
      (message, status) => new WorkbenchError(message, status),
    );
  }

  private async inputs(): Promise<Inputs> {
    const texts = await Promise.all(
      ["sounds", "game"].map((name) => readFile(join(this.root, `fixtures/${name}.json`), "utf8")),
    );
    const catalog = this.validateCatalog(JSON.parse(texts[0]));
    const game = JSON.parse(texts[1]);
    const audio = game.presentation.audio as AudioPresentation;
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
      runMps: game.presentation.pose.gait.run_mps,
      // Each row's lineage only: what it inherits a firing choice from.
      weapons: Object.fromEntries(
        Object.entries(game.weapons as Record<string, { extends?: string }>).map(([kind, row]) => [
          kind,
          row.extends ? { extends: row.extends } : {},
        ]),
      ),
      media,
      revision: hash(JSON.stringify([texts, media])),
    };
  }

  private snapshotOf(input: Inputs): Snapshot {
    return {
      revision: input.revision,
      catalog: input.catalog,
      weapons: input.weapons,
      firing: input.audio.shots,
      vehicles: input.audio.vehicles,
      footsteps: input.audio.footsteps,
      runMps: input.runMps,
      // A glance is a contact too: each round may choose its ricochet.
      materials: [...Object.keys(input.audio.impacts), GLANCE],
    };
  }

  snapshot(): Promise<Snapshot> {
    return this.publication.capture(async () => this.snapshotOf(await this.inputs()));
  }

  private validate(candidate: unknown, input: Inputs): SoundCatalog {
    object(candidate, ["sources", "clips", "sounds", "defaults", "impacts", "effects"]);
    let catalog;
    try {
      catalog = this.validateCatalog(candidate);
    } catch (error) {
      throw new WorkbenchError(error instanceof Error ? error.message : String(error));
    }
    if (
      !equal(catalog.sources, input.catalog.sources) ||
      !equal(catalog.clips, input.catalog.clips)
    )
      throw new WorkbenchError("Source and clip metadata are immutable in this editor");
    for (const name of this.baselines)
      if (!equal(catalog.sounds[name], input.catalog.sounds[name]))
        throw new WorkbenchError(`Baseline ${name} is immutable; clone it to a new recipe`);
    for (const kind of Object.keys(catalog.defaults))
      if (kind !== "default" && !Object.hasOwn(input.weapons, kind))
        throw new WorkbenchError(`Unknown weapon row ${kind}`);
    for (const [material, rounds] of Object.entries(catalog.impacts)) {
      if (!Object.hasOwn(input.audio.impacts, material))
        throw new WorkbenchError(`Unknown impact material ${material}`);
      for (const round of Object.keys(rounds))
        if (round !== "default" && !Object.hasOwn(input.weapons, round))
          throw new WorkbenchError(`Unknown impact round ${round}`);
    }
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
  return async (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    const path = request.url?.split("?")[0];
    if (!path?.startsWith("/__sound-workbench/")) return next();
    response.setHeader("Content-Type", "application/json");
    response.setHeader("Cache-Control", "no-store");
    try {
      const host = request.headers.host ?? "";
      if (
        !/^(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?$/.test(host) ||
        !["127.0.0.1", "::1", "::ffff:127.0.0.1"].includes(request.socket.remoteAddress ?? "")
      )
        throw new WorkbenchError("The sound workbench is local-only", 403);
      if (request.method === "GET" && path === "/__sound-workbench/snapshot") {
        response.end(JSON.stringify(await store.snapshot()));
        return;
      }
      if (request.method !== "POST")
        throw new WorkbenchError("Unknown sound workbench endpoint", 404);
      if (
        request.headers.origin !== `http://${host}` ||
        !/^application\/json(?:\s*;|$)/i.test(request.headers["content-type"] ?? "")
      )
        throw new WorkbenchError("Requests must come from this local workbench", 403);
      const chunks: Buffer[] = [];
      let bytes = 0;
      for await (const chunk of request) {
        const part = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
        bytes += part.length;
        if (bytes > 1024 * 1024)
          throw new WorkbenchError("Request body exceeds the workbench limit", 413);
        chunks.push(part);
      }
      let body;
      try {
        body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
      } catch {
        throw new WorkbenchError("Expected a JSON request object");
      }
      const result =
        path === "/__sound-workbench/preview"
          ? await store.preview(body)
          : path === "/__sound-workbench/save"
            ? await store.save(body)
            : (() => {
                throw new WorkbenchError("Unknown sound workbench endpoint", 404);
              })();
      response.end(JSON.stringify(result));
    } catch (error) {
      response.statusCode =
        error instanceof WorkbenchError ? error.status : error instanceof SyntaxError ? 400 : 500;
      response.end(
        JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
      );
    }
  };
}

export function soundWorkbenchPlugin(root: string): Plugin {
  return {
    name: "sound-workbench",
    apply: "serve",
    configureServer(server) {
      let middleware: Promise<ReturnType<typeof soundWorkbenchMiddleware>> | undefined;
      server.middlewares.use(async (request, response, next) => {
        if (!request.url?.startsWith("/__sound-workbench/")) return next();
        // Config loading precedes Vite aliases; shared runtime validation loads through its own module graph.
        middleware ??= Promise.all([
          server.ssrLoadModule(resolve(root, "packages/battle-audio/src/catalog.ts")),
          server.ssrLoadModule(resolve(root, "packages/battle-audio/src/synth.ts")),
        ]).then(([catalog, synth]) =>
          soundWorkbenchMiddleware(
            new SoundWorkbenchStore(root, catalog.validateSoundCatalog, Object.keys(synth.SOUNDS)),
          ),
        );
        try {
          await (
            await middleware
          )(request, response, next);
        } catch (error) {
          middleware = undefined;
          response.statusCode = 500;
          response.end(
            JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
          );
        }
      });
    },
    handleHotUpdate(context) {
      if (resolve(context.file) === resolve(root, "fixtures/sounds.json")) {
        for (const module of context.modules) context.server.moduleGraph.invalidateModule(module);
        return [];
      }
    },
  };
}
