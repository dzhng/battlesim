import { useLocation, useNavigate } from "react-router";
import type { SkirmishView } from "@web/battle/sim/observationView";
import type { Seat } from "@web/battle/present/playerName";
import { MatchResultOverlay } from "../MatchResultOverlay";
import "./result.css";

/** The last observation remains on screen after the server deletes the battle. */
export function OnlineMatchResult({
  match,
  seats,
  viewer,
  callsigns,
}: {
  match: SkirmishView | null;
  seats: readonly Seat[];
  viewer: string;
  callsigns: Record<string, string>;
}) {
  const navigate = useNavigate();
  const location = useLocation();
  if (!match) return null;
  return (
    <MatchResultOverlay
      match={match}
      seats={seats}
      viewer={viewer}
      online={{
        callsigns,
        onReturn: () => navigate(`/multiplayer${location.search}`, { replace: true }),
      }}
    />
  );
}
