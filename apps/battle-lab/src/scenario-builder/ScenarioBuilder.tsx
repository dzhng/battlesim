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
  createBuilderDraft,
  parseBuilderDraft,
  unitName,
  type BuilderController,
  type BuilderSlot,
  type BuilderUnit,
  type ScenarioBundleDraft,
} from "./model";
import "./builder.css";

const SIDES = ["blue", "red", "neutral"] as const;
const CONTROLLERS: BuilderController[] = ["human", "active_ai", "passive_ai"];
const FACTION_BY_SIDE = { blue: "us", red: "eastern" } as const;

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

function assignUnit(
  slots: BuilderSlot[],
  unit: BuilderUnit,
  oldSide: BuilderUnit["side"],
): BuilderSlot[] {
  return slots.map((slot) => {
    const units = slot.units.filter((id) => id !== unit.id);
    if (slot.side === unit.side && !units.includes(unit.id)) units.push(unit.id);
    if (slot.side === oldSide && slot.side !== unit.side) return { ...slot, units };
    return { ...slot, units: units.sort((a, b) => a - b) };
  });
}

function updateSlot(slots: BuilderSlot[], id: string, patch: Partial<BuilderSlot>): BuilderSlot[] {
  return slots.map((slot) => (slot.id === id ? { ...slot, ...patch } : slot));
}

export default function ScenarioBuilder() {
  const { rules, units } = useSessionCatalog();
  const initial = useMemo(() => createBuilderDraft(rules, units), [rules, units]);
  const [draft, setDraft] = useState<ScenarioBundleDraft>(() => initial);
  const [selected, setSelected] = useState<number | null>(0);
  const [paletteKind, setPaletteKind] = useState(() => units.ids[0] ?? "");
  const [status, setStatus] = useState<Status>({
    kind: "idle",
    text: "Draft lives in memory until you download it.",
  });
  const [dragging, setDragging] = useState<number | null>(null);
  const upload = useRef<HTMLInputElement>(null);
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

  function place(point: [number, number]) {
    if (!paletteKind) return;
    const nextId = draft.scenario.units.reduce((max, unit) => Math.max(max, unit.id), -1) + 1;
    const side = selectedUnit?.side ?? "blue";
    const unit: BuilderUnit = {
      id: nextId,
      side,
      kind: paletteKind,
      position: point,
      yaw: side === "red" ? Math.PI : 0,
    };
    setDraft((current) => ({
      ...current,
      scenario: {
        ...current.scenario,
        units: [...current.scenario.units, unit].sort((a, b) => a.id - b.id),
      },
      slots: assignUnit(current.slots, unit, side),
    }));
    setSelected(nextId);
  }

  function onCanvasPointerDown(event: PointerEvent<SVGSVGElement>) {
    if (event.target === event.currentTarget) place(pointFromPointer(event, mapSize));
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
      return {
        ...current,
        scenario: { ...current.scenario, units: next },
        slots: assignUnit(current.slots, moved, selectedUnit.side),
      };
    });
  }

  async function download() {
    setStatus({ kind: "idle", text: "Validating draft through the simulation contract…" });
    try {
      const wasm = (await loadWasm()) as unknown as ScenarioBundleWasm & {
        author_scenario_bundle?: (json: string) => string;
      };
      if (!wasm.author_scenario_bundle)
        throw new Error("this build has no scenario authoring boundary yet");
      const next = cloneDraft(draft);
      next.runtime_build = APP_COMMIT;
      const canonical = wasm.author_scenario_bundle(JSON.stringify(next));
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
          <path
            className="map-road"
            d={`M 0 ${mapSize[1] * 0.52} C ${mapSize[0] * 0.25} ${mapSize[1] * 0.42}, ${mapSize[0] * 0.7} ${mapSize[1] * 0.62}, ${mapSize[0]} ${mapSize[1] * 0.5}`}
          />
          <path
            className="map-river"
            d={`M ${mapSize[0] * 0.48} 0 C ${mapSize[0] * 0.43} ${mapSize[1] * 0.28}, ${mapSize[0] * 0.55} ${mapSize[1] * 0.7}, ${mapSize[0] * 0.49} ${mapSize[1]} L ${mapSize[0] * 0.57} ${mapSize[1]} C ${mapSize[0] * 0.62} ${mapSize[1] * 0.7}, ${mapSize[0] * 0.5} ${mapSize[1] * 0.3}, ${mapSize[0] * 0.56} 0 Z`}
          />
          <text className="map-label" x="16" y="26">
            resolved map snapshot · {mapSize[0]} × {mapSize[1]} m
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
              <circle r="13" />
              <text y="1">{unit.id}</text>
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
        </div>
        <input
          ref={upload}
          hidden
          type="file"
          accept="application/json,.json"
          onChange={importBundle}
        />
        <input id="scenario-asset" hidden type="file" onChange={addAsset} />
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
        <h2>Slots and controllers</h2>
        <div className="scenario-builder-slots">
          {draft.slots.map((slot) => (
            <div className="scenario-builder-slot" data-side={slot.side} key={slot.id}>
              <i aria-hidden="true" />
              <label>
                {slot.id} · {slot.units.length} units
                <select
                  aria-label={`${slot.id} controller`}
                  value={slot.controller}
                  onChange={(event) =>
                    setDraft((current) => ({
                      ...current,
                      slots: updateSlot(current.slots, slot.id, {
                        controller: event.target.value as BuilderController,
                      }),
                    }))
                  }
                >
                  {(slot.side === "neutral"
                    ? (["passive_ai"] as BuilderController[])
                    : slot.human_capable
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
          </section>
        )}
        <h2>Mode and victory</h2>
        <label>
          Mode
          <select
            value={draft.mode}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                mode: event.target.value as ScenarioBundleDraft["mode"],
              }))
            }
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
