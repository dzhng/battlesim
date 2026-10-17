// The lobby map picture, alone: real admitted maps for 1v1, 3v4 and 5v5 at
// every size, each through the lobby's preview controller (`mapPreviews`) and
// drawn by the one picture owner (`mapgen::inspect::skirmish_picture`), at the
// size the lobby shows it. A pairing below a size's floor shows the refusal
// the preview gets. `?cell=3v4-large` draws one; the scene
// (`web/scenes/map-picture.mjs`) shoots each.
import { useEffect, useMemo, useState } from "react";
import config from "@fixtures/generated-battle.json";
import presets from "@fixtures/map-presets.json?raw";
import templates from "@fixtures/prototype-building-templates.json?raw";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { mapPreviews, type PreviewState } from "@web/battle/prepare/preview";
import type { PrepareBattleRequest } from "@web/battle/prepare/protocol";
import { loadWasm, type Wasm } from "@web/battle/sim/module";
import { generationRequest, MAP_SIZES, type MapSize } from "@web/maps/source";
import { add, entriesPerSide, preset, type LobbyState } from "../lobby/model";

const PAIRINGS: Record<string, () => LobbyState> = {
  "1v1": () => preset("1v1"),
  "3v4": () => add(preset("3v3"), "red"),
  "5v5": () => preset("5v5"),
};
interface Cell {
  id: string;
  pairing: string;
  size: MapSize;
}
const CELLS: Cell[] = Object.keys(PAIRINGS).flatMap((pairing) =>
  MAP_SIZES.map((size) => ({ id: `${pairing}-${size}`, pairing, size })),
);
/** Previews preparing at once: each is a worker generating a whole map. */
const CONCURRENCY = 2;
/** The lab judges the picture, not preparation time: an XL map can take
 *  longer in a browser than the game's own deadline allows, so the lab waits
 *  longer and keeps the game's attempt count. */
const POLICY = { ...config.admission, generated_deadline_ms: 90_000 };
const SIZE_LETTER: Record<MapSize, string> = { small: "S", medium: "M", large: "L", xl: "XL" };

/** The preview's candidates for `cell`: the lobby's players on a mixed map of
 *  its size, with seeds counted from 1 so every visit draws the same maps. */
function candidates(wasm: Wasm, cell: Cell) {
  const lobby = PAIRINGS[cell.pairing]();
  const base = generationRequest(
    wasm,
    { ...lobby.map, size: cell.size, profile: "skirmish" },
    { presets, templates },
    config.limits,
  );
  const entries = entriesPerSide(lobby.players);
  let next = 0;
  return {
    seed: () => String(++next),
    candidate: (seed: string): PrepareBattleRequest => ({
      map_source: {
        kind: "generated",
        request: { ...base, seed, ...(entries > 1 && { entries_per_side: entries }) },
      },
      players: lobby.players,
      battle_seed: lobby.battleSeed,
    }),
  };
}

export default function MapPicture() {
  const { rules } = useSessionCatalog();
  const only = new URLSearchParams(location.search).get("cell");
  const cells = useMemo(() => CELLS.filter((cell) => !only || cell.id === only), [only]);
  const [states, setStates] = useState<Record<string, PreviewState>>({});

  useEffect(() => {
    let live = true;
    const documents = { rules: JSON.stringify(rules), presets, templates };
    const running = new Set<() => void>();
    void loadWasm().then(async (wasm) => {
      const queue = [...cells];
      const worker = async () => {
        for (let cell = queue.shift(); cell && live; cell = queue.shift()) {
          const { seed, candidate } = candidates(wasm, cell);
          const id = cell.id;
          await new Promise<void>((settled) => {
            const previews = mapPreviews(
              {
                documents,
                policy: POLICY,
                onChange: (state) => {
                  if (live) setStates((all) => ({ ...all, [id]: state }));
                  if (state.status === "ready" || state.status === "failed") settled();
                },
              },
              { seed },
            );
            running.add(previews.dispose);
            previews.show(candidate);
          });
        }
      };
      await Promise.all(Array.from({ length: CONCURRENCY }, worker));
    });
    return () => {
      live = false;
      for (const dispose of running) dispose();
    };
  }, [cells, rules]);

  const settled = cells.every((cell) => {
    const status = states[cell.id]?.status;
    return status === "ready" || status === "failed";
  });
  useEffect(() => {
    window.__lab = {
      ready: settled,
      error: null,
      frame: async () => {},
      route: {
        /** Each cell's preview status. */
        states: () =>
          Object.fromEntries(cells.map((cell) => [cell.id, states[cell.id]?.status ?? "idle"])),
      },
    };
  }, [cells, states, settled]);
  useEffect(
    () => () => {
      delete window.__lab;
    },
    [],
  );

  return (
    <main className="mp">
      <style>{CSS}</style>
      {cells.map((cell) => (
        <figure key={cell.id} className="mp-cell" data-cell={cell.id}>
          <PictureFrame cell={cell} state={states[cell.id] ?? { status: "loading" }} />
          <figcaption>
            {cell.pairing.replace("v", " vs ")} · {SIZE_LETTER[cell.size]}
          </figcaption>
        </figure>
      ))}
    </main>
  );
}

/** The map frame: the picture once it is ready, and until then what the
 *  preview is doing. */
function PictureFrame({ cell, state }: { cell: Cell; state: PreviewState }) {
  const lobby = PAIRINGS[cell.pairing]();
  const belowFloor = MAP_SIZES.indexOf(cell.size) < MAP_SIZES.indexOf(lobby.sizeFloor);
  return (
    <div className="mp-frame" data-status={state.status}>
      {state.status === "ready" ? (
        <div className="mp-picture" dangerouslySetInnerHTML={{ __html: state.preview.svg }} />
      ) : (
        <div className="mp-state" title={state.status === "failed" ? state.message : undefined}>
          {state.status === "failed" ? (
            <>
              <strong>No map</strong>
              <span>
                {belowFloor
                  ? `Too small for ${cell.pairing.replace("v", " vs ")}`
                  : "The map could not be generated"}
              </span>
            </>
          ) : (
            <strong>
              {state.status === "retrying" ? `Rerolling map · try ${state.attempt}` : "Generating map"}
            </strong>
          )}
        </div>
      )}
    </div>
  );
}

const CSS = `
.mp { display: flex; flex-wrap: wrap; gap: 24px; padding: 24px; box-sizing: border-box;
  height: 100%; overflow: auto; align-content: flex-start;
  background: #060b10 radial-gradient(120% 90% at 50% 40%, #0d1b26 0%, #05090d 70%);
  font-family: var(--hud-font); color: rgb(var(--hud-text)); }
.mp-cell { margin: 0; }
.mp-cell figcaption { margin-top: 6px; font-size: 12px; letter-spacing: 0.16em;
  text-transform: uppercase; color: rgb(var(--hud-dim)); }
.mp-frame { position: relative; width: 576px; height: 576px; overflow: hidden;
  background: #0b1612; }
.mp-picture, .mp-picture svg { width: 100%; height: 100%; display: block; }
.mp-state { position: absolute; inset: 0; display: flex; flex-direction: column;
  align-items: center; justify-content: center; gap: 8px; padding: 32px; text-align: center;
  font-size: 11px; letter-spacing: 0.12em; text-transform: uppercase; color: rgb(var(--hud-dim)); }
.mp-state strong { font-size: 13px; letter-spacing: 0.24em; text-transform: uppercase;
  color: rgb(var(--hud-text)); }
.mp-frame[data-status=loading] .mp-state strong,
.mp-frame[data-status=retrying] .mp-state strong { animation: mp-pulse 1.4s ease-in-out infinite; }
@keyframes mp-pulse { 50% { opacity: 0.45; } }
`;
