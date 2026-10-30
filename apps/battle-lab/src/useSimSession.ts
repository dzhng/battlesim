import { useLabLoading } from "./LabLoading";
import type { PreparedConnectionSession } from "@web/battle/prepare/client";
import { useCallback, useEffect, useRef, useState } from "react";
import { createSimClient, type Publication, type SimClient } from "@web/battle/sim/client";
import type { LearnedGround } from "@web/battle/sim/presentationGround";
import type { ObservationView, RefusalView } from "@web/battle/sim/observationView";
import type { AuthorityStatus } from "@web/battle/sim/protocol";
import type { SideName } from "@web/battle/sim/protocol";

export interface SimSessionOptions {
  scenario: string;
  seed: number;
  prepared?: PreparedConnectionSession;
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

/** One worker authority for a lab scenario. Publications are consumed (their
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
  const [client, setClient] = useState<SimClient | null>(null);
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

  useEffect(() => {
    const plan = scriptedRef.current;
    // Restart is a new battle: its fresh worker prepares the same scenario once.
    const next = createSimClient({
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
    });
    let warm = !plan;
    if (plan) {
      // Queued ahead of `start`: the battle steps to warmTo as fast as its
      // publications are consumed, then runs in real time.
      next.pause();
      void next
        .advance(plan.warmTo)
        .then(() => plan.hold?.())
        .then(() => {
          warm = true;
          next.resume();
          plan.onWarm();
        });
    }
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
      onDecodedRef.current?.(publication.observation, publication.digest);
      if (held.current) held.current.push(publication);
      else publication.release();
    });
    setError(null);
    next.ready.catch((e: Error) => setError(e.message));
    void next.ready.then(
      (info) => {
        plan?.onReady?.(info);
        if (viewportReady.current) next.start();
      },
      () => {}, // reported through `error`
    );
    return () => {
      if (frame !== null) cancelAnimationFrame(frame);
      next.dispose();
    };
  }, [scenario, seed, generation, replay, prepared, player]);

  const onViewportReady = useCallback(() => {
    viewportReady.current = true;
    if (client)
      void client.ready.then(
        () => client.start(),
        () => {}, // reported through `error`
      );
  }, [client]);

  /** Stall test: keep publications instead of returning their credit. */
  const holdCredit = useCallback((hold: boolean) => {
    if (hold) held.current ??= [];
    else {
      for (const p of held.current ?? []) p.release();
      held.current = null;
    }
  }, []);

  return {
    client,
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
  };
}
