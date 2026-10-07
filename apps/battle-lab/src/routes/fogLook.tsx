// /lab/fog-look: how unseen looks, its edge (a
// soft fade and a rim on the seen side), and the contact
// glyphs drawn over it, on the village street (`streetScenario.ts`). The panel
// picks one of the fixture's named styles, tunes every number of it live, and
// writes the `presentation.fog` block to copy back into the fixture. A 16:00
// sun lays long shadows beside the sight shadows, the case fog must never be
// mistaken for; two material glyph specimens, at infantry and vehicle
// uncertainty scales, stand beside the battle's published reports.
import { useMemo, useState } from "react";
import { ReadoutLayer } from "@web/battle/present/readouts";
import { buildContactGlyphs, type ContactShape } from "@packages/battle-renderer/src/contactGlyph";
import type { FogEye, FogInput } from "@packages/battle-renderer/src/frame/fogInputs";
import type { FogProbeInput } from "@packages/battle-renderer/src/frame/fogVisibility";
import {
  FOG_EDGE_REACH_PX,
  validateFogStyle,
  type FogPresentation,
  type FogStyle,
} from "@packages/battle-renderer/src/frame/fogStyle";
import { concatMeshes } from "@packages/battle-renderer/src/mesh";
import type { FrameView } from "@packages/battle-renderer/src/scene";
import type { LightPresentation } from "@packages/battle-renderer/src/light/sceneLight";
import game from "@fixtures/game.json";
import type { UnitCatalog } from "@packages/scene-assets/src/units";
import { contactLayer } from "../battleOverlay";
import { LabViewport } from "../LabViewport";
import { useBattleSession } from "../useBattleSession";
import { STREET_CAMERA, STREET_SEED, useStreetScenario } from "../streetScenario";
import { gameContactStyle, gameFogPresentation } from "../gameFog";
import { gameLight } from "../gameLight";
import { useFeed } from "../feed";
import { TickStatus } from "../TickStatus";

/** A 16:00 summer sun: lower than the fixture's, so shadows run long beside
 *  the sight shadows. */
const LOW_SUN_ELEVATION = 0.42;
type Sun = "fixture" | "low";
/** The light: the fixture's, or its 16:00 sun; without bloom for the
 *  scene's pixel identity check (bloom spreads unseen's dimming a little
 *  into seen pixels). */
const lightFor = (sun: Sun, bloom: boolean): LightPresentation => ({
  ...gameLight,
  sun_elevation: sun === "low" ? LOW_SUN_ELEVATION : gameLight.sun_elevation,
  bloom: bloom ? gameLight.bloom : { ...gameLight.bloom, strength: 0 },
});

/** A contact's radius as the simulation sizes it (`Unit::contact_radius`):
 *  the fixture's factor over its cause's catalog footprint, a hull's
 *  half-diagonal or half a full squad's spread plus a soldier's body. */
function contactRadius(units: UnitCatalog, kind: string): number {
  const hull = units.hull(kind);
  const m = game.infantry_movement;
  const footprint = hull
    ? Math.hypot(hull.half_extents_m[0], hull.half_extents_m[1])
    : (m.spread_m * Math.sqrt(units.slots(kind).length / m.spread_squad_size)) / 2 +
      game.physics.soldier_radius_m;
  return game.sensors.contact_radius_factor * footprint;
}

/** Material specimens at infantry and vehicle uncertainty scales. */
const specimensOf = (units: UnitCatalog): ContactShape[] => [
  {
    center: [1120, 930],
    radius: contactRadius(units, "rifle"),
    opacity: 1,
  },
  {
    center: [1260, 700],
    radius: contactRadius(units, "tank"),
    opacity: 0.6,
  },
];

/** A slider row: `label`, value, range and step. */
function Knob({
  label,
  value,
  min,
  max,
  step,
  onChange,
  id,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  onChange: (v: number) => void;
  id: string;
}) {
  return (
    <label className="lab-row" style={{ gap: 8 }}>
      <span style={{ width: 110 }}>{label}</span>
      <input
        type="range"
        data-testid={id}
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
      />
      <span style={{ width: 44, textAlign: "right" }}>{value}</span>
    </label>
  );
}

export default function FogLook() {
  const built = useStreetScenario("fog-look");
  if (built && typeof built !== "string")
    return <main className="lab-rejected">{built.error}</main>;
  if (!built) return null;
  return <FogLookLab scenario={built} />;
}

function FogLookLab({ scenario }: { scenario: string }) {
  const session = useBattleSession({ scenario, seed: STREET_SEED, destroyable: "apart" });
  const { meshes, sim, surfaceZ } = session;
  const specimenShapes = useMemo(() => specimensOf(session.units), [session.units]);
  const { observation } = sim;
  const [styles, setStyles] = useState<FogPresentation["styles"]>(() =>
    structuredClone(gameFogPresentation.styles),
  );
  const [name, setName] = useState(gameFogPresentation.style);
  const [sun, setSun] = useState<Sun>("low");
  const [bloom, setBloom] = useState(true);
  const [fogOn, setFogOn] = useState(true);
  /** Blue's whole sight, or the street recon's alone (ARMAPHRACT's wedge). */
  const [reconOnly, setReconOnly] = useState(false);
  /** Eyes placed by the scene in place of the side's (null: the side's). */
  const [eyes, setEyes] = useState<FogEye[] | null>(null);
  const [specimens, setSpecimens] = useState(true);
  /** Grass on the ground (off for the pixel-identity checks). */
  const [grass, setGrass] = useState(false);
  const [view, setView] = useState<FrameView>("final");
  const style = styles[name];

  const edit = (next: FogStyle) => {
    validateFogStyle(next);
    setStyles((all) => ({ ...all, [name]: next }));
  };
  const block = JSON.stringify({ fog: { style: name, styles } }, null, 2);

  const fog = useMemo<FogInput | null>(() => {
    if (!fogOn || !session.fog) return null;
    if (eyes) return { ...session.fog, sight: { ...session.fog.sight, eyes } };
    if (!reconOnly) return session.fog;
    const recon = observation?.own.find((u) => u.kind === "recon");
    const reconEyes = session.fog.sight.eyes.filter((e) => e.key.startsWith(`${recon?.id}:`));
    return { ...session.fog, sight: { ...session.fog.sight, eyes: reconEyes } };
  }, [fogOn, reconOnly, eyes, session.fog, observation]);
  const fogFeed = useFeed(fog);
  const overlay = useMemo(() => {
    const battle = contactLayer(session.contacts, surfaceZ);
    const shown = specimens ? buildContactGlyphs(specimenShapes, surfaceZ, gameContactStyle) : null;
    return {
      opaque: battle.opaque,
      translucent: concatMeshes(
        shown ? [battle.translucent, shown.translucent] : [battle.translucent],
      ),
    };
  }, [surfaceZ, specimens, specimenShapes, session.contacts]);
  const overlayFeed = useFeed(overlay);

  const show = (next: FrameView) => {
    setView(next);
    void window.__lab?.setFrameView?.(next);
  };
  const diagnostics = {
    ...session.probes,
    surfaceZ,
    /** Select a named style, or replace the selected one's numbers. */
    setStyle: (next: string | FogStyle) => (typeof next === "string" ? setName(next) : edit(next)),
    style: () => style,
    styles: () => Object.keys(styles),
    block: () => block,
    setFogOn,
    setReconOnly,
    setEyes,
    setSpecimens,
    setGrass,
    specimens: () => specimenShapes,
    showMask: (on: boolean) => show(on ? "fog-mask" : "final"),
    showWorld: (on: boolean) => show(on ? "world" : "final"),
    /** White where a pixel is mostly ground (terrain and grass), black elsewhere. */
    showGround: (on: boolean) => show(on ? "ground-mask" : "final"),
    /** Fog at surface points, with the roof rule (see fogTerm.ts). */
    probe: (points: FogProbeInput[]) => window.__lab!.fog!().probe(points),
    /** The structures that take fog whole, and which the last frame flagged seen. */
    wholes: async () => {
      const { boxes, seen } = await window.__lab!.fog!().wholes();
      return boxes.map((b, i) => ({ ...b, seen: seen[i] === 1 }));
    },
    sun: () => sun,
    /** Rebuilds the frame (the light is fixed for a frame's life). */
    setBloom,
  };

  // The fog look's rim and seen-pixel checks are measured on bare ground:
  // they count every pixel that differs between two captures, and dense
  // grass flips a stray pixel between captures now and then (a blade depth
  // tie). The gate frames draw the grass, as the village does.
  const bare = useMemo(
    () => meshes && (grass ? meshes : { ...meshes, grass: null }),
    [meshes, grass],
  );
  const worldFeed = useFeed(bare);
  if (!bare) return null;
  const set = <K extends keyof FogStyle>(key: K, value: FogStyle[K]) =>
    edit({ ...style, [key]: value });
  const setLine = <K extends keyof FogStyle["lines"]>(key: K, value: number) =>
    edit({ ...style, lines: { ...style.lines, [key]: value } });
  const setRim = <K extends "width_px" | "alpha">(key: K, value: number) =>
    edit({ ...style, rim: { ...style.rim, [key]: value } });
  const setRimColor = (i: number, v: number) => {
    const color = [...style.rim.color] as FogStyle["rim"]["color"];
    color[i] = v;
    edit({ ...style, rim: { ...style.rim, color } });
  };
  const setTint = (i: number, v: number) => {
    const tint = [...style.tint] as FogStyle["tint"];
    tint[i] = v;
    set("tint", tint);
  };
  return (
    <>
      <LabViewport
        key={`${sun}-${bloom}`}
        fixture="fog-look"
        world={worldFeed}
        structures={session.structures}
        obstacles={session.cameraObstaclesFeed}
        buildings={session.buildingsFeed}
        overlay={overlayFeed}
        fog={fogFeed}
        fogStyle={style}
        light={lightFor(sun, bloom)}
        frame={session.frame}
        appearances={session.appearances}
        initialCamera={STREET_CAMERA}
        groundAt={surfaceZ}
        onFrame={session.placePanels}
        onReady={session.onReady}
        diagnostics={diagnostics}
      />
      <ReadoutLayer
        own={[]}
        contacts={session.contacts}
        tick={observation?.tick}
        rules={session.rules}
        selected={[]}
        handle={session.readouts}
      />
      <aside
        className="hud-panel lab-panel"
        data-testid="fog-look-panel"
        data-occludes-readouts
        style={{ maxHeight: "96vh", overflow: "auto" }}
      >
        <strong>Unseen look</strong>
        <div className="lab-hint">
          Seen is the world as lit. Unseen takes the style below; the lines inside fog and the rim
          along its edge are the cues sun shadow never has.
        </div>
        <label className="lab-row">
          Style{" "}
          <select data-testid="fog-style" value={name} onChange={(e) => setName(e.target.value)}>
            {Object.keys(styles).map((s) => (
              <option key={s}>{s}</option>
            ))}
          </select>
        </label>
        <Knob
          id="dim"
          label="Dim"
          value={style.dim}
          min={0}
          max={1}
          step={0.01}
          onChange={(v) => set("dim", v)}
        />
        <Knob
          id="cool"
          label="Cool"
          value={style.cool}
          min={0}
          max={1}
          step={0.01}
          onChange={(v) => set("cool", v)}
        />
        <Knob
          id="saturation"
          label="Saturation"
          value={style.saturation}
          min={0}
          max={1}
          step={0.01}
          onChange={(v) => set("saturation", v)}
        />
        <Knob
          id="veil"
          label="Veil"
          value={style.veil}
          min={0}
          max={0.3}
          step={0.005}
          onChange={(v) => set("veil", v)}
        />
        {(["Tint red", "Tint green", "Tint blue"] as const).map((label, i) => (
          <Knob
            key={label}
            id={`tint-${i}`}
            label={label}
            value={style.tint[i]}
            min={0}
            max={2}
            step={0.01}
            onChange={(v) => setTint(i, v)}
          />
        ))}
        <Knob
          id="line-strength"
          label="Lines"
          value={style.lines.strength}
          min={-1}
          max={1}
          step={0.01}
          onChange={(v) => setLine("strength", v)}
        />
        <Knob
          id="line-floor"
          label="Line floor"
          value={style.lines.floor}
          min={0}
          max={0.1}
          step={0.001}
          onChange={(v) => setLine("floor", v)}
        />
        <Knob
          id="line-spacing"
          label="Line spacing px"
          value={style.lines.spacing_px}
          min={2}
          max={24}
          step={0.5}
          onChange={(v) => setLine("spacing_px", v)}
        />
        <Knob
          id="line-width"
          label="Line width px"
          value={style.lines.width_px}
          min={0}
          max={8}
          step={0.25}
          onChange={(v) => setLine("width_px", v)}
        />
        <Knob
          id="line-angle"
          label="Line angle°"
          value={style.lines.angle_deg}
          min={-90}
          max={90}
          step={5}
          onChange={(v) => setLine("angle_deg", v)}
        />
        <Knob
          id="edge-softness"
          label="Edge softness px"
          value={style.edge_softness}
          min={0}
          max={FOG_EDGE_REACH_PX}
          step={0.25}
          onChange={(v) => set("edge_softness", v)}
        />
        <Knob
          id="rim-width"
          label="Rim width px"
          value={style.rim.width_px}
          min={0}
          max={FOG_EDGE_REACH_PX}
          step={0.25}
          onChange={(v) => setRim("width_px", v)}
        />
        <Knob
          id="rim-alpha"
          label="Rim alpha"
          value={style.rim.alpha}
          min={0}
          max={1}
          step={0.01}
          onChange={(v) => setRim("alpha", v)}
        />
        {(["Rim red", "Rim green", "Rim blue"] as const).map((label, i) => (
          <Knob
            key={label}
            id={`rim-color-${i}`}
            label={label}
            value={style.rim.color[i]}
            min={0}
            max={1}
            step={0.01}
            onChange={(v) => setRimColor(i, v)}
          />
        ))}
        <div className="lab-row">
          <button type="button" aria-pressed={sun === "low"} onClick={() => setSun("low")}>
            16:00 sun
          </button>
          <button type="button" aria-pressed={sun === "fixture"} onClick={() => setSun("fixture")}>
            Fixture sun
          </button>
        </div>
        <div className="lab-row">
          <button type="button" aria-pressed={fogOn} onClick={() => setFogOn(!fogOn)}>
            Fog {fogOn ? "on" : "off"}
          </button>
          <button
            type="button"
            aria-pressed={view === "fog-mask"}
            onClick={() => show(view === "fog-mask" ? "final" : "fog-mask")}
          >
            Mask
          </button>
          <button type="button" aria-pressed={reconOnly} onClick={() => setReconOnly(!reconOnly)}>
            Recon's sight only
          </button>
          <button type="button" aria-pressed={grass} onClick={() => setGrass(!grass)}>
            Grass
          </button>
          <button type="button" aria-pressed={specimens} onClick={() => setSpecimens(!specimens)}>
            Specimen contacts
          </button>
        </div>
        <TickStatus tick={observation?.tick} status={sim.status.status} />
        <strong>Fixture block</strong>
        <div className="lab-hint">Paste over `presentation.fog` in fixtures/game.json.</div>
        <textarea
          data-testid="fog-block"
          readOnly
          value={block}
          rows={10}
          style={{ width: "100%", fontFamily: "monospace", fontSize: 11 }}
        />
        <button type="button" onClick={() => void navigator.clipboard?.writeText(block)}>
          Copy
        </button>
      </aside>
    </>
  );
}
