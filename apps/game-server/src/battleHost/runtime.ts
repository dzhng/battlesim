// A battle on its own thread: the prepared world, the authority that ticks
// it, and each publication turned into the bytes a seat's stream carries.
// The worker thread runs this, and so does a test on its own clock.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { battleRequest, type SetupData } from "../../../../packages/skirmish-setup/src/lobby.ts";
import { fromBattleSetup } from "../../../../packages/multiplayer/src/setup.ts";
import { KEYFRAME } from "../../../../packages/multiplayer/src/stream/index.ts";
import type { BattleSetup } from "../../../../packages/multiplayer/src/wire.ts";
import { prepare, type PreparationModule } from "../../../../web/src/battle/prepare/prepare.ts";
import type { MapGenerator } from "../../../../web/src/maps/source.ts";
import {
  createAuthority,
  type Authority,
  type SimModule,
} from "../../../../web/src/battle/sim/authority.ts";
import type { SimReply, SimRequest, SideName } from "../../../../web/src/battle/sim/protocol.ts";

/** What a battle's thread is given to build its battle: plain data that
 *  crosses to a worker, and the repository root it reads the simulation
 *  file from. */
export type BattleData = Omit<SetupData, "generator"> & { rules: string; root: string };

export interface PreparedBattle {
  sim: SimModule;
  scenario: string;
  seed: number;
  /** The generated map's hash: what a page checks its own map against. */
  mapIdentity: string;
  dispose(): void;
}

/** Generate the map and field the battle `setup` describes. `generator`
 *  stands in for the simulation's own map generator when a caller has one. */
export async function prepareBattle(
  setup: BattleSetup,
  data: BattleData,
  generator?: MapGenerator,
): Promise<PreparedBattle> {
  const wasm = await import("../../../../web/src/wasm/game_wasm.js");
  const { memory } = wasm.initSync({
    module: readFileSync(join(data.root, "web/src/wasm/game_wasm_bg.wasm")),
  });
  const prepared = await prepare(
    wasm as unknown as PreparationModule,
    memory,
    battleRequest(fromBattleSetup(setup), {
      ...data,
      generator: generator ?? (wasm as unknown as MapGenerator),
    }),
    { presets: data.presets, templates: data.templates, rules: data.rules },
  );
  const identity = prepared.report.identity;
  if (identity.kind !== "generated") throw new Error("a server battle runs on a generated map");
  return {
    sim: {
      memory,
      createBattle: (scenario, seed) => prepared.world.into_battle(scenario, seed),
      replayBattle: (scenario, replay) => prepared.world.into_replay(scenario, replay),
    },
    scenario: prepared.scenario,
    seed: setup.battleSeed,
    mapIdentity: identity.generation.map_hash,
    dispose: () => prepared.world.free(),
  };
}

/** What the battle's thread tells the host. A publication arrives as the
 *  finished payload of its stream record (wire.md), already copied out of
 *  the authority's buffer. */
export type RuntimeMessage =
  | Exclude<SimReply, { type: "publication" }>
  | { type: "prepared"; mapIdentity: string; candidate: number }
  | { type: "publication"; side: SideName; flags: number; payload: Uint8Array };

/** `u32 tick`, 16 ASCII bytes of digest, `f32 stepMs`, then the words. */
function publicationPayload(reply: Extract<SimReply, { type: "publication" }>): Uint8Array {
  const bytes = reply.length * Float32Array.BYTES_PER_ELEMENT;
  const payload = new Uint8Array(24 + bytes);
  const view = new DataView(payload.buffer);
  view.setUint32(0, reply.tick, true);
  payload.set(new TextEncoder().encode(reply.digest).subarray(0, 16), 4);
  view.setFloat32(20, reply.stepMs, true);
  payload.set(new Uint8Array(reply.buffer, 0, bytes), 24);
  return payload;
}

/** One worker message keeps the administrative transaction indivisible. */
export type HostRequest =
  | SimRequest
  | {
      type: "seat_human";
      id: number;
      player: string;
      opening: boolean;
    };

export interface BattleRuntime {
  /** Adopt the prepared world. Requests before then wait their turn. */
  start(prepared: PreparedBattle, candidate: number): void;
  handle(request: HostRequest): void;
}

export function createBattleRuntime(options: {
  now: () => number;
  schedule: (run: () => void, delayMs: number) => void;
  post: (message: RuntimeMessage, transfer?: ArrayBuffer[]) => void;
  close: () => void;
}): BattleRuntime {
  let authority: Authority | null = null;
  let disposed = false;
  const backlog: HostRequest[] = [];
  /** Where a publication's header holds its epoch, from the layout. */
  let epochAt = -1;
  const epochs = new Map<SideName, number>();

  // One timer, for the earliest deadline asked for: a pump asks again each
  // time it runs, and an order pumps too, so one timer per ask would multiply.
  let due = Infinity;
  let latest = 0;
  const schedule = (delayMs: number) => {
    const at = options.now() + delayMs;
    if (at >= due) return;
    due = at;
    const mine = ++latest;
    options.schedule(() => {
      if (mine !== latest) return;
      due = Infinity;
      authority?.pump();
    }, delayMs);
  };

  const post = (reply: SimReply, transfer?: Transferable[]) => {
    if (reply.type !== "publication") {
      if (reply.type === "ready")
        epochAt = (JSON.parse(reply.layout) as { header: string[] }).header.indexOf("groundEpoch");
      options.post(reply, transfer as ArrayBuffer[] | undefined);
      return;
    }
    // A record that opens a new epoch is the full snapshot a reader can
    // start from: the stream's keyframe.
    const epoch = new Float32Array(reply.buffer, 0, reply.length)[epochAt];
    const keyframe = epoch > (epochs.get(reply.side) ?? 0);
    epochs.set(reply.side, epoch);
    const payload = publicationPayload(reply);
    // The payload is the copy, so the authority has its buffer back before
    // anything crosses to another thread: the battle never waits for one.
    authority?.handle({ type: "credit", buffer: reply.buffer });
    options.post(
      { type: "publication", side: reply.side, flags: keyframe ? KEYFRAME : 0, payload },
      [payload.buffer as ArrayBuffer],
    );
  };

  const handle = (request: HostRequest) => {
    if (request.type === "dispose") {
      disposed = true;
      backlog.length = 0;
    }
    if (!authority) {
      if (!disposed) backlog.push(request);
      return;
    }
    if (request.type === "seat_human") {
      // These requests never pump. No timer, command or publication can
      // run between them; both events target the same next boundary.
      authority.handle({ type: "hand_over", player: request.player });
      authority.handle({ ...request, type: "reset_seat" });
      authority.handle({
        type: "controllers",
        controllers: JSON.stringify([[request.player, "human"]]),
      });
    } else authority.handle(request);
  };
  return {
    start(admitted, candidate) {
      if (disposed) {
        admitted.dispose();
        return;
      }
      options.post({ type: "prepared", mapIdentity: admitted.mapIdentity, candidate });
      authority = createAuthority({
        post,
        now: options.now,
        schedule,
        close: options.close,
        load: async () => admitted.sim,
      });
      authority.handle({
        type: "init",
        scenario: admitted.scenario,
        seed: admitted.seed,
        sides: ["blue", "red"],
      });
      authority.handle({ type: "start" });
      for (const request of backlog.splice(0)) handle(request);
    },
    handle,
  };
}
