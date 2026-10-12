import { spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { FixturePublication } from "../fixture-publication/publication";
import {
  ALL_TRIGGERS,
  takeId,
  voiceTriggers,
  type VoiceCatalog,
} from "../../packages/battle-audio/src/voices/voiceCatalog";
import {
  LOCK,
  readLock,
  verdictOf,
  type TakeRecord,
  type VoiceLock,
} from "../../packages/battle-audio/tools/generate/lock";
import { WorkbenchError, equal, hash, jsonRoutes, object } from "./http";
import type { Review, Save } from "./src/protocol";
import type {
  SetCounts,
  TakeView,
  VerdictResult,
  VoiceDraft,
  VoiceJob,
  VoiceScope,
  VoiceSnapshot,
} from "./src/voices/protocol";

/** Runs the generation tool with `args`, reporting each output line; resolves to its exit code. */
export type GenerateRunner = (args: string[], line: (text: string) => void) => Promise<number>;

/** The generation tool as a child process: the only writer of the lock and of voice audio. */
export const generateTool =
  (root: string): GenerateRunner =>
  (args, line) =>
    new Promise((resolve) => {
      const child = spawn("bun", ["run", "generate", ...args], { cwd: root });
      for (const stream of [child.stdout, child.stderr])
        createInterface({ input: stream }).on("line", line);
      child.on("error", (error) => {
        line(error.message);
        resolve(1);
      });
      child.on("close", (code) => resolve(code ?? 1));
    });

const VOICES = "fixtures/voices.json";
const TAKE = /^[a-z0-9_]+\/[a-z0-9_]+\/[a-z_]+\/(steady|stressed)\/[01]$/;
const SHA = /^[0-9a-f]{64}$/;
const KEPT_JOBS = 10;
const KEPT_LINES = 400;

interface Inputs {
  text: string;
  catalog: VoiceCatalog;
  lock: VoiceLock;
}
interface Candidate {
  review: Review;
  before: string;
  after: string;
  catalog: VoiceCatalog;
}

/** Every take id a set's voices speak. */
function setTakes(catalog: VoiceCatalog, set: string): string[] {
  return catalog.sets[set].voices.flatMap((voice) =>
    voiceTriggers(voice).flatMap((trigger) =>
      (["steady", "stressed"] as const).flatMap((intensity) =>
        [0, 1].map((take) => takeId({ set, voice: voice.id, trigger, intensity, take })),
      ),
    ),
  );
}
const view = (record: TakeRecord): TakeView => ({
  text: record.text,
  sha256: record.served_sha256,
  duration_s: record.duration_s,
  transcript: record.transcript,
  match: record.match,
  verdict: verdictOf(record),
});
function counts(catalog: VoiceCatalog, lock: VoiceLock, set: string): SetCounts {
  const result = { expected: 0, takes: 0, unreviewed: 0, flagged: 0, ok: 0, bad: 0 };
  for (const id of setTakes(catalog, set)) {
    result.expected++;
    const record = lock.takes[id];
    if (!record) continue;
    result.takes++;
    const verdict = verdictOf(record);
    if (verdict) result[verdict]++;
    else {
      result.unreviewed++;
      if (record.match.flagged) result.flagged++;
    }
  }
  return result;
}
const strings = (value: unknown, allowed: (item: string) => boolean, what: string) => {
  if (
    value !== undefined &&
    (!Array.isArray(value) ||
      !value.length ||
      value.some((item) => typeof item !== "string" || !allowed(item)))
  )
    throw new WorkbenchError(`Unknown ${what} in generation scope`);
  return value as string[] | undefined;
};

/**
 * The Voices tab's store. People edit `voices.json` through fixture publication;
 * the lock is read here and written only by the generation tool, which this store
 * runs one command at a time so two writers never race.
 */
export class VoiceWorkbenchStore {
  private readonly publication: FixturePublication;
  private readonly candidates = new Map<string, Candidate>();
  private readonly jobs: VoiceJob[] = [];
  private tool: Promise<unknown> = Promise.resolve();
  private nextJob = 1;
  constructor(
    private readonly root: string,
    private readonly validateCatalog: (value: unknown) => VoiceCatalog,
    private readonly run: GenerateRunner,
  ) {
    this.publication = new FixturePublication(
      root,
      "voices",
      (message, status) => new WorkbenchError(message, status),
    );
  }

  /** The JSON documents only: audio is proved by `generate check`, never hashed per snapshot. */
  private async inputs(): Promise<Inputs> {
    const text = await readFile(join(this.root, VOICES), "utf8");
    return {
      text,
      catalog: this.validate(JSON.parse(text)),
      lock: readLock(join(this.root, LOCK)),
    };
  }

  private validate(value: unknown): VoiceCatalog {
    try {
      return this.validateCatalog(value);
    } catch (error) {
      throw new WorkbenchError(error instanceof Error ? error.message : String(error));
    }
  }

  private snapshotOf({ text, catalog, lock }: Inputs): VoiceSnapshot {
    return {
      revision: hash(text),
      catalog,
      takes: Object.fromEntries(Object.entries(lock.takes).map(([id, take]) => [id, view(take)])),
      counts: Object.fromEntries(
        Object.keys(catalog.sets).map((set) => [set, counts(catalog, lock, set)]),
      ),
    };
  }

  snapshot(): Promise<VoiceSnapshot> {
    return this.publication.capture(async () => this.snapshotOf(await this.inputs()));
  }

  /** This editor changes what is said and how a voice is described, and asks for retakes. */
  private check(candidate: unknown, current: VoiceCatalog): VoiceCatalog {
    object(candidate, ["roles", "sets", "assignments", "retakes"]);
    const catalog = this.validate(candidate);
    const speakers = (sets: VoiceCatalog["sets"]) =>
      Object.fromEntries(
        Object.entries(sets).map(([id, set]) => [
          id,
          { ...set, voices: set.voices.map(({ description: _, ...voice }) => voice) },
        ]),
      );
    if (
      !equal(Object.keys(catalog.roles).sort(), Object.keys(current.roles).sort()) ||
      !equal(speakers(catalog.sets), speakers(current.sets)) ||
      !equal(catalog.assignments, current.assignments)
    )
      throw new WorkbenchError("Only line text, voice descriptions and retakes are editable here");
    const known = new Set(Object.keys(catalog.sets).flatMap((set) => setTakes(catalog, set)));
    for (const [id, count] of Object.entries(catalog.retakes))
      if (!known.has(id) || !Number.isSafeInteger(count) || count < 0)
        throw new WorkbenchError(`Retake ${id} must name a take and count from 0`);
    return catalog;
  }

  preview(value: VoiceDraft): Promise<Review> {
    return this.publication.capture(async () => {
      const draft = object(value, ["revision", "catalog"]);
      const input = await this.inputs();
      const revision = hash(input.text);
      if (draft.revision !== revision)
        throw new WorkbenchError("voices.json changed; reload before reviewing this draft", 409);
      const catalog = structuredClone(this.check(draft.catalog, input.catalog));
      const after = equal(catalog, input.catalog)
        ? input.text
        : `${JSON.stringify(catalog, null, 2)}\n`;
      const review: Review = {
        candidateId: randomUUID(),
        revision,
        files: after === input.text ? [] : [{ path: VOICES, before: input.text, after }],
      };
      if (this.candidates.size >= 8) this.candidates.delete(this.candidates.keys().next().value!);
      this.candidates.set(review.candidateId, { review, before: input.text, after, catalog });
      return review;
    });
  }

  async save(value: Save): Promise<VoiceSnapshot> {
    const request = object(value, ["candidateId", "revision"]);
    const candidate = this.candidates.get(request.candidateId as string);
    if (!candidate || request.revision !== candidate.review.revision)
      throw new WorkbenchError("Review expired; preview the draft again", 409);
    const verify = async (phase: "before" | "after") => {
      const current = await this.inputs();
      if (current.text !== (phase === "before" ? candidate.before : candidate.after))
        throw new WorkbenchError("voices.json changed after review; outside edits were kept", 409);
    };
    // A repeated save of an already published candidate keeps its bytes.
    const already = await this.publication.capture(
      async () =>
        candidate.after !== candidate.before && (await this.inputs()).text === candidate.after,
    );
    if (!already) await this.publication.publish(candidate.review.files, verify);
    return this.snapshot();
  }

  /** One tool command at a time: a verdict waits for a running generation, never races it. */
  private serial<T>(run: () => Promise<T>): Promise<T> {
    const job = this.tool.then(run);
    this.tool = job.catch(() => {});
    return job;
  }

  async verdict(value: unknown): Promise<VerdictResult> {
    const request = object(value, ["verdict", "takes"]);
    if (request.verdict !== "ok" && request.verdict !== "bad")
      throw new WorkbenchError("A verdict is ok or bad");
    const judged = request.takes;
    if (
      !Array.isArray(judged) ||
      !judged.length ||
      judged.some((item) => {
        const { take, sha256 } = object(item, ["take", "sha256"]);
        return (
          typeof take !== "string" ||
          !TAKE.test(take) ||
          typeof sha256 !== "string" ||
          !SHA.test(sha256)
        );
      })
    )
      throw new WorkbenchError("Name each judged take with the sha256 it was heard as");
    const lines: string[] = [];
    const code = await this.serial(() =>
      this.run(
        [
          "verdict",
          request.verdict as string,
          ...judged.map(({ take, sha256 }) => `${take}@${sha256}`),
        ],
        (line) => lines.push(line),
      ),
    );
    const changed = lines
      .filter((line) => /^(CHANGED|UNKNOWN) /.test(line))
      .map((line) => line.split(" ")[1].split("@")[0]);
    if (code !== 0 && !changed.length)
      throw new WorkbenchError(`generate verdict failed: ${lines.join("\n")}`, 500);
    const { catalog, lock } = await this.inputs();
    const ids: string[] = judged.map(({ take }) => take);
    const sets = new Set(ids.map((id) => id.split("/")[0]).filter((set) => catalog.sets[set]));
    return {
      takes: Object.fromEntries(
        ids.filter((id) => lock.takes[id]).map((id) => [id, view(lock.takes[id])]),
      ),
      counts: Object.fromEntries([...sets].map((set) => [set, counts(catalog, lock, set)])),
      changed,
    };
  }

  /** Starts `generate voices` over a scope; its progress is read through `jobs`. */
  async generate(value: unknown): Promise<{ job: VoiceJob }> {
    const { scope: raw } = object(value, ["scope"]);
    const scope = object(raw, ["sets", "voices", "triggers", "intensities", "take", "only"]);
    const { catalog } = await this.inputs();
    const sets = strings(scope.sets, (set) => Object.hasOwn(catalog.sets, set), "set");
    if (!sets) throw new WorkbenchError("A generation scope names its sets");
    const voices = strings(
      scope.voices,
      (voice) => sets.some((set) => catalog.sets[set].voices.some(({ id }) => id === voice)),
      "voice",
    );
    const triggers = strings(
      scope.triggers,
      (trigger) => (ALL_TRIGGERS as readonly string[]).includes(trigger),
      "trigger",
    );
    const intensities = strings(
      scope.intensities,
      (intensity) => intensity === "steady" || intensity === "stressed",
      "intensity",
    );
    if (scope.take !== undefined && scope.take !== 0 && scope.take !== 1)
      throw new WorkbenchError("A take is 0 or 1");
    if (scope.only !== undefined && !["missing", "bad", "flagged"].includes(scope.only as string))
      throw new WorkbenchError("Unknown take filter in generation scope");
    const selection = scope as Partial<VoiceScope>;
    const args = [
      "voices",
      "--set",
      sets.join(","),
      ...(voices ? ["--voice", voices.join(",")] : []),
      ...(triggers ? ["--trigger", triggers.join(",")] : []),
      ...(intensities ? ["--intensity", intensities.join(",")] : []),
      ...(selection.take === undefined ? [] : ["--take", String(selection.take)]),
      ...(selection.only ? ["--only", selection.only] : []),
    ];
    const job: VoiceJob = { id: String(this.nextJob++), args, status: "queued", lines: [] };
    this.jobs.push(job);
    if (this.jobs.length > KEPT_JOBS) this.jobs.splice(0, this.jobs.length - KEPT_JOBS);
    const line = (text: string) => {
      job.lines.push(text);
      if (job.lines.length > KEPT_LINES) job.lines.splice(0, job.lines.length - KEPT_LINES);
    };
    void this.serial(async () => {
      job.status = "running";
      try {
        job.status = (await this.run(args, line)) === 0 ? "done" : "failed";
      } catch (error) {
        line(error instanceof Error ? error.message : String(error));
        job.status = "failed";
      }
    });
    return { job };
  }

  listJobs(): Promise<{ jobs: VoiceJob[] }> {
    return Promise.resolve({ jobs: this.jobs });
  }
}

export function voiceWorkbenchMiddleware(store: VoiceWorkbenchStore) {
  return jsonRoutes(
    "/__sound-workbench/voices/",
    { snapshot: () => store.snapshot(), jobs: () => store.listJobs() },
    {
      preview: (body) => store.preview(body as VoiceDraft),
      save: (body) => store.save(body as Save),
      verdict: (body) => store.verdict(body),
      generate: (body) => store.generate(body),
    },
  );
}
