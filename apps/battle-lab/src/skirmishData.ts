import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { battleRequest, type LobbyState, type SetupData } from "@packages/skirmish-setup/src/lobby";
import type { MapGenerator } from "@web/maps/source";
import type { AskedBattle } from "./battleLinks";
import type { Wasm } from "@web/battle/sim/module";

export function setupData(generator: MapGenerator): SetupData {
  return { generator, presets, templates, limits: config.limits };
}

export function lobbyCandidate(generator: MapGenerator, lobby: LobbyState) {
  return (seed: string) =>
    battleRequest({ ...lobby, map: { ...lobby.map, seed } }, setupData(generator));
}

/** The battle route's request builder for a generated link. */
export function battleRequestForAsked(
  wasm: Wasm,
  match: Extract<AskedBattle, { kind: "generated" }>,
) {
  const factions = {
    blue: match.players.find((player) => player.side === "blue")?.faction ?? "us",
    red: match.players.find((player) => player.side === "red")?.faction ?? "eastern",
  } as LobbyState["factions"];
  const lobby: LobbyState = {
    seats: match.players.map((player) => ({
      id: player.id,
      side: player.side as "blue" | "red",
      occupant: player.controller === "human" ? { kind: "human", who: player.id } : { kind: "ai" },
    })),
    nextSeat: match.players.length + 1,
    factions,
    map: match.map,
    battleSeed: match.battleSeed,
    lighting: match.lighting,
  };
  return battleRequest(lobby, setupData(wasm));
}
