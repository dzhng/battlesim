/** Main-thread client of the one battle authority. Sends ordered commands,
 * resolves their acknowledgements, and hands each completed tick to the
 * consumer; the credit goes back only when the consumer releases it. */
import { createAuthority, type AuthorityHost } from "./authority";
import { loadSimModule } from "./module";
import { GroundView } from "./ground";
import { ObservationDecoder, type ObservationLayout, type ObservationView } from "./observation";
import type {
  AuthorityStatus,
  CommandAck,
  Order,
  PurchasePlacement,
  MovePreviewRequest,
  MoveDestination,
  BuildingPreviewRequest,
  BuildingPlacement,
  SideName,
  SimReply,
  SimRequest,
} from "./protocol";

export interface Publication {
  tick: number;
  digest: string;
  observation: ObservationView;
  /** The side's learned ground, with this publication's patch applied. One
   *  view for the client's life: it only ever moves forward. */
  ground: GroundView;
  /** Size of the packed frame on the wire. */
  bytes: number;
  /** Wall time the authority spent stepping this tick, ms. */
  stepMs: number;
  /** Copy the packed authority words while the credit is held, preserving bits. */
  copyPacked(): number[];
  /** Return the buffer to the producer. Until then the authority may stall. */
  release(): void;
}

export interface SimClientOptions {
  scenario: string;
  seed: number;
  side: SideName;
  /** "worker" in production; "direct" runs the same authority in-thread. */
  transport: "worker" | "direct";
  /** Adopt a worker that already holds this battle's prepared world. */
  connect?: SimConnection;
  /** Replay a recorded battle instead of accepting input. */
  replay?: string;
}

export interface SimClient {
  readonly ready: Promise<{ tickHz: number; tick: number; layout: string }>;
  /** Explicit pause intent, independent of loading, visibility or consumer stalls. */
  readonly paused: boolean;
  start(): void;
  command(order: Order, queued?: boolean): Promise<CommandAck>;
  previewPurchase(variant: string, destination: [number, number]): Promise<PurchasePlacement>;
  previewMove(move: MovePreviewRequest): Promise<MoveDestination[]>;
  previewBuilding(building: BuildingPreviewRequest): Promise<BuildingPlacement>;
  onPublication(consumer: (publication: Publication) => void): void;
  onStatus(listener: (status: AuthorityStatus, slow: boolean) => void): void;
  pause(): void;
  resume(): void;
  /** Advance up to `ticks` or match completion; resolves after the final publication is released. */
  advance(ticks: number): Promise<number>;
  replay(): Promise<string>;
  /** Lab diagnostic: observe as the other side. Commands keep their side.
   *  The ground view empties until that side's snapshot arrives. */
  observeAs(side: SideName): void;
  dispose(): void;
}

export type SimConnection = (
  receive: (reply: SimReply) => void,
  fail: (message: string) => void,
) => SimChannel;

export interface SimChannel {
  send(request: SimRequest, transfer?: Transferable[]): void;
  close(): void;
}

function workerChannel(
  receive: (reply: SimReply) => void,
  fail: (message: string) => void,
): SimChannel {
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
function directChannel(receive: (reply: SimReply) => void): SimChannel {
  let timer: ReturnType<typeof setTimeout> | null = null;
  const host: AuthorityHost = {
    post(reply, transfer) {
      const delivered = transfer?.length ? structuredClone(reply, { transfer }) : reply;
      queueMicrotask(() => receive(delivered));
    },
    now: () => performance.now(),
    schedule(delayMs) {
      if (timer !== null) clearTimeout(timer);
      timer = setTimeout(() => authority.pump(), delayMs);
    },
    close() {
      if (timer !== null) clearTimeout(timer);
    },
    load: loadSimModule,
  };
  const authority = createAuthority(host);
  return {
    send(request, transfer) {
      const delivered = transfer?.length ? structuredClone(request, { transfer }) : request;
      queueMicrotask(() => authority.handle(delivered));
    },
    close() {
      authority.handle({ type: "dispose" });
    },
  };
}

export function createSimClient(options: SimClientOptions): SimClient {
  let decoder: ObservationDecoder | null = null;
  let observedSide = options.side;
  let ground: GroundView | null = null;
  let slow = false;
  let paused = false;
  let seq = 0;
  let nextAdvance = 1;
  let nextPreview = 1;
  let disposed = false;
  let failure: Error | null = null;
  const heldBuffers = new Set<ArrayBuffer>();
  // Highest tick the consumer has released; an advance resolves once its target is covered.
  let releasedTick = -1;
  type Pending<T> = { resolve: (value: T) => void; reject: (error: Error) => void };
  const pendingAcks = new Map<number, Pending<CommandAck>>();
  const advances = new Map<number, Pending<number>>();
  const previews = new Map<
    number,
    | (Pending<MoveDestination[]> & { kind: "move" })
    | (Pending<BuildingPlacement> & { kind: "building" })
    | (Pending<PurchasePlacement> & { kind: "purchase" })
  >();
  const advanceTargets = new Map<number, number>();
  const statusListeners: ((status: AuthorityStatus, slow: boolean) => void)[] = [];
  let consumer: ((publication: Publication) => void) | null = null;
  let replayWaiter: Pending<string> | null = null;
  let resolveReady!: (info: { tickHz: number; tick: number; layout: string }) => void;
  let rejectReady!: (error: Error) => void;
  const ready = new Promise<{ tickHz: number; tick: number; layout: string }>((resolve, reject) => {
    resolveReady = resolve;
    rejectReady = reject;
  });
  // A failed start must not become an unhandled rejection before anyone awaits it.
  ready.catch(() => {});

  const setStatus = (next: AuthorityStatus, nextSlow = slow) => {
    slow = nextSlow;
    for (const listener of statusListeners) listener(next, slow);
  };

  const returnCredit = (buffer: ArrayBuffer) => {
    if (!heldBuffers.delete(buffer)) return;
    channel.send({ type: "credit", buffer }, [buffer]);
  };

  const fail = (message: string) => {
    if (failure || disposed) return;
    failure = new Error(message);
    for (const buffer of heldBuffers) returnCredit(buffer);
    channel.send({ type: "pause" });
    rejectReady(failure);
    for (const pending of pendingAcks.values()) pending.reject(failure);
    for (const pending of advances.values()) pending.reject(failure);
    for (const pending of previews.values()) pending.reject(failure);
    replayWaiter?.reject(failure);
    pendingAcks.clear();
    advances.clear();
    previews.clear();
    advanceTargets.clear();
    replayWaiter = null;
    setStatus("failed");
  };

  const settleAdvances = () => {
    for (const [id, target] of advanceTargets) {
      if (releasedTick < target) continue;
      advanceTargets.delete(id);
      advances.get(id)?.resolve(target);
      advances.delete(id);
    }
  };

  const receive = (reply: SimReply) => {
    if (disposed) return;
    if (reply.type === "publication") heldBuffers.add(reply.buffer);
    if (failure) {
      if (reply.type === "publication") returnCredit(reply.buffer);
      return;
    }
    try {
      switch (reply.type) {
        case "ready": {
          const layout = JSON.parse(reply.layout) as ObservationLayout;
          decoder = new ObservationDecoder(layout);
          decoder.invalidate(observedSide);
          ground = new GroundView(layout.ground);
          resolveReady({ tickHz: reply.tickHz, tick: reply.tick, layout: reply.layout });
          break;
        }
        case "status":
          setStatus(reply.status, reply.slow);
          break;
        case "ack":
          pendingAcks.get(reply.ack.seq)?.resolve(reply.ack);
          pendingAcks.delete(reply.ack.seq);
          break;
        case "purchase_preview": {
          const preview = previews.get(reply.id);
          if (preview?.kind === "purchase") preview.resolve(reply.placement);
          previews.delete(reply.id);
          break;
        }
        case "move_preview": {
          const preview = previews.get(reply.id);
          if (preview?.kind === "move") preview.resolve(reply.destinations);
          previews.delete(reply.id);
          break;
        }
        case "building_preview": {
          const preview = previews.get(reply.id);
          if (preview?.kind === "building") {
            if (reply.placement) preview.resolve(reply.placement);
            else preview.reject(new Error(reply.error ?? "Building action is unavailable"));
          }
          previews.delete(reply.id);
          break;
        }
        case "publication": {
          const buffer = reply.buffer;
          if (!decoder || !ground)
            throw new Error("publication arrived before the authority was ready");
          const observation = decoder.decode(new Float32Array(buffer, 0, reply.length));
          if (!observation) {
            returnCredit(buffer);
            releasedTick = Math.max(releasedTick, reply.tick);
            settleAdvances();
            return;
          }
          // Before the credit can return: the patch is part of this record.
          ground!.applyRuns(observation.groundPatch);
          let released = false;
          const publication: Publication = {
            tick: reply.tick,
            digest: reply.digest,
            observation,
            ground: ground!,
            bytes: reply.length * Float32Array.BYTES_PER_ELEMENT,
            stepMs: reply.stepMs,
            copyPacked() {
              return Array.from(new Uint32Array(buffer, 0, reply.length));
            },
            release() {
              if (released || disposed) return;
              released = true;
              returnCredit(buffer);
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
          replayWaiter?.resolve(reply.json);
          replayWaiter = null;
          break;
        case "error":
          fail(reply.message);
          break;
      }
    } catch (error) {
      fail(error instanceof Error ? error.message : String(error));
    }
  };

  const channel = options.connect
    ? options.connect(receive, fail)
    : options.transport === "worker"
      ? workerChannel(receive, fail)
      : directChannel(receive);
  channel.send({
    type: "init",
    scenario: options.scenario,
    seed: options.seed,
    side: options.side,
    replay: options.replay,
  });

  const send = (request: SimRequest) => {
    if (!failure && !disposed) channel.send(request);
  };
  const onVisibility = () => send({ type: "hidden", hidden: document.hidden });
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibility);

  return {
    ready,
    get paused() {
      return paused;
    },
    start: () => send({ type: "start" }),
    command(order, queued = false) {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      seq += 1;
      const command = { side: options.side, seq, order, queued };
      return new Promise<CommandAck>((resolve, reject) => {
        pendingAcks.set(seq, { resolve, reject });
        channel.send({ type: "command", command });
      });
    },
    previewPurchase(variant, destination) {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      const id = nextPreview++;
      return new Promise<PurchasePlacement>((resolve, reject) => {
        previews.set(id, { kind: "purchase", resolve, reject });
        channel.send({ type: "purchase_preview", id, side: options.side, variant, destination });
      });
    },
    previewMove(move) {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      const id = nextPreview++;
      return new Promise<MoveDestination[]>((resolve, reject) => {
        previews.set(id, { kind: "move", resolve, reject });
        channel.send({ type: "move_preview", id, side: options.side, move });
      });
    },
    previewBuilding(building) {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      const id = nextPreview++;
      return new Promise<BuildingPlacement>((resolve, reject) => {
        previews.set(id, { kind: "building", resolve, reject });
        channel.send({ type: "building_preview", id, side: options.side, building });
      });
    },
    onPublication(next) {
      consumer = next;
    },
    onStatus(listener) {
      statusListeners.push(listener);
    },
    pause() {
      paused = true;
      send({ type: "pause" });
    },
    resume() {
      paused = false;
      send({ type: "resume" });
    },
    advance(ticks) {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      const id = nextAdvance++;
      return new Promise<number>((resolve, reject) => {
        advances.set(id, { resolve, reject });
        channel.send({ type: "advance", id, ticks });
      });
    },
    observeAs(side) {
      if (failure || disposed) return;
      observedSide = side;
      ground?.invalidate();
      decoder?.invalidate(side);
      channel.send({ type: "side", side });
    },
    replay() {
      if (failure || disposed)
        return Promise.reject(failure ?? new Error("simulation client disposed"));
      return new Promise<string>((resolve, reject) => {
        replayWaiter = { resolve, reject };
        channel.send({ type: "replay" });
      });
    },
    dispose() {
      if (disposed) return;
      disposed = true;
      for (const pending of previews.values())
        pending.reject(new Error("simulation client disposed"));
      previews.clear();
      if (typeof document !== "undefined")
        document.removeEventListener("visibilitychange", onVisibility);
      channel.close();
      setStatus("disposed");
    },
  };
}
