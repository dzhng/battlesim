import { BATTLE_FAILED_NOTICE } from "./roomRequests";
import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router";
import type { OnlineBattleEntry } from "../onlineSession";
import { stateStream } from "./stateStream";

/** Map verification precedes the battle stream. Keep the member present
 * through that work, including a page whose local preparation is slow. */
export function useRoomBattlePresence({ address, room }: OnlineBattleEntry) {
  const navigate = useNavigate();
  const location = useLocation();
  useEffect(() => {
    const visit = new AbortController();
    stateStream(
      address,
      `/rooms/${encodeURIComponent(room)}`,
      visit.signal,
      () => {},
      (reason) => {
        // A finished room can close before the battle's final publication
        // leaves playout. That observation, not this stream, owns the result.
        if (reason === "battle_failed")
          navigate(`/multiplayer${location.search}`, {
            replace: true,
            state: { roomNotice: BATTLE_FAILED_NOTICE },
          });
      },
      () => {},
    );
    return () => visit.abort();
  }, [address, room, navigate, location.search]);
}
