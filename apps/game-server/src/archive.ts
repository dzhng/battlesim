// Every battle the lobby ran, one file each: players export from it after
// their battle server is gone, and analytics read the folder.
import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { join } from "node:path";
import type { BattleSetup, BuildIdentity } from "../../../packages/multiplayer/src/wire.ts";

/** A battle ends with a result, is stopped by its lobby before one, or is lost with its machine. */
export type BattleOutcome = "finished" | "stopped" | "crashed";

type BattleRecord = {
  id: string;
  /** The build that ran it; replaying the history needs the same simulation. */
  build: BuildIdentity;
  setup: BattleSetup;
  mapIdentity: string;
  startedAt: string;
  endedAt: string;
  outcome: BattleOutcome;
  /** A digest of each browser that held a seat. The credential itself is never written. */
  seated: string[];
  /** Every accepted command, or null when the battle was lost with its machine. */
  replay: string | null;
};

type ArchivedBattle = Omit<BattleRecord, "build" | "seated"> & { seated: Iterable<string> };

export const seatDigest = (who: string) => createHash("sha256").update(who).digest("hex");

/** Battle ids are server-made; anything else names no file. */
const BATTLE_ID = /^[\w-]+$/;

export function battleArchive(directory: string, build: BuildIdentity) {
  const fileOf = (id: string) => join(directory, `${id}.json`);
  return {
    async save(battle: ArchivedBattle) {
      const record: BattleRecord = {
        ...battle,
        build,
        seated: [...battle.seated].map(seatDigest),
      };
      await mkdir(directory, { recursive: true });
      // A reader never sees half a record.
      await writeFile(`${fileOf(battle.id)}.partial`, JSON.stringify(record));
      await rename(`${fileOf(battle.id)}.partial`, fileOf(battle.id));
    },
    async find(id: string): Promise<BattleRecord | null> {
      if (!BATTLE_ID.test(id)) return null;
      try {
        return JSON.parse(await readFile(fileOf(id), "utf8")) as BattleRecord;
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") return null;
        throw error;
      }
    },
  };
}
export type BattleArchive = ReturnType<typeof battleArchive>;
