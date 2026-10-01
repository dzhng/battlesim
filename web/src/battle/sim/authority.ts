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
  /** A live battle; with `script`, blue is played by that comparison script. */
  createBattle(scenario: string, seed: number, script?: string): SimBattle;
  replayBattle(scenario: string, replay: string): SimBattle;
}

export interface SimBattle {
  /** The publication layout: offsets, tags and this battle's round kinds. */
  observation_layout(): string;
  accept(commandJson: string): string;
  preview_move(side: string, moveJson: string): string;
  step(): number;
  tick(): number;
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
  let sim: SimModule | null = null;
  let battle: SimBattle | null = null;
  let side: SideName = "blue";
  let tickMs = 1000 / 30;
  let disposed = false;
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

  const setStatus = (next: AuthorityStatus) => {
    if (next === status) return;
    status = next;
    host.post({ type: "status", status, slow });
  };

  const fail = (error: unknown) => {
    host.post({ type: "error", message: error instanceof Error ? error.message : String(error) });
    setStatus("failed");
    teardown();
  };

  function teardown() {
    disposed = true;
    battle?.free();
    battle = null;
    credits.length = 0;
    host.close();
  }

  /** Step one tick and publish it into a credited buffer. */
  function stepAndPublish() {
    const b = battle!;
    const started = host.now();
    const tick = b.step();
    const stepMs = host.now() - started;
    const length = b.publish(side);
    // Re-derive the view every time: memory may have grown since the last tick.
    const view = new Float32Array(sim!.memory.buffer, b.publication_ptr(), length);
    let buffer = credits.pop()!;
    // Declared admission point: a publication larger than its credit gets a
    // bigger buffer, never a truncated copy.
    if (buffer.byteLength < length * 4)
      buffer = new ArrayBuffer(Math.max(length * 4, buffer.byteLength * 2));
    new Float32Array(buffer, 0, length).set(view);
    host.post({ type: "publication", tick, digest: b.digest(), length, buffer, stepMs }, [buffer]);
    if (script && tick >= script.target) {
      host.post({ type: "advanced", id: script.id, tick });
      script = null;
    }
  }

  function pump() {
    if (disposed || !battle || !started) return;
    if (script) {
      while (script && credits.length > 0) stepAndPublish();
      if (script) return setStatus("waiting-consumer");
      nextTickAt = host.now();
    }
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
      stepAndPublish();
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
    if (disposed) return;
    if (request.type === "dispose") {
      setStatus("disposed");
      teardown();
      return;
    }
    if (!battle && request.type !== "init") {
      backlog.push(request);
      return;
    }
    try {
      switch (request.type) {
        case "init": {
          side = request.side;
          tickMs = 1000 / JSON.parse(request.scenario).rules.tick_hz;
          // A refused scenario or replay throws while building: report it too.
          void host
            .load()
            .then((module) => {
              if (disposed) return;
              sim = module;
              battle = request.replay
                ? module.replayBattle(request.scenario, request.replay)
                : module.createBattle(request.scenario, request.seed, request.script);
              for (let i = 0; i < PUBLICATION_POOL; i++) credits.push(new ArrayBuffer(4096));
              host.post({
                type: "ready",
                layout: battle.observation_layout(),
                tickHz: 1000 / tickMs,
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
            ack: JSON.parse(battle!.accept(JSON.stringify(request.command))),
          });
          return;
        case "move_preview":
          host.post({
            type: "move_preview",
            id: request.id,
            destinations: JSON.parse(
              battle!.preview_move(request.side, JSON.stringify(request.move)),
            ),
          });
          return;
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
          script = { id: request.id, target: battle!.tick() + request.ticks };
          break;
        case "replay":
          host.post({ type: "replay", json: battle!.replay_json() });
          return;
        case "side":
          side = request.side;
          battle!.resync_observation();
          return;
      }
      pump();
    } catch (error) {
      fail(error);
    }
  }

  return { handle, pump };
}
