import { useLabLoading } from "./LabLoading";
import type { PreparedSession } from "@web/battle/prepare/client";
import { useCallback, useEffect, useRef, useState } from "react";
import { createSimClient, type Publication, type SimClient } from "@web/battle/sim/client";
import type { GroundView } from "@web/battle/sim/ground";
import type { ObservationView } from "@web/battle/sim/observation";
import type { AuthorityStatus } from "@web/battle/sim/protocol";
import { TickInterpolator } from "@web/battle/present/interpolate";

export interface SimSessionOptions {
  scenario: string;
  seed: number;
  prepared?: PreparedSession;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView, digest: string) => void;
  /** Replay these accepted commands instead of taking input. */
  replay?: string;
  /** A scripted battle (the benchmark). */
  scripted?: ScriptedSim;
}

/** A battle blue does not play by hand: a comparison script commands it, and
 *  the real simulation is stepped to `warmTo` before real time starts. */
export interface ScriptedSim {
  /** Blue's comparison script (`village_report`'s name); absent when the
   * scenario already carries its two sides' scripted orders. */
  script?: string;
  warmTo: number;
  /** Real time starts: the battle stands at `warmTo`. */
  onWarm: () => void;
  /** Every tick published once warm, with its cost. */
  onTick: (t: { tick: number; stepMs: number; bytes: number }) => void;
}

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
}: SimSessionOptions) {
  const [generation, setGeneration] = useState(0);
  const [client, setClient] = useState<SimClient | null>(null);
  /** Why the authority failed to start (for example a mismatched replay). */
  const [error, setError] = useState<string | null>(null);
  const [observation, setObservation] = useState<ObservationView | null>(null);
  const [status, setStatus] = useState<{ status: AuthorityStatus; slow: boolean }>({
    status: "loading",
    slow: false,
  });
  useLabLoading("world", !!observation, error);
  const interpolator = useRef<TickInterpolator | null>(null);
  // The newest decoded frame, updated synchronously (React state lags a render).
  const latest = useRef<ObservationView | null>(null);
  // Bytes of the newest published frame, for frame-cost telemetry.
  const lastBytes = useRef(0);
  // The side's learned ground, patched by every publication (one per client).
  const ground = useRef<GroundView | null>(null);
  // The latest publication's digest; diagnostics own any history they record.
  const digest = useRef<string | null>(null);
  const held = useRef<Publication[] | null>(null);
  const viewportReady = useRef(false);
  const onDecodedRef = useRef(onDecoded);
  onDecodedRef.current = onDecoded;
  // Fixed for the session's life, like the scenario it scripts.
  const scriptedRef = useRef(scripted);

  useEffect(() => {
    const plan = scriptedRef.current;
    // Restart is a new battle: its fresh worker prepares the same scenario once.
    const next = createSimClient({
      scenario,
      seed,
      side: "blue",
      transport: "worker",
      replay,
      script: plan?.script,
      connect: generation === 0 ? prepared?.connect : undefined,
    });
    let warm = !plan;
    if (plan) {
      // Queued ahead of `start`: the battle steps to warmTo as fast as its
      // publications are consumed, then runs in real time.
      next.pause();
      void next.advance(plan.warmTo).then(() => {
        warm = true;
        next.resume();
        plan.onWarm();
      });
    }
    setClient(next);
    setObservation(null);
    digest.current = null;
    latest.current = null;
    ground.current = null;
    held.current = null;
    next.onStatus((s, slow) => setStatus({ status: s, slow }));
    next.onPublication((publication) => {
      digest.current = publication.digest;
      interpolator.current?.push(publication.observation, performance.now());
      latest.current = publication.observation;
      ground.current = publication.ground;
      lastBytes.current = publication.bytes;
      if (warm) {
        const { tick, stepMs, bytes } = publication;
        plan?.onTick({ tick, stepMs, bytes });
      }
      setObservation(publication.observation);
      onDecodedRef.current?.(publication.observation, publication.digest);
      if (held.current) held.current.push(publication);
      else publication.release();
    });
    setError(null);
    next.ready.catch((e: Error) => setError(e.message));
    void next.ready.then(
      ({ tickHz }) => {
        interpolator.current = new TickInterpolator(1000 / tickHz);
        if (viewportReady.current) next.start();
      },
      () => {}, // reported through `error`
    );
    return () => next.dispose();
  }, [scenario, seed, generation, replay, prepared]);

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
    status,
    interpolator,
    latest,
    ground,
    lastBytes,
    digest,
    holdCredit,
    onViewportReady,
    restart: () => setGeneration((g) => g + 1),
  };
}
