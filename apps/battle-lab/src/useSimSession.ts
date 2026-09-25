import { useCallback, useEffect, useRef, useState } from "react";
import { createSimClient, type SimClient } from "@web/battle/sim/client";
import type { ObservationView } from "@web/battle/sim/observation";
import type { AuthorityStatus } from "@web/battle/sim/protocol";
import { TickInterpolator } from "@web/battle/present/interpolate";

/** One worker authority for a lab scenario. Publications are consumed (and
 *  their credit returned) as soon as they are decoded; drawing interpolates
 *  between the last two. Reset disposes the client and starts from the seed. */
export function useSimSession(
  scenario: string,
  seed: number,
  onDecoded?: (o: ObservationView) => void,
) {
  const [generation, setGeneration] = useState(0);
  const [client, setClient] = useState<SimClient | null>(null);
  const [observation, setObservation] = useState<ObservationView | null>(null);
  const [status, setStatus] = useState<AuthorityStatus>("loading");
  const interpolator = useRef<TickInterpolator | null>(null);
  const viewportReady = useRef(false);
  const onDecodedRef = useRef(onDecoded);
  onDecodedRef.current = onDecoded;

  useEffect(() => {
    const next = createSimClient({ scenario, seed, side: "blue", transport: "worker" });
    setClient(next);
    setObservation(null);
    next.onStatus((s) => setStatus(s));
    next.onPublication((publication) => {
      interpolator.current?.push(publication.observation, performance.now());
      setObservation(publication.observation);
      onDecodedRef.current?.(publication.observation);
      publication.release();
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

  return {
    client,
    observation,
    status,
    interpolator,
    onViewportReady,
    reset: () => setGeneration((g) => g + 1),
  };
}
