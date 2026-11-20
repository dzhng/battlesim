import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { battleArchive } from "../../../../apps/game-server/src/archive";

/** A fresh folder for one lobby's battle records. */
export const archiveFolder = () => mkdtempSync(join(tmpdir(), "battle-archive-"));
export const testArchive = (folder = archiveFolder()) =>
  battleArchive(folder, { sim: "test", data: "test", wire: "test" });
