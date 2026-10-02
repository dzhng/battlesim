import * as fs from "node:fs/promises";
import { createHash, randomUUID } from "node:crypto";
import { spawn } from "node:child_process";
import { dirname, join, relative, resolve as resolvePath } from "node:path";
import type { Plugin } from "vite";
import { gameplayField, validateGameplayValue, valueAt } from "./src/fields";
import {
  MECHANICS_API,
  targetKey,
  type Json,
  type JsonObject,
  type MechanicsChange,
  type MechanicsDraft,
  type MechanicsPreview,
  type MechanicsSnapshot,
} from "./src/protocol";

type Validator = (
  game: JsonObject,
  catalog: JsonObject[],
  texts?: string[],
  changes?: MechanicsChange[],
) => Promise<string>;
type FileState = { path: string; text: string; value: JsonObject };
type Journal = { pid: number; files: MechanicsPreview["files"] };

function matchesIntent(actual: Json | undefined, expected: Json): boolean {
  if (expected === null) return actual === null || actual === undefined;
  if (Array.isArray(expected))
    return (
      Array.isArray(actual) &&
      actual.length === expected.length &&
      expected.every((value, index) => matchesIntent(actual[index], value))
    );
  if (typeof expected === "object")
    return (
      !!actual &&
      typeof actual === "object" &&
      !Array.isArray(actual) &&
      Object.entries(expected).every(([key, value]) => matchesIntent(actual[key], value))
    );
  return actual === expected;
}

export class MechanicsError extends Error {
  constructor(
    message: string,
    public status = 400,
  ) {
    super(message);
  }
}

function object(value: Json | undefined): JsonObject {
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new MechanicsError("Expected a JSON object");
  return value;
}

function entries(catalog: JsonObject, section: string): JsonObject {
  return section === "weapons"
    ? object(catalog.weapons)
    : object(object((catalog.documents as Json[])[0])[section]);
}

function entry(catalog: JsonObject, section: string, id: string): JsonObject {
  const records = entries(catalog, section);
  if (!Object.hasOwn(records, id)) throw new MechanicsError(`Unknown ${section}.${id}`);
  return object(records[id]);
}

function override(
  target: JsonObject,
  resolved: JsonObject,
  path: string[],
  value: Json | undefined,
  restore: boolean,
): void {
  const [key, ...rest] = path;
  if (!rest.length) {
    if (restore) delete target[key];
    else target[key] = structuredClone(value!);
    return;
  }
  const effective = resolved[key];
  if (Array.isArray(effective)) {
    const [name, ...tail] = rest;
    const inherited =
      effective.find(
        (item) => item && typeof item === "object" && !Array.isArray(item) && item.name === name,
      ) ??
      (Array.isArray(target[key])
        ? target[key].find(
            (item) =>
              item && typeof item === "object" && !Array.isArray(item) && item.name === name,
          )
        : undefined);
    if (!inherited || !tail.length)
      throw new MechanicsError("Edit lists as a whole value; mounts are addressed by name");
    const rows = (target[key] ??= []) as Json[];
    let row = rows.find(
      (item) => item && typeof item === "object" && !Array.isArray(item) && item.name === name,
    );
    if (!row) {
      if (restore) return;
      row = { name };
      rows.push(row);
    }
    override(object(row), object(inherited), tail, value, restore);
    if (Object.keys(object(row)).length === 1) rows.splice(rows.indexOf(row), 1);
    if (!rows.length) delete target[key];
  } else {
    if (!target[key]) {
      if (restore) return;
      target[key] = {};
    }
    override(
      object(target[key]),
      effective && typeof effective === "object" ? object(effective) : object(target[key]),
      rest,
      value,
      restore,
    );
    if (!Object.keys(object(target[key])).length) delete target[key];
  }
}

const pretty = (value: Json) => `${JSON.stringify(value, null, 2)}\n`;
const revision = (files: { path: string; text: string }[]) =>
  createHash("sha256")
    .update(JSON.stringify(files.map((file) => [file.path, file.text])))
    .digest("hex");

async function paths(root: string): Promise<string[]> {
  const out = ["fixtures/game.json"];
  async function walk(path: string) {
    for (const item of await fs.readdir(join(root, path), { withFileTypes: true })) {
      const child = `${path}/${item.name}`;
      if (item.isDirectory()) await walk(child);
      else if (item.isFile() && child.endsWith(".json")) out.push(child);
    }
  }
  await walk("fixtures/units");
  await walk("fixtures/props");
  return out.sort();
}

export function nativeValidator(root: string): Validator {
  return (game, catalog, texts) =>
    new Promise((resolve, reject) => {
      // Raw source text reaches Rust intact so duplicate keys cannot disappear
      // in JavaScript's otherwise permissive JSON parser.
      const input = texts
        ? `{"game":${texts[0]},"catalog":[${texts.slice(1).join(",")}]}`
        : JSON.stringify({ game, catalog });
      const child = spawn(
        resolvePath(
          root,
          process.env.CARGO_TARGET_DIR ?? "throwaway/target",
          "debug/examples/mechanics_validate",
        ),
        [],
        {
          cwd: root,
          stdio: ["pipe", "pipe", "pipe"],
        },
      );
      let stdout = "",
        stderr = "";
      child.stdout.setEncoding("utf8");
      child.stderr.setEncoding("utf8");
      child.stdout.on("data", (data) => {
        stdout += data;
      });
      child.stderr.on("data", (data) => {
        stderr += data;
      });
      child.on("error", (error) =>
        reject(
          new Error(
            `Native mechanics validator could not start. Run bun run build:mechanics. ${error.message}`,
          ),
        ),
      );
      child.on("close", (code) => {
        if (code !== 0) reject(new MechanicsError(stderr.trim() || "Mechanics validation failed"));
        else resolve(stdout);
      });
      child.stdin.on("error", () => {});
      child.stdin.end(input);
    });
}

export class MechanicsStore {
  private queue: Promise<unknown> = Promise.resolve();
  private journal: string;
  constructor(
    private root: string,
    private validate: Validator,
  ) {
    this.journal = join(root, "throwaway/mechanics-editor-transaction.json");
  }

  private serial<T>(run: () => Promise<T>): Promise<T> {
    const job = this.queue.then(run);
    this.queue = job.catch(() => {});
    return job;
  }

  private async replace(path: string, text: string, exclusive = false): Promise<void> {
    const staged = join(this.root, "throwaway", `mechanics-${randomUUID()}.json`);
    try {
      await fs.writeFile(staged, text);
      // Linking a complete journal acquires the lock without exposing a partial
      // JSON write if the process exits before publication starts.
      if (exclusive) await fs.link(staged, join(this.root, path));
      else await fs.rename(staged, join(this.root, path));
    } finally {
      await fs.rm(staged, { force: true });
    }
  }

  private async recover(): Promise<void> {
    let journal: Journal;
    try {
      journal = JSON.parse(await fs.readFile(this.journal, "utf8"));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
      throw error;
    }
    try {
      process.kill(journal.pid, 0);
      throw new MechanicsError(
        "Another mechanics save is in progress. Try again when it finishes.",
        409,
      );
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
    }
    const allowed = new Set([...(await paths(this.root)), "fixtures/catalog.json"]);
    for (const file of journal.files) {
      if (!allowed.has(file.path))
        throw new MechanicsError("Interrupted save has an unknown destination");
      const current = await fs.readFile(join(this.root, file.path), "utf8");
      if (current !== file.before && current !== file.after)
        throw new MechanicsError(
          `Outside edits to ${file.path} conflict with interrupted-save recovery`,
          409,
        );
    }
    for (const file of journal.files) await this.replace(file.path, file.before);
    await fs.rm(this.journal);
  }

  private async read(): Promise<{ files: FileState[]; generated: string; revision: string }> {
    const files = await Promise.all(
      (await paths(this.root)).map(async (path) => {
        const text = await fs.readFile(join(this.root, path), "utf8");
        return { path, text, value: object(JSON.parse(text)) };
      }),
    );
    const generated = await fs.readFile(join(this.root, "fixtures/catalog.json"), "utf8");
    return {
      files,
      generated,
      revision: revision([...files, { path: "fixtures/catalog.json", text: generated }]),
    };
  }

  private async loaded() {
    await fs.mkdir(dirname(this.journal), { recursive: true });
    await this.recover();
    const state = await this.read();
    const game = state.files.find((file) => file.path === "fixtures/game.json")!;
    const documents = state.files.filter((file) => file !== game);
    const text = await this.validate(
      game.value,
      documents.map((file) => file.value),
      [game.text, ...documents.map((file) => file.text)],
    );
    return { ...state, catalog: object(JSON.parse(text)) };
  }

  snapshot(): Promise<MechanicsSnapshot> {
    return this.serial(async () => {
      const state = await this.loaded();
      return {
        revision: state.revision,
        documents: state.files.map(({ path, value }) => ({ path, value })),
        catalog: state.catalog,
      };
    });
  }

  private async candidate(draft: MechanicsDraft): Promise<MechanicsPreview> {
    if (!draft || typeof draft !== "object" || Array.isArray(draft))
      throw new MechanicsError("A draft must be a JSON object");
    const state = await this.loaded();
    if (draft.revision !== state.revision)
      throw new MechanicsError(
        "Fixtures changed outside this draft. Reload the saved values before applying edits.",
        409,
      );
    if (!Array.isArray(draft.changes)) throw new MechanicsError("Changes must be a list");
    const documents = structuredClone(state.files);
    const raw = (section: string, id: string) => {
      const doc =
        section === "weapons"
          ? documents.find((file) => file.path === "fixtures/game.json")
          : documents.find((file) => {
              const rows = file.value[section];
              return (
                rows && typeof rows === "object" && !Array.isArray(rows) && Object.hasOwn(rows, id)
              );
            });
      if (!doc) throw new MechanicsError(`No source for ${section}.${id}`);
      return { doc, row: object(object(doc.value[section])[id]) };
    };
    const seen = new Set<string>();
    const soldierIds: Record<string, string> = {};
    for (const change of draft.changes) {
      if (
        !change ||
        typeof change !== "object" ||
        Array.isArray(change) ||
        (change.restore !== undefined && typeof change.restore !== "boolean") ||
        (change.unit !== undefined &&
          (change.section !== "soldiers" || typeof change.unit !== "string")) ||
        (change.restore && Object.hasOwn(change, "value"))
      )
        throw new MechanicsError("Invalid edit operation");
      if (
        !["units", "soldiers", "weapons"].includes(change.section) ||
        typeof change.id !== "string" ||
        !Array.isArray(change.path) ||
        change.path.some(
          (key) =>
            typeof key !== "string" || ["__proto__", "prototype", "constructor"].includes(key),
        )
      )
        throw new MechanicsError("Invalid field address");
      const address = JSON.stringify([change.section, change.id, change.unit, change.path]);
      if (seen.has(address))
        throw new MechanicsError("A draft must contain one final value per field");
      seen.add(address);
      if (!gameplayField(change.section, change.path))
        throw new MechanicsError("This field is not an editable gameplay value");
      if (!change.restore) {
        if (!Object.hasOwn(change, "value")) throw new MechanicsError("An edit needs a value");
        const error = validateGameplayValue(change.section, change.path, change.value!);
        if (error) throw new MechanicsError(error);
      }
      const effective = entry(state.catalog, change.section, change.id);
      let target = raw(change.section, change.id).row;
      if (change.section === "soldiers") {
        if (!change.unit) throw new MechanicsError("Soldier edits must name the selected unit");
        const unit = entry(state.catalog, "units", change.unit);
        const slots = object(object(unit.body).squad).slots as Json[];
        if (!slots.includes(change.id))
          throw new MechanicsError("This unit does not carry the selected soldier kind");
        const ownUnit = raw("units", change.unit);
        const localId = change.id.startsWith(`${change.unit}__`)
          ? change.id
          : `${change.unit}__${change.id}`;
        soldierIds[targetKey(change)] = localId;
        if (localId !== change.id) {
          const localRows = (ownUnit.doc.value.soldiers ??= {}) as JsonObject;
          if (Object.hasOwn(localRows, localId)) {
            const existing = object(localRows[localId]);
            if (existing.extends !== change.id)
              throw new MechanicsError(`Soldier variant id ${localId} is already in use`);
            target = existing;
          } else target = object((localRows[localId] = { extends: change.id }));
          ownUnit.row.body ??= {};
          const currentSlots = valueAt(ownUnit.row, ["body", "squad", "slots"]);
          const squad = object((object(ownUnit.row.body).squad ??= {}));
          squad.slots = (Array.isArray(currentSlots) ? currentSlots : slots).map((kind) =>
            kind === change.id ? localId : kind,
          );
        }
      }
      override(target, effective, change.path, change.value, !!change.restore);
    }
    const cleanedSoldiers = new Set<string>();
    const restoredUnits = new Set<string>();
    for (const change of draft.changes) {
      if (
        change.section !== "soldiers" ||
        !change.unit ||
        !change.id.startsWith(`${change.unit}__`)
      )
        continue;
      if (cleanedSoldiers.has(change.id)) continue;
      cleanedSoldiers.add(change.id);
      const own = raw("soldiers", change.id);
      if (Object.keys(own.row).length !== 1 || typeof own.row.extends !== "string") continue;
      const unit = raw("units", change.unit);
      const effective = entry(state.catalog, "units", change.unit);
      const slots =
        valueAt(unit.row, ["body", "squad", "slots"]) ??
        valueAt(effective, ["body", "squad", "slots"]);
      const squad = object((object((unit.row.body ??= {})).squad ??= {}));
      squad.slots = (slots as Json[]).map((id) => (id === change.id ? own.row.extends! : id));
      for (const [key, id] of Object.entries(soldierIds))
        if (id === change.id) soldierIds[key] = String(own.row.extends);
      delete object(own.doc.value.soldiers)[change.id];
      if (!Object.keys(object(own.doc.value.soldiers)).length) delete own.doc.value.soldiers;
      restoredUnits.add(change.unit);
    }
    const game = documents.find((file) => file.path === "fixtures/game.json")!;
    const originals = new Map(state.files.map((file) => [file.path, JSON.stringify(file.value)]));
    const sourceText = (file: FileState) =>
      JSON.stringify(file.value) === originals.get(file.path) ? file.text : pretty(file.value);
    const validate = () => {
      const rows = documents.filter((file) => file !== game);
      return this.validate(
        game.value,
        rows.map((file) => file.value),
        [sourceText(game), ...rows.map(sourceText)],
        draft.changes,
      );
    };
    let generated = await validate();
    let catalog = object(JSON.parse(generated));
    // Ask the catalog owner whether the slot override is still needed. Removing
    // the last local soldier override should return an inherited unit to its
    // authored form, including inheritance rather than a copied slot list.
    for (const id of restoredUnits) {
      const unit = raw("units", id).row;
      if (!unit.extends) continue;
      const original = structuredClone(unit);
      override(unit, entry(catalog, "units", id), ["body", "squad", "slots"], undefined, true);
      try {
        const inheritedText = await validate();
        const inherited = object(JSON.parse(inheritedText));
        if (!matchesIntent(entry(inherited, "units", id), entry(catalog, "units", id))) {
          Object.assign(unit, original);
        } else {
          generated = inheritedText;
          catalog = inherited;
        }
      } catch {
        Object.assign(unit, original);
      }
    }
    for (const change of draft.changes) {
      if (change.restore) continue;
      const id = soldierIds[targetKey(change)] ?? change.id;
      if (
        change.section === "soldiers" &&
        !(
          valueAt(entry(catalog, "units", change.unit!), ["body", "squad", "slots"]) as Json[]
        ).includes(id)
      )
        throw new MechanicsError(
          "The selected unit no longer carries the edited soldier kind. Update its slots or remove the soldier edit.",
        );
      if (!matchesIntent(valueAt(entry(catalog, change.section, id), change.path), change.value!))
        throw new MechanicsError(
          "Inheritance or an upgrade part overrides this field. The requested value would not take effect.",
        );
    }
    const changedRows = (section: string) =>
      new Set(
        Object.keys(entries(catalog, section)).filter(
          (id) =>
            JSON.stringify(entries(catalog, section)[id]) !==
            JSON.stringify(entries(state.catalog, section)[id]),
        ),
      );
    const changedWeapons = changedRows("weapons");
    const changedSoldiers = changedRows("soldiers");
    const oldUnits = state.catalog.units as JsonObject[];
    const units = catalog.units as JsonObject[];
    const affectedUnits = units
      .filter(
        (unit) =>
          JSON.stringify(unit) !== JSON.stringify(oldUnits.find((old) => old.id === unit.id)) ||
          (unit.mounts as JsonObject[]).some((mount) =>
            (mount.weapons as string[]).some((id) => changedWeapons.has(id)),
          ) ||
          (valueAt(unit, ["body", "squad", "slots"]) as string[] | undefined)?.some((id) =>
            changedSoldiers.has(id),
          ),
      )
      .map((unit) => String(unit.id));
    const files = documents.flatMap((file, i) =>
      JSON.stringify(file.value) === JSON.stringify(state.files[i].value)
        ? []
        : [{ path: file.path, before: state.files[i].text, after: pretty(file.value) }],
    );
    if (generated !== state.generated)
      files.push({ path: "fixtures/catalog.json", before: state.generated, after: generated });
    const geometry = draft.changes.some((change) =>
      ["half_extents_m", "eye_m", "pivot_m", "muzzle_m"].some((key) => change.path.includes(key)),
    );
    return {
      revision: state.revision,
      soldierIds,
      catalog,
      files,
      affectedUnits,
      warnings: geometry
        ? ["These physical values also affect the model. The current model-fit checks passed."]
        : [],
    };
  }

  preview(draft: MechanicsDraft): Promise<MechanicsPreview> {
    return this.serial(() => this.candidate(draft));
  }

  save(draft: MechanicsDraft): Promise<MechanicsSnapshot> {
    return this.serial(async () => {
      const proposal = await this.candidate(draft);
      let state = await this.read();
      if (state.revision !== draft.revision)
        throw new MechanicsError("Fixtures changed while validating. Reload before saving.", 409);
      const replacements = new Map(proposal.files.map((file) => [file.path, file.after]));
      const expectedRevision = revision([
        ...state.files.map(({ path, text }) => ({ path, text: replacements.get(path) ?? text })),
        {
          path: "fixtures/catalog.json",
          text: replacements.get("fixtures/catalog.json") ?? state.generated,
        },
      ]);
      if (proposal.files.length) {
        const journal: Journal = { pid: process.pid, files: proposal.files };
        try {
          await this.replace(relative(this.root, this.journal), JSON.stringify(journal), true);
        } catch (error) {
          if ((error as NodeJS.ErrnoException).code === "EEXIST")
            throw new MechanicsError("Another mechanics save is in progress", 409);
          throw error;
        }
        try {
          if ((await this.read()).revision !== draft.revision)
            throw new MechanicsError("Fixtures changed before publication", 409);
          for (const file of proposal.files) {
            if ((await fs.readFile(join(this.root, file.path), "utf8")) !== file.before)
              throw new MechanicsError(
                `Outside edits to ${file.path} interrupted publication`,
                409,
              );
            await this.replace(file.path, file.after);
          }
          const published = await this.read();
          if (published.revision !== expectedRevision)
            throw new MechanicsError(
              "Fixtures changed during publication. Your outside edits were preserved.",
              409,
            );
          state = published;
        } catch (error) {
          for (const file of proposal.files) {
            const current = await fs.readFile(join(this.root, file.path), "utf8");
            if (current === file.after) await this.replace(file.path, file.before);
          }
          await fs.rm(this.journal);
          throw error;
        }
        await fs.rm(this.journal);
      }
      return {
        revision: state.revision,
        documents: state.files.map(({ path, value }) => ({ path, value })),
        catalog: proposal.catalog,
      };
    });
  }
}

export function mechanicsPlugin(root: string): Plugin {
  let store: MechanicsStore;
  return {
    name: "mechanics-editor",
    apply: "serve",
    configureServer(server) {
      const native = nativeValidator(root);
      store = new MechanicsStore(root, async (game, documents, texts, changes) => {
        const catalogText = await native(game, documents, texts);
        if (
          changes?.some((change) =>
            ["half_extents_m", "eye_m", "pivot_m", "muzzle_m", "parts", "mounts", "deploy"].some(
              (key) => change.path.includes(key),
            ),
          )
        ) {
          const fit = await server.ssrLoadModule(join(root, "apps/mechanics-editor/modelFit.ts"));
          await fit.validateGeometry(root, object(JSON.parse(catalogText)));
        }
        return catalogText;
      });
      server.middlewares.use(async (req, res, next) => {
        const path = req.url?.split("?")[0];
        if (!path?.startsWith(MECHANICS_API)) return next();
        res.setHeader("Content-Type", "application/json");
        res.setHeader("Cache-Control", "no-store");
        try {
          const host = req.headers.host ?? "";
          const address = req.socket.remoteAddress ?? "";
          if (
            !/^(localhost|127\.0\.0\.1|\[::1\])(?::\d+)?$/.test(host) ||
            !["127.0.0.1", "::1", "::ffff:127.0.0.1"].includes(address)
          )
            throw new MechanicsError("The mechanics editor is local-only", 403);
          if (req.method === "GET" && path === MECHANICS_API)
            res.end(JSON.stringify(await store.snapshot()));
          else if (
            req.method === "POST" &&
            [MECHANICS_API + "/preview", MECHANICS_API + "/save"].includes(path)
          ) {
            if (
              req.headers.origin !== `http://${host}` ||
              !req.headers["content-type"]?.startsWith("application/json")
            )
              throw new MechanicsError("Save requests must come from this local editor", 403);
            req.setEncoding("utf8");
            let text = "";
            for await (const chunk of req) text += chunk;
            const draft: MechanicsDraft = JSON.parse(text);
            res.end(
              JSON.stringify(
                path.endsWith("/save") ? await store.save(draft) : await store.preview(draft),
              ),
            );
          } else throw new MechanicsError("Unknown mechanics endpoint", 404);
        } catch (error) {
          res.statusCode = error instanceof MechanicsError ? error.status : 500;
          res.end(
            JSON.stringify({ error: error instanceof Error ? error.message : String(error) }),
          );
        }
      });
    },
    handleHotUpdate(context) {
      const path = relative(root, context.file).replaceAll("\\", "/");
      if (
        path === "fixtures/game.json" ||
        path === "fixtures/catalog.json" ||
        path.startsWith("fixtures/units/") ||
        path.startsWith("fixtures/props/")
      ) {
        for (const module of context.modules) context.server.moduleGraph.invalidateModule(module);
        return [];
      }
    },
  };
}
