import * as fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { dirname, join } from "node:path";

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

/** Atomic files and a recoverable multi-file transaction for local fixture editors. */
export class FixturePublication {
  readonly journal: string;
  constructor(
    private root: string,
    journalName: string,
    private allowed: () => Promise<Set<string>>,
    private failure: Failure,
  ) {
    this.journal = join(root, "throwaway", journalName);
  }

  async replace(path: string, text: string, exclusive = false): Promise<void> {
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

  private async checkFiles(files: Replacement[]) {
    const allowed = await this.allowed();
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

  async recover(): Promise<void> {
    await fs.mkdir(dirname(this.journal), { recursive: true });
    let journal: Journal;
    try {
      journal = JSON.parse(await fs.readFile(this.journal, "utf8"));
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
    await this.checkFiles(journal.files);
    for (const file of journal.files) {
      const current = await fs.readFile(join(this.root, file.path), "utf8");
      if (current !== file.before && current !== file.after)
        throw this.failure(
          `Outside edits to ${file.path} conflict with interrupted-save recovery`,
          409,
        );
    }
    for (const file of journal.files) await this.replace(join(this.root, file.path), file.before);
    await fs.rm(this.journal);
  }

  async publish(
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
