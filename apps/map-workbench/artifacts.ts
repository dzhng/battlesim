import * as fs from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { join } from "node:path";
import { WorkbenchError } from "./error";
import type { NativeInputs, Report, RunRequest } from "./src/protocol";

export interface MapArtifact {
  dir: string | null;
  inputs: NativeInputs;
  report: Report;
  receipts: Record<string, string>;
  purpose: RunRequest["purpose"];
}

/** Owns retained map geometry and its temporary storage. */
export class MapArtifactStore {
  private artifacts = new Map<string, MapArtifact>();

  constructor(private readonly root: string) {}

  async allocate() {
    const id = randomUUID();
    const dir = join(this.root, "throwaway", "map-workbench", id);
    await fs.mkdir(dir, { recursive: true });
    return { id, dir };
  }

  get(id: string): MapArtifact {
    const artifact = this.artifacts.get(id);
    if (!artifact) throw new WorkbenchError("This result expired. Generate it again.", 410);
    return artifact;
  }

  async register(id: string, artifact: MapArtifact) {
    if (artifact.report.status === "refused") {
      await this.release(artifact);
    }
    this.artifacts.set(id, artifact);
  }

  async discard(id: string, dir: string) {
    this.artifacts.delete(id);
    await fs.rm(dir, { recursive: true, force: true });
  }

  retainedPreviews(ids: string[]): Set<string> {
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

  async release(artifact: MapArtifact) {
    if (artifact.dir !== null) {
      await fs.rm(artifact.dir, { recursive: true, force: true });
      artifact.dir = null;
    }
  }

  async releaseUndisplayedPreviews(retained: Set<string>) {
    for (const [id, artifact] of this.artifacts) {
      if (artifact.purpose === "preview" && artifact.report.status === "ok" && !retained.has(id)) {
        await this.release(artifact);
        this.artifacts.delete(id);
      }
    }
  }

  async releaseSamples() {
    for (const [id, artifact] of this.artifacts) {
      if (artifact.purpose === "sample") {
        await this.release(artifact);
        this.artifacts.delete(id);
      }
    }
  }

  async releaseRefusedPreviews() {
    for (const [id, artifact] of this.artifacts) {
      if (artifact.purpose === "preview" && artifact.report.status === "refused") {
        await this.release(artifact);
        this.artifacts.delete(id);
      }
    }
  }

  admittedDir(artifact: MapArtifact): string {
    if (artifact.dir === null)
      throw new WorkbenchError("This sample was exported. Generate a preview to inspect it.", 410);
    return artifact.dir;
  }

  async close() {
    await Promise.all([...this.artifacts.values()].map((artifact) => this.release(artifact)));
    this.artifacts.clear();
  }
}
