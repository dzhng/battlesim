/** Main-thread client of the one battle authority. Sends ordered commands,
 * resolves their acknowledgements, and hands each completed tick to the
 * consumer; the credit goes back only when the consumer releases it. */
import { createAuthority, type AuthorityHost } from "./authority";
import { loadSimModule } from "./module";
import { decodeObservation, type ObservationLayout, type ObservationView } from "./observation";
import type {
  AuthorityStatus,
  CommandAck,
  Order,
  SideName,
  SimReply,
  SimRequest,
} from "./protocol";

export interface Publication {
  tick: number;
  digest: string;
  observation: ObservationView;
  /** Size of the packed frame on the wire. */
  bytes: number;
  /** Wall time the authority spent stepping this tick, ms. */
  stepMs: number;
  /** Return the buffer to the producer. Until then the authority may stall. */
  release(): void;
}

export interface SimClientOptions {
  scenario: string;
  seed: number;
  side: SideName;
  /** "worker" in production; "direct" runs the same authority in-thread. */
  transport: "worker" | "direct";
  /** Replay a recorded battle instead of accepting input. */
  replay?: string;
  /** Blue is played by this comparison script; blue input is refused. */
  script?: string;
}

export interface SimClient {
  readonly ready: Promise<{ tickHz: number; tick: number }>;
  readonly status: AuthorityStatus;
  readonly slow: boolean;
  start(): void;
  command(order: Order, queued?: boolean): Promise<CommandAck>;
  onPublication(consumer: (publication: Publication) => void): void;
  onStatus(listener: (status: AuthorityStatus, slow: boolean) => void): void;
  pause(): void;
  resume(): void;
  /** Advance exactly `ticks`; resolves once that tick's publication is released. */
  advance(ticks: number): Promise<number>;
  replay(): Promise<string>;
  /** Lab diagnostic: the authoritative ground layer (see `SimRequest` "ground"). */
  ground(): Promise<Float32Array>;
  /** Lab diagnostic: observe as the other side. Commands keep their side. */
  observeAs(side: SideName): void;
  dispose(): void;
}

interface Channel {
  send(request: SimRequest, transfer?: Transferable[]): void;
  close(): void;
}

function workerChannel(
  receive: (reply: SimReply) => void,
  fail: (message: string) => void,
): Channel {
  const worker = new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  worker.onmessage = (event: MessageEvent<SimReply>) => receive(event.data);
  worker.onerror = (event) => fail(event.message || "The simulation worker failed to start.");
  return {
    send: (request, transfer) => worker.postMessage(request, { transfer: transfer ?? [] }),
    close: () => worker.terminate(),
  };
}

/** The same authority in this thread. Transfers really detach the buffers,
 * so a use-after-transfer fails here exactly as it would across a worker. */
function directChannel(receive: (reply: SimReply) => void): Channel {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let closed = false;
  const detach = (transfer?: Transferable[]) =>
    transfer?.map((t) => (t instanceof ArrayBuffer ? structuredClone(t, { transfer: [t] }) : t));
  const host: AuthorityHost = {
    post(reply, transfer) {
      const moved = detach(transfer);
      const delivered =
        reply.type === "publication" && moved
          ? { ...reply, buffer: moved[0] as ArrayBuffer }
          : reply;
      queueMicrotask(() => !closed && receive(delivered));
    },
    now: () => performance.now(),
    schedule(delayMs) {
      if (timer !== null) clearTimeout(timer);
      timer = setTimeout(() => authority.pump(), delayMs);
    },
    close() {
      closed = true;
      if (timer !== null) clearTimeout(timer);
    },
    load: loadSimModule,
  };
  const authority = createAuthority(host);
  return {
    send(request, transfer) {
      const moved = detach(transfer);
      const delivered =
        request.type === "credit" && moved
          ? { ...request, buffer: moved[0] as ArrayBuffer }
          : request;
      queueMicrotask(() => authority.handle(delivered));
    },
    close() {
      authority.handle({ type: "dispose" });
    },
  };
}

export function createSimClient(options: SimClientOptions): SimClient {
  let layout: ObservationLayout | null = null;
  let status: AuthorityStatus = "loading";
  let slow = false;
  let seq = 0;
  let nextAdvance = 1;
  let disposed = false;
  // Highest tick the consumer has released; an advance resolves once its target is covered.
  let releasedTick = -1;
  const pendingAcks = new Map<number, (ack: CommandAck) => void>();
  const advances = new Map<number, (tick: number) => void>();
  const advanceTargets = new Map<number, number>();
  const statusListeners: ((status: AuthorityStatus, slow: boolean) => void)[] = [];
  let consumer: ((publication: Publication) => void) | null = null;
  let replayWaiter: ((json: string) => void) | null = null;
  const groundWaiters: ((cells: Float32Array) => void)[] = [];
  let resolveReady!: (info: { tickHz: number; tick: number }) => void;
  let rejectReady!: (error: Error) => void;
  const ready = new Promise<{ tickHz: number; tick: number }>((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });
  // A failed start must not become an unhandled rejection before anyone awaits it.
  ready.catch(() => {});

  const setStatus = (next: AuthorityStatus, nextSlow = slow) => {
    status = next;
    slow = nextSlow;
    for (const listener of statusListeners) listener(status, slow);
  };

  const fail = (message: string) => {
    rejectReady(new Error(message));
    setStatus("failed");
  };

  const settleAdvances = () => {
    for (const [id, target] of advanceTargets) {
      if (releasedTick < target) continue;
      advanceTargets.delete(id);
      advances.get(id)?.(target);
      advances.delete(id);
    }
  };

  const receive = (reply: SimReply) => {
    if (disposed) return;
    switch (reply.type) {
      case "ready":
        layout = JSON.parse(reply.layout);
        resolveReady({ tickHz: reply.tickHz, tick: reply.tick });
        break;
      case "status":
        setStatus(reply.status, reply.slow);
        break;
      case "ack":
        pendingAcks.get(reply.ack.seq)?.(reply.ack);
        pendingAcks.delete(reply.ack.seq);
        break;
      case "publication": {
        const buffer = reply.buffer;
        const observation = decodeObservation(layout!, new Float32Array(buffer, 0, reply.length));
        let released = false;
        const publication: Publication = {
          tick: reply.tick,
          digest: reply.digest,
          observation,
          bytes: reply.length * Float32Array.BYTES_PER_ELEMENT,
          stepMs: reply.stepMs,
          release() {
            if (released || disposed) return;
            released = true;
            channel.send({ type: "credit", buffer }, [buffer]);
            releasedTick = Math.max(releasedTick, reply.tick);
            settleAdvances();
          },
        };
        if (consumer) consumer(publication);
        else publication.release();
        break;
      }
      case "advanced":
        advanceTargets.set(reply.id, reply.tick);
        settleAdvances();
        break;
      case "replay":
        replayWaiter?.(reply.json);
        replayWaiter = null;
        break;
      case "ground":
        groundWaiters.shift()?.(reply.cells);
        break;
      case "error":
        fail(reply.message);
        break;
    }
  };

  const channel =
    options.transport === "worker" ? workerChannel(receive, fail) : directChannel(receive);
  channel.send({
    type: "init",
    scenario: options.scenario,
    seed: options.seed,
    side: options.side,
    replay: options.replay,
    script: options.script,
  });

  const onVisibility = () => channel.send({ type: "hidden", hidden: document.hidden });
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibility);

  return {
    ready,
    get status() {
      return status;
    },
    get slow() {
      return slow;
    },
    start: () => channel.send({ type: "start" }),
    command(order, queued = false) {
      seq += 1;
      const command = { side: options.side, seq, order, queued };
      return new Promise<CommandAck>((resolve) => {
        pendingAcks.set(seq, resolve);
        channel.send({ type: "command", command });
      });
    },
    onPublication(next) {
      consumer = next;
    },
    onStatus(listener) {
      statusListeners.push(listener);
    },
    pause: () => channel.send({ type: "pause" }),
    resume: () => channel.send({ type: "resume" }),
    advance(ticks) {
      const id = nextAdvance++;
      return new Promise<number>((resolve) => {
        advances.set(id, resolve);
        channel.send({ type: "advance", id, ticks });
      });
    },
    observeAs(side) {
      channel.send({ type: "side", side });
    },
    replay() {
      return new Promise<string>((resolve) => {
        replayWaiter = resolve;
        channel.send({ type: "replay" });
      });
    },
    ground() {
      return new Promise<Float32Array>((resolve) => {
        groundWaiters.push(resolve);
        channel.send({ type: "ground" });
      });
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      if (typeof document !== "undefined")
        document.removeEventListener("visibilitychange", onVisibility);
      channel.close();
      setStatus("disposed");
    },
  };
}
