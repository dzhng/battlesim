import * as fs from "node:fs/promises";
import { createHash, randomUUID } from "node:crypto";
import { join, relative } from "node:path";
import { FixturePublication } from "../fixture-publication/publication";
import { changedPaths } from "../fixture-publication/changes";
import { nativeReporter, type NativeReporter } from "./native";
import { WorkbenchError } from "./error";
import { canonicalSeed, MAP_TYPES, MAP_SIZES } from "../../web/src/maps/source";
import type {
  Draft,
  Field,
  NativeInputs,
  NativeValidation,
  NativeReport,
  Inspection,
  SightReport,
  Snapshot,
  SaveReview,
  Json,
  JsonObject,
  Report,
  RunRequest,
} from "./src/protocol";

export { nativeReporter, type NativeReporter } from "./native";
export { WorkbenchError } from "./error";

const paths = {
  presets: "fixtures/map-presets.json",
  defaults: "fixtures/generated-battle.json",
  templates: "fixtures/prototype-building-templates.json",
  rules: "fixtures/game.json",
  catalog: "fixtures/catalog.json",
  recipes: "fixtures/encounters.json",
} as const;
const hash = (value: string) => createHash("sha256").update(value).digest("hex");
const pretty = (value: Json) => `${JSON.stringify(value, null, 2)}\n`;
function canonical(value: Json): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object")
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonical(value[key])}`)
      .join(",")}}`;
  return JSON.stringify(value);
}
function object(value: unknown): JsonObject {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new WorkbenchError("Expected a JSON object");
  return value as JsonObject;
}
interface State {
  revision: string;
  receipts: Record<string, string>;
  inputs: NativeInputs;
}
interface Candidate {
  review: SaveReview;
  inputs: NativeInputs;
  expectedRevision: string;
}
interface Artifact {
  dir: string | null;
  inputs: NativeInputs;
  report: Report;
  receipts: Record<string, string>;
  purpose: RunRequest["purpose"];
}

export class WorkbenchStore {
  private publication: FixturePublication;
  private queue: Promise<unknown> = Promise.resolve();
  private candidate: Candidate | null = null;
  private artifacts = new Map<string, Artifact>();
  private controllers = new Set<AbortController>();
  private closed = false;
  constructor(
    private root: string,
    private report: NativeReporter = nativeReporter(root),
  ) {
    this.publication = new FixturePublication(
      root,
      "maps",
      (message, status) => new WorkbenchError(message, status),
    );
  }
  private serial<T>(run: (signal: AbortSignal) => Promise<T>, signal?: AbortSignal): Promise<T> {
    if (this.closed) return Promise.reject(new WorkbenchError("Workbench is closed", 503));
    if (this.controllers.size >= 2)
      return Promise.reject(
        new WorkbenchError("Workbench is busy; retry when the current request finishes", 429),
      );
    const controller = new AbortController();
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort();
    else signal?.addEventListener("abort", abort, { once: true });
    this.controllers.add(controller);
    const job = this.queue
      .then(() => {
        controller.signal.throwIfAborted();
        return run(controller.signal);
      })
      .finally(() => {
        this.controllers.delete(controller);
        signal?.removeEventListener("abort", abort);
      });
    this.queue = job.catch(() => {});
    return job;
  }
  private async read(): Promise<State> {
    const entries = await Promise.all(
      Object.entries(paths).map(
        async ([name, path]) => [name, await fs.readFile(join(this.root, path), "utf8")] as const,
      ),
    );
    const inputs = Object.fromEntries(entries) as unknown as NativeInputs;
    const receipts = Object.fromEntries(entries.map(([name, text]) => [name, hash(text)]));
    return { inputs, receipts, revision: hash(JSON.stringify(entries)) };
  }
  private async state() {
    return this.publication.capture(() => this.read());
  }
  private async validate(inputs: NativeInputs, signal?: AbortSignal) {
    const result = (await this.report(
      { operation: "validate", inputs },
      signal,
    )) as NativeValidation;
    return result;
  }
  private requireValid(result: NativeValidation) {
    if (result.status !== "valid")
      throw new WorkbenchError(
        result.diagnostics.map((item) => item.message).join("; "),
        400,
        result.diagnostics,
      );
    return result;
  }
  private async snapshotFrom(state: State, signal?: AbortSignal): Promise<Snapshot> {
    const validated = this.requireValid(await this.validate(state.inputs, signal));
    return {
      revision: state.revision,
      documents: {
        presets: object(JSON.parse(state.inputs.presets)),
        defaults: object(JSON.parse(state.inputs.defaults)),
      },
      receipts: state.receipts,
      fields: validated.fields,
    };
  }
  snapshot(signal?: AbortSignal) {
    return this.serial(async (owned) => this.snapshotFrom(await this.state(), owned), signal);
  }
  private inputsFor(draft: Draft, state: State, fields: Field[]) {
    if (!draft || draft.revision !== state.revision)
      throw new WorkbenchError("Sources changed. Reload sources before saving or generating.", 409);
    const documents = object(draft.documents);
    const presets = structuredClone(object(documents.presets));
    const defaults = structuredClone(object(documents.defaults));
    const oldPresets = object(JSON.parse(state.inputs.presets));
    if (Object.keys(documents).some((key) => key !== "presets" && key !== "defaults"))
      throw new WorkbenchError("Unknown draft document");
    if (presets.revision !== oldPresets.revision)
      throw new WorkbenchError("Preset revision is managed by the workbench");
    for (const [document, before, after] of [
      ["presets", oldPresets, presets],
      ["defaults", object(JSON.parse(state.inputs.defaults)), defaults],
    ] as const) {
      for (const path of changedPaths(before, after)) {
        if (
          !fields.some(
            (field) =>
              field.document === document &&
              field.editable &&
              field.path.length <= path.length &&
              field.path.every((key, index) => key === path[index]),
          )
        )
          throw new WorkbenchError(`Field ${document}.${path.join(".")} is not editable`);
      }
    }
    const oldBody = { ...oldPresets };
    delete oldBody.revision;
    const newBody = { ...presets };
    delete newBody.revision;
    if (canonical(oldBody) !== canonical(newBody)) presets.revision = hash(canonical(newBody));
    return {
      ...state.inputs,
      presets:
        canonical(oldPresets) === canonical(presets) ? state.inputs.presets : pretty(presets),
      defaults:
        canonical(object(JSON.parse(state.inputs.defaults))) === canonical(defaults)
          ? state.inputs.defaults
          : pretty(defaults),
    };
  }
  preview(draft: Draft, signal?: AbortSignal): Promise<SaveReview> {
    return this.serial(async (owned) => {
      const state = await this.state();
      const current = this.requireValid(await this.validate(state.inputs, owned));
      const inputs = this.inputsFor(draft, state, current.fields);
      const validation = await this.validate(inputs, owned);
      const files = (["presets", "defaults"] as const).flatMap((name) =>
        state.inputs[name] === inputs[name]
          ? []
          : [{ path: paths[name], before: state.inputs[name], after: inputs[name] }],
      );
      const candidateId = hash(JSON.stringify([state.revision, inputs]));
      const review = {
        candidateId,
        revision: state.revision,
        files,
        diagnostics: validation.diagnostics,
      };
      const expectedRevision = hash(
        JSON.stringify(
          Object.keys(paths).map((name) => [name, inputs[name as keyof NativeInputs]]),
        ),
      );
      this.candidate = { review, inputs, expectedRevision };
      return structuredClone(review);
    }, signal);
  }
  save(review: SaveReview, signal?: AbortSignal): Promise<Snapshot> {
    return this.serial(async (owned) => {
      const candidate = this.candidate;
      if (
        !review ||
        !candidate ||
        candidate.review.candidateId !== review.candidateId ||
        candidate.review.revision !== review.revision
      )
        throw new WorkbenchError("Preview these settings before saving", 409);
      const state = await this.state();
      if (state.revision === candidate.expectedRevision) return this.snapshotFrom(state, owned);
      if (state.revision !== candidate.review.revision)
        throw new WorkbenchError("Sources changed. Reload sources before saving.", 409);
      this.requireValid(await this.validate(candidate.inputs, owned));
      owned.throwIfAborted();
      await this.publication.publish(candidate.review.files, async (phase) => {
        const current = await this.read();
        const expected =
          phase === "before" ? candidate.review.revision : candidate.expectedRevision;
        if (current.revision !== expected)
          throw new WorkbenchError(
            "Sources changed during publication. Outside edits were preserved.",
            409,
          );
      });
      return this.snapshotFrom(await this.state(), owned);
    }, signal);
  }
  generate(request: RunRequest, signal?: AbortSignal): Promise<Report> {
    return this.serial(async (owned) => {
      const choice = request?.choice;
      if (
        !choice ||
        (request.purpose !== "preview" && request.purpose !== "sample") ||
        !MAP_TYPES.includes(choice.type) ||
        !MAP_SIZES.includes(choice.size) ||
        typeof choice.seed !== "string" ||
        canonicalSeed(choice.seed) !== choice.seed
      )
        throw new WorkbenchError("Expected a map type, size and canonical u64 seed");
      const retained = this.retainedPreviews(request.retainedArtifactIds);
      const state = await this.state();
      const current = this.requireValid(await this.validate(state.inputs, owned));
      const inputs = this.inputsFor(request.draft, state, current.fields);
      // Native generation reports invalid draft policy with its exact tested inputs.
      if (request.purpose === "preview") {
        for (const [id, artifact] of this.artifacts) {
          if (
            artifact.purpose === "preview" &&
            artifact.report.status === "ok" &&
            !retained.has(id)
          ) {
            await this.release(artifact);
            this.artifacts.delete(id);
          }
        }
      }
      if (request.purpose === "sample") {
        for (const [id, artifact] of this.artifacts) {
          if (artifact.purpose === "sample") {
            await this.release(artifact);
            this.artifacts.delete(id);
          }
        }
      }
      const artifactId = randomUUID();
      const dir = join(this.root, "throwaway", "map-workbench", artifactId);
      await fs.mkdir(dir, { recursive: true });
      try {
        const native = (await this.report(
          { operation: "generate", inputs, choice, artifactDir: dir },
          owned,
        )) as NativeReport;
        owned.throwIfAborted();
        const result: Report = {
          ...native,
          choice,
          fingerprint: hash(JSON.stringify({ inputs, choice })),
          artifactId,
        };
        for (const [id, old] of this.artifacts) {
          if (
            request.purpose === "preview" &&
            result.status === "refused" &&
            old.purpose === "preview" &&
            old.report.status === "refused"
          ) {
            await this.release(old);
            this.artifacts.delete(id);
          }
        }
        if (result.status === "refused") await fs.rm(dir, { recursive: true, force: true });
        const receipts = Object.fromEntries(
          Object.entries(inputs).map(([name, text]) => [name, hash(text)]),
        );
        this.artifacts.set(artifactId, {
          dir: result.status === "ok" ? dir : null,
          inputs,
          report: result,
          receipts,
          purpose: request.purpose,
        });
        return structuredClone(result);
      } catch (error) {
        await fs.rm(dir, { recursive: true, force: true });
        this.artifacts.delete(artifactId);
        throw error;
      }
    }, signal);
  }
  private artifact(id: string): Artifact {
    const result = this.artifacts.get(id);
    if (!result) throw new WorkbenchError("This result expired. Generate it again.", 410);
    return result;
  }
  private retainedPreviews(ids: string[]): Set<string> {
    if (!Array.isArray(ids) || ids.length > 2 || new Set(ids).size !== ids.length)
      throw new WorkbenchError("Retain at most two distinct displayed preview results");
    for (const id of ids) {
      const artifact = this.artifacts.get(id);
      if (
        typeof id !== "string" ||
        !artifact ||
        artifact.purpose !== "preview" ||
        artifact.report.status !== "ok" ||
        artifact.dir === null
      )
        throw new WorkbenchError("Retained IDs must name admitted preview results");
    }
    return new Set(ids);
  }
  private async release(artifact: Artifact) {
    if (artifact.dir !== null) {
      await fs.rm(artifact.dir, { recursive: true, force: true });
      artifact.dir = null;
    }
  }
  private admittedDir(artifact: Artifact): string {
    if (artifact.dir === null)
      throw new WorkbenchError("This sample was exported. Generate a preview to inspect it.", 410);
    return artifact.dir;
  }
  inspect(id: string, crop?: string, signal?: AbortSignal): Promise<Inspection> {
    return this.serial(async (owned) => {
      const artifact = this.artifact(id);
      if (artifact.report.status !== "ok")
        throw new WorkbenchError("A refused map has no admitted plan to inspect");
      if (crop !== undefined && (typeof crop !== "string" || crop.length > 1024))
        throw new WorkbenchError("Invalid inspection crop");
      return this.report(
        {
          operation: "inspect",
          artifactDir: this.admittedDir(artifact),
          ...(crop === undefined ? {} : { crop }),
        },
        owned,
      ) as Promise<Inspection>;
    }, signal);
  }
  sight(id: string, signal?: AbortSignal): Promise<SightReport> {
    return this.serial(async (owned) => {
      const artifact = this.artifact(id);
      if (artifact.report.status !== "ok")
        throw new WorkbenchError("A refused map has no admitted sight report");
      return this.report(
        { operation: "sight", artifactDir: this.admittedDir(artifact) },
        owned,
      ) as Promise<SightReport>;
    }, signal);
  }
  export(id: string, signal?: AbortSignal): Promise<JsonObject> {
    return this.serial(async () => {
      const artifact = this.artifact(id);
      if (artifact.purpose === "sample") await this.release(artifact);
      return structuredClone({
        inputs: artifact.inputs,
        report: artifact.report,
        receipts: artifact.receipts,
      }) as unknown as JsonObject;
    }, signal);
  }
  async close() {
    this.closed = true;
    for (const controller of this.controllers) controller.abort();
    await this.queue;
    await Promise.all([...this.artifacts.values()].map((artifact) => this.release(artifact)));
    this.artifacts.clear();
    this.candidate = null;
  }
}

export function mapWorkbenchPlugin(
  root: string,
  reporter: NativeReporter = nativeReporter(root),
): import("vite").Plugin {
  let store: WorkbenchStore;
  return {
    name: "map-workbench",
    apply: "serve",
    async configureServer(server) {
      store = new WorkbenchStore(root, reporter);
      const { workbenchMiddleware } = await import("./httpServer");
      server.middlewares.use(workbenchMiddleware(store));
      server.httpServer?.once("close", () => {
        void store.close();
      });
    },
    async closeBundle() {
      await store?.close();
    },
    handleHotUpdate(context) {
      const path = relative(root, context.file).replaceAll("\\", "/");
      if (path === paths.presets || path === paths.defaults) {
        for (const module of context.modules) context.server.moduleGraph.invalidateModule(module);
        return [];
      }
    },
  };
}
