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
}

export interface SimBattle {
  /** The publication layout: offsets, tags and this battle's round kinds. */
  observation_layout(): string;
  set_controllers(controllersJson: string): void;
  accept(commandJson: string): string;
  preview_purchase(side: string, variant: string, x: number, y: number): string;
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
  resync_observation(): void;
  publication_ptr(): number;
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
  let side: SideName = "blue";
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
  // Requests that arrive while the module loads keep their order.
  const backlog: SimRequest[] = [];

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
    const length = b.publish(side);
    // Re-derive the view every time: memory may have grown since the last tick.
    const view = new Float32Array(resources.sim.memory.buffer, b.publication_ptr(), length);
    let buffer = credits.pop()!;
    // Declared admission point: a publication larger than its credit gets a
    // bigger buffer, never a truncated copy.
    if (buffer.byteLength < length * 4)
      buffer = new ArrayBuffer(Math.max(length * 4, buffer.byteLength * 2));
    new Float32Array(buffer, 0, length).set(view);
    host.post({ type: "publication", tick, digest: b.digest(), length, buffer, stepMs }, [buffer]);
    if (script && (tick >= script.target || b.finished())) {
      host.post({ type: "advanced", id: script.id, tick });
      script = null;
    }
  }

  function pump() {
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
      while (script && credits.length > 0) stepAndPublish(resources);
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
      if (credits.length === 0) {
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
          side = request.side;
          tickHz = JSON.parse(request.scenario).rules.tick_hz;
          tickMs = 1000 / tickHz;
          // A refused scenario or replay throws while building: report it too.
          void host
            .load()
            .then((module) => {
              if (lifetime.kind === "closed") return;
              const battle = request.replay
                ? module.replayBattle(request.scenario, request.replay)
                : module.createBattle(request.scenario, request.seed);
              lifetime = { kind: "live", resources: { sim: module, battle } };
              for (let i = 0; i < PUBLICATION_POOL; i++) credits.push(new ArrayBuffer(4096));
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
        case "command":
          host.post({
            type: "ack",
            ack: JSON.parse(activeResources().battle.accept(JSON.stringify(request.command))),
          });
          return;
        case "purchase_preview":
          host.post({
            type: "purchase_preview",
            id: request.id,
            placement: JSON.parse(
              activeResources().battle.preview_purchase(
                request.side,
                request.variant,
                ...request.destination,
              ),
            ),
          });
          return;
        case "move_preview":
          host.post({
            type: "move_preview",
            id: request.id,
            destinations: JSON.parse(
              activeResources().battle.preview_move(request.side, JSON.stringify(request.move)),
            ),
          });
          return;
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
          host.post({ type: "replay", json: activeResources().battle.replay_json() });
          return;
        case "side":
          side = request.side;
          activeResources().battle.resync_observation();
          return;
        case "controllers":
          activeResources().battle.set_controllers(request.controllers);
          return;
      }
      pump();
    } catch (error) {
      fail(error);
    }
  }

  return { handle, pump };
}
