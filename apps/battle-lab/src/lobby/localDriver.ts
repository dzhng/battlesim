import { useEffect, useRef, useState } from "react";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { catalogSet } from "@web/battle/catalog/sets";
import { mapPreviews, type MapPreviews, type PreviewState } from "@web/battle/prepare/preview";
import type { LanesModule } from "@web/battle/prepare/lanes";
import { loadWasm } from "@web/battle/sim/module";
import { newSeed } from "@web/maps/source";
import { toUrl, type AskedLobby } from "../battleLinks";
import type { LobbyDriver } from "./driver";
import type { SeatId } from "@packages/skirmish-setup/src/seats";
import {
  add,
  remove,
  seatHuman,
  isMapInputChange,
  preset,
  reroll,
  setFaction,
  setLighting,
  setRegion,
  setSize,
  setType,
  type LobbyState,
} from "@packages/skirmish-setup/src/lobby";
import { lobbyCandidate } from "../skirmishData";

function seatOfflineHuman(state: LobbyState, target: SeatId): LobbyState {
  const next = seatHuman(state, target);
  return {
    ...next,
    seats: next.seats.map((seat) =>
      seat.id === target
        ? seat
        : seat.occupant.kind === "human"
          ? { ...seat, occupant: { kind: "ai" } }
          : seat,
    ),
  };
}

/** A 1v1 on the asked map, at the asked seed or a fresh one. */
function opening(asked: AskedLobby): LobbyState {
  let state = preset("1v1");
  if (asked.faction) state = setFaction(state, "blue", asked.faction);
  if (asked.type) state = setType(state, asked.type);
  if (asked.size) state = setSize(state, asked.size);
  if (asked.lighting) state = setLighting(state, asked.lighting);
  return reroll(setRegion(state, asked.region), asked.seed ?? newSeed());
}

/** The lobby and its map preview. `update` takes the lobby's next state and
 *  starts a new preview at once when a map input changed, so the screen
 *  never shows an older map as the one Deploy would start. A preview that
 *  had to draw another seed hands that seed to the lobby without starting
 *  again. */
export function useLocalLobby(asked: AskedLobby): Extract<LobbyDriver, { kind: "local" }> {
  const [lobby, setLobby] = useState(() => opening(asked));
  const [preview, setPreview] = useState<PreviewState>({ status: "loading" });
  const current = useRef(lobby);
  /** The lobby state the running preview shows. */
  const shown = useRef<LobbyState | null>(null);
  const previews = useRef<{ show(state: LobbyState): void } | null>(null);
  const [wasm, setWasm] = useState<LanesModule | null>(null);

  const update = (next: LobbyState) => {
    current.current = next;
    setLobby(next);
    if (shown.current && !isMapInputChange(shown.current, next)) return;
    shown.current = null;
    previews.current?.show(next);
  };

  useEffect(() => {
    let live = true;
    let controller: MapPreviews | null = null;
    // Each preview tries the lobby's own seed first; a hidden retry draws a
    // fresh one.
    let firstSeed: string | null = null;
    const adopt = (state: PreviewState) => {
      setPreview(state);
      if (state.status !== "ready" || !shown.current) return;
      const seed = state.preview.request.map_source.request.seed;
      if (seed === shown.current.map.seed) return;
      const adopted = reroll(current.current, seed);
      shown.current = adopted;
      current.current = adopted;
      setLobby(adopted);
    };
    void Promise.all([loadWasm(), catalogSet("game")]).then(
      ([wasm, catalog]) => {
        if (!live) return;
        setWasm(wasm);
        controller = mapPreviews(
          {
            documents: { rules: JSON.stringify(catalog.rules), presets, templates },
            policy: config.admission,
            onChange: (state) => live && adopt(state),
          },
          {
            seed: () => {
              const seed = firstSeed ?? newSeed();
              firstSeed = null;
              return seed;
            },
          },
        );
        const running = controller;
        previews.current = {
          show(state) {
            shown.current = state;
            firstSeed = state.map.seed;
            running.show(lobbyCandidate(wasm, state));
          },
        };
        previews.current.show(current.current);
      },
      (error: unknown) =>
        live &&
        setPreview({
          status: "failed",
          message: error instanceof Error ? error.message : String(error),
          attempts: [],
        }),
    );
    return () => {
      live = false;
      controller?.dispose();
      previews.current = null;
      shown.current = null;
    };
  }, []);

  return {
    kind: "local",
    lobby,
    preview,
    wasm,
    deploy: preview.status === "ready" ? `/battle${toUrl(lobby)}` : null,
    preset: (kind) => update(preset(kind, lobby)),
    setType: (type) => update(setType(lobby, type)),
    setSize: (size) => update(setSize(lobby, size)),
    setRegion: (region) => update(setRegion(lobby, region)),
    setLighting: (lighting) => update(setLighting(lobby, lighting)),
    setFaction: (side, faction) => update(setFaction(lobby, side, faction)),
    reroll: () => update(reroll(lobby)),
    sit: (seat) => update(seatOfflineHuman(lobby, seat)),
    remove: (seat) => update(remove(lobby, seat)),
    add: (side) => update(add(lobby, side)),
  };
}
