import type { IncomingMessage, ServerResponse } from "node:http";
import { randomUUID } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { FixturePublication } from "../fixture-publication/publication";
import {
  ALL_TRIGGERS,
  backgroundChoices,
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
import {
  designId,
  designRetake,
  previewFile,
} from "../../packages/battle-audio/tools/generate/design";
import type { GenerationQueue } from "./generation";
import { WorkbenchError, equal, hash, jsonRoutes, localOnly, object } from "./http";
import type { GenerationJob, Review, Save } from "./src/protocol";
import type {
  SetCounts,
  TakeView,
  VerdictResult,
  VoiceDraft,
  VoiceScope,
  VoiceSnapshot,
} from "./src/voices/protocol";

const VOICES = "fixtures/voices.json";
const TAKE = /^[a-z0-9_]+\/[a-z0-9_]+\/[a-z_]+\/(steady|stressed)\/[01]$/;
const SHA = /^[0-9a-f]{64}$/;

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
 * runs through the workbench's one generation queue so two writers never race.
 */
export class VoiceWorkbenchStore {
  private readonly publication: FixturePublication;
  private readonly candidates = new Map<string, Candidate>();
  /** Voices whose candidate previews the tool brought up to date this session. */
  private readonly rendered = new Set<string>();
  constructor(
    private readonly root: string,
    private readonly validateCatalog: (value: unknown) => VoiceCatalog,
    private readonly queue: GenerationQueue,
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
      designs: Object.fromEntries(
        Object.entries(lock.designs).map(([id, design]) => [
          id,
          {
            text: design.text,
            candidates: design.candidates.map(({ transcript, match, duration_s }) => ({
              transcript,
              score: match?.score ?? null,
              duration_s,
            })),
            picked: design.picked?.candidate ?? null,
          },
        ]),
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
    const known = new Set(
      Object.entries(catalog.sets).flatMap(([set, spec]) => [
        ...setTakes(catalog, set),
        ...spec.voices.map((voice) => designRetake(designId(set, voice.id))),
      ]),
    );
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

  /**
   * Re-mixes a set onto another background: its `processing` becomes another of its
   * placement's profiles, published like any edit to voices.json, then the tool
   * re-renders the set's takes from their raw masters (no calls) and re-packs it.
   * Re-mixed takes are new bytes, so they wait for review before they are packed.
   */
  async background(value: unknown): Promise<VoiceSnapshot> {
    const request = object(value, ["revision", "set", "processing"]);
    const input = await this.publication.capture(() => this.inputs());
    if (request.revision !== hash(input.text))
      throw new WorkbenchError("voices.json changed; reload before choosing a background", 409);
    const name = request.set as string;
    if (typeof name !== "string" || !Object.hasOwn(input.catalog.sets, name))
      throw new WorkbenchError("Name a set the catalog has");
    const current = input.catalog.sets[name].processing;
    const choices: string[] = backgroundChoices(current);
    if (!choices.includes(request.processing as string))
      throw new WorkbenchError(
        `${String(request.processing)} is not a background for ${name}: choose ${choices.join(" or ")}`,
      );
    if (request.processing === current) return this.snapshot();
    const catalog = structuredClone(input.catalog);
    catalog.sets[name].processing = request.processing as typeof current;
    const after = `${JSON.stringify(this.validate(catalog), null, 2)}\n`;
    await this.publication.publish([{ path: VOICES, before: input.text, after }], async (phase) => {
      if ((await this.inputs()).text !== (phase === "before" ? input.text : after))
        throw new WorkbenchError("voices.json changed while saving; outside edits were kept", 409);
    });
    this.queue.start(["process", "--set", name]);
    this.queue.start(["pack", "--set", name]);
    return this.snapshot();
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
    const code = await this.queue.serial(() =>
      this.queue.run(
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
  async generate(value: unknown): Promise<{ job: GenerationJob }> {
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
    return { job: this.queue.start(args) };
  }

  /** A `<set>/<voice>` the catalog has. */
  private async voiceOf(value: unknown): Promise<string> {
    const { catalog } = await this.inputs();
    const [set, voice, ...rest] = typeof value === "string" ? value.split("/") : [];
    if (rest.length || !catalog.sets[set]?.voices.some(({ id }) => id === voice))
      throw new WorkbenchError("Name a voice as <set>/<voice>");
    return designId(set, voice);
  }

  /** `generate design` for one voice; "None of these" bumps its design retake first. */
  async design(value: unknown): Promise<{ job: GenerationJob }> {
    const { voice } = object(value, ["voice"]);
    return { job: this.queue.start(["design", await this.voiceOf(voice)]) };
  }

  /** `generate pick`: saves a candidate as the voice; the tool writes voices.json.
   *  `replace` swaps a voice already picked for another of its candidates; the
   *  old voice's takes are then superseded, so its takes are made again and the
   *  set's pack is rebuilt. The new takes wait for review before they are packed. */
  async pick(value: unknown): Promise<VoiceSnapshot> {
    const request = object(value, ["voice", "candidate", "replace"]);
    const voice = await this.voiceOf(request.voice);
    if (![1, 2, 3].includes(request.candidate as number))
      throw new WorkbenchError("A candidate is 1, 2 or 3");
    if (request.replace !== undefined && typeof request.replace !== "boolean")
      throw new WorkbenchError("replace is true or false");
    const args = [
      "pick",
      voice,
      String(request.candidate),
      ...(request.replace ? ["--replace"] : []),
    ];
    const lines: string[] = [];
    const code = await this.queue.serial(() => this.queue.run(args, (line) => lines.push(line)));
    if (code !== 0) throw new WorkbenchError(`generate pick failed: ${lines.join("\n")}`, 500);
    if (request.replace) {
      const [set, id] = voice.split("/");
      this.queue.start(["voices", "--set", set, "--voice", id]);
      this.queue.start(["pack", "--set", set]);
    }
    return this.snapshot();
  }

  /**
   * A candidate's preview through its set's profile. The tool renders it (locally,
   * no calls) the first time a voice is played in this session, so a tuned profile
   * is heard after a restart.
   */
  async candidateAudio(voice: string, candidate: number): Promise<Buffer> {
    const id = await this.voiceOf(voice);
    if (![1, 2, 3].includes(candidate)) throw new WorkbenchError("A candidate is 1, 2 or 3", 404);
    if (!this.rendered.has(id)) {
      const lines: string[] = [];
      const code = await this.queue.serial(() =>
        this.queue.run(["preview", id], (l) => lines.push(l)),
      );
      if (code !== 0) throw new WorkbenchError(`generate preview failed: ${lines.join("\n")}`, 500);
      this.rendered.add(id);
    }
    try {
      return await readFile(join(this.root, previewFile(id, candidate)));
    } catch {
      throw new WorkbenchError(`${id} has no candidate ${candidate}`, 404);
    }
  }

  listJobs(): Promise<{ jobs: GenerationJob[] }> {
    return Promise.resolve({ jobs: this.queue.jobs });
  }
}

const CANDIDATE_AUDIO =
  /^\/__sound-workbench\/voices\/candidate\/([a-z0-9_]+\/[a-z0-9_]+)\/([0-9])$/;

export function voiceWorkbenchMiddleware(store: VoiceWorkbenchStore) {
  const json = jsonRoutes(
    "/__sound-workbench/voices/",
    { snapshot: () => store.snapshot(), jobs: () => store.listJobs() },
    {
      preview: (body) => store.preview(body as VoiceDraft),
      save: (body) => store.save(body as Save),
      verdict: (body) => store.verdict(body),
      generate: (body) => store.generate(body),
      design: (body) => store.design(body),
      pick: (body) => store.pick(body),
      background: (body) => store.background(body),
    },
  );
  return async (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    // A design candidate's audio: `candidate/<set>/<voice>/<n>`.
    const audio = request.method === "GET" && request.url?.split("?")[0].match(CANDIDATE_AUDIO);
    if (!audio) return json(request, response, next);
    try {
      localOnly(request);
      const bytes = await store.candidateAudio(audio[1], Number(audio[2]));
      response.setHeader("Content-Type", "audio/mpeg");
      response.setHeader("Cache-Control", "no-store");
      response.end(bytes);
    } catch (error) {
      response.statusCode = error instanceof WorkbenchError ? error.status : 500;
      response.setHeader("Content-Type", "application/json");
      response.end(
        JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
      );
    }
  };
}
