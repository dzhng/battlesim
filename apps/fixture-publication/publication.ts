import * as fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { dirname, join, resolve } from "node:path";

export interface Replacement {
  path: string;
  before: string;
  after: string;
}
type Failure = (message: string, status?: number) => Error;
interface Journal {
  pid: number;
  files: Replacement[];
}
type Editor = "maps" | "mechanics" | "sounds";
const journals: Record<Editor, string> = {
  maps: "map-workbench-transaction.json",
  mechanics: "mechanics-editor-transaction.json",
  sounds: "sound-workbench-transaction.json",
};
const queues = new Map<string, Promise<unknown>>();

/** Authored mechanics sources, also used to validate interrupted destinations. */
export async function mechanicsSourcePaths(root: string): Promise<string[]> {
  const out = ["fixtures/game.json"];
  async function walk(path: string) {
    let entries;
    try {
      entries = await fs.readdir(join(root, path), { withFileTypes: true });
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
      throw error;
    }
    for (const item of entries) {
      const child = `${path}/${item.name}`;
      // Model manifests are review metadata for the unit workbench, not
      // mechanics documents. They do not have catalog sections and must not be
      // handed to the native catalog validator.
      if (child === "fixtures/units/model-manifest.json") continue;
      if (item.isDirectory()) await walk(child);
      else if (item.isFile() && child.endsWith(".json")) out.push(child);
    }
  }
  await walk("fixtures/units");
  await walk("fixtures/props");
  return out.sort();
}

/** Atomic files and a recoverable multi-file transaction for local fixture editors. */
export class FixturePublication {
  private readonly journal: string;
  constructor(
    private root: string,
    private editor: Editor,
    private failure: Failure,
  ) {
    this.root = resolve(root);
    this.journal = join(this.root, "throwaway", journals[editor]);
  }

  private serial<T>(run: () => Promise<T>): Promise<T> {
    const job = (queues.get(this.root) ?? Promise.resolve()).then(run);
    const settled = job.catch(() => {});
    queues.set(this.root, settled);
    void settled.then(() => {
      if (queues.get(this.root) === settled) queues.delete(this.root);
    });
    return job;
  }

  /** Captures cannot overlap either editor's publication or recovery. */
  capture<T>(read: () => Promise<T>): Promise<T> {
    return this.serial(async () => {
      await this.recover();
      return read();
    });
  }

  private async replace(path: string, text: string, exclusive = false): Promise<void> {
    await fs.mkdir(dirname(this.journal), { recursive: true });
    const staged = join(this.root, "throwaway", `fixture-${randomUUID()}.json`);
    try {
      await fs.writeFile(staged, text);
      if (exclusive) await fs.link(staged, path);
      else await fs.rename(staged, path);
    } finally {
      await fs.rm(staged, { force: true });
    }
  }

  private async checkFiles(files: Replacement[], editor: Editor = this.editor) {
    const allowed = new Set(
      editor === "maps"
        ? ["fixtures/map-presets.json", "fixtures/generated-battle.json"]
        : editor === "sounds"
          ? ["fixtures/sounds.json"]
          : [...(await mechanicsSourcePaths(this.root)), "fixtures/catalog.json"],
    );
    if (
      !Array.isArray(files) ||
      files.some(
        (file) =>
          !file ||
          !allowed.has(file.path) ||
          typeof file.before !== "string" ||
          typeof file.after !== "string",
      ) ||
      new Set(files.map((file) => file.path)).size !== files.length
    )
      throw this.failure("Interrupted save has an unknown destination");
  }

  private async recover(): Promise<void> {
    for (const editor of Object.keys(journals) as Editor[]) await this.recoverJournal(editor);
  }

  private async recoverJournal(editor: Editor): Promise<void> {
    const path = join(this.root, "throwaway", journals[editor]);
    await fs.mkdir(dirname(this.journal), { recursive: true });
    let journal: Journal;
    try {
      journal = JSON.parse(await fs.readFile(path, "utf8"));
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
      throw error;
    }
    if (!Number.isSafeInteger(journal.pid) || journal.pid <= 0)
      throw this.failure("Interrupted save has an invalid owner");
    try {
      process.kill(journal.pid, 0);
      throw this.failure("Another fixture save is in progress. Try again when it finishes.", 409);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
    }
    await this.checkFiles(journal.files, editor);
    for (const file of journal.files) {
      const current = await fs.readFile(join(this.root, file.path), "utf8");
      if (current !== file.before && current !== file.after)
        throw this.failure(
          `Outside edits to ${file.path} conflict with interrupted-save recovery`,
          409,
        );
    }
    for (const file of journal.files) await this.replace(join(this.root, file.path), file.before);
    await fs.rm(path);
  }

  publish(
    files: Replacement[],
    verify: (phase: "before" | "after") => Promise<void>,
  ): Promise<void> {
    return this.serial(async () => {
      await this.recover();
      await this.publishFiles(files, verify);
    });
  }

  private async publishFiles(
    files: Replacement[],
    verify: (phase: "before" | "after") => Promise<void>,
  ): Promise<void> {
    await this.checkFiles(files);
    await verify("before");
    if (!files.length) return;
    try {
      await this.replace(this.journal, JSON.stringify({ pid: process.pid, files }), true);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "EEXIST")
        throw this.failure("Another fixture save is in progress", 409);
      throw error;
    }
    try {
      await verify("before");
      for (const file of files) {
        if ((await fs.readFile(join(this.root, file.path), "utf8")) !== file.before)
          throw this.failure(`Outside edits to ${file.path} interrupted publication`, 409);
        await this.replace(join(this.root, file.path), file.after);
      }
      await verify("after");
    } catch (error) {
      for (const file of files) {
        const current = await fs.readFile(join(this.root, file.path), "utf8");
        if (current === file.after) await this.replace(join(this.root, file.path), file.before);
      }
      await fs.rm(this.journal);
      throw error;
    }
    await fs.rm(this.journal);
  }
}
