// The menu's Skirmish page: the lobby, opened on the map its address asks for
// (`/skirmish?type=…`, read by `askedChoice`).
import { useLocation } from "react-router";
import { askedChoice } from "../battleLinks";
import { Lobby } from "../lobby/Lobby";

export function SkirmishPage() {
  // The lobby reads it once, when it opens.
  return <Lobby asked={askedChoice(useLocation().search)} />;
}
