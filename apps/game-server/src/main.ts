import { childBattleMachines } from "./machines/child.ts";
import { startLobby } from "./lobby.ts";

startLobby(
  childBattleMachines(),
  new URL(import.meta.url),
  process.env.HOST ?? "127.0.0.1",
  Number(process.env.PORT ?? 8787),
  // Local clients connect directly; no proxy states their address.
  null,
);
