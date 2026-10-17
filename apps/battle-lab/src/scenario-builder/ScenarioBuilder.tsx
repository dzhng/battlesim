import { useMemo, useRef, useState, type ChangeEvent, type PointerEvent } from "react";
import { useSessionCatalog } from "@web/battle/catalog/context";
import { loadWasm } from "@web/battle/sim/module";
import {
  downloadScenarioBundle,
  readScenarioBundle,
  validateScenarioBundle,
  type ScenarioBundleWasm,
} from "@web/battle/scenarioBundle";
import { APP_COMMIT } from "../buildIdentity";
import {
  cloneDraft,
  builderEconomy,
  BUILDER_FACTIONS,
  createBuilderDraft,
  nextPlayerId,
  parseBuilderDraft,
  removeBuilderUnit,
  unitName,
  type BuilderController,
  type BuilderUnit,
  type BuilderSkirmish,
  type ScenarioBundleDraft,
} from "./model";
import type { Player } from "@web/battle/prepare/protocol";
import "./builder.css";

const SIDES = ["blue", "red", "neutral"] as const;
const CONTROLLERS: BuilderController[] = ["human", "active_ai", "passive_ai"];

type Status = { kind: "idle" | "ok" | "error"; text: string };

function pointFromPointer(
  event: PointerEvent<SVGSVGElement>,
  size: readonly [number, number],
): [number, number] {
  const box = event.currentTarget.getBoundingClientRect();
  return [
    Math.max(0, Math.min(size[0], ((event.clientX - box.left) / box.width) * size[0])),
    Math.max(0, Math.min(size[1], ((event.clientY - box.top) / box.height) * size[1])),
  ];
}

function colorFor(side: BuilderUnit["side"]): string {
  return side === "blue" ? "blue" : side === "red" ? "red" : "neutral";
}

function updatePlayer(players: Player[], id: string, patch: Partial<Player>): Player[] {
  return players.map((player) => (player.id === id ? { ...player, ...patch } : player));
}

export default function ScenarioBuilder() {
  const { rules, units } = useSessionCatalog();
  const initial = useMemo(() => createBuilderDraft(rules, units), [rules, units]);
  const [draft, setDraft] = useState<ScenarioBundleDraft>(() => initial);
  const [selected, setSelected] = useState<number | null>(0);
  const [selectedPlayer, setSelectedPlayer] = useState("blue-1");
  const [paletteKind, setPaletteKind] = useState(() => units.ids[0] ?? "");
  const [status, setStatus] = useState<Status>({
    kind: "idle",
    text: "Draft lives in memory until you download it.",
  });
  const [dragging, setDragging] = useState<number | null>(null);
  const upload = useRef<HTMLInputElement>(null);
  const mapUpload = useRef<HTMLInputElement>(null);
  const assets = draft.assets;
  const mapSize = draft.scenario.map.size;
  const selectedUnit = draft.scenario.units.find((unit) => unit.id === selected) ?? null;
  const palette = useMemo(() => units.ids.slice(0, 24), [units.ids]);

  function changeUnit(id: number, patch: Partial<BuilderUnit>) {
    setDraft((current) => ({
      ...current,
      scenario: {
        ...current.scenario,
        units: current.scenario.units.map((unit) =>
          unit.id === id ? { ...unit, ...patch } : unit,
        ),
      },
    }));
  }

  async function changeMode(mode: ScenarioBundleDraft["mode"]) {
    if (mode !== "economy") {
      setDraft((current) => ({
        ...current,
        mode,
        scenario: { ...current.scenario, skirmish: undefined },
      }));
      return;
    }
    setStatus({ kind: "idle", text: "Loading the simulation's skirmish site owner…" });
    try {
      const wasm = await loadWasm();
      const sitesFor = (size: readonly [number, number], entriesPerSide: number) =>
        (
          JSON.parse(wasm.synthetic_sites(JSON.stringify(size), entriesPerSide)) as {
            entries: BuilderSkirmish["sites"]["entries"];
          }
        ).entries;
      setDraft((current) => ({
        ...current,
        mode,
        scenario: {
          ...current.scenario,
          units: [],
          skirmish: builderEconomy(current.scenario.map.size, current.scenario.players, sitesFor),
        },
      }));
      setStatus({
        kind: "ok",
        text: "Economy mode uses the skirmish wallet; authored units were cleared.",
      });
    } catch (error) {
      setStatus({ kind: "error", text: error instanceof Error ? error.message : String(error) });
    }
  }

  function place(point: [number, number]) {
    if (draft.mode === "economy") {
      setStatus({
        kind: "error",
        text: "Economy mode fields units through its wallet; switch to sandbox to place authored units.",
      });
      return;
    }
    if (!paletteKind) return;
    const nextId = draft.scenario.units.reduce((max, unit) => Math.max(max, unit.id), -1) + 1;
    const owner =
      draft.scenario.players.find((player) => player.id === selectedPlayer) ??
      draft.scenario.players[0];
    const side = owner?.side ?? selectedUnit?.side ?? "blue";
    const unit: BuilderUnit = {
      id: nextId,
      side,
      kind: paletteKind,
      position: point,
      yaw: side === "red" ? Math.PI : 0,
      owner: owner?.id ?? "blue-1",
    };
    setDraft((current) => ({
      ...current,
      scenario: {
        ...current.scenario,
        units: [...current.scenario.units, unit].sort((a, b) => a.id - b.id),
      },
    }));
    setSelected(nextId);
  }

  function onCanvasPointerDown(event: PointerEvent<SVGSVGElement>) {
    const target = event.target;
    if (target instanceof Element && target.closest(".builder-unit")) return;
    place(pointFromPointer(event, mapSize));
  }

  function onCanvasPointerMove(event: PointerEvent<SVGSVGElement>) {
    if (dragging === null) return;
    changeUnit(dragging, { position: pointFromPointer(event, mapSize) });
  }

  function onCanvasPointerUp() {
    setDragging(null);
  }

  function changeSide(side: BuilderUnit["side"]) {
    if (!selectedUnit) return;
    setDraft((current) => {
      const next = current.scenario.units.map((unit) =>
        unit.id === selectedUnit.id ? { ...unit, side, yaw: side === "red" ? Math.PI : 0 } : unit,
      );
      const moved = next.find((unit) => unit.id === selectedUnit.id)!;
      const replacement = current.scenario.players.find((player) => player.side === side);
      if (!replacement) return current;
      return {
        ...current,
        scenario: {
          ...current.scenario,
          units: next.map((unit) =>
            unit.id === moved.id ? { ...unit, owner: replacement.id } : unit,
          ),
        },
      };
    });
  }

  async function download() {
    setStatus({ kind: "idle", text: "Validating draft through the simulation contract…" });
    try {
      const wasm = (await loadWasm()) as unknown as ScenarioBundleWasm;
      if (!wasm.author_scenario_bundle)
        throw new Error("this build has no scenario authoring boundary yet");
      const next = cloneDraft(draft);
      next.runtime_build = APP_COMMIT;
      const authoring = { ...next, authored_scripts: next.authored_scripts } as Record<
        string,
        unknown
      >;
      const canonical = wasm.author_scenario_bundle(JSON.stringify(authoring));
      downloadScenarioBundle(canonical, "scenario.json");
      const identity = validateScenarioBundle(wasm, canonical);
      setStatus({
        kind: "ok",
        text: `Downloaded ${identity.sha256.slice(0, 12)}… · ${next.scenario.units.length} units`,
      });
    } catch (error) {
      setStatus({ kind: "error", text: error instanceof Error ? error.message : String(error) });
    }
  }

  async function importBundle(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    setStatus({ kind: "idle", text: `Reading ${file.name}…` });
    try {
      const text = await readScenarioBundle(file);
      const wasm = (await loadWasm()) as unknown as ScenarioBundleWasm;
      const identity = validateScenarioBundle(wasm, text);
      const loaded = parseBuilderDraft(text);
      setDraft(loaded);
      setSelected(loaded.scenario.units[0]?.id ?? null);
      setSelectedPlayer(
        loaded.scenario.players.find((player) => player.human_capable)?.id ??
          loaded.scenario.players[0]?.id ??
          "",
      );
      setStatus({ kind: "ok", text: `Loaded ${identity.sha256.slice(0, 12)}… in memory` });
    } catch (error) {
      setStatus({ kind: "error", text: error instanceof Error ? error.message : String(error) });
    }
  }

  async function addAsset(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      if (bytes.byteLength > 8 * 1024 * 1024) throw new Error("asset exceeds the 8 MiB limit");
      const digest = await crypto.subtle.digest("SHA-256", bytes);
      const sha256 = [...new Uint8Array(digest)]
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("");
      let binary = "";
      for (let i = 0; i < bytes.length; i += 0x8000)
        binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
      const asset = {
        id: file.name.replace(/[^a-zA-Z0-9._-]/g, "_"),
        media_type: file.type || "application/octet-stream",
        sha256,
        provenance: "user-upload",
        bytes_base64: btoa(binary),
      };
      setDraft((current) => ({
        ...current,
        assets: [...current.assets.filter((old) => old.id !== asset.id), asset].sort((a, b) =>
          a.id.localeCompare(b.id),
        ),
      }));
      setStatus({
        kind: "ok",
        text: `Attached ${file.name}; it travels inside the next download.`,
      });
    } catch (error) {
      setStatus({ kind: "error", text: error instanceof Error ? error.message : String(error) });
    }
  }

  async function importMap(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;
    try {
      const map = JSON.parse(await file.text()) as ScenarioBundleDraft["scenario"]["map"];
      if (!Array.isArray(map.size) || map.size.length !== 2)
        throw new Error("map snapshot has no finite size");
      setDraft((current) => ({
        ...current,
        map: {
          ...current.map,
          map_hash: "",
          template_catalog_hash: map.template_catalog_hash ?? null,
        },
        scenario: { ...current.scenario, map },
      }));
      setStatus({
        kind: "ok",
        text: `Loaded resolved map snapshot ${file.name}; download to validate it.`,
      });
    } catch (error) {
      setStatus({ kind: "error", text: error instanceof Error ? error.message : String(error) });
    }
  }

  return (
    <main className="scenario-builder" data-testid="scenario-builder">
      <section className="scenario-builder-canvas" aria-label="Tactical map canvas">
        <svg
          viewBox={`0 0 ${mapSize[0]} ${mapSize[1]}`}
          role="application"
          aria-label="Scenario tactical map"
          onPointerDown={onCanvasPointerDown}
          onPointerMove={onCanvasPointerMove}
          onPointerUp={onCanvasPointerUp}
        >
          <rect width={mapSize[0]} height={mapSize[1]} fill="#1b2b23" />
          {Array.from({ length: 9 }, (_, i) => (
            <line
              key={`x${i}`}
              className="map-grid"
              x1={((i + 1) * mapSize[0]) / 10}
              y1="0"
              x2={((i + 1) * mapSize[0]) / 10}
              y2={mapSize[1]}
            />
          ))}
          {Array.from({ length: 7 }, (_, i) => (
            <line
              key={`y${i}`}
              className="map-grid"
              x1="0"
              y1={((i + 1) * mapSize[1]) / 8}
              x2={mapSize[0]}
              y2={((i + 1) * mapSize[1]) / 8}
            />
          ))}
          <text className="map-label" x="16" y="26">
            resolved map snapshot · {mapSize[0]} × {mapSize[1]} m
          </text>
          <rect
            className="map-boundary"
            x="18"
            y="42"
            width={mapSize[0] - 36}
            height={mapSize[1] - 60}
            rx="8"
          />
          <rect className="map-lane" x="18" y="42" width={mapSize[0] - 36} height="96" rx="8" />
          <rect
            className="map-lane"
            x="18"
            y={mapSize[1] - 114}
            width={mapSize[0] - 36}
            height="96"
            rx="8"
          />
          <line className="map-scale" x1="24" y1={mapSize[1] - 24} x2="124" y2={mapSize[1] - 24} />
          <text className="map-scale-label" x="24" y={mapSize[1] - 30}>
            100 m
          </text>
          <text className="map-compass" x={mapSize[0] - 28} y="62">
            N
          </text>
          {draft.scenario.units.map((unit) => (
            <g
              key={unit.id}
              className="builder-unit"
              data-side={colorFor(unit.side)}
              data-selected={String(unit.id === selected)}
              transform={`translate(${unit.position[0]} ${unit.position[1]})`}
              onPointerDown={(event) => {
                event.stopPropagation();
                setSelected(unit.id);
                setDragging(unit.id);
              }}
            >
              <title>
                {unitName(units, unit.kind)} · {unit.side} · unit {unit.id}
              </title>
              <circle r="13" />
              <g transform={`rotate(${(unit.yaw * 180) / Math.PI})`}>
                <path className="unit-heading" d="M0 -11 L7 7 L0 4 L-7 7 Z" />
                <rect className="unit-body" x="-4" y="-4" width="8" height="9" rx="1" />
              </g>
              <text className="unit-id" y="1">
                {unit.id}
              </text>
              <text className="unit-label" y="25">
                {unitName(units, unit.kind)}
              </text>
            </g>
          ))}
        </svg>
      </section>
      <aside className="hud-panel scenario-builder-panel">
        <h1>Scenario builder</h1>
        <div className="scenario-builder-hint">
          Click the map to place · drag a marker to move · download is the durable save
        </div>
        <div className="scenario-builder-actions">
          <button type="button" onClick={download}>
            Download
          </button>
          <button type="button" onClick={() => upload.current?.click()}>
            Upload
          </button>
          <button type="button" onClick={() => document.getElementById("scenario-asset")?.click()}>
            Attach asset
          </button>
          <button type="button" onClick={() => mapUpload.current?.click()}>
            Import map snapshot
          </button>
        </div>
        <input
          ref={upload}
          hidden
          type="file"
          accept="application/json,.json"
          onChange={importBundle}
        />
        <input id="scenario-asset" hidden type="file" onChange={addAsset} />
        <input
          ref={mapUpload}
          hidden
          type="file"
          accept="application/json,.json"
          onChange={importMap}
        />
        <div className="scenario-builder-status" data-kind={status.kind} role="status">
          {status.text}
        </div>
        <h2>Unit palette</h2>
        <select
          value={paletteKind}
          onChange={(event) => setPaletteKind(event.target.value)}
          aria-label="Unit palette"
        >
          {palette.map((kind) => (
            <option key={kind} value={kind}>
              {unitName(units, kind)}
            </option>
          ))}
        </select>
        <h2>Players and controllers</h2>
        <div className="scenario-builder-actions">
          <button
            type="button"
            onClick={() =>
              setDraft((current) => {
                const side = "blue" as const;
                const id = nextPlayerId(current.scenario.players, side);
                const newPlayer: Player = {
                  id,
                  side,
                  faction: "us",
                  controller: "human",
                  human_capable: true,
                };
                return {
                  ...current,
                  scenario: {
                    ...current.scenario,
                    players: [...current.scenario.players, newPlayer].sort((a, b) =>
                      a.id.localeCompare(b.id),
                    ),
                  },
                };
              })
            }
          >
            Add human player
          </button>
          <button
            type="button"
            disabled={draft.scenario.players.length <= 1}
            onClick={() =>
              setDraft((current) => {
                const removablePlayer = current.scenario.players.find(
                  (player) =>
                    player.id === selectedPlayer &&
                    !current.scenario.units.some((unit) => unit.owner === player.id),
                );
                if (!removablePlayer) {
                  setStatus({ kind: "error", text: "Select an empty player before removing it." });
                  return current;
                }
                const players = current.scenario.players.filter(
                  (player) => player.id !== removablePlayer.id,
                );
                setSelectedPlayer(players[0]?.id ?? "");
                return { ...current, scenario: { ...current.scenario, players } };
              })
            }
          >
            Remove empty player
          </button>
        </div>
        <div className="scenario-builder-players">
          {draft.scenario.players.map((player) => (
            <div className="scenario-builder-player" data-side={player.side} key={player.id}>
              <i aria-hidden="true" />
              <label>
                <span>
                  {player.id} ·{" "}
                  {draft.scenario.units.filter((unit) => unit.owner === player.id).length} units
                  <button
                    type="button"
                    className="scenario-builder-place"
                    aria-label={`Place unit for ${player.id}`}
                    aria-pressed={selectedPlayer === player.id}
                    onClick={() => setSelectedPlayer(player.id)}
                  >
                    {selectedPlayer === player.id ? "placing" : "place"}
                  </button>
                </span>
                <select
                  aria-label={`${player.id} side`}
                  value={player.side}
                  onChange={(event) => {
                    const side = event.target.value as Player["side"];
                    setDraft((current) => {
                      const nextPlayer = {
                        side,
                        ...(side === "neutral" ? {} : { faction: player.faction ?? "us" }),
                        controller:
                          side === "neutral" ? ("passive_ai" as const) : player.controller,
                        human_capable: side === "neutral" ? false : player.human_capable,
                      };
                      return {
                        ...current,
                        scenario: {
                          ...current.scenario,
                          units: current.scenario.units.map((unit) =>
                            unit.owner === player.id
                              ? { ...unit, side, yaw: side === "red" ? Math.PI : 0 }
                              : unit,
                          ),
                          players: updatePlayer(current.scenario.players, player.id, nextPlayer),
                        },
                      };
                    });
                  }}
                >
                  {SIDES.map((side) => (
                    <option key={side} value={side}>
                      {side}
                    </option>
                  ))}
                </select>
                {player.side !== "neutral" && (
                  <select
                    aria-label={`${player.id} faction`}
                    value={player.faction ?? "us"}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        scenario: {
                          ...current.scenario,
                          players: updatePlayer(current.scenario.players, player.id, {
                            faction: event.target.value as Player["faction"],
                          }),
                        },
                      }))
                    }
                  >
                    {BUILDER_FACTIONS.map((faction) => (
                      <option key={faction} value={faction}>
                        {faction}
                      </option>
                    ))}
                  </select>
                )}
                <label className="scenario-builder-check">
                  <input
                    type="checkbox"
                    checked={player.human_capable}
                    disabled={player.side === "neutral"}
                    onChange={(event) =>
                      setDraft((current) => ({
                        ...current,
                        scenario: {
                          ...current.scenario,
                          players: updatePlayer(current.scenario.players, player.id, {
                            human_capable: event.target.checked,
                          }),
                        },
                      }))
                    }
                  />
                  human-capable
                </label>
                <select
                  aria-label={`${player.id} controller`}
                  value={player.controller}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      scenario: {
                        ...current.scenario,
                        players: updatePlayer(current.scenario.players, player.id, {
                          controller: event.target.value as BuilderController,
                        }),
                      },
                    }))
                  }
                >
                  {(player.side === "neutral"
                    ? (["passive_ai"] as BuilderController[])
                    : player.human_capable
                      ? CONTROLLERS
                      : (["active_ai", "passive_ai"] as BuilderController[])
                  ).map((controller) => (
                    <option key={controller} value={controller}>
                      {controller.replace("_", " ")}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          ))}
        </div>
        {selectedUnit && (
          <section className="scenario-builder-inspector" aria-label="Selected unit inspector">
            <h2>
              Unit {selectedUnit.id} · {unitName(units, selectedUnit.kind)}
            </h2>
            <label>
              Side
              <select
                value={selectedUnit.side}
                onChange={(event) => changeSide(event.target.value as BuilderUnit["side"])}
              >
                {SIDES.map((side) => (
                  <option key={side} value={side}>
                    {side}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Facing
              <input
                type="number"
                step="0.1"
                value={selectedUnit.yaw}
                onChange={(event) =>
                  changeUnit(selectedUnit.id, { yaw: Number(event.target.value) })
                }
              />
            </label>
            <label>
              X
              <input
                type="number"
                step="0.1"
                value={selectedUnit.position[0]}
                onChange={(event) =>
                  changeUnit(selectedUnit.id, {
                    position: [Number(event.target.value), selectedUnit.position[1]],
                  })
                }
              />
            </label>
            <label>
              Y
              <input
                type="number"
                step="0.1"
                value={selectedUnit.position[1]}
                onChange={(event) =>
                  changeUnit(selectedUnit.id, {
                    position: [selectedUnit.position[0], Number(event.target.value)],
                  })
                }
              />
            </label>
            <button
              type="button"
              onClick={() => {
                setDraft((current) => removeBuilderUnit(current, selectedUnit.id));
                setSelected(null);
              }}
            >
              Delete unit
            </button>
          </section>
        )}
        <h2>Mode and victory</h2>
        <label>
          Mode
          <select
            value={draft.mode}
            onChange={(event) => void changeMode(event.target.value as ScenarioBundleDraft["mode"])}
          >
            <option value="sandbox">sandbox</option>
            <option value="economy">economy</option>
          </select>
        </label>
        <label>
          Victory source
          <textarea
            value={draft.victory.source}
            onChange={(event) =>
              setDraft((current) => ({ ...current, victory: { source: event.target.value } }))
            }
          />
        </label>
        <label>
          Authored events JSON
          <textarea
            aria-label="Authored events JSON"
            value={JSON.stringify(draft.scenario.events, null, 2)}
            onChange={(event) => {
              try {
                const events = JSON.parse(event.target.value) as unknown;
                if (!Array.isArray(events)) throw new Error("events must be an array");
                setDraft((current) => ({
                  ...current,
                  scenario: { ...current.scenario, events },
                }));
                setStatus({ kind: "ok", text: "Authored events JSON is valid." });
              } catch (error) {
                setStatus({
                  kind: "error",
                  text: error instanceof Error ? error.message : String(error),
                });
              }
            }}
          />
        </label>
        <h2>Advanced scripts</h2>
        {draft.authored_scripts.map((script, index) => (
          <section className="scenario-builder-script" key={script.id}>
            <label>
              Script id
              <input
                value={script.id}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    authored_scripts: current.authored_scripts.map((item, i) =>
                      i === index ? { ...item, id: event.target.value } : item,
                    ),
                  }))
                }
              />
            </label>
            <label>
              Source
              <textarea
                value={script.source}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    authored_scripts: current.authored_scripts.map((item, i) =>
                      i === index ? { ...item, source: event.target.value } : item,
                    ),
                  }))
                }
              />
            </label>
            <label>
              Script side
              <select
                value={script.side}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    authored_scripts: current.authored_scripts.map((item, i) =>
                      i === index
                        ? { ...item, side: event.target.value as BuilderUnit["side"] }
                        : item,
                    ),
                  }))
                }
              >
                {SIDES.filter((side) => side !== "neutral").map((side) => (
                  <option key={side} value={side}>
                    {side}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Script player
              <select
                value={script.player ?? ""}
                onChange={(event) =>
                  setDraft((current) => ({
                    ...current,
                    authored_scripts: current.authored_scripts.map((item, i) =>
                      i === index ? { ...item, player: event.target.value || null } : item,
                    ),
                  }))
                }
              >
                <option value="">No command player (events/victory only)</option>
                {draft.scenario.players
                  .filter((player) => player.side === script.side)
                  .map((player) => (
                    <option key={player.id} value={player.id}>
                      {player.id}
                    </option>
                  ))}
              </select>
            </label>
            <button
              type="button"
              onClick={() =>
                setDraft((current) => ({
                  ...current,
                  authored_scripts: current.authored_scripts.filter((_, i) => i !== index),
                }))
              }
            >
              Remove script
            </button>
          </section>
        ))}
        <button
          type="button"
          onClick={() =>
            setDraft((current) => ({
              ...current,
              authored_scripts: [
                ...current.authored_scripts,
                {
                  id: `script-${current.authored_scripts.length + 1}`,
                  source: 'event("pulse");',
                  side: "blue",
                },
              ],
            }))
          }
        >
          Add authored script
        </button>
        <div className="scenario-builder-hint">
          {assets.length
            ? `${assets.length} embedded asset${assets.length === 1 ? "" : "s"}`
            : "No uploaded assets"}{" "}
          · no browser persistence
        </div>
      </aside>
    </main>
  );
}
