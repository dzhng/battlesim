/** The battle authority: one simulation, one tick owner, one command authority.
 *
 * It applies ordered commands, advances fixed ticks, and hands every completed
 * tick's side observation away as one buffer. With no free buffer it stops
 * ticking rather than dropping or queueing observations: a consumer that cannot
 * keep up makes the battle visibly wait instead of silently losing ticks.
 *
 * The host supplies the clock, the timer, the channel and the module loader,
 * so this state machine runs the same in a worker, in-thread, and in a test. */
import type { AuthorityStatus, SideName, SimReply, SimRequest } from "./protocol";
import { MAX_CATCHUP_TICKS, PUBLICATION_POOL } from "./timing";

/** The slice of the WASM module the authority needs. */
export interface SimModule {
  memory: WebAssembly.Memory;
  /** A live battle. */
  createBattle(scenario: string, seed: number): SimBattle;
  replayBattle(scenario: string, replay: string): SimBattle;
  /** A recorded battle replayed to a tick, then live. */
  resumeBattle(scenario: string, replay: string, tick: number): SimBattle;
}

export interface SimBattle {
  /** The publication layout: offsets, tags and this battle's round kinds. */
  observation_layout(): string;
  set_controllers(controllersJson: string): void;
  hand_over(player: string): void;
  reset_seat(player: string, opening: boolean): void;
  referee_view(): string;
  accept_order(
    side: string,
    player: string | undefined,
    orderJson: string,
    queued: boolean,
  ): string;
  preview_purchase(
    side: string,
    variant: string,
    carrier: string | undefined,
    x: number,
    y: number,
  ): string;
  preview_move(side: string, moveJson: string): string;
  preview_building(side: string, buildingJson: string): string;
  step(): number;
  tick(): number;
  finished(): boolean;
  digest(): string;
  replay_json(): string;
  /** Pack `side`'s observation with visibility and learned-ground changes. */
  publish(side: string): number;
  /** The next publication opens a new epoch with full visibility and learned ground. */
  resync_observation(side: string): void;
  publication_ptr(side: string): number;
  free(): void;
}

export interface AuthorityHost {
  post(reply: SimReply, transfer?: Transferable[]): void;
  /** Monotonic milliseconds. */
  now(): number;
  /** Ask to be pumped after at most this delay; the earliest ask wins. */
  schedule(delayMs: number): void;
  /** The authority has said its last word. */
  close(): void;
  load(): Promise<SimModule>;
}

export interface Authority {
  handle(request: SimRequest): void;
  /** Run whatever is due. The host calls this from its timer. */
  pump(): void;
}

export function createAuthority(host: AuthorityHost): Authority {
  type Resources = { sim: SimModule; battle: SimBattle };
  type Lifetime = { kind: "loading" } | { kind: "live"; resources: Resources } | { kind: "closed" };
  let lifetime: Lifetime = { kind: "loading" };
  let sides: SideName[] = ["blue"];
  // The rules' rate, kept as given: the page's presentation times every
  // effect from it, so a rate derived back from `tickMs` would drift.
  let tickHz = 30;
  let tickMs = 1000 / tickHz;
  let started = false;
  let paused = false;
  let hidden = false;
  let slow = false;
  let waitingForCredit = false;
  let nextTickAt = 0;
  let status: AuthorityStatus = "loading";
  let script: { id: number; target: number } | null = null;
  const credits: ArrayBuffer[] = [];
  let pumping = false;
  const resets: { id: number; tick: number }[] = [];
  const applied: { player: string; tick: number }[] = [];
  // Requests that arrive while the module loads keep their order.
  const backlog: SimRequest[] = [];

  /** An order or preview the battle cannot read is refused, and the battle
   *  goes on: one seat's bad input is not every seat's crash. A trap in the
   *  simulation is not bad input, and still ends the battle. */
  const refusing = (request: "command" | "preview", id: number, answer: () => void) => {
    try {
      answer();
    } catch (error) {
      if (error instanceof WebAssembly.RuntimeError) throw error;
      host.post({ type: "request_refused", request, id, reason: "invalid" });
    }
  };

  const activeResources = (): Resources => {
    if (lifetime.kind !== "live") throw new Error("simulation resources are not admitted");
    return lifetime.resources;
  };

  const setStatus = (next: AuthorityStatus) => {
    if (next === status) return;
    status = next;
    host.post({ type: "status", status, slow });
  };

  const fail = (error: unknown) => {
    host.post({
      type: "error",
      message: error instanceof Error ? error.message : String(error),
    });
    setStatus("failed");
    teardown();
  };

  function teardown() {
    if (lifetime.kind === "live") lifetime.resources.battle.free();
    lifetime = { kind: "closed" };
    credits.length = 0;
    host.close();
  }

  /** Step one tick and publish it into a credited buffer. */
  function stepAndPublish(resources: Resources) {
    const b = resources.battle;
    const started = host.now();
    const tick = b.step();
    const stepMs = host.now() - started;
    for (const side of sides) {
      const length = b.publish(side);
      // Re-derive the view every time: memory may have grown since the last tick.
      const view = new Float32Array(resources.sim.memory.buffer, b.publication_ptr(side), length);
      let buffer = credits.pop()!;
      // Declared admission point: a publication larger than its credit gets a
      // bigger buffer, never a truncated copy.
      if (buffer.byteLength < length * 4)
        buffer = new ArrayBuffer(Math.max(length * 4, buffer.byteLength * 2));
      new Float32Array(buffer, 0, length).set(view);
      host.post({ type: "publication", side, tick, digest: b.digest(), length, buffer, stepMs }, [
        buffer,
      ]);
    }
    while (resets.length && resets[0].tick <= tick)
      host.post({ type: "seat_reset", id: resets.shift()!.id });
    while (applied.length && applied[0].tick <= tick)
      host.post({ type: "order_applied", player: applied.shift()!.player });
    if (script && (tick >= script.target || b.finished())) {
      host.post({ type: "advanced", id: script.id, tick });
      script = null;
    }
  }

  /** Runs the ticks that are due. A host that returns a credit from inside
   *  `post` re-enters here through `handle`; the run in progress picks the
   *  credit up, so the inner call has nothing to do. */
  function pump() {
    if (pumping) return;
    pumping = true;
    try {
      runDue();
    } finally {
      pumping = false;
    }
  }

  function runDue() {
    if (lifetime.kind !== "live" || !started) return;
    const resources = lifetime.resources;
    const battle = resources.battle;
    if (battle.finished()) {
      if (script) {
        host.post({ type: "advanced", id: script.id, tick: battle.tick() });
        script = null;
      }
      return setStatus("finished");
    }
    if (script) {
      while (script && credits.length >= sides.length) stepAndPublish(resources);
      if (script) return setStatus("waiting-consumer");
      nextTickAt = host.now();
    }
    if (battle.finished()) return setStatus("finished");
    if (hidden) return setStatus("hidden");
    if (paused) return setStatus("paused");
    const now = host.now();
    if (waitingForCredit) {
      if (credits.length === 0) return;
      // Credit is back: resume from now rather than bursting through the wait.
      waitingForCredit = false;
      nextTickAt = now;
    }
    let ran = 0;
    while (now >= nextTickAt && ran < MAX_CATCHUP_TICKS) {
      if (credits.length < sides.length) {
        waitingForCredit = true;
        return setStatus("waiting-consumer");
      }
      stepAndPublish(resources);
      if (battle.finished()) return setStatus("finished");
      nextTickAt += tickMs;
      ran++;
    }
    const behind = now >= nextTickAt;
    if (behind) nextTickAt = now + tickMs; // shed debt; never spiral
    if (behind !== slow) {
      slow = behind;
      host.post({ type: "status", status: "running", slow });
    }
    setStatus("running");
    host.schedule(Math.max(0, nextTickAt - host.now()));
  }

  function handle(request: SimRequest) {
    if (lifetime.kind === "closed") return;
    if (request.type === "dispose") {
      setStatus("disposed");
      teardown();
      return;
    }
    if (lifetime.kind !== "live" && request.type !== "init") {
      backlog.push(request);
      return;
    }
    try {
      switch (request.type) {
        case "init": {
          sides = [...new Set(request.sides)];
          if (sides.length === 0) throw new Error("at least one publication side is required");
          tickHz = JSON.parse(request.scenario).rules.tick_hz;
          tickMs = 1000 / tickHz;
          // A refused scenario or replay throws while building: report it too.
          void host
            .load()
            .then((module) => {
              if (lifetime.kind === "closed") return;
              const { resume } = request;
              const battle = resume
                ? module.resumeBattle(request.scenario, resume.replay, resume.tick)
                : request.replay
                  ? module.replayBattle(request.scenario, request.replay)
                  : module.createBattle(request.scenario, request.seed);
              lifetime = { kind: "live", resources: { sim: module, battle } };
              for (let i = 0; i < PUBLICATION_POOL * sides.length; i++)
                credits.push(new ArrayBuffer(4096));
              host.post({
                type: "ready",
                layout: battle.observation_layout(),
                tickHz,
                tick: battle.tick(),
              });
              setStatus("ready");
              for (const queued of backlog.splice(0)) handle(queued);
            })
            .catch((error: unknown) => fail(error));
          return;
        }
        case "start":
          started = true;
          nextTickAt = host.now();
          break;
        case "command": {
          const { battle } = activeResources();
          refusing("command", request.id, () => {
            const ack = JSON.parse(
              battle.accept_order(
                request.seat.side,
                request.seat.player,
                JSON.stringify(request.order),
                request.queued,
              ),
            );
            if (ack.error === null && request.seat.player)
              applied.push({ player: request.seat.player, tick: ack.applied_tick });
            host.post({ type: "ack", id: request.id, ack });
          });
          return;
        }
        case "purchase_preview": {
          const { battle } = activeResources();
          refusing("preview", request.id, () =>
            host.post({
              type: "purchase_preview",
              id: request.id,
              placement: JSON.parse(
                battle.preview_purchase(
                  request.side,
                  request.variant,
                  request.carrier,
                  ...request.destination,
                ),
              ),
            }),
          );
          return;
        }
        case "move_preview": {
          const { battle } = activeResources();
          refusing("preview", request.id, () =>
            host.post({
              type: "move_preview",
              id: request.id,
              destinations: JSON.parse(
                battle.preview_move(request.side, JSON.stringify(request.move)),
              ),
            }),
          );
          return;
        }
        case "building_preview": {
          // A changing selection can invalidate a preview without failing the battle.
          try {
            host.post({
              type: "building_preview",
              id: request.id,
              placement: JSON.parse(
                activeResources().battle.preview_building(
                  request.side,
                  JSON.stringify(request.building),
                ),
              ),
            });
          } catch (error) {
            host.post({
              type: "building_preview",
              id: request.id,
              placement: null,
              error: error instanceof Error ? error.message : String(error),
            });
          }
          return;
        }
        case "credit":
          credits.push(request.buffer);
          break;
        case "pause":
          paused = true;
          break;
        case "resume":
          paused = false;
          nextTickAt = host.now();
          break;
        case "hidden":
          hidden = request.hidden;
          if (!hidden) nextTickAt = host.now();
          break;
        case "advance":
          script = { id: request.id, target: activeResources().battle.tick() + request.ticks };
          break;
        case "replay":
          host.post({
            type: "replay",
            id: request.id,
            json: activeResources().battle.replay_json(),
          });
          return;
        case "sides": {
          const nextSides = [...new Set(request.sides)];
          if (nextSides.length === 0) throw new Error("at least one publication side is required");
          const previous = new Set(sides);
          sides = nextSides;
          for (const side of sides)
            if (!previous.has(side)) activeResources().battle.resync_observation(side);
          return;
        }
        case "controllers":
          activeResources().battle.set_controllers(request.controllers);
          return;
        case "hand_over":
          activeResources().battle.hand_over(request.player);
          return;
        case "reset_seat":
          activeResources().battle.reset_seat(request.player, request.opening);
          resets.push({ id: request.id, tick: activeResources().battle.tick() + 1 });
          return;
        case "referee":
          host.post({
            type: "referee",
            id: request.id,
            view: JSON.parse(activeResources().battle.referee_view()),
          });
          return;
        case "resync":
          activeResources().battle.resync_observation(request.side);
          return;
      }
      pump();
    } catch (error) {
      fail(error);
    }
  }

  return { handle, pump };
}
