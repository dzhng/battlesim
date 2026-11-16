import type { PreviewState } from "@web/battle/prepare/preview";
import type { LanesModule } from "@web/battle/prepare/lanes";
import type { LobbySetup, LobbyPreset, LobbySide } from "@packages/skirmish-setup/src/lobby";
import type { SeatId } from "@packages/skirmish-setup/src/seats";
import type { RoomView } from "@packages/multiplayer/src/wire";

/** The view receives data and callbacks, never a mutation or transport primitive. */
export type LobbyDriver = {
  lobby: LobbySetup;
  preview: PreviewState;
  wasm: LanesModule | null;
  setType(type: LobbySetup["map"]["type"]): void;
  setSize(size: LobbySetup["map"]["size"]): void;
  setRegion(region: string | undefined): void;
  setLighting(lighting: string): void;
  setFaction(side: LobbySide, faction: LobbySetup["factions"][LobbySide]): void;
  reroll(): void;
  sit(seat: SeatId): void;
  remove(seat: SeatId): void;
  add(side: LobbySide): void;
} & (
  | { kind: "local"; deploy: string | null; preset(kind: LobbyPreset): void }
  | {
      kind: "room";
      view: RoomView;
      previewStatus: string;
      editable: boolean;
      setSeat(seat: SeatId, to: "open" | "ai"): void;
      ready(ready: boolean): void;
      start(): void;
    }
);
