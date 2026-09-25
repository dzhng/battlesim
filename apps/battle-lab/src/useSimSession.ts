import { useCallback, useEffect, useRef, useState } from "react";
import { createSimClient, type Publication, type SimClient } from "@web/battle/sim/client";
import type { ObservationView } from "@web/battle/sim/observation";
import type { AuthorityStatus } from "@web/battle/sim/protocol";
import { TickInterpolator } from "@web/battle/present/interpolate";

export interface SimSessionOptions {
  scenario: string;
  seed: number;
  /** Called for every decoded frame, before its credit returns. */
  onDecoded?: (o: ObservationView) => void;
}

/** One worker authority for a lab scenario. Publications are consumed (their
 *  credit returned) as soon as they are decoded, unless the lab holds credit
 *  to stall the producer; drawing interpolates between the last two frames.
 *  Reset disposes the client and starts again from the seed. */
export function useSimSession({ scenario, seed, onDecoded }: SimSessionOptions) {
  const [generation, setGeneration] = useState(0);
  const [client, setClient] = useState<SimClient | null>(null);
  const [observation, setObservation] = useState<ObservationView | null>(null);
  const [status, setStatus] = useState<{ status: AuthorityStatus; slow: boolean }>({
    status: "loading",
    slow: false,
  });
  const interpolator = useRef<TickInterpolator | null>(null);
  // The newest decoded frame, updated synchronously (React state lags a render).
  const latest = useRef<ObservationView | null>(null);
  // Every published tick's state digest, for replay and parity checks.
  const digests = useRef(new Map<number, string>());
  const held = useRef<Publication[] | null>(null);
  const viewportReady = useRef(false);
  const onDecodedRef = useRef(onDecoded);
  onDecodedRef.current = onDecoded;

  useEffect(() => {
    const next = createSimClient({ scenario, seed, side: "blue", transport: "worker" });
    setClient(next);
    setObservation(null);
    digests.current = new Map();
    held.current = null;
    next.onStatus((s, slow) => setStatus({ status: s, slow }));
    next.onPublication((publication) => {
      digests.current.set(publication.tick, publication.digest);
      interpolator.current?.push(publication.observation, performance.now());
      latest.current = publication.observation;
      setObservation(publication.observation);
      onDecodedRef.current?.(publication.observation);
      if (held.current) held.current.push(publication);
      else publication.release();
    });
    void next.ready.then(({ tickHz }) => {
      interpolator.current = new TickInterpolator(1000 / tickHz);
      if (viewportReady.current) next.start();
    });
    return () => next.dispose();
  }, [scenario, seed, generation]);

  const onViewportReady = useCallback(() => {
    viewportReady.current = true;
    if (client) void client.ready.then(() => client.start());
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
    observation,
    status,
    interpolator,
    latest,
    digests,
    holdCredit,
    onViewportReady,
    reset: () => setGeneration((g) => g + 1),
  };
}
