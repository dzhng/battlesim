import { BATTLE_FAILED_NOTICE } from "./requests";
import { useEffect } from "react";
import { useLocation, useNavigate } from "react-router";
import type { OnlineBattleEntry } from "../onlineSession";
import { stateStream } from "./stateStream";

/** Map preparation and the first renderer frame can outlive a battle stream's
 * description. Keep presence through both: a room's state, or the shared list
 * for Frontline. Neither adds another presence clock or transport. */
export function useOnlineBattlePresence({ address, room }: OnlineBattleEntry) {
  const navigate = useNavigate();
  const location = useLocation();
  useEffect(() => {
    const visit = new AbortController();
    stateStream(
      address,
      room ? `/rooms/${encodeURIComponent(room)}` : "/rooms",
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
