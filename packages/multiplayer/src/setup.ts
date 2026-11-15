import { setOccupant, type LobbyState } from "../../skirmish-setup/src/lobby";
import { battlePlayers } from "../../skirmish-setup/src/seats";
import type { BattleSetup } from "./wire";

/** Converts the shared lobby model into the setup consumed by both clients. */
export function toBattleSetup(lobby: LobbyState, rules: BattleSetup["rules"]): BattleSetup {
  const { players } = battlePlayers(lobby.seats);
  return {
    seats: players
      .filter(
        (player): player is typeof player & { side: "blue" | "red" } => player.side !== "neutral",
      )
      .map((player) => ({
        player: player.id,
        side: player.side,
        controller: player.controller,
      })),
    factions: lobby.factions,
    map: { ...lobby.map, profile: "skirmish" },
    battleSeed: lobby.battleSeed,
    lighting: lobby.lighting,
    rules,
  };
}

/** Reconstructs a lobby accepted by the shared battle request builder. */
export function fromBattleSetup(setup: BattleSetup): LobbyState {
  const seats = setup.seats.map((seat, index) => ({
    id: `seat-${index + 1}`,
    side: seat.side,
    occupant:
      seat.controller === "human"
        ? ({ kind: "human", who: `battle-player:${seat.player}` } as const)
        : ({ kind: "ai" } as const),
  }));
  return {
    seats,
    nextSeat: seats.length + 1,
    factions: setup.factions,
    map: { ...setup.map, profile: "skirmish" },
    battleSeed: setup.battleSeed,
    lighting: setup.lighting,
  };
}

/** Makes every seat AI, preserving the lobby's size floor and map settings. */
export function allAi(lobby: LobbyState): LobbyState {
  return lobby.seats.reduce((state, seat) => setOccupant(state, seat.id, { kind: "ai" }), lobby);
}
