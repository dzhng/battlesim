import { onlineRequest } from "./requests";

export const FRONTLINE_IDLE_NOTICE =
  "Your command was idle, so an AI took over. Press Play to rejoin.";
export const FRONTLINE_FAILED_NOTICE =
  "The battle could not continue. Press Play to find another command.";

export function leaveFrontline(address: string, battle: string) {
  return onlineRequest(
    address,
    `/frontline/${encodeURIComponent(battle)}/leave`,
    {},
    new AbortController().signal,
  );
}
