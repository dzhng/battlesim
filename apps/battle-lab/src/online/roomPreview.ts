import { useEffect, useState } from "react";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { catalogSet } from "@web/battle/catalog/sets";
import { mapPreviews, type MapPreviews, type PreviewState } from "@web/battle/prepare/preview";
import type { LanesModule } from "@web/battle/prepare/lanes";
import { loadWasm } from "@web/battle/sim/module";
import { entryCountForPlayers } from "@packages/skirmish-setup/src/seats";
import { DEFAULT_BATTLE_SEED, type LobbySetup } from "@packages/skirmish-setup/src/lobby";
import type { RoomView } from "@packages/multiplayer/src/wire";
import { lobbyCandidate } from "../skirmishData";

/** A transient presentation input, never another editable room or a credential. */
export function roomLobby(view: RoomView): LobbySetup {
  return {
    seats: view.seats.map((seat) => ({
      ...seat,
      occupant: seat.occupant.kind === "human" ? { kind: "human", who: seat.id } : seat.occupant,
    })),
    factions: view.settings.factions,
    map: view.settings.map,
    lighting: view.settings.lighting,
    battleSeed: DEFAULT_BATTLE_SEED,
  };
}

/** Exact room seed through the existing preview path; no local seed adoption. */
export function useRoomPreview(lobby: LobbySetup) {
  const fighters = lobby.seats.filter((seat) => seat.occupant.kind !== "open");
  const bothSides = ["blue", "red"].every((side) => fighters.some((seat) => seat.side === side));
  const key = JSON.stringify([lobby.map, entryCountForPlayers(fighters), bothSides]);
  const [result, setResult] = useState<{
    key: string;
    preview: PreviewState;
    wasm: LanesModule | null;
  }>({ key: "", preview: { status: "loading" }, wasm: null });
  useEffect(() => {
    let live = true;
    let previews: MapPreviews | null = null;
    if (!bothSides) {
      setResult({
        key,
        wasm: null,
        preview: {
          status: "failed",
          message: "Each side needs a fighter to preview the map.",
          attempts: [],
        },
      });
      return;
    }
    void Promise.all([loadWasm(), catalogSet("game")])
      .then(([wasm, catalog]) => {
        if (!live) return;
        previews = mapPreviews(
          {
            documents: { rules: JSON.stringify(catalog.rules), presets, templates },
            policy: { ...config.admission, max_generated_attempts: 1 },
            onChange: (preview) => {
              if (live)
                setResult({
                  key,
                  wasm,
                  preview:
                    preview.status === "failed"
                      ? { ...preview, message: "This map could not be made. The host can reroll." }
                      : preview,
                });
            },
          },
          { seed: () => lobby.map.seed },
        );
        previews.show(lobbyCandidate(wasm, lobby));
      })
      .catch(() => {
        if (live)
          setResult({
            key,
            wasm: null,
            preview: { status: "failed", message: "The map preview is unavailable.", attempts: [] },
          });
      });
    return () => {
      live = false;
      previews?.dispose();
    };
  }, [key]);
  return result.key === key
    ? { preview: result.preview, wasm: result.wasm }
    : { preview: { status: "loading" } as PreviewState, wasm: null };
}
