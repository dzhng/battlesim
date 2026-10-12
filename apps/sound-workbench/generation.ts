import { spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { MUSIC_TRACKS } from "../../packages/battle-audio/tools/generate/music";
import { WorkbenchError, jsonRoutes, object } from "./http";
import type { GenerationJob } from "./src/protocol";

/** Runs the generation tool with `args`, reporting each output line; resolves to its exit code. */
export type GenerateRunner = (args: string[], line: (text: string) => void) => Promise<number>;

/** The generation tool as a child process: the only writer of the lock and of generated audio. */
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

const KEPT_JOBS = 10;
const KEPT_LINES = 400;

/**
 * Every generation-tool command the workbench runs, voices and music alike, one at
 * a time, so the files the tool writes keep one writer and a verdict never races a
 * generation. Long runs are jobs whose progress the page polls.
 */
export class GenerationQueue {
  readonly jobs: GenerationJob[] = [];
  private tool: Promise<unknown> = Promise.resolve();
  private nextJob = 1;
  constructor(readonly run: GenerateRunner) {}

  /** Runs `command` after every earlier one. */
  serial<T>(command: () => Promise<T>): Promise<T> {
    const job = this.tool.then(command);
    this.tool = job.catch(() => {});
    return job;
  }

  /** Queues the tool with `args` as a job and returns it at once. */
  start(args: string[]): GenerationJob {
    const job: GenerationJob = { id: String(this.nextJob++), args, status: "queued", lines: [] };
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
    return job;
  }
}

/** The Music tab's routes: buying a battle track again, and the queue's progress. */
export function musicWorkbenchMiddleware(queue: GenerationQueue) {
  return jsonRoutes(
    "/__sound-workbench/music/",
    { jobs: async () => ({ jobs: queue.jobs }) },
    {
      regenerate: async (body) => {
        const { track } = object(body, ["track"]);
        if (typeof track !== "string" || !Object.hasOwn(MUSIC_TRACKS, track))
          throw new WorkbenchError("Name one battle music track to regenerate");
        return { job: queue.start(["music", "--track", track]) };
      },
    },
  );
}
