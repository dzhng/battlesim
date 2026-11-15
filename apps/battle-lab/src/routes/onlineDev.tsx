import { useEffect, useState } from "react";
import { useNavigate } from "react-router";
import type { BattleTicket } from "@packages/multiplayer/src/wire";
import { browserId, devServerAddress, type OnlineBattleEntry } from "../onlineSession";
import { LoadingScreen } from "../LoadingScreen";
import { usePageVisitActive } from "../navigation";

/** Temporary local entry; the server owns which of its two seats is free. */
export default function OnlineDev() {
  const navigate = useNavigate();
  const active = usePageVisitActive();
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    const lifetime = new AbortController();
    const address = devServerAddress(window.location.search);
    void fetch(`${address}/dev/battle`, {
      method: "POST",
      headers: { "X-Player": browserId() },
      signal: lifetime.signal,
    })
      .then(async (response) => {
        const result: BattleTicket | { refused: string } = await response.json();
        if ("refused" in result) throw new Error(`Dev battle refused: ${result.refused}`);
        if (lifetime.signal.aborted || !active()) return;
        const state: OnlineBattleEntry = { ticket: result, address };
        // Preserve only harness quality selection; credentials never enter an address.
        const quality = new URLSearchParams(window.location.search).get("quality");
        navigate(
          `/online/battle/${encodeURIComponent(result.battle)}${quality ? `?quality=${quality}` : ""}`,
          { replace: true, state },
        );
      })
      .catch((reason: Error) => {
        if (!lifetime.signal.aborted && active()) setError(reason.message);
      });
    return () => lifetime.abort();
  }, [navigate, active]);
  return (
    <LoadingScreen
      title="Joining battle"
      stages={[]}
      current=""
      failure={
        error
          ? {
              message: "The dev battle could not be joined.",
              details: [error],
            }
          : null
      }
    />
  );
}
