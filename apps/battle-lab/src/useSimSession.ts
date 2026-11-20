import { useLabLoading } from "./LabLoading";
import type { BattleConnectionSession } from "@web/battle/prepare/client";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  createSimClient,
  type Publication,
  type SeatClient,
  type SimClient,
} from "@web/battle/sim/client";
import type { LearnedGround } from "@web/battle/sim/presentationGround";
import type { ObservationView, RefusalView } from "@web/battle/sim/observationView";
import type { AuthorityStatus, Resume, SideName } from "@web/battle/sim/protocol";

export interface SimSessionOptions {
  scenario: string;
  seed: number;
  prepared?: BattleConnectionSession;
  side?: SideName;
  /** Player issuing commands in a player-owned scenario. */
  player?: string;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView, digest: string) => void;
  /** Replay these accepted commands instead of taking input. */
  replay?: string;
  /** A scripted battle (the benchmark). */
  scripted?: ScriptedSim;
}

/** A battle nobody plays by hand: the scenario carries its sides' scripted
 *  orders, and the real simulation is stepped to `warmTo` before real time
 *  starts. */
export interface ScriptedSim {
  warmTo: number;
  /** The producer's exact publication layout, before warm-up begins. */
  onReady?: (info: { tickHz: number; tick: number; layout: string }) => void;
  /** Called when the battle first stands at `warmTo`; it holds there until
   *  the promise settles. Without it real time starts at once. */
  hold?: () => Promise<void>;
  /** Real time starts: the battle stands at `warmTo`. */
  onWarm: () => void;
  /** Every tick published once warm, with its cost. */
  onTick: (t: { tick: number; stepMs: number; bytes: number }) => void;
  /** Called while a publication still owns its producer credit, including preroll. */
  onPublication?: (p: { tick: number; digest: string; packed: number[]; warm: boolean }) => void;
}

const REFUSAL_LOG = 8;

/** A battle waiting at a chapter's start: where it resumes, and the chapter. */
interface Standby {
  client: SimClient;
  start: Resume;
  chapter: string;
}

interface SessionStartup {
  onViewportReady(): void;
  cancel(): void;
}

/** Own the one session's readiness and optional scripted warm-up. React keeps
 * observations and status; this operation keeps continuations tied to the
 * client they were created for. */
function startSessionStartup(
  client: SeatClient | SimClient,
  plan: ScriptedSim | undefined,
  onWarm: () => void,
  viewportInitiallyReady: boolean,
): SessionStartup {
  let cancelled = false;
  let authorityReady = false;
  let viewportReady = viewportInitiallyReady;
  const startWhenBothReady = () => {
    if (authorityReady && viewportReady && !cancelled) client.start();
  };
  if (plan && "pause" in client) {
    client.pause();
    void client
      .advance(plan.warmTo)
      .then(() => plan.hold?.())
      .then(() => {
        if (cancelled) return;
        client.resume();
        onWarm();
        plan.onWarm();
      });
  }
  void client.ready.then(
    (info) => {
      if (cancelled) return;
      authorityReady = true;
      plan?.onReady?.(info);
      startWhenBothReady();
    },
    () => {},
  );
  return {
    onViewportReady() {
      viewportReady = true;
      startWhenBothReady();
    },
    cancel() {
      cancelled = true;
    },
  };
}

/** One battle connection for the page. Publications are consumed (their
 *  credit returned) as soon as they are decoded, unless the lab holds credit
 *  to stall the producer; drawing interpolates between the last two frames.
 *  Reset disposes the client and starts again from the seed. */
export function useSimSession({
  scenario,
  seed,
  onDecoded,
  replay,
  scripted,
  prepared,
  player,
  side = "blue",
}: SimSessionOptions) {
  const [generation, setGeneration] = useState(0);
  const [client, setClient] = useState<SeatClient | SimClient | null>(null);
  /** Why the authority failed to start (for example a mismatched replay). */
  const [error, setError] = useState<string | null>(null);
  const [observation, setObservation] = useState<ObservationView | null>(null);
  /** The side's newest per-unit refusals, newest first: every publication's,
   *  though React renders only the latest observation. */
  const [refusals, setRefusals] = useState<readonly RefusalView[]>([]);
  const [status, setStatus] = useState<{ status: AuthorityStatus; slow: boolean }>({
    status: "loading",
    slow: false,
  });
  useLabLoading("world", !!observation, error);
  // The newest decoded frame, updated synchronously (React state lags a render).
  const latest = useRef<ObservationView | null>(null);
  // The side's learned ground, patched by every publication (one per client).
  const ground = useRef<LearnedGround | null>(null);
  // The latest publication's digest; diagnostics own any history they record.
  const digest = useRef<string | null>(null);
  const held = useRef<Publication[] | null>(null);
  const viewportReady = useRef(false);
  const onDecodedRef = useRef(onDecoded);
  onDecodedRef.current = onDecoded;
  // Fixed for the session's life, like the scenario it scripts.
  const scriptedRef = useRef(scripted);
  // The side a new battle opens as. A lab that switches sides mid-battle
  // observes as the other side (`observeAs`) in the same battle; a change of
  // side never starts a new one.
  const openSide = useRef(side);
  openSide.current = side;
  const startup = useRef<SessionStartup | null>(null);
  /** The client playing now: a replay asked of an older one comes too late. */
  const playing = useRef<SeatClient | SimClient | null>(null);
  /** A battle waiting live at the open chapter's start, which
   *  `restartChapter` swaps in for the one playing; `standbyAt` is its tick,
   *  once it stands ready. */
  const standby = useRef<Standby | null>(null);
  const [standbyAt, setStandbyAt] = useState<number | null>(null);
  const adopt = useRef<Standby | null>(null);
  const dropStandby = useCallback(() => {
    standby.current?.client.dispose();
    standby.current = null;
    setStandbyAt(null);
  }, []);
  /** Ready a standby at `start`, the opening of `chapter`: offered once it
   *  stands ready, and let go if the simulation refuses it. */
  const standFrom = (start: Resume, chapter: string) => {
    dropStandby();
    const waiting: Standby = {
      client: createSimClient({
        scenario,
        seed,
        side: openSide.current,
        player,
        transport: "worker",
        resume: start,
      }),
      start,
      chapter,
    };
    standby.current = waiting;
    waiting.client.ready.then(
      () => {
        if (standby.current === waiting) setStandbyAt(start.tick);
      },
      () => {
        if (standby.current === waiting) dropStandby();
      },
    );
  };
  const standFromRef = useRef(standFrom);
  standFromRef.current = standFrom;

  useEffect(() => {
    const plan = scriptedRef.current;
    // Restart is a new battle: its fresh worker prepares the same scenario
    // once, or a standby already waits at the chapter's start.
    const adopted = adopt.current;
    adopt.current = null;
    let next: SeatClient | SimClient;
    if (adopted) next = adopted.client;
    else if (prepared && "transport" in prepared)
      next = createSimClient({
        transport: "remote",
        side: openSide.current,
        connect: prepared.connect,
      });
    else
      next = createSimClient({
        scenario,
        seed,
        side: openSide.current,
        player,
        transport: "worker",
        replay,
        // A prepared bundle connection owns one live authority. Once the
        // session is replaced (for example by uploading a replay), start a
        // fresh worker rather than sending init through the closed connection.
        connect: generation === 0 && !replay ? prepared?.connect : undefined,
        // A battle started at a chapter starts there again on Restart.
        resume: replay ? undefined : prepared?.resume,
      });
    playing.current = next;
    // A local battle played by hand keeps its open chapter's start waiting,
    // read from every publication, so restarting the chapter is a swap. An
    // adopted standby is in its chapter already, and the next waits at the
    // same start.
    const chapters = !replay && !plan && "pause" in next ? next : null;
    let chapter = adopted?.chapter;
    if (adopted) standFromRef.current(adopted.start, adopted.chapter);
    let warm = !plan;
    const operation = startSessionStartup(
      next,
      plan,
      () => {
        warm = true;
      },
      viewportReady.current,
    );
    startup.current = operation;
    setClient(next);
    setObservation(null);
    setRefusals([]);
    digest.current = null;
    latest.current = null;
    ground.current = null;
    held.current = null;
    next.onStatus((s, slow) => setStatus({ status: s, slow }));
    // React sees the latest observation once a frame: ticks that arrive
    // between two frames are each decoded and interpolated, but rendering
    // them all would cost the page a render per tick.
    let frame: number | null = null;
    const render = () => {
      frame = null;
      setObservation(latest.current);
    };
    next.onPublication((publication) => {
      digest.current = publication.digest;
      latest.current = publication.observation;
      ground.current = publication.ground;
      const refused = publication.observation.refusals;
      if (refused.length) setRefusals((log) => [...refused, ...log].slice(0, REFUSAL_LOG));
      if (warm) {
        const { tick, stepMs, bytes } = publication;
        plan?.onTick({ tick, stepMs, bytes });
      }
      plan?.onPublication?.({
        tick: publication.tick,
        digest: publication.digest,
        packed: publication.copyPacked(),
        warm,
      });
      frame ??= requestAnimationFrame(render);
      const opened = publication.observation.scenario?.chapter;
      if (chapters && opened && opened !== chapter) {
        chapter = opened;
        // The last chapter's start is behind the player now.
        dropStandby();
        const { tick } = publication;
        // The recording holds every command up to this tick by the time it
        // answers; the standby replays it only that far.
        void chapters.replay().then(
          (recorded) => {
            if (playing.current !== chapters) return;
            standFromRef.current({ replay: recorded, tick }, opened);
          },
          () => {},
        );
      }
      onDecodedRef.current?.(publication.observation, publication.digest);
      if (held.current) held.current.push(publication);
      else publication.release();
    });
    setError(null);
    next.ready.catch((e: Error) => setError(e.message));
    return () => {
      operation.cancel();
      if (playing.current === next) playing.current = null;
      if (startup.current === operation) startup.current = null;
      if (frame !== null) cancelAnimationFrame(frame);
      next.dispose();
      // The next battle keeps no standby of this one (an adopted one moved on).
      dropStandby();
    };
  }, [scenario, seed, generation, replay, prepared, player, dropStandby]);

  const onViewportReady = useCallback(() => {
    viewportReady.current = true;
    startup.current?.onViewportReady();
  }, []);

  /** Stall test: keep publications instead of returning their credit. */
  const holdCredit = useCallback((hold: boolean) => {
    if (hold) held.current ??= [];
    else {
      for (const p of held.current ?? []) p.release();
      held.current = null;
    }
  }, []);

  const local = client && "pause" in client ? client : null;
  return {
    client,
    local,
    authorityProbes: local
      ? {
          pause: () => local.pause(),
          resume: () => local.resume(),
          advance: (n: number) => local.advance(n),
          reset: () => setGeneration((g) => g + 1),
        }
      : {},
    error,
    observation,
    refusals,
    status,
    latest,
    ground,
    digest,
    holdCredit,
    onViewportReady,
    restart: () => setGeneration((g) => g + 1),
    /** The tick a chapter restart would take the battle back to, once its
     *  standby stands ready; null when there is none. */
    standbyAt,
    /** Play on from the standby: the battle as it stood at `standbyAt`. */
    restartChapter: () => {
      adopt.current = standby.current;
      standby.current = null;
      setStandbyAt(null);
      if (adopt.current) setGeneration((g) => g + 1);
    },
  };
}
