/** Main-thread client of the one battle authority. Sends ordered commands,
 * resolves their acknowledgements, and hands each completed tick to the
 * consumer; the credit goes back only when the consumer releases it. */
import { createAuthority, type AuthorityHost } from "./authority";
import { loadSimModule } from "./module";
import type { ObservationView } from "./observationView";
import { ScenePresentation, loadPresentation } from "./presentation";
import type { LearnedGround } from "./presentationGround";
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
  side: SideName;
  tick: number;
  digest: string;
  observation: ObservationView;
  /** The side's learned ground, with this publication's patch applied (the
   *  presentation's). One for the client's life: it only ever moves forward. */
  ground: LearnedGround;
  /** Size of the packed frame on the wire. */
  bytes: number;
  /** Wall time the authority spent stepping this tick, ms. */
  stepMs: number;
  /** Copy the packed authority words while the credit is held, preserving bits. */
  copyPacked(): number[];
  /** Return the buffer to the producer. Until then the authority may stall. */
  release(): void;
}

export interface LocalSimClientOptions {
  scenario: string;
  seed: number;
  side: SideName;
  /** Player issuing commands in a player-owned scenario. */
  player?: string;
  /** "worker" in production; "direct" runs the same authority in-thread. */
  transport: "worker" | "direct";
  /** Adopt a worker that already holds this battle's prepared world. */
  connect?: SimConnection;
  /** Replay a recorded battle instead of accepting input. */
  replay?: string;
}

export type RemoteSimClientOptions = Omit<
  LocalSimClientOptions,
  "transport" | "connect" | "replay"
> & {
  transport: "remote";
  connect: SimConnection;
};
export type SimClientOptions = LocalSimClientOptions | RemoteSimClientOptions;
/** Online seats offer only actions the server permits. Local clients add
 * authority controls below; both use the same decoder and request tracking. */
export type OnlineSimClient = Pick<
  SimClient,
  | "ready"
  | "presentation"
  | "start"
  | "command"
  | "previewPurchase"
  | "previewMove"
  | "previewBuilding"
  | "onPublication"
  | "onStatus"
  | "dispose"
>;

export interface SimClient {
  readonly ready: Promise<{ tickHz: number; tick: number; layout: string }>;
  /** The side's presentation (`presentation.ts`), once the authority is ready. */
  readonly presentation: ScenePresentation | null;
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
  setControllers(controllers: string): void;
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

export function createSimClient(options: RemoteSimClientOptions): OnlineSimClient;
export function createSimClient(options: LocalSimClientOptions): SimClient;
export function createSimClient(options: SimClientOptions): OnlineSimClient | SimClient {
  const online = options.transport === "remote";
  let decoder: ScenePresentation | null = null;
  let observedSide = options.side;
  let slow = false;
  let paused = false;
  let nextCommand = 1;
  let nextAdvance = 1;
  let nextPreview = 1;
  type Lifetime =
    | { kind: "active" }
    | { kind: "failed"; error: Error }
    | { kind: "disposed"; error: Error | null };
  let lifetime: Lifetime = { kind: "active" };
  const heldBuffers = new Set<ArrayBuffer>();
  // Highest tick the consumer has released; an advance resolves once its target is covered.
  let releasedTick = -1;
  type Pending<T> = { resolve: (value: T) => void; reject: (error: Error) => void };
  const pendingAcks = new Map<number, Pending<CommandAck>>();
  const advances = new Map<number, Pending<number> & { target?: number }>();
  const previews = new Map<
    number,
    | (Pending<MoveDestination[]> & { kind: "move" })
    | (Pending<BuildingPlacement> & { kind: "building" })
    | (Pending<PurchasePlacement> & { kind: "purchase" })
  >();
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
  // The presentation module loads while the authority prepares its battle.
  loadPresentation().catch(() => {});

  const setStatus = (next: AuthorityStatus, nextSlow = slow) => {
    slow = nextSlow;
    for (const listener of statusListeners) listener(next, slow);
  };

  const returnCredit = (buffer: ArrayBuffer) => {
    if (!heldBuffers.delete(buffer)) return;
    channel.send({ type: "credit", buffer }, [buffer]);
  };

  const terminate = (next: Exclude<Lifetime, { kind: "active" }>) => {
    lifetime = next;
    if (next.kind === "failed") {
      for (const buffer of heldBuffers) returnCredit(buffer);
      if (online) channel.close();
      else channel.send({ type: "pause" });
      rejectReady(next.error);
      for (const pending of pendingAcks.values()) pending.reject(next.error);
      for (const pending of advances.values()) pending.reject(next.error);
    }
    for (const pending of previews.values())
      pending.reject(next.error ?? new Error("simulation client disposed"));
    if (next.kind === "failed") {
      replayWaiter?.reject(next.error);
      pendingAcks.clear();
      advances.clear();
      previews.clear();
      replayWaiter = null;
    } else {
      previews.clear();
      if (!online && typeof document !== "undefined")
        document.removeEventListener("visibilitychange", onVisibility);
      channel.close();
    }
    setStatus(next.kind);
  };

  const fail = (message: string) => {
    if (lifetime.kind === "active") terminate({ kind: "failed", error: new Error(message) });
  };

  const settleAdvances = () => {
    for (const [id, pending] of advances) {
      if (pending.target === undefined || releasedTick < pending.target) continue;
      pending.resolve(pending.target);
      advances.delete(id);
    }
  };

  const receive = (reply: SimReply) => {
    if (lifetime.kind === "disposed") return;
    if (reply.type === "publication") heldBuffers.add(reply.buffer);
    if (lifetime.kind === "failed") {
      if (reply.type === "publication") returnCredit(reply.buffer);
      return;
    }
    try {
      switch (reply.type) {
        case "ready": {
          // Publications follow `start`, which waits on `ready`.
          void loadPresentation()
            .then(() => {
              if (lifetime.kind !== "active") return;
              decoder = new ScenePresentation(reply.layout, reply.tickHz);
              decoder.invalidate(observedSide);
              resolveReady({ tickHz: reply.tickHz, tick: reply.tick, layout: reply.layout });
            })
            .catch((error: unknown) =>
              fail(error instanceof Error ? error.message : String(error)),
            );
          break;
        }
        case "status":
          setStatus(reply.status, reply.slow);
          break;
        case "request_refused": {
          const pending = reply.request === "command" ? pendingAcks : previews;
          pending.get(reply.id)?.reject(new Error(reply.reason));
          pending.delete(reply.id);
          break;
        }
        case "ack":
          pendingAcks.get(reply.id)?.resolve(reply.ack);
          pendingAcks.delete(reply.id);
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
          if (reply.side !== observedSide) {
            returnCredit(buffer);
            releasedTick = Math.max(releasedTick, reply.tick);
            settleAdvances();
            return;
          }
          if (!decoder?.ground)
            throw new Error("publication arrived before the authority was ready");
          const observation = decoder.decode(
            new Float32Array(buffer, 0, reply.length),
            performance.now(),
          );
          if (!observation) {
            returnCredit(buffer);
            releasedTick = Math.max(releasedTick, reply.tick);
            settleAdvances();
            return;
          }
          let released = false;
          const publication: Publication = {
            side: reply.side,
            tick: reply.tick,
            digest: reply.digest,
            observation,
            ground: decoder.ground,
            bytes: reply.length * Float32Array.BYTES_PER_ELEMENT,
            stepMs: reply.stepMs,
            copyPacked() {
              return Array.from(new Uint32Array(buffer, 0, reply.length));
            },
            release() {
              if (released || lifetime.kind === "disposed") return;
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
        case "advanced": {
          const pending = advances.get(reply.id);
          if (pending) pending.target = reply.tick;
          settleAdvances();
          break;
        }
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
    sides: [options.side],
    replay: options.transport === "remote" ? undefined : options.replay,
  });

  const send = (request: SimRequest) => {
    if (lifetime.kind === "active") channel.send(request);
  };
  const onVisibility = () => send({ type: "hidden", hidden: document.hidden });
  if (!online && typeof document !== "undefined")
    document.addEventListener("visibilitychange", onVisibility);

  const client: OnlineSimClient = {
    ready,
    get presentation() {
      return decoder;
    },
    start: () => send({ type: "start" }),
    command(order, queued = false) {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
      const id = nextCommand++;
      return new Promise<CommandAck>((resolve, reject) => {
        pendingAcks.set(id, { resolve, reject });
        channel.send({
          type: "command",
          id,
          order,
          queued,
          seat: { side: options.side, player: options.player },
        });
      });
    },
    previewPurchase(variant, destination) {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
      const id = nextPreview++;
      return new Promise<PurchasePlacement>((resolve, reject) => {
        previews.set(id, { kind: "purchase", resolve, reject });
        channel.send({ type: "purchase_preview", id, side: options.side, variant, destination });
      });
    },
    previewMove(move) {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
      const id = nextPreview++;
      return new Promise<MoveDestination[]>((resolve, reject) => {
        previews.set(id, { kind: "move", resolve, reject });
        channel.send({ type: "move_preview", id, side: options.side, move });
      });
    },
    previewBuilding(building) {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
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
    dispose() {
      if (lifetime.kind === "disposed") return;
      terminate({ kind: "disposed", error: lifetime.kind === "failed" ? lifetime.error : null });
    },
  };
  if (online) return client;
  const controls = {
    get paused() {
      return paused;
    },
    pause() {
      paused = true;
      send({ type: "pause" });
    },
    resume() {
      paused = false;
      send({ type: "resume" });
    },
    advance(ticks: number) {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
      const id = nextAdvance++;
      return new Promise<number>((resolve, reject) => {
        advances.set(id, { resolve, reject });
        channel.send({ type: "advance", id, ticks });
      });
    },
    setControllers(controllers: string) {
      if (lifetime.kind !== "active") return;
      channel.send({ type: "controllers", controllers });
    },
    observeAs(side: SideName) {
      if (lifetime.kind !== "active") return;
      observedSide = side;
      decoder?.invalidate(side);
      channel.send({ type: "sides", sides: [side] });
    },
    replay() {
      if (lifetime.kind !== "active")
        return Promise.reject(lifetime.error ?? new Error("simulation client disposed"));
      return new Promise<string>((resolve, reject) => {
        replayWaiter = { resolve, reject };
        channel.send({ type: "replay" });
      });
    },
  };
  // Preserve the shared client's live presentation getter and the local
  // pause getter; object spread would evaluate both at construction time.
  Object.defineProperties(client, Object.getOwnPropertyDescriptors(controls));
  return client as SimClient;
}
