import { decodePublication } from "../../../../packages/multiplayer/src/stream/publication";
// @vitest-environment node
// The battle host, on a clock the test drives, over in-memory seat links.
import { playerPort, playerLink } from "./player";
import { join } from "node:path";
import { randomBytes } from "node:crypto";
import { inProcessBattleMachines } from "../../../../apps/game-server/src/machines/inProcess";
import { createRemoteBattleHost } from "../../../../apps/game-server/src/battleHost/remote";

import { KEYFRAME, readJson } from "../../../../packages/multiplayer/src/stream/index";
import { loopback } from "../../../../packages/multiplayer/src/link";
import type { BattleSeat } from "../../../../packages/multiplayer/src/link";
import type { BattleSetup, StreamOpen } from "../../../../packages/multiplayer/src/wire";
import { preset, setOccupant } from "../../../../packages/skirmish-setup/src/lobby";
import { toBattleSetup } from "../../../../packages/multiplayer/src/setup";
import {
  createBattleHost,
  type BattleClock,
  type BattleEnd,
  type BattleHost,
} from "../../../../apps/game-server/src/battleHost/index";
import { prepareBattle } from "../../../../apps/game-server/src/battleHost/runtime";
import { nodeSetupData } from "../../../../apps/game-server/src/serverData";
import { nodeCatalogSet } from "../../../src/battle/catalog/node";
import type { MapGenerator } from "../../../src/maps/source";
import { createAuthority } from "../../../src/battle/sim/authority";
import type { CommandAck, Order } from "../../../src/battle/sim/protocol";
import type { ObservationView } from "../../../src/battle/sim/observationView";
import { ScenePresentation, loadPresentation } from "../../../src/battle/sim/presentation";

const ROOT = join(import.meta.dirname, "../../../..");
type Data = Awaited<ReturnType<typeof nodeSetupData>>;
export let data: Data;
/** The simulation's generator, generating each requested map once: making
 *  the map is most of a battle's preparation, and no test here is about it. */
export let generator: MapGenerator;
/** Every card of the game, to find one a seat may buy. */
export let cards: string[];
/** A 1v1 of two humans, and a 3v3 with two humans on Blue. */
export let duel: BattleSetup;
export let squad: BattleSetup;

export async function loadBattleSupport() {
  await loadPresentation();
  data = await nodeSetupData(ROOT);
  const wasm = await import("@wasm/game_wasm.js");
  const { readFileSync } = await import("node:fs");
  wasm.initSync({ module: readFileSync(join(ROOT, "web/src/wasm/game_wasm_bg.wasm")) });

  const made = new Map<string, string>();
  generator = {
    ...wasm,
    generate_map(request, ...documents) {
      if (!made.has(request)) made.set(request, wasm.generate_map(request, ...documents));
      return made.get(request)!;
    },
  };
  cards = (await nodeCatalogSet("game", ROOT)).units.view.cards
    .filter((card) => card.disabled_reason === null)
    .map((card) => card.id);
  duel = toBattleSetup(
    setOccupant(preset("1v1"), "seat-2", { kind: "human", who: "a guest" }),
    "skirmish",
  );
  squad = toBattleSetup(
    setOccupant(preset("3v3"), "seat-2", { kind: "human", who: "a guest" }),
    "skirmish",
  );
}

/** Let every promise and stream reader that can run, run. */
export const flush = () => new Promise<void>((resolve) => setImmediate(resolve));

class TestClock implements BattleClock {
  private time = 0;
  private waiting: { at: number; run: () => void }[] = [];
  now = () => this.time;
  schedule = (run: () => void, delayMs: number) => {
    this.waiting.push({ at: this.time + delayMs, run });
  };
  advance(ms: number) {
    this.time += ms;
    for (;;) {
      const due = this.waiting.filter((entry) => entry.at <= this.time);
      if (due.length === 0) return;
      this.waiting = this.waiting.filter((entry) => entry.at > this.time);
      for (const entry of due) entry.run();
    }
  }
}

let remoteHosts = false;
/** Run a contract suite against either host with the same simulation clock. */
export const useRemoteHosts = (remote: boolean) => {
  remoteHosts = remote;
};
export function testHost(candidates: BattleSetup[], clock: BattleClock): BattleHost {
  if (!remoteHosts) return createBattleHost({ candidates }, { clock, generator }, data);
  const key = randomBytes(16).toString("hex");
  const machines = inProcessBattleMachines({
    capacity: 8,
    now: clock.now,
    createBattle: (spec) => createBattleHost(spec, { clock, generator }, data),
    ordersPerSecond: JSON.parse(data.rules).commands.orders_per_s,
  });
  const remote = createRemoteBattleHost({ candidates }, "contract", machines.start(key), key);
  let ended: string | null = null;
  remote.onEnd((end) => {
    ended = end.kind === "finished" ? "finished" : "crashed";
  });
  return {
    ...remote,
    dispose() {
      ended = "disposed";
      remote.dispose();
    },
    attach(port) {
      if (
        !candidates[0].seats.some((s) => s.player === port.seat.player && s.side === port.seat.side)
      )
        throw new Error("unknown battle seat");
      playerPort(remote, "contract", port, () => ended);
    },
  };
}
export const hosts: BattleHost[] = [];
export function disposeBattles() {
  for (const host of hosts.splice(0)) host.dispose();
}

/** A battle on a test clock. `run(ticks)` lets that much battle time pass. */
export async function battle(setup: BattleSetup, candidates = [setup]) {
  const clock = new TestClock();
  const host = testHost(candidates, clock);
  hosts.push(host);
  const ended: BattleEnd[] = [];
  host.onEnd((end) => ended.push(end));
  const ready = await host.ready;
  const run = async (ticks: number) => {
    for (let i = 0; i < ticks; i++) {
      if (remoteHosts && !ended.length) await host.referee();
      clock.advance(1000 / ready.tickHz);
      await flush();
      if (remoteHosts && !ended.length) await host.referee().catch(() => {});
      await flush();
    }
  };
  const advance = async () => {
    clock.advance(1000 / ready.tickHz);
    await flush();
  };
  return { host, ready, run, advance, ended };
}
type Battle = Pick<Awaited<ReturnType<typeof battle>>, "host" | "ready" | "run">;

/** What one reader made of one publication. */
export interface Seen {
  id: number;
  tick: number;
  digest: string;
  keyframe: boolean;
  side: string;
  epoch: number;
  units: number[];
  players: NonNullable<ObservationView["skirmish"]>["players"];
}

/** A page at a seat: its link, and one connection read through the page's
 *  own decoder for as long as the connection lasts. */
export function seat(b: Battle, who: BattleSeat) {
  const pair = loopback(who);
  const transport = playerLink(b.host, who) ?? pair.link;
  const link = {
    ...transport,
    up(batch: Parameters<typeof transport.up>[0]) {
      seq = Math.max(seq, batch.seq);
      return transport.up(batch);
    },
  };
  const cut = () => {
    pair.cut();
  };
  // The runtime tap preserves observations while the browser is disconnected;
  // player orders and previews still go through the authenticated route.
  b.host.attach(pair.port);
  const sent: { tick: number; seat: BattleSeat; order: Order }[] = [];
  let seq = 0;
  return {
    link,
    cut,
    sent,
    /** Send one order and wait for its acknowledgement. */
    async order(order: Order): Promise<CommandAck> {
      const reply = await link.up({ seq: ++seq, items: [{ id: 1, order, queued: false }] });
      if (!("results" in reply)) throw new Error(`the batch was refused: ${reply.refused}`);
      const { ack } = reply.results[0];
      sent.push({ tick: ack.applied_tick - 1, seat: who, order });
      return ack;
    },
    /** Read one connection, from `after`, with `decoder` (a page keeps its
     *  decoder across reconnects). */
    read(after: number | null = null, decoder = new ScenePresentation(b.ready.layout)) {
      const reading = {
        decoder,
        opens: [] as StreamOpen[],
        seen: [] as Seen[],
        latest: null as ObservationView | null,
        end: null as string | null,
        failure: null as unknown,
        lastId: after,
      };
      void (async () => {
        try {
          for await (const record of pair.link.records(after)) {
            if (record.kind === "open") reading.opens.push(readJson<StreamOpen>(record));
            if (record.kind === "end") reading.end = readJson<{ reason: string }>(record).reason;
            if (record.kind !== "publication") continue;
            const publication = decodePublication(record.payload);
            const frame = decoder.decode(new Float32Array(publication.buffer));
            if (!frame) throw new Error(`the page's reader dropped record ${record.id}`);
            reading.latest = frame;
            reading.lastId = record.id;
            reading.seen.push({
              id: record.id,
              tick: publication.tick,
              digest: publication.digest,
              keyframe: (record.flags & KEYFRAME) !== 0,
              side: frame.groundPatch.side,
              epoch: frame.groundPatch.epoch,
              units: frame.own.map((unit) => unit.id),
              players: frame.skirmish?.players ?? [],
            });
          }
        } catch (error) {
          reading.failure = error;
        }
      })();
      return reading;
    },
  };
}
export type Reading = ReturnType<ReturnType<typeof seat>["read"]>;

export const tickOf = (reading: Reading) => reading.seen.at(-1)?.tick ?? 0;
export const digests = (reading: Reading) => new Map(reading.seen.map((s) => [s.tick, s.digest]));

/** Buy the first card the battle lets this seat buy, placed where its side
 *  may place it; returns the card. */
export async function buy(at: ReturnType<typeof seat>): Promise<string> {
  return buyFrom(at, cards);
}

export async function buyFrom(at: ReturnType<typeof seat>, variants: string[]): Promise<string> {
  for (const variant of variants) {
    const ack = await at.order({ kind: "confirm_purchase", variant, destination: [0, 0] });
    if (ack.error === null) return variant;
  }
  throw new Error("the battle sold this seat nothing");
}

/** Run until `reading` shows a unit of its own, and return its id. */
export async function firstUnit(b: Battle, reading: Reading): Promise<number> {
  for (let i = 0; i < 4000 && !reading.latest?.own.length; i++) await b.run(1);
  const unit = reading.latest?.own[0];
  if (!unit) throw new Error("no unit entered the battle");
  return unit.id;
}

/** An order sending `unit` to the battle's first objective. */
export const moveTo = (reading: Reading, unit: number): Order => {
  const [x, y] = reading.latest!.skirmish!.objectives[0].center;
  return { kind: "move", units: [unit], gesture: 1, goal: [x, y], route: "shortest" };
};

/** An authority with no host around it, stepped by hand: the yardstick for
 *  what the host's battle should have computed. */
export async function yardstick(setup: BattleSetup, replay?: string) {
  const prepared = await prepareBattle(setup, data, generator);
  const seen = new Map<number, string>();
  const acks: CommandAck[] = [];
  let failure: string | null = null;
  let tick = 0;
  const authority = createAuthority({
    post(reply) {
      if (reply.type === "ack") acks.push(reply.ack);
      if (reply.type === "error") failure = reply.message;
      if (reply.type !== "publication") return;
      tick = reply.tick;
      seen.set(reply.tick, reply.digest);
      authority.handle({ type: "credit", buffer: reply.buffer });
    },
    now: () => 0,
    schedule: () => {},
    close: () => {},
    load: async () => prepared.sim,
  });
  authority.handle({
    type: "init",
    scenario: prepared.scenario,
    seed: prepared.seed,
    sides: ["blue"],
    replay,
  });
  await flush();
  authority.handle({ type: "start" });
  authority.handle({ type: "pause" });
  return {
    seen,
    acks,
    failure: () => failure,
    runTo(target: number) {
      if (target > tick) authority.handle({ type: "advance", id: 1, ticks: target - tick });
    },
    order(from: BattleSeat, order: Order) {
      authority.handle({ type: "command", id: 1, seat: from, order, queued: false });
    },
  };
}

/** Record from before preparation, including the first authoritative tick. */
export function recordHost(host: BattleHost, who: BattleSeat) {
  const frames = new Map<number, string>();
  let detach = () => {};
  host.attach({
    seat: who,
    onReader(read) {
      detach = read(null, (record) => {
        if (record.kind === "publication") {
          const { tick, digest } = decodePublication(record.payload);
          frames.set(tick, digest);
        }
        return true;
      });
    },
    onUp() {},
    onPreview() {},
  });
  return { frames, close: () => detach() };
}

/** A scoring fixture at the preparation boundary: real capture, income,
 *  finish, publication and replay, with a short match and no opponent buying.
 *  The production target and scoring algorithm remain the rules' own. */
export async function shortScoringFixture() {
  const { vi } = await import("vitest");
  const preparation = await import("../../../src/battle/prepare/prepare");
  const original = preparation.prepare;
  return vi.spyOn(preparation, "prepare").mockImplementation(async (...args) => {
    const prepared = await original(...args);
    const scenario = JSON.parse(prepared.scenario);
    for (const p of scenario.players) if (p.controller !== "human") p.controller = "passive_ai";
    scenario.skirmish.rules.majority_s = 10;
    scenario.skirmish.rules.capture_s = 1;
    const entry = scenario.skirmish.sites.entries.find((e: { side: string }) => e.side === "blue");
    for (const objective of scenario.skirmish.sites.objectives) {
      objective.center = entry.center;
      // Cover the whole friendly entry edge, whichever road the purchase takes.
      objective.radius_m = prepared.report.size[0] + prepared.report.size[1];
    }
    return { ...prepared, scenario: JSON.stringify(scenario) };
  });
}
